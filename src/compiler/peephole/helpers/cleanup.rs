use super::super::{
    PeepholeAnalysisCache, collapse_inlined_copy_vec_sequences_ir,
    collapse_singleton_assign_slice_scalar_edits_ir, collapse_trivial_dot_product_wrappers_ir,
    collapse_trivial_scalar_clamp_wrappers_ir, strip_dead_simple_eval_lines_ir,
    strip_empty_else_blocks_ir, strip_noop_self_assignments_ir, strip_unused_arg_aliases_ir,
};

pub(crate) fn strip_empty_else_blocks(lines: Vec<String>) -> Vec<String> {
    strip_empty_else_blocks_ir(lines)
}

pub(crate) fn strip_dead_simple_eval_lines(lines: Vec<String>) -> Vec<String> {
    strip_dead_simple_eval_lines_ir(lines)
}

pub(crate) fn strip_noop_self_assignments(lines: Vec<String>) -> Vec<String> {
    strip_noop_self_assignments_ir(lines)
}

pub(crate) fn strip_unused_arg_aliases(lines: Vec<String>) -> Vec<String> {
    let mut cache = PeepholeAnalysisCache::default();
    strip_unused_arg_aliases_with_cache(lines, &mut cache)
}

pub(crate) fn strip_unused_arg_aliases_with_cache(
    lines: Vec<String>,
    _cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    strip_unused_arg_aliases_ir(lines)
}

pub(crate) fn collapse_trivial_scalar_clamp_wrappers(lines: Vec<String>) -> Vec<String> {
    collapse_trivial_scalar_clamp_wrappers_ir(lines)
}

pub(crate) fn collapse_singleton_assign_slice_scalar_edits(lines: Vec<String>) -> Vec<String> {
    collapse_singleton_assign_slice_scalar_edits_ir(lines)
}

pub(crate) fn collapse_trivial_dot_product_wrappers(lines: Vec<String>) -> Vec<String> {
    collapse_trivial_dot_product_wrappers_ir(lines)
}

pub(crate) fn collapse_inlined_copy_vec_sequences(lines: Vec<String>) -> Vec<String> {
    collapse_inlined_copy_vec_sequences_ir(lines)
}
