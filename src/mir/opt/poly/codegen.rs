//! Schedule-tree guided polyhedral codegen entrypoints.
//!
//! This layer decides whether a discovered poly schedule can lower through
//! specialized vectorized builders or should fall back to the generic poly MIR
//! reconstruction path.

use super::ScopRegion;
use super::codegen_generic::{
    generic_mir_effective_for_schedule, generic_schedule_supports_map,
    generic_schedule_supports_reduce, lower_fission_sequence_generic, lower_identity_map_generic,
    lower_identity_reduce_generic, lower_interchange_map_generic, lower_interchange_reduce_generic,
    lower_skew2d_map_generic, lower_skew2d_reduce_generic, lower_tile1d_map_generic,
    lower_tile1d_reduce_generic, lower_tile2d_map_generic, lower_tile2d_reduce_generic,
    lower_tile3d_map_generic, lower_tile3d_reduce_generic,
};
use super::schedule::{SchedulePlan, SchedulePlanKind};
use super::tree::{ScheduleTransform, ScheduleTree, ScheduleTreeNode};
use super::{PolyStmtKind, access, affine, poly_trace_enabled, schedule};
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::opt::v_opt::{
    Axis3D, PreparedVectorAssignment, ReduceKind, VectorPlan, build_slice_assignment_value,
    emit_same_array3_shape_or_scalar_guard, emit_same_matrix_shape_or_scalar_guard,
    finish_vector_assignments_versioned, prepare_partial_slice_value, same_length_proven,
    try_apply_vectorization_transactionally, vector_apply_site,
};
use crate::mir::{FnIR, Lit, ValueId, ValueKind};
use crate::syntax::ast::BinOp;

#[path = "codegen_lower.rs"]
mod codegen_lower;
use self::codegen_lower::*;

#[path = "codegen/entry.rs"]
mod entry;
pub(crate) use self::entry::*;
#[path = "codegen/operands.rs"]
mod operands;
pub(crate) use self::operands::*;
#[path = "codegen/map_vector.rs"]
mod map_vector;
pub(crate) use self::map_vector::*;
#[path = "codegen/map_nd.rs"]
mod map_nd;
pub(crate) use self::map_nd::*;
#[path = "codegen/guards.rs"]
mod guards;
pub(crate) use self::guards::*;
#[path = "codegen/reduce_nested.rs"]
mod reduce_nested;
pub(crate) use self::reduce_nested::*;
#[path = "codegen/schedules.rs"]
mod schedules;
pub(crate) use self::schedules::*;
#[cfg(test)]
#[path = "codegen/tests.rs"]
mod tests;
