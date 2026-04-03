use super::analysis::{
    choose_call_map_lowering, estimate_loop_trip_count_hint, estimate_vector_plan_helper_cost,
    induction_origin_var, loop_vectorize_skip_reason, match_2d_col_map, match_2d_row_map,
    match_3d_axis_map,
};
use super::debug::{
    VectorizeSkipReason, proof_engine_enabled, trace_no_iv_context, trace_proof_apply_result,
    vectorize_trace_enabled,
};
use super::planning::{
    Axis3D, ReduceKind, VectorPlan, match_2d_col_reduction_sum, match_2d_row_reduction_sum,
    match_3d_axis_reduction, match_call_map, match_call_map_3d, match_conditional_map,
    match_conditional_map_3d, match_cube_slice_expr_map, match_expr_map, match_expr_map_3d,
    match_map, match_multi_expr_map_3d, match_recurrence_add_const, match_recurrence_add_const_3d,
    match_reduction, match_scatter_expr_map, match_scatter_expr_map_3d, match_shifted_map,
    match_shifted_map_3d,
};
use super::proof;
use super::transform::{apply_vectorization, try_apply_vectorization_transactionally};
use super::types::MemoryStrideClass;
use super::types::{CallMapLoweringMode, ProofFallbackReason, ProofOutcome};
use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo};
use crate::mir::opt::poly::is_generated_poly_loop_var_name;
use crate::mir::{BlockId, FnIR, Terminator};
use rustc_hash::FxHashSet;

#[derive(Debug, Default, Clone, Copy)]
pub struct VOptStats {
    pub vectorized: usize,
    pub reduced: usize,
    pub loops_seen: usize,
    pub skipped: usize,
    pub skip_no_iv: usize,
    pub skip_non_canonical_bound: usize,
    pub skip_unsupported_cfg_shape: usize,
    pub skip_indirect_index_access: usize,
    pub skip_store_effects: usize,
    pub skip_no_supported_pattern: usize,
    pub candidate_total: usize,
    pub candidate_reductions: usize,
    pub candidate_conditionals: usize,
    pub candidate_recurrences: usize,
    pub candidate_shifted: usize,
    pub candidate_call_maps: usize,
    pub candidate_expr_maps: usize,
    pub candidate_scatters: usize,
    pub candidate_cube_slices: usize,
    pub candidate_basic_maps: usize,
    pub candidate_multi_exprs: usize,
    pub candidate_2d: usize,
    pub candidate_3d: usize,
    pub candidate_call_map_direct: usize,
    pub candidate_call_map_runtime: usize,
    pub applied_total: usize,
    pub applied_reductions: usize,
    pub applied_conditionals: usize,
    pub applied_recurrences: usize,
    pub applied_shifted: usize,
    pub applied_call_maps: usize,
    pub applied_expr_maps: usize,
    pub applied_scatters: usize,
    pub applied_cube_slices: usize,
    pub applied_basic_maps: usize,
    pub applied_multi_exprs: usize,
    pub applied_2d: usize,
    pub applied_3d: usize,
    pub applied_call_map_direct: usize,
    pub applied_call_map_runtime: usize,
    pub legacy_poly_fallback_candidate_total: usize,
    pub legacy_poly_fallback_candidate_reductions: usize,
    pub legacy_poly_fallback_candidate_maps: usize,
    pub legacy_poly_fallback_applied_total: usize,
    pub legacy_poly_fallback_applied_reductions: usize,
    pub legacy_poly_fallback_applied_maps: usize,
    pub trip_tier_tiny: usize,
    pub trip_tier_small: usize,
    pub trip_tier_medium: usize,
    pub trip_tier_large: usize,
    pub proof_certified: usize,
    pub proof_applied: usize,
    pub proof_apply_failed: usize,
    pub proof_fallback_pattern: usize,
    pub proof_fallback_reason_counts: [usize; super::PROOF_FALLBACK_REASON_COUNT],
}

impl VOptStats {
    pub fn changed(self) -> bool {
        self.vectorized > 0 || self.reduced > 0
    }

    fn record_skip(&mut self, reason: VectorizeSkipReason) {
        self.skipped += 1;
        match reason {
            VectorizeSkipReason::NoIv => self.skip_no_iv += 1,
            VectorizeSkipReason::NonCanonicalBound => self.skip_non_canonical_bound += 1,
            VectorizeSkipReason::UnsupportedCfgShape => self.skip_unsupported_cfg_shape += 1,
            VectorizeSkipReason::IndirectIndexAccess => self.skip_indirect_index_access += 1,
            VectorizeSkipReason::StoreEffects => self.skip_store_effects += 1,
            VectorizeSkipReason::NoSupportedPattern => self.skip_no_supported_pattern += 1,
        }
    }

