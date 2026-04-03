mod analysis;
mod api;
mod debug;
mod planning;
mod proof;
mod reconstruct;
mod transform;
mod types;

pub(crate) use analysis::same_length_proven;
pub use api::{VOptStats, optimize, optimize_with_stats, optimize_with_stats_with_whitelist};
pub(crate) use planning::is_builtin_vector_safe_call;
pub(crate) use planning::{Axis3D, ReduceKind, VectorPlan};
pub(crate) use transform::try_apply_vectorization_transactionally;
pub(crate) use transform::{
    build_slice_assignment_value, emit_same_array3_shape_or_scalar_guard,
    emit_same_matrix_shape_or_scalar_guard, finish_vector_assignments_versioned,
    prepare_partial_slice_value, vector_apply_site,
};
pub(crate) use types::PreparedVectorAssignment;
pub(crate) use types::{PROOF_FALLBACK_REASON_COUNT, format_proof_fallback_counts};
