use super::*;

pub(in super::super) fn rewrite_readonly_param_aliases(lines: Vec<String>) -> Vec<String> {
    rewrite_readonly_param_aliases_ir(lines)
}

pub(in super::super) fn rewrite_readonly_param_aliases_with_cache(
    lines: Vec<String>,
    _cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    rewrite_readonly_param_aliases_ir(lines)
}

pub(in super::super) fn rewrite_remaining_readonly_param_shadow_uses(
    lines: Vec<String>,
) -> Vec<String> {
    rewrite_remaining_readonly_param_shadow_uses_ir(lines)
}

pub(in super::super) fn rewrite_remaining_readonly_param_shadow_uses_with_cache(
    lines: Vec<String>,
    _cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    rewrite_remaining_readonly_param_shadow_uses_ir(lines)
}

pub(in super::super) fn rewrite_index_only_mutated_param_shadow_aliases(
    lines: Vec<String>,
) -> Vec<String> {
    rewrite_index_only_mutated_param_shadow_aliases_ir(lines)
}

pub(in super::super) fn rewrite_index_only_mutated_param_shadow_aliases_with_cache(
    lines: Vec<String>,
    _cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    rewrite_index_only_mutated_param_shadow_aliases_ir(lines)
}
