use super::super::analysis::{match_2d_col_map, match_2d_row_map, match_3d_axis_map};
use super::super::planning::{
    VectorPlan, match_2d_col_reduction_sum, match_2d_row_reduction_sum, match_3d_axis_reduction,
    match_call_map, match_call_map_3d, match_conditional_map, match_conditional_map_3d,
    match_cube_slice_expr_map, match_expr_map, match_expr_map_3d, match_map,
    match_multi_expr_map_3d, match_recurrence_add_const, match_recurrence_add_const_3d,
    match_reduction, match_scatter_expr_map, match_scatter_expr_map_3d, match_shifted_map,
    match_shifted_map_3d,
};
use crate::mir::FnIR;
use crate::mir::opt::loop_analysis::LoopInfo;
use rustc_hash::FxHashSet;

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
