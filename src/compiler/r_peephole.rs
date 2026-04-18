use crate::compiler::peephole::{
    PeepholeProfile,
    alias::{
        invalidate_aliases_for_write, is_peephole_temp, normalize_expr_with_aliases, resolve_alias,
        rewrite_known_aliases,
    },
    patterns::{
        IDENT_PATTERN, assign_re, assign_slice_re, call_map_slice_re, call_map_whole_builtin_re,
        compile_regex, expr_idents, floor_re, ident_re, indexed_store_base_re, length_call_re,
        nested_index_vec_floor_re, plain_ident_re, range_re, rep_int_re, scalar_lit_re, seq_len_re,
        split_top_level_args,
    },
    vector::{
        hoist_repeated_vector_helper_calls_within_lines, rewrite_direct_vec_helper_expr,
        rewrite_forward_exact_vector_helper_reuse, rewrite_forward_temp_aliases,
    },
};
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::OnceLock;

#[path = "peephole/core_utils.rs"]
mod core_utils_rewrites;
#[path = "peephole/dead_code.rs"]
mod dead_code_rewrites;
#[path = "peephole/emitted_ir.rs"]
mod emitted_ir_rewrites;
#[path = "peephole/expr_reuse.rs"]
mod expr_reuse_rewrites;
#[path = "peephole/facts.rs"]
mod facts_rewrites;
#[path = "peephole/full_range.rs"]
mod full_range_rewrites;
#[path = "peephole/guard_simplify.rs"]
mod guard_simplify_rewrites;
#[path = "peephole/helpers.rs"]
mod helpers_cleanup;
#[path = "peephole/index_reads.rs"]
mod index_read_rewrites;
#[path = "peephole/inline_scalar.rs"]
mod inline_scalar_rewrites;
#[path = "peephole/late_pass.rs"]
mod late_pass_rewrites;
#[path = "peephole/loop_restore.rs"]
mod loop_restore_rewrites;
#[path = "peephole/pipeline_impl.rs"]
mod pipeline_impl_rewrites;
#[path = "peephole/pipeline_stage.rs"]
mod pipeline_stage_rewrites;
#[path = "peephole/scalar_reuse.rs"]
mod scalar_reuse_rewrites;
#[path = "peephole/shadow_alias.rs"]
mod shadow_alias_rewrites;
use self::core_utils_rewrites::{
    IndexedFunction, build_function_text_index, collect_prologue_arg_aliases,
};
use self::dead_code_rewrites::{
    strip_dead_temps_with_cache, strip_dead_temps_with_cache_and_profile,
};
use self::emitted_ir_rewrites::{
    collapse_identical_if_else_tail_assignments_late_ir, collapse_inlined_copy_vec_sequences_ir,
    collapse_singleton_assign_slice_scalar_edits_ir, collapse_trivial_dot_product_wrappers_ir,
    collapse_trivial_passthrough_return_wrappers_ir, collapse_trivial_scalar_clamp_wrappers_ir,
    rewrite_dead_zero_loop_seeds_before_for_ir, rewrite_forward_exact_expr_reuse_ir,
    rewrite_forward_exact_pure_call_reuse_ir, rewrite_index_only_mutated_param_shadow_aliases_ir,
    rewrite_literal_field_get_calls_ir, rewrite_literal_named_list_calls_ir,
    rewrite_metric_helper_return_calls_ir, rewrite_metric_helper_statement_calls_ir,
    rewrite_readonly_param_aliases_ir, rewrite_remaining_readonly_param_shadow_uses_ir,
    rewrite_simple_expr_helper_calls_ir, run_empty_else_match_cleanup_bundle_ir,
    run_exact_finalize_cleanup_bundle_ir, run_exact_pre_cleanup_bundle_ir,
    run_exact_pre_full_ir_bundle, run_exact_pre_ir_bundle, run_exact_reuse_ir_bundle,
    run_post_passthrough_metric_bundle_ir, run_secondary_empty_else_finalize_bundle_ir,
    run_secondary_exact_bundle_ir, run_secondary_exact_expr_bundle_ir,
    run_secondary_helper_ir_bundle, run_simple_expr_cleanup_bundle_ir,
    simplify_nested_index_vec_floor_calls_ir, strip_arg_aliases_in_trivial_return_wrappers_ir,
    strip_dead_simple_eval_lines_ir, strip_empty_else_blocks_ir, strip_noop_self_assignments_ir,
    strip_redundant_identical_pure_rebinds_ir, strip_redundant_nested_temp_reassigns_ir,
    strip_redundant_tail_assign_slice_return_ir, strip_terminal_repeat_nexts_ir,
    strip_unreachable_sym_helpers_ir, strip_unused_arg_aliases_ir, strip_unused_helper_params_ir,
};
use self::expr_reuse_rewrites::{
    expr_is_exact_reusable_scalar, rewrite_adjacent_duplicate_assignments,
    rewrite_forward_exact_expr_reuse, rewrite_forward_exact_pure_call_reuse,
    rewrite_forward_exact_pure_call_reuse_with_cache, rewrite_forward_simple_alias_guards,
    rewrite_loop_index_alias_ii, rewrite_temp_minus_one_scaled_to_named_scalar,
    strip_redundant_nested_temp_reassigns, strip_redundant_tail_assign_slice_return,
    strip_terminal_repeat_nexts,
};
use self::facts_rewrites::{
    FunctionFacts, FunctionLineFacts, PeepholeAnalysisCache, cached_function_facts,
    clear_linear_facts, clear_loop_boundary_facts, collect_mutated_arg_aliases,
    expr_is_logical_comparison, expr_proven_no_na, helper_call_candidate_lines,
    helper_heavy_runtime_auto_args, helper_heavy_runtime_auto_args_with_temps,
    identity_index_end_expr, infer_len_from_expr, is_control_flow_boundary,
    is_dead_parenthesized_eval_line, is_dead_plain_ident_eval_line, is_one,
    next_def_after_in_facts, normalize_expr, read_vec_re, rewrite_known_length_calls,
    rewrite_strict_ifelse_expr, uses_in_region,
};
use self::full_range_rewrites::{
    compact_expr, has_one_based_full_range_index_alias_read_candidates, parse_break_guard,
    rewrite_full_range_conditional_scalar_loops, rewrite_inline_full_range_slice_ops,
    rewrite_one_based_full_range_index_alias_reads, run_secondary_full_range_bundle,
    strip_redundant_outer_parens,
};
use self::guard_simplify_rewrites::{
    rewrite_guard_truthy_line, rewrite_if_truthy_line, simplify_not_finite_or_zero_guard_parens,
    simplify_same_var_is_na_or_not_finite_guards, simplify_wrapped_not_finite_parens,
};
#[cfg(test)]
use self::helpers_cleanup::strip_unused_arg_aliases;
use self::helpers_cleanup::{
    collapse_inlined_copy_vec_sequences, collapse_trivial_dot_product_wrappers,
    strip_dead_simple_eval_lines, strip_noop_self_assignments,
};
#[cfg(test)]
use self::helpers_cleanup::{collect_simple_expr_helpers, substitute_helper_expr};
use self::index_read_rewrites::{
    literal_field_get_re, literal_record_field_name, rewrite_index_access_patterns,
};
use self::inline_scalar_rewrites::{
    hoist_branch_local_named_scalar_assigns_used_after_branch_with_cache,
    inline_one_or_two_use_named_scalar_index_reads_within_straight_line_region_with_cache,
    inline_one_or_two_use_scalar_temps_within_straight_line_region_with_cache,
    rewrite_temp_uses_after_named_copy_with_cache,
    run_immediate_single_use_inline_bundle_with_cache,
    run_named_index_scalar_region_inline_bundle_with_cache,
};
use self::late_pass_rewrites::{
    PureCallBinding, collapse_common_if_else_tail_assignments, expr_has_only_pure_calls,
    expr_is_fresh_allocation_like, extract_pure_call_binding, find_matching_block_end,
    is_loop_open_boundary, maybe_expand_fresh_alias_rhs, rewrite_pure_call_reuse,
    rewrite_return_expr_line, rewrite_safe_loop_index_write_calls,
    rewrite_safe_loop_neighbor_read_calls, strip_redundant_identical_pure_rebinds,
    strip_redundant_identical_pure_rebinds_with_cache, unquoted_sym_refs, written_base_var,
};
use self::loop_restore_rewrites::{
    RepeatLoopAnalysisCache, count_unquoted_braces,
    hoist_loop_invariant_pure_assignments_from_counted_repeat_loops_with_cache,
    latest_literal_assignment_before, literal_integer_value, literal_one_re, literal_positive_re,
    normalize_repeat_loop_counters_with_cache, parse_repeat_guard_cmp_line,
    positive_guard_for_var_before, rewrite_canonical_counted_repeat_loops_to_for_with_cache,
    rewrite_dead_zero_loop_seeds_before_for, var_has_known_positive_progression_before,
};
#[cfg(test)]
#[allow(unused_imports)]
use self::loop_restore_rewrites::{
    hoist_loop_invariant_pure_assignments_from_counted_repeat_loops,
    normalize_repeat_loop_counters, restore_constant_one_guard_repeat_loop_counters,
    rewrite_canonical_counted_repeat_loops_to_for,
};
use self::pipeline_impl_rewrites::optimize_emitted_r_pipeline_impl_with_profile;
use self::pipeline_stage_rewrites::compose_line_maps;
use self::scalar_reuse_rewrites::rewrite_shifted_square_scalar_reuse;
#[cfg(test)]
use self::shadow_alias_rewrites::rewrite_readonly_param_aliases;

