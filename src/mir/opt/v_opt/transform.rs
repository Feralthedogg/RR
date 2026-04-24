//! IR rewriting routines that materialize approved vectorization plans.
//!
//! Analysis and planning choose the vector form; this module owns the actual
//! MIR mutation that installs vector values, repairs shadow state, and rewrites
//! exit-path uses to preserve scalar semantics.

#[path = "transform_linear.rs"]
mod transform_linear;
use self::transform_linear::*;

use super::analysis::{
    as_safe_loop_index, canonical_value, choose_call_map_lowering, expr_has_iv_dependency,
    hoist_vector_expr_temp, induction_origin_var, intrinsic_for_call, is_const_number,
    is_const_one, is_invariant_reduce_scalar, is_iv_equivalent, is_loop_invariant_scalar_expr,
    loop_entry_seed_source_in_loop, maybe_hoist_callmap_arg_expr, resolve_base_var,
    resolve_materialized_value, rewrite_returns_for_var, same_length_proven, value_depends_on,
    vector_length_key,
};
use super::debug::vectorize_trace_enabled;
use super::reconstruct::{
    MaterializedExprKey, add_int_offset, adjusted_loop_limit, build_loop_index_vector,
    has_assignment_in_loop, has_non_passthrough_assignment_in_loop, intern_materialized_value,
    is_int_index_vector_value, is_scalar_broadcast_value, materialize_loop_invariant_scalar_expr,
    materialize_vector_expr, unique_assign_source_in_loop,
    unique_assign_source_reaching_block_in_loop, value_use_block_in_loop,
};
use super::types::{
    Axis3D, CallMap3DApplyPlan, CallMap3DGeneralApplyPlan, CallMapArg, CallMapLoweringMode,
    CondMap3DApplyPlan, CondMap3DGeneralApplyPlan, ExprMap3DApplyPlan, ExprMapEntry,
    ExprMapEntry3D, Map2DApplyPlan, Map3DApplyPlan, PreparedVectorAssignment,
    RecurrenceAddConst3DApplyPlan, RecurrenceAddConstApplyPlan, Reduce2DApplyPlan,
    Reduce3DApplyPlan, ReduceCondEntry, ReduceKind, ScatterExprMap3DApplyPlan,
    ScatterExprMap3DGeneralApplyPlan, ShiftedMap3DApplyPlan, ShiftedMapApplyPlan,
    VectorAccessOperand3D, VectorApplySite, VectorLoopRange, VectorPlan,
};
use crate::mir::opt::loop_analysis::{LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::{FxHashMap, FxHashSet};
include!("transform/apply_site.rs");
include!("transform/versioned_exit.rs");
include!("transform/assignment_emit.rs");
include!("transform/call_plans.rs");
include!("transform/expr3d_plans.rs");
include!("transform/expr_map_plans.rs");
include!("transform/scatter_slice.rs");
include!("transform/plan_apply.rs");
include!("transform/tests.rs");
