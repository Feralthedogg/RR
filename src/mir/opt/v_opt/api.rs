use super::analysis::{
    choose_call_map_lowering, estimate_loop_trip_count_hint, estimate_vector_plan_helper_cost,
    induction_origin_var, loop_vectorize_skip_reason,
};
use super::debug::{
    VectorizeSkipReason, proof_engine_enabled, trace_no_iv_context, vectorize_trace_enabled,
};
use super::planning::{Axis3D, ReduceKind, VectorPlan};
use super::proof;
use super::transform::apply_vectorization;
use super::types::MemoryStrideClass;
use super::types::{CallMapLoweringMode, ProofFallbackReason, ProofOutcome};
use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo};
use crate::mir::opt::poly::is_generated_poly_loop_var_name;
use crate::mir::{BlockId, FnIR};
use rustc_hash::FxHashSet;

mod candidates;
mod plan_counts;
mod proof_driver;
mod reachability;
mod stats;

use self::candidates::{collect_reduction_candidates, collect_vector_candidates};
use self::plan_counts::{
    VectorPlanFamily, record_plan_counts, vector_plan_call_map_lowering, vector_plan_family,
};
use self::proof_driver::{trace_proof_outcome, try_apply_proof_plan};
use self::reachability::reachable_blocks;
pub use self::stats::VOptStats;

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    optimize_with_stats(fn_ir).changed()
}

pub fn optimize_with_stats(fn_ir: &mut FnIR) -> VOptStats {
    optimize_with_stats_with_whitelist(fn_ir, &FxHashSet::default())
}