pub(crate) fn optimize_emitted_r(code: &str, direct_builtin_call_map: bool) -> String {
    optimize_emitted_r_with_context(code, direct_builtin_call_map, &FxHashSet::default()).0
}

pub(crate) fn optimize_emitted_r_with_line_map(
    code: &str,
    direct_builtin_call_map: bool,
) -> (String, Vec<u32>) {
    optimize_emitted_r_with_context(code, direct_builtin_call_map, &FxHashSet::default())
}

pub(crate) fn optimize_emitted_r_with_context(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
) -> (String, Vec<u32>) {
    optimize_emitted_r_with_context_and_fresh_with_options(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        &FxHashSet::default(),
        false,
    )
}

pub(crate) fn optimize_emitted_r_with_context_and_fresh(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
) -> (String, Vec<u32>) {
    optimize_emitted_r_with_context_and_fresh_with_options(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        fresh_user_calls,
        false,
    )
}

/// Run the emitted-R peephole pipeline with the explicit call purity/freshness
/// context collected by earlier compiler stages.
pub(crate) fn optimize_emitted_r_with_context_and_fresh_with_options(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
) -> (String, Vec<u32>) {
    optimize_emitted_r_pipeline_impl_with_profile(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        fresh_user_calls,
        preserve_all_defs,
        false,
    )
    .0
}

pub(crate) fn optimize_emitted_r_with_context_and_fresh_with_options_and_profile(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
    fast_dev: bool,
) -> ((String, Vec<u32>), PeepholeProfile) {
    optimize_emitted_r_pipeline_impl_with_profile(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        fresh_user_calls,
        preserve_all_defs,
        fast_dev,
    )
}

pub(crate) fn rewrite_selected_simple_expr_helper_calls_in_text(
    code: &str,
    helper_names: &[&str],
) -> String {
    helpers_cleanup::rewrite_selected_simple_expr_helper_calls_in_text(code, helper_names)
}

pub(crate) fn simplify_nested_index_vec_floor_calls_in_text(code: &str) -> String {
    helpers_cleanup::simplify_nested_index_vec_floor_calls_in_text(code)
}

#[cfg(test)]
#[path = "peephole/tests.rs"]
mod tests;
