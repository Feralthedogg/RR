use super::analysis::{
    canonical_value, classify_3d_general_vector_access, classify_3d_vector_access_axis,
    collapse_prior_origin_phi_state, expr_has_iv_dependency,
    expr_has_non_vector_safe_call_in_vector_context, find_conditional_phi_shape,
    find_conditional_phi_shape_with_blocks, floor_like_index_source, is_iv_equivalent,
    is_loop_compatible_base, is_passthrough_load_of_var, is_prior_origin_phi_state,
    last_assign_to_var_in_block, last_effective_assign_before_value_use_in_block,
    loop_covers_whole_destination, preserve_phi_value, resolve_materialized_value,
    value_depends_on, vector_length_key,
};
use super::debug::{
    trace_block_instrs, trace_materialize_reject, trace_value_tree, vectorize_trace_enabled,
};
use super::types::{Axis3D, VectorAccessOperand3D};
use crate::mir::opt::loop_analysis::{LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use crate::typeck::{PrimTy, ShapeTy};
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "reconstruct/index_materialized.rs"]
mod index_materialized;
#[path = "reconstruct/loop_sources.rs"]
mod loop_sources;
#[path = "reconstruct/passthrough_phi.rs"]
mod passthrough_phi;
#[path = "reconstruct_materialize.rs"]
mod reconstruct_materialize;
#[path = "reconstruct/state_chain.rs"]
mod state_chain;
#[path = "reconstruct/vector_materialize.rs"]
mod vector_materialize;

pub(in crate::mir::opt::v_opt) use self::index_materialized::*;
pub(in crate::mir::opt::v_opt) use self::loop_sources::*;
pub(in crate::mir::opt::v_opt) use self::passthrough_phi::*;
pub(super) use self::reconstruct_materialize::*;
pub(in crate::mir::opt::v_opt) use self::state_chain::*;
pub(in crate::mir::opt::v_opt) use self::vector_materialize::*;
