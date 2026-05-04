use super::super::analysis::choose_call_map_lowering;
use super::super::planning::VectorPlan;
use super::super::types::CallMapLoweringMode;
use super::VOptStats;
use crate::mir::FnIR;

#[derive(Clone, Copy)]
pub(super) enum VectorPlanFamily {
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

pub(super) fn vector_plan_family(plan: &VectorPlan) -> VectorPlanFamily {
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

pub(super) fn vector_plan_call_map_lowering(
    fn_ir: &FnIR,
    plan: &VectorPlan,
) -> Option<CallMapLoweringMode> {
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

pub(super) fn record_plan_counts(
    stats: &mut VOptStats,
    fn_ir: &FnIR,
    plan: &VectorPlan,
    applied: bool,
) {
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