fn vectorization_disabled_from_env() -> bool {
    std::env::var("RR_DISABLE_VECTORIZE")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

pub fn optimize_with_stats_with_whitelist(
    fn_ir: &mut FnIR,
    user_call_whitelist: &FxHashSet<String>,
) -> VOptStats {
    let loops = LoopAnalyzer::new(fn_ir).find_loops();
    optimize_with_stats_with_whitelist_and_loops(fn_ir, user_call_whitelist, &loops)
}

pub fn optimize_with_stats_with_whitelist_and_loops(
    fn_ir: &mut FnIR,
    user_call_whitelist: &FxHashSet<String>,
    loops: &[LoopInfo],
) -> VOptStats {
    if vectorization_disabled_from_env() {
        return VOptStats::default();
    }
    let mut stats = VOptStats::default();
    let trace_enabled = vectorize_trace_enabled();
    let proof_enabled = proof_engine_enabled();
    let reachable = reachable_blocks(fn_ir);
    let loops = loops
        .iter()
        .filter(|lp| reachable.contains(&lp.header))
        .cloned()
        .collect::<Vec<_>>();

    for (loop_idx, lp) in loops.iter().enumerate() {
        stats.loops_seen += 1;
        stats.record_trip_tier(trip_count_tier(fn_ir, lp));
        let before = stats;
        if trace_enabled {
            let mut body_ids: Vec<BlockId> = lp.body.iter().copied().collect();
            body_ids.sort_unstable();
            let iv_origin = lp
                .iv
                .as_ref()
                .and_then(|iv| induction_origin_var(fn_ir, iv.phi_val));
            eprintln!(
                "   [vec-loop] {} header={} latch={} exits={:?} iv_origin={:?} limit={:?} limit_adjust={} body={:?}",
                fn_ir.name,
                lp.header,
                lp.latch,
                lp.exits,
                iv_origin,
                lp.limit.map(|v| &fn_ir.values[v].kind),
                lp.limit_adjust,
                body_ids
            );
        }
        if lp
            .iv
            .as_ref()
            .and_then(|iv| induction_origin_var(fn_ir, iv.phi_val))
            .as_deref()
            .is_some_and(is_generated_poly_loop_var_name)
        {
            if trace_enabled {
                eprintln!("   [vec-skip] {}: generated-poly-loop", fn_ir.name,);
            }
            stats.record_skip(VectorizeSkipReason::NoSupportedPattern);
            continue;
        }
        let has_nested_loop = loops.iter().enumerate().any(|(other_idx, other)| {
            other_idx != loop_idx
                && other.header != lp.header
                && lp.body.contains(&other.header)
                && other.body.len() < lp.body.len()
        });
        let proof_outcome = if has_nested_loop {
            ProofOutcome::NotApplicable {
                reason: ProofFallbackReason::UnsupportedLoopShape,
            }
        } else {
            proof::analyze_loop(fn_ir, lp, user_call_whitelist)
        };
        trace_proof_outcome(fn_ir, lp, &proof_outcome);
        if proof_enabled {
            match &proof_outcome {
                ProofOutcome::Certified(_) => stats.proof_certified += 1,
                ProofOutcome::NotApplicable { .. } => {}
                ProofOutcome::FallbackToPattern { reason } => {
                    stats.proof_fallback_pattern += 1;
                    stats.proof_fallback_reason_counts[reason.index()] += 1;
                }
            }
        }
        if try_apply_proof_plan(fn_ir, lp, &proof_outcome, &mut stats) {
            continue;
        }
        let mut reduction_candidates = collect_reduction_candidates(fn_ir, lp, user_call_whitelist);
        for plan in &reduction_candidates {
            record_plan_counts(&mut stats, fn_ir, plan, false);
        }
        rank_vector_plans_for_loop(fn_ir, lp, &mut reduction_candidates);
        let mut vector_candidates = collect_vector_candidates(fn_ir, lp, user_call_whitelist);
        for plan in &vector_candidates {
            record_plan_counts(&mut stats, fn_ir, plan, false);
        }
        rank_vector_plans_for_loop(fn_ir, lp, &mut vector_candidates);
        if trace_enabled && (!reduction_candidates.is_empty() || !vector_candidates.is_empty()) {
            let reductions = reduction_candidates
                .iter()
                .map(|plan| vector_plan_trace_label_with_lowering(fn_ir, plan))
                .collect::<Vec<_>>()
                .join(", ");
            let vectors = vector_candidates
                .iter()
                .map(|plan| vector_plan_trace_label_with_lowering(fn_ir, plan))
                .collect::<Vec<_>>()
                .join(", ");
            eprintln!(
                "   [vec-candidates] {} trip_hint={:?} reductions=[{}] vectors=[{}]",
                fn_ir.name,
                estimate_loop_trip_count_hint(fn_ir, lp),
                reductions,
                vectors
            );
        }

        if let Some(plan) = reduction_candidates
            .into_iter()
            .find(|plan| apply_vectorization(fn_ir, lp, plan.clone()))
        {
            record_plan_counts(&mut stats, fn_ir, &plan, true);
            if trace_enabled {
                eprintln!(
                    "   [vec-choose] {} reduction={}",
                    fn_ir.name,
                    vector_plan_trace_label_with_lowering(fn_ir, &plan)
                );
            }
            stats.reduced += 1;
        } else if let Some(plan) = vector_candidates
            .into_iter()
            .find(|plan| apply_vectorization(fn_ir, lp, plan.clone()))
        {
            record_plan_counts(&mut stats, fn_ir, &plan, true);
            if trace_enabled {
                eprintln!(
                    "   [vec-choose] {} vector={}",
                    fn_ir.name,
                    vector_plan_trace_label_with_lowering(fn_ir, &plan)
                );
            }
            stats.vectorized += 1;
        }

        let applied = stats.vectorized != before.vectorized || stats.reduced != before.reduced;
        if trace_enabled && !applied {
            let reason = loop_vectorize_skip_reason(fn_ir, lp);
            eprintln!("   [vec-skip] {}: {}", fn_ir.name, reason.label());
            if reason == VectorizeSkipReason::NoIv {
                trace_no_iv_context(fn_ir, lp);
            }
        }
        if !applied {
            stats.record_skip(loop_vectorize_skip_reason(fn_ir, lp));
        }
    }

    stats
}

fn vector_plan_trace_label_with_lowering(fn_ir: &FnIR, plan: &VectorPlan) -> String {
    let base = vector_plan_label(plan);
    match vector_plan_call_map_lowering(fn_ir, plan) {
        Some(CallMapLoweringMode::DirectVector) => format!("{base}[direct]"),
        Some(CallMapLoweringMode::RuntimeAuto { helper_cost }) => {
            format!("{base}[runtime:{helper_cost}]")
        }
        None => base.to_string(),
    }
}

pub(super) fn rank_vector_plans(plans: &mut [VectorPlan]) {
    plans.sort_by_key(|plan| std::cmp::Reverse(vector_plan_score(plan)));
}

pub(super) fn rank_vector_plans_for_loop(fn_ir: &FnIR, lp: &LoopInfo, plans: &mut [VectorPlan]) {
    plans.sort_by_key(|plan| {
        std::cmp::Reverse((
            vector_plan_profit_tier(fn_ir, lp, plan),
            vector_plan_score(plan),
        ))
    });
}

fn trip_count_tier(fn_ir: &FnIR, lp: &LoopInfo) -> u8 {
    match estimate_loop_trip_count_hint(fn_ir, lp) {
        Some(0..=4) => 0,
        Some(5..=16) => 1,
        Some(17..=64) => 2,
        Some(_) => 3,
        None => 2,
    }
}

fn vector_plan_base_tier(plan: &VectorPlan) -> u8 {
    match plan {
        VectorPlan::Reduce2DRowSum { .. }
        | VectorPlan::Reduce2DColSum { .. }
        | VectorPlan::Reduce3D { .. } => 6,
        VectorPlan::Reduce { .. }
        | VectorPlan::ReduceCond { .. }
        | VectorPlan::MultiReduceCond { .. } => 5,
        VectorPlan::Map { .. }
        | VectorPlan::RecurrenceAddConst { .. }
        | VectorPlan::RecurrenceAddConst3D { .. } => 4,
        VectorPlan::CondMap { whole_dest, .. } | VectorPlan::ExprMap { whole_dest, .. } => {
            if *whole_dest {
                4
            } else {
                3
            }
        }
        VectorPlan::CondMap3D { .. } => 3,
        VectorPlan::CondMap3DGeneral { .. } => 3,
        VectorPlan::CallMap3D { .. } => 3,
        VectorPlan::CallMap3DGeneral { .. } => 3,
        VectorPlan::ExprMap3D { .. } => 3,
        VectorPlan::CallMap { whole_dest, .. } => 3 + u8::from(*whole_dest),
        VectorPlan::MultiExprMap3D { entries, .. } => 2 + u8::from(entries.len() > 1),
        VectorPlan::MultiExprMap { entries, .. } => 2 + u8::from(entries.len() > 1),
        VectorPlan::Map2DRow { .. } | VectorPlan::Map2DCol { .. } => 2,
        VectorPlan::Map3D { .. } => 2,
        VectorPlan::CubeSliceExprMap { .. } => 2,
        VectorPlan::ShiftedMap { .. } => 2,
        VectorPlan::ShiftedMap3D { .. } => 4,
        VectorPlan::ScatterExprMap { .. } => 1,
        VectorPlan::ScatterExprMap3D { .. } => 1,
        VectorPlan::ScatterExprMap3DGeneral { .. } => 1,
    }
}

fn vector_plan_stride_class(plan: &VectorPlan) -> MemoryStrideClass {
    match plan {
        VectorPlan::Reduce2DRowSum { .. } | VectorPlan::Map2DRow { .. } => {
            MemoryStrideClass::Strided
        }
        VectorPlan::Reduce2DColSum { .. } | VectorPlan::Map2DCol { .. } => {
            MemoryStrideClass::Contiguous
        }
        VectorPlan::Reduce3D { axis, .. }
        | VectorPlan::Map3D { axis, .. }
        | VectorPlan::CallMap3D { axis, .. }
        | VectorPlan::CallMap3DGeneral { axis, .. }
        | VectorPlan::ExprMap3D { axis, .. }
        | VectorPlan::CondMap3D { axis, .. }
        | VectorPlan::CondMap3DGeneral { axis, .. }
        | VectorPlan::ShiftedMap3D { axis, .. }
        | VectorPlan::RecurrenceAddConst3D { axis, .. }
        | VectorPlan::ScatterExprMap3D { axis, .. } => match axis {
            Axis3D::Dim1 => MemoryStrideClass::Contiguous,
            Axis3D::Dim2 | Axis3D::Dim3 => MemoryStrideClass::Strided,
        },
        _ => MemoryStrideClass::Contiguous,
    }
}

pub(super) fn vector_plan_profit_tier(fn_ir: &FnIR, lp: &LoopInfo, plan: &VectorPlan) -> u8 {
    let base = vector_plan_base_tier(plan);
    let trip = trip_count_tier(fn_ir, lp);
    let helper_penalty = (estimate_vector_plan_helper_cost(fn_ir, plan) / 6).min(3) as u8;
    let stride_penalty = u8::from(matches!(
        vector_plan_stride_class(plan),
        MemoryStrideClass::Strided
    ));
    match plan {
        VectorPlan::CallMap {
            callee,
            args,
            whole_dest,
            shadow_vars,
            ..
        } => match choose_call_map_lowering(fn_ir, callee, args, *whole_dest, shadow_vars) {
            CallMapLoweringMode::DirectVector => {
                base + trip.saturating_sub(helper_penalty.saturating_add(stride_penalty))
            }
            CallMapLoweringMode::RuntimeAuto { .. } => {
                (base.saturating_sub(1))
                    + trip.saturating_sub(helper_penalty.saturating_add(stride_penalty))
            }
        },
        _ => base + trip.saturating_sub(helper_penalty.saturating_add(stride_penalty)),
    }
}

pub(super) fn vector_plan_score(plan: &VectorPlan) -> (u8, u8, u8) {
    match plan {
        VectorPlan::Reduce2DColSum { .. } => (120, 0, 0),
        VectorPlan::Reduce2DRowSum { .. } => (119, 0, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Sum,
            axis: Axis3D::Dim1,
            ..
        } => (118, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Sum,
            axis: Axis3D::Dim2,
            ..
        } => (117, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Sum,
            axis: Axis3D::Dim3,
            ..
        } => (116, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Prod,
            axis: Axis3D::Dim1,
            ..
        } => (115, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Prod,
            axis: Axis3D::Dim2,
            ..
        } => (114, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Prod,
            axis: Axis3D::Dim3,
            ..
        } => (113, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Min,
            axis: Axis3D::Dim1,
            ..
        } => (112, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Min,
            axis: Axis3D::Dim2,
            ..
        } => (111, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Min,
            axis: Axis3D::Dim3,
            ..
        } => (110, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Max,
            axis: Axis3D::Dim1,
            ..
        } => (109, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Max,
            axis: Axis3D::Dim2,
            ..
        } => (108, 1, 0),
        VectorPlan::Reduce3D {
            kind: ReduceKind::Max,
            axis: Axis3D::Dim3,
            ..
        } => (107, 1, 0),
        VectorPlan::ReduceCond { .. } => (119, 0, 0),
        VectorPlan::MultiReduceCond { entries, .. } => (119, entries.len().min(255) as u8, 0),
        VectorPlan::Reduce { .. } => (118, 0, 0),
        VectorPlan::CondMap { .. } => (100, 0, 0),
        VectorPlan::CondMap3D {
            axis: Axis3D::Dim1, ..
        } => (89, 0, 0),
        VectorPlan::CondMap3D {
            axis: Axis3D::Dim2, ..
        } => (88, 0, 0),
        VectorPlan::CondMap3D {
            axis: Axis3D::Dim3, ..
        } => (87, 0, 0),
        VectorPlan::CondMap3DGeneral {
            axis: Axis3D::Dim1, ..
        } => (86, 0, 0),
        VectorPlan::CondMap3DGeneral {
            axis: Axis3D::Dim2, ..
        } => (85, 0, 0),
        VectorPlan::CondMap3DGeneral {
            axis: Axis3D::Dim3, ..
        } => (84, 0, 0),
        VectorPlan::RecurrenceAddConst3D {
            axis: Axis3D::Dim1, ..
        } => (99, 0, 0),
        VectorPlan::RecurrenceAddConst3D {
            axis: Axis3D::Dim2, ..
        } => (98, 0, 0),
        VectorPlan::RecurrenceAddConst3D {
            axis: Axis3D::Dim3, ..
        } => (97, 0, 0),
        VectorPlan::ShiftedMap3D {
            axis: Axis3D::Dim1,
            offset,
            ..
        } => (
            96,
            0,
            u8::MAX.saturating_sub(offset.unsigned_abs().min(u8::MAX as u64) as u8),
        ),
        VectorPlan::ShiftedMap3D {
            axis: Axis3D::Dim2,
            offset,
            ..
        } => (
            95,
            0,
            u8::MAX.saturating_sub(offset.unsigned_abs().min(u8::MAX as u64) as u8),
        ),
        VectorPlan::ShiftedMap3D {
            axis: Axis3D::Dim3,
            offset,
            ..
        } => (
            94,
            0,
            u8::MAX.saturating_sub(offset.unsigned_abs().min(u8::MAX as u64) as u8),
        ),
        VectorPlan::ShiftedMap { offset, .. } => (
            98,
            0,
            u8::MAX.saturating_sub(offset.unsigned_abs().min(u8::MAX as u64) as u8),
        ),
        VectorPlan::CubeSliceExprMap { .. } => (96, 0, 0),
        VectorPlan::ScatterExprMap { .. } => (94, 0, 0),
        VectorPlan::ScatterExprMap3D {
            axis: Axis3D::Dim1, ..
        } => (93, 0, 0),
        VectorPlan::ScatterExprMap3D {
            axis: Axis3D::Dim2, ..
        } => (92, 0, 0),
        VectorPlan::ScatterExprMap3D {
            axis: Axis3D::Dim3, ..
        } => (91, 0, 0),
        VectorPlan::ScatterExprMap3DGeneral { .. } => (90, 0, 0),
        VectorPlan::MultiExprMap3D { entries, .. } => {
            (89, entries.len().min(u8::MAX as usize) as u8, 0)
        }
        VectorPlan::MultiExprMap { entries, .. } => {
            (92, entries.len().min(u8::MAX as usize) as u8, 0)
        }
        VectorPlan::ExprMap { whole_dest, .. } => (90, u8::from(*whole_dest), 0),
        VectorPlan::CallMap { args, .. } => (86, 0, u8::MAX.saturating_sub(args.len() as u8)),
        VectorPlan::CallMap3D {
            axis: Axis3D::Dim1, ..
        } => (85, 0, 0),
        VectorPlan::CallMap3D {
            axis: Axis3D::Dim2, ..
        } => (84, 0, 0),
        VectorPlan::CallMap3D {
            axis: Axis3D::Dim3, ..
        } => (83, 0, 0),
        VectorPlan::CallMap3DGeneral {
            axis: Axis3D::Dim1, ..
        } => (82, 0, 0),
        VectorPlan::CallMap3DGeneral {
            axis: Axis3D::Dim2, ..
        } => (81, 0, 0),
        VectorPlan::CallMap3DGeneral {
            axis: Axis3D::Dim3, ..
        } => (80, 0, 0),
        VectorPlan::ExprMap3D {
            axis: Axis3D::Dim1, ..
        } => (79, 1, 0),
        VectorPlan::ExprMap3D {
            axis: Axis3D::Dim2, ..
        } => (78, 1, 0),
        VectorPlan::ExprMap3D {
            axis: Axis3D::Dim3, ..
        } => (77, 1, 0),
        VectorPlan::Map2DCol { .. } => (82, 0, 0),
        VectorPlan::Map2DRow { .. } => (81, 0, 0),
        VectorPlan::Map3D {
            axis: Axis3D::Dim1, ..
        } => (80, 0, 0),
        VectorPlan::Map3D {
            axis: Axis3D::Dim2, ..
        } => (79, 0, 0),
        VectorPlan::Map3D {
            axis: Axis3D::Dim3, ..
        } => (78, 0, 0),
        VectorPlan::RecurrenceAddConst { .. } => (80, 0, 0),
        VectorPlan::Map { .. } => (70, 0, 0),
    }
}

pub(super) fn vector_plan_label(plan: &VectorPlan) -> &'static str {
    match plan {
        VectorPlan::ReduceCond { .. } => "reduce_cond",
        VectorPlan::MultiReduceCond { .. } => "multi_reduce_cond",
        VectorPlan::Reduce { .. } => "reduce",
        VectorPlan::Reduce2DRowSum { .. } => "reduce2d_row_sum",
        VectorPlan::Reduce2DColSum { .. } => "reduce2d_col_sum",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Sum,
            axis: Axis3D::Dim1,
            ..
        } => "reduce3d_dim1_sum",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Sum,
            axis: Axis3D::Dim2,
            ..
        } => "reduce3d_dim2_sum",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Sum,
            axis: Axis3D::Dim3,
            ..
        } => "reduce3d_dim3_sum",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Prod,
            axis: Axis3D::Dim1,
            ..
        } => "reduce3d_dim1_prod",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Prod,
            axis: Axis3D::Dim2,
            ..
        } => "reduce3d_dim2_prod",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Prod,
            axis: Axis3D::Dim3,
            ..
        } => "reduce3d_dim3_prod",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Min,
            axis: Axis3D::Dim1,
            ..
        } => "reduce3d_dim1_min",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Min,
            axis: Axis3D::Dim2,
            ..
        } => "reduce3d_dim2_min",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Min,
            axis: Axis3D::Dim3,
            ..
        } => "reduce3d_dim3_min",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Max,
            axis: Axis3D::Dim1,
            ..
        } => "reduce3d_dim1_max",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Max,
            axis: Axis3D::Dim2,
            ..
        } => "reduce3d_dim2_max",
        VectorPlan::Reduce3D {
            kind: ReduceKind::Max,
            axis: Axis3D::Dim3,
            ..
        } => "reduce3d_dim3_max",
        VectorPlan::Map { .. } => "map",
        VectorPlan::CondMap { .. } => "cond_map",
        VectorPlan::CondMap3D {
            axis: Axis3D::Dim1, ..
        } => "cond_map3d_dim1",
        VectorPlan::CondMap3D {
            axis: Axis3D::Dim2, ..
        } => "cond_map3d_dim2",
        VectorPlan::CondMap3D {
            axis: Axis3D::Dim3, ..
        } => "cond_map3d_dim3",
        VectorPlan::CondMap3DGeneral {
            axis: Axis3D::Dim1, ..
        } => "cond_map3d_general_dim1",
        VectorPlan::CondMap3DGeneral {
            axis: Axis3D::Dim2, ..
        } => "cond_map3d_general_dim2",
        VectorPlan::CondMap3DGeneral {
            axis: Axis3D::Dim3, ..
        } => "cond_map3d_general_dim3",
        VectorPlan::RecurrenceAddConst { .. } => "recurrence_add_const",
        VectorPlan::RecurrenceAddConst3D {
            axis: Axis3D::Dim1, ..
        } => "recurrence_add_const3d_dim1",
        VectorPlan::RecurrenceAddConst3D {
            axis: Axis3D::Dim2, ..
        } => "recurrence_add_const3d_dim2",
        VectorPlan::RecurrenceAddConst3D {
            axis: Axis3D::Dim3, ..
        } => "recurrence_add_const3d_dim3",
        VectorPlan::ShiftedMap { .. } => "shifted_map",
        VectorPlan::ShiftedMap3D {
            axis: Axis3D::Dim1, ..
        } => "shifted_map3d_dim1",
        VectorPlan::ShiftedMap3D {
            axis: Axis3D::Dim2, ..
        } => "shifted_map3d_dim2",
        VectorPlan::ShiftedMap3D {
            axis: Axis3D::Dim3, ..
        } => "shifted_map3d_dim3",
        VectorPlan::CallMap { .. } => "call_map",
        VectorPlan::CallMap3D {
            axis: Axis3D::Dim1, ..
        } => "call_map3d_dim1",
        VectorPlan::CallMap3D {
            axis: Axis3D::Dim2, ..
        } => "call_map3d_dim2",
        VectorPlan::CallMap3D {
            axis: Axis3D::Dim3, ..
        } => "call_map3d_dim3",
        VectorPlan::CallMap3DGeneral {
            axis: Axis3D::Dim1, ..
        } => "call_map3d_general_dim1",
        VectorPlan::CallMap3DGeneral {
            axis: Axis3D::Dim2, ..
        } => "call_map3d_general_dim2",
        VectorPlan::CallMap3DGeneral {
            axis: Axis3D::Dim3, ..
        } => "call_map3d_general_dim3",
        VectorPlan::MultiExprMap3D { .. } => "multi_expr_map3d",
        VectorPlan::ExprMap3D {
            axis: Axis3D::Dim1, ..
        } => "expr_map3d_dim1",
        VectorPlan::ExprMap3D {
            axis: Axis3D::Dim2, ..
        } => "expr_map3d_dim2",
        VectorPlan::ExprMap3D {
            axis: Axis3D::Dim3, ..
        } => "expr_map3d_dim3",
        VectorPlan::CubeSliceExprMap { .. } => "cube_slice_expr_map",
        VectorPlan::ExprMap { .. } => "expr_map",
        VectorPlan::MultiExprMap { .. } => "multi_expr_map",
        VectorPlan::ScatterExprMap { .. } => "scatter_expr_map",
        VectorPlan::ScatterExprMap3D {
            axis: Axis3D::Dim1, ..
        } => "scatter_expr_map3d_dim1",
        VectorPlan::ScatterExprMap3D {
            axis: Axis3D::Dim2, ..
        } => "scatter_expr_map3d_dim2",
        VectorPlan::ScatterExprMap3D {
            axis: Axis3D::Dim3, ..
        } => "scatter_expr_map3d_dim3",
        VectorPlan::ScatterExprMap3DGeneral { .. } => "scatter_expr_map3d_general",
        VectorPlan::Map2DRow { .. } => "map2d_row",
        VectorPlan::Map2DCol { .. } => "map2d_col",
        VectorPlan::Map3D {
            axis: Axis3D::Dim1, ..
        } => "map3d_dim1",
        VectorPlan::Map3D {
            axis: Axis3D::Dim2, ..
        } => "map3d_dim2",
        VectorPlan::Map3D {
            axis: Axis3D::Dim3, ..
        } => "map3d_dim3",
    }
}