    fn record_trip_tier(&mut self, tier: u8) {
        match tier {
            0 => self.trip_tier_tiny += 1,
            1 => self.trip_tier_small += 1,
            2 => self.trip_tier_medium += 1,
            _ => self.trip_tier_large += 1,
        }
    }
}

#[derive(Clone, Copy)]
enum VectorPlanFamily {
    Reduction,
    Conditional,
    Recurrence,
    Shifted,
    CallMap,
    ExprMap,
    Scatter,
    CubeSlice,
    BasicMap,
    MultiExpr,
}

#[derive(Clone, Copy)]
enum VectorPlanShape {
    General,
    TwoD,
    ThreeD,
}

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
    if vectorization_disabled_from_env() {
        return VOptStats::default();
    }
    let mut stats = VOptStats::default();
    let trace_enabled = vectorize_trace_enabled();
    let proof_enabled = proof_engine_enabled();
    let analyzer = LoopAnalyzer::new(fn_ir);
    let reachable = reachable_blocks(fn_ir);
    let loops = analyzer
        .find_loops()
        .into_iter()
        .filter(|lp| reachable.contains(&lp.header))
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

fn reachable_blocks(fn_ir: &FnIR) -> FxHashSet<BlockId> {
    let mut reachable = FxHashSet::default();
    let mut stack = vec![fn_ir.entry];
    reachable.insert(fn_ir.entry);

    while let Some(bb) = stack.pop() {
        match &fn_ir.blocks[bb].term {
            Terminator::Goto(target) => {
                if reachable.insert(*target) {
                    stack.push(*target);
                }
            }
            Terminator::If {
                then_bb, else_bb, ..
            } => {
                if reachable.insert(*then_bb) {
                    stack.push(*then_bb);
                }
                if reachable.insert(*else_bb) {
                    stack.push(*else_bb);
                }
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }

    reachable
}

fn trace_proof_outcome(fn_ir: &FnIR, lp: &LoopInfo, outcome: &ProofOutcome) {
    match outcome {
        ProofOutcome::Certified(certified) => super::debug::trace_proof_status(
            fn_ir,
            lp,
            &format!(
                "certified={} attempting-transactional-apply",
                vector_plan_label(&certified.plan)
            ),
        ),
        ProofOutcome::NotApplicable { reason } => super::debug::trace_proof_status(
            fn_ir,
            lp,
            &format!("not-applicable: {}", reason.label()),
        ),
        ProofOutcome::FallbackToPattern { reason } => super::debug::trace_proof_status(
            fn_ir,
            lp,
            &format!("fallback-pattern: {}", reason.label()),
        ),
    }
}

fn try_apply_proof_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    outcome: &ProofOutcome,
    stats: &mut VOptStats,
) -> bool {
    let ProofOutcome::Certified(certified) = outcome else {
        return false;
    };

    let plan = certified.plan.clone();
    let applied = try_apply_vectorization_transactionally(fn_ir, lp, plan.clone());
    trace_proof_apply_result(
        fn_ir,
        lp,
        applied,
        &format!("plan={}", vector_plan_label(&plan)),
    );
    if !applied {
        stats.proof_apply_failed += 1;
        return false;
    }

    stats.proof_applied += 1;
    record_plan_counts(stats, fn_ir, &plan, true);
    if matches!(vector_plan_family(&plan), VectorPlanFamily::Reduction) {
        stats.reduced += 1;
    } else {
        stats.vectorized += 1;
    }
    true
}

fn vector_plan_family(plan: &VectorPlan) -> VectorPlanFamily {
    match plan {
        VectorPlan::Reduce { .. }
        | VectorPlan::ReduceCond { .. }
        | VectorPlan::MultiReduceCond { .. }
        | VectorPlan::Reduce2DRowSum { .. }
        | VectorPlan::Reduce2DColSum { .. }
        | VectorPlan::Reduce3D { .. } => VectorPlanFamily::Reduction,
        VectorPlan::CondMap { .. }
        | VectorPlan::CondMap3D { .. }
        | VectorPlan::CondMap3DGeneral { .. } => VectorPlanFamily::Conditional,
        VectorPlan::RecurrenceAddConst { .. } | VectorPlan::RecurrenceAddConst3D { .. } => {
            VectorPlanFamily::Recurrence
        }
        VectorPlan::ShiftedMap { .. } | VectorPlan::ShiftedMap3D { .. } => {
            VectorPlanFamily::Shifted
        }
        VectorPlan::CallMap { .. }
        | VectorPlan::CallMap3D { .. }
        | VectorPlan::CallMap3DGeneral { .. } => VectorPlanFamily::CallMap,
        VectorPlan::ExprMap { .. } | VectorPlan::ExprMap3D { .. } => VectorPlanFamily::ExprMap,
        VectorPlan::ScatterExprMap { .. }
        | VectorPlan::ScatterExprMap3D { .. }
        | VectorPlan::ScatterExprMap3DGeneral { .. } => VectorPlanFamily::Scatter,
        VectorPlan::CubeSliceExprMap { .. } => VectorPlanFamily::CubeSlice,
        VectorPlan::Map { .. }
        | VectorPlan::Map2DRow { .. }
        | VectorPlan::Map2DCol { .. }
        | VectorPlan::Map3D { .. } => VectorPlanFamily::BasicMap,
        VectorPlan::MultiExprMap { .. } | VectorPlan::MultiExprMap3D { .. } => {
            VectorPlanFamily::MultiExpr
        }
    }
}

fn vector_plan_shape(plan: &VectorPlan) -> VectorPlanShape {
    match plan {
        VectorPlan::Reduce2DRowSum { .. }
        | VectorPlan::Reduce2DColSum { .. }
        | VectorPlan::Map2DRow { .. }
        | VectorPlan::Map2DCol { .. } => VectorPlanShape::TwoD,
        VectorPlan::Reduce3D { .. }
        | VectorPlan::CondMap3D { .. }
        | VectorPlan::CondMap3DGeneral { .. }
        | VectorPlan::RecurrenceAddConst3D { .. }
        | VectorPlan::ShiftedMap3D { .. }
        | VectorPlan::CallMap3D { .. }
        | VectorPlan::CallMap3DGeneral { .. }
        | VectorPlan::CubeSliceExprMap { .. }
        | VectorPlan::ExprMap3D { .. }
        | VectorPlan::MultiExprMap3D { .. }
        | VectorPlan::ScatterExprMap3D { .. }
        | VectorPlan::ScatterExprMap3DGeneral { .. }
        | VectorPlan::Map3D { .. } => VectorPlanShape::ThreeD,
        _ => VectorPlanShape::General,
    }
}

fn vector_plan_call_map_lowering(fn_ir: &FnIR, plan: &VectorPlan) -> Option<CallMapLoweringMode> {
    match plan {
        VectorPlan::CallMap {
            callee,
            args,
            whole_dest,
            shadow_vars,
            ..
        } => Some(choose_call_map_lowering(
            fn_ir,
            callee,
            args,
            *whole_dest,
            shadow_vars,
        )),
        _ => None,
    }
}

fn record_plan_counts(stats: &mut VOptStats, fn_ir: &FnIR, plan: &VectorPlan, applied: bool) {
    let (
        total,
        reductions,
        conditionals,
        recurrences,
        shifted,
        call_maps,
        expr_maps,
        scatters,
        cube_slices,
        basic_maps,
        multi_exprs,
        dims_2d,
        dims_3d,
        callmap_direct,
        callmap_runtime,
    ) = if applied {
        (
            &mut stats.applied_total,
            &mut stats.applied_reductions,
            &mut stats.applied_conditionals,
            &mut stats.applied_recurrences,
            &mut stats.applied_shifted,
            &mut stats.applied_call_maps,
            &mut stats.applied_expr_maps,
            &mut stats.applied_scatters,
            &mut stats.applied_cube_slices,
            &mut stats.applied_basic_maps,
            &mut stats.applied_multi_exprs,
            &mut stats.applied_2d,
            &mut stats.applied_3d,
            &mut stats.applied_call_map_direct,
            &mut stats.applied_call_map_runtime,
        )
    } else {
        (
            &mut stats.candidate_total,
            &mut stats.candidate_reductions,
            &mut stats.candidate_conditionals,
            &mut stats.candidate_recurrences,
            &mut stats.candidate_shifted,
            &mut stats.candidate_call_maps,
            &mut stats.candidate_expr_maps,
            &mut stats.candidate_scatters,
            &mut stats.candidate_cube_slices,
            &mut stats.candidate_basic_maps,
            &mut stats.candidate_multi_exprs,
            &mut stats.candidate_2d,
            &mut stats.candidate_3d,
            &mut stats.candidate_call_map_direct,
            &mut stats.candidate_call_map_runtime,
        )
    };

    *total += 1;
    match vector_plan_family(plan) {
        VectorPlanFamily::Reduction => *reductions += 1,
        VectorPlanFamily::Conditional => *conditionals += 1,
        VectorPlanFamily::Recurrence => *recurrences += 1,
        VectorPlanFamily::Shifted => *shifted += 1,
        VectorPlanFamily::CallMap => *call_maps += 1,
        VectorPlanFamily::ExprMap => *expr_maps += 1,
        VectorPlanFamily::Scatter => *scatters += 1,
        VectorPlanFamily::CubeSlice => *cube_slices += 1,
        VectorPlanFamily::BasicMap => *basic_maps += 1,
        VectorPlanFamily::MultiExpr => *multi_exprs += 1,
    }
    match vector_plan_shape(plan) {
        VectorPlanShape::TwoD => *dims_2d += 1,
        VectorPlanShape::ThreeD => *dims_3d += 1,
        VectorPlanShape::General => {}
    }
    if let Some(lowering) = vector_plan_call_map_lowering(fn_ir, plan) {
        match lowering {
            CallMapLoweringMode::DirectVector => *callmap_direct += 1,
            CallMapLoweringMode::RuntimeAuto { .. } => *callmap_runtime += 1,
        }
    }

    record_legacy_poly_fallback_counts(stats, plan, applied);
}

fn is_legacy_poly_fallback_plan(plan: &VectorPlan) -> bool {
    matches!(
        plan,
        VectorPlan::Reduce { .. }
            | VectorPlan::Reduce2DRowSum { .. }
            | VectorPlan::Reduce2DColSum { .. }
            | VectorPlan::Reduce3D { .. }
            | VectorPlan::Map { .. }
            | VectorPlan::Map2DRow { .. }
            | VectorPlan::Map2DCol { .. }
            | VectorPlan::Map3D { .. }
    )
}

fn record_legacy_poly_fallback_counts(stats: &mut VOptStats, plan: &VectorPlan, applied: bool) {
    if !crate::mir::opt::poly::poly_enabled() || !is_legacy_poly_fallback_plan(plan) {
        return;
    }
    let is_reduction = matches!(
        plan,
        VectorPlan::Reduce { .. }
            | VectorPlan::Reduce2DRowSum { .. }
            | VectorPlan::Reduce2DColSum { .. }
            | VectorPlan::Reduce3D { .. }
    );
    let is_map = matches!(
        plan,
        VectorPlan::Map { .. }
            | VectorPlan::Map2DRow { .. }
            | VectorPlan::Map2DCol { .. }
            | VectorPlan::Map3D { .. }
    );
    if applied {
        stats.legacy_poly_fallback_applied_total += 1;
        if is_reduction {
            stats.legacy_poly_fallback_applied_reductions += 1;
        }
        if is_map {
            stats.legacy_poly_fallback_applied_maps += 1;
        }
    } else {
        stats.legacy_poly_fallback_candidate_total += 1;
        if is_reduction {
            stats.legacy_poly_fallback_candidate_reductions += 1;
        }
        if is_map {
            stats.legacy_poly_fallback_candidate_maps += 1;
        }
    }
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

pub(super) fn collect_reduction_candidates(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Vec<VectorPlan> {
    let mut out = Vec::new();
    if let Some(plan) = match_reduction(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_2d_row_reduction_sum(fn_ir, lp) {
        out.push(plan);
    }
    if let Some(plan) = match_2d_col_reduction_sum(fn_ir, lp) {
        out.push(plan);
    }
    if let Some(plan) = match_3d_axis_reduction(fn_ir, lp) {
        out.push(plan);
    }
    out
}

pub(super) fn collect_vector_candidates(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Vec<VectorPlan> {
    let mut out = Vec::new();
    if let Some(plan) = match_conditional_map(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_conditional_map_3d(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_recurrence_add_const(fn_ir, lp) {
        out.push(plan);
    }
    if let Some(plan) = match_recurrence_add_const_3d(fn_ir, lp) {
        out.push(plan);
    }
    if let Some(plan) = match_shifted_map(fn_ir, lp) {
        out.push(plan);
    }
    if let Some(plan) = match_shifted_map_3d(fn_ir, lp) {
        out.push(plan);
    }
    if let Some(plan) = match_2d_row_map(fn_ir, lp) {
        out.push(plan);
    }
    if let Some(plan) = match_2d_col_map(fn_ir, lp) {
        out.push(plan);
    }
    if let Some(plan) = match_3d_axis_map(fn_ir, lp) {
        out.push(plan);
    }
    if let Some(plan) = match_call_map_3d(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_multi_expr_map_3d(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_expr_map_3d(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_call_map(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_cube_slice_expr_map(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_expr_map(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_scatter_expr_map(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_scatter_expr_map_3d(fn_ir, lp, user_call_whitelist) {
        out.push(plan);
    }
    if let Some(plan) = match_map(fn_ir, lp) {
        out.push(plan);
    }
    out
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
