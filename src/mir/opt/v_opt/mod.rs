mod analysis;
mod api;
mod debug;
mod planning;
mod proof;
mod reconstruct;
mod transform;
mod types;

pub use api::{VOptStats, optimize, optimize_with_stats, optimize_with_stats_with_whitelist};
pub(crate) use planning::is_builtin_vector_safe_call;
pub(crate) use types::{PROOF_FALLBACK_REASON_COUNT, format_proof_fallback_counts};
