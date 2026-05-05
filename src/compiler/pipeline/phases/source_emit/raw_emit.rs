use super::*;

#[path = "raw_emit/assembly.rs"]
pub(crate) mod assembly;
#[path = "raw_emit/debug.rs"]
pub(crate) mod debug;
#[path = "raw_emit/entry_quote.rs"]
pub(crate) mod entry_quote;
#[path = "raw_emit/raw_pass_manager.rs"]
pub(crate) mod raw_pass_manager;
#[path = "raw_emit/rewrite_pipeline.rs"]
pub(crate) mod rewrite_pipeline;

pub(crate) use assembly::assemble_emitted_fragments;
pub(crate) use debug::{contains_unsafe_r_escape, maybe_emit_raw_debug_output};
pub(crate) use entry_quote::{quoted_body_entry_targets, wrap_zero_arg_function_body_in_quote};
pub(crate) use rewrite_pipeline::{
    apply_full_peephole_to_output, apply_full_raw_rewrites, apply_post_assembly_finalize_rewrites,
    optimize_emitted_fragment,
};
