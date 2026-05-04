use super::analysis::{
    affine_iv_offset, array3_access_stride, as_safe_loop_index, axis3_operand_source,
    axis3_vector_operand_source, canonical_value, classify_3d_general_vector_access,
    classify_3d_map_axis, classify_3d_vector_access_axis, classify_store_1d_in_block,
    classify_store_3d_in_block, collect_loop_shadow_vars_for_dest, expr_has_iv_dependency,
    expr_has_non_vector_safe_call_in_vector_context, expr_reads_base, expr_reads_base_non_iv,
    induction_origin_var, is_condition_vectorizable, is_floor_like_iv_expr, is_iv_equivalent,
    is_loop_compatible_base, is_loop_invariant_axis, is_loop_invariant_scalar_expr,
    is_origin_var_iv_alias_in_loop, is_prev_element, is_prev_element_3d, is_vector_safe_call,
    is_vector_safe_call_chain_expr, is_vectorizable_expr, loop_covers_whole_destination,
    loop_has_inner_branch, loop_has_store_effect, loop_matches_vec, matrix_access_stride,
    resolve_base_var, resolve_call_info, resolve_load_alias_value, resolve_match_alias_value,
    same_base_value, same_loop_invariant_value, structured_reduction_stride_allowed,
    unique_assign_source,
};
use super::debug::vectorize_trace_enabled;
use super::reconstruct::{
    expr_has_ambiguous_loop_local_load, expr_has_unstable_loop_local_load, expr_reads_var,
    merged_assign_source_in_loop, phi_state_var, unique_assign_source_in_loop,
    unique_origin_phi_value_in_loop,
};
pub use super::types::{Axis3D, CallMapArg, ExprMapEntry, ExprMapEntry3D, ReduceKind, VectorPlan};
use super::types::{
    BlockStore1DMatch, BlockStore3DMatch, CallMap3DMatchCandidate, VectorAccessOperand3D,
    VectorAccessPattern3D,
};
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::FxHashSet;

#[path = "planning/call_map.rs"]
mod call_map;
#[path = "planning/expr_map.rs"]
mod expr_map;
#[path = "planning/map.rs"]
mod map;
#[path = "planning_expr_map.rs"]
pub(super) mod planning_expr_map;
#[path = "planning/recurrence_shift.rs"]
mod recurrence_shift;
#[path = "planning/reduction.rs"]
mod reduction;

pub(in crate::mir::opt::v_opt) use self::call_map::*;
pub(crate) use self::expr_map::is_builtin_vector_safe_call;
pub(in crate::mir::opt::v_opt) use self::expr_map::*;
pub(in crate::mir::opt::v_opt) use self::map::*;
pub(super) use self::planning_expr_map::*;
pub(in crate::mir::opt::v_opt) use self::recurrence_shift::*;
pub(in crate::mir::opt::v_opt) use self::reduction::*;
