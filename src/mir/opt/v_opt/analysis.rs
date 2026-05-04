use super::debug::{VectorizeSkipReason, vectorize_trace_enabled};
use super::planning::{Axis3D, CallMapArg, VectorPlan, is_builtin_vector_safe_call};
use super::reconstruct::{
    has_non_passthrough_assignment_in_loop, is_scalar_broadcast_value,
    unique_assign_source_in_loop, unique_assign_source_reaching_block_in_loop,
    value_use_block_in_loop,
};
use super::types::{
    BlockStore1D, BlockStore1DMatch, BlockStore3D, BlockStore3DMatch, CallMapLoweringMode,
    MemoryStrideClass, VectorAccessOperand3D, VectorAccessPattern3D,
};
use crate::mir::analyze::effects;
use crate::mir::opt::loop_analysis::{LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "analysis_vectorization.rs"]
mod analysis_vectorization;
mod call_and_cfg;
mod conditional_phi;
mod cost_model;
mod dependence;
mod loop_state;
mod map_shape;
mod safety;

pub(crate) use self::analysis_vectorization::*;
pub(super) use self::call_and_cfg::*;
pub(super) use self::conditional_phi::*;
pub(super) use self::cost_model::*;
pub(super) use self::dependence::*;
pub(super) use self::loop_state::*;
pub(super) use self::map_shape::*;
pub(super) use self::safety::*;

pub(super) const CALL_MAP_AUTO_HELPER_COST_THRESHOLD: u32 = 6;
pub(super) const MAX_STRIDED_REDUCTION_TRIP_HINT: u64 = 16;
const VOPT_PROOF_RECURSION_LIMIT: usize = 256;
const VOPT_PROOF_VISIT_LIMIT: usize = 8_192;
