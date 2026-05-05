pub(crate) mod alias;
pub(crate) mod patterns;
pub(crate) mod stage_catalog;
pub(crate) mod vector;

use rustc_hash::FxHashSet;

#[derive(Clone, Debug, Default)]
pub(crate) struct PeepholeProfile {
    pub(crate) linear_scan_elapsed_ns: u128,
    pub(crate) primary_rewrite_elapsed_ns: u128,
    pub(crate) primary_flow_elapsed_ns: u128,
    pub(crate) primary_inline_elapsed_ns: u128,
    pub(crate) primary_reuse_elapsed_ns: u128,
    pub(crate) primary_loop_cleanup_elapsed_ns: u128,
    pub(crate) primary_loop_dead_zero_elapsed_ns: u128,
    pub(crate) primary_loop_normalize_elapsed_ns: u128,
    pub(crate) primary_loop_hoist_elapsed_ns: u128,
    pub(crate) primary_loop_repeat_to_for_elapsed_ns: u128,
    pub(crate) primary_loop_tail_cleanup_elapsed_ns: u128,
    pub(crate) primary_loop_guard_cleanup_elapsed_ns: u128,
    pub(crate) primary_loop_helper_cleanup_elapsed_ns: u128,
    pub(crate) primary_loop_exact_cleanup_elapsed_ns: u128,
    pub(crate) primary_loop_exact_pre_elapsed_ns: u128,
    pub(crate) primary_loop_exact_reuse_elapsed_ns: u128,
    pub(crate) primary_loop_exact_reuse_prepare_elapsed_ns: u128,
    pub(crate) primary_loop_exact_reuse_forward_elapsed_ns: u128,
    pub(crate) primary_loop_exact_reuse_pure_call_elapsed_ns: u128,
    pub(crate) primary_loop_exact_reuse_expr_elapsed_ns: u128,
    pub(crate) primary_loop_exact_reuse_vector_alias_elapsed_ns: u128,
    pub(crate) primary_loop_exact_reuse_rebind_elapsed_ns: u128,
    pub(crate) primary_loop_exact_fixpoint_elapsed_ns: u128,
    pub(crate) primary_loop_exact_fixpoint_prepare_elapsed_ns: u128,
    pub(crate) primary_loop_exact_fixpoint_forward_elapsed_ns: u128,
    pub(crate) primary_loop_exact_fixpoint_pure_call_elapsed_ns: u128,
    pub(crate) primary_loop_exact_fixpoint_expr_elapsed_ns: u128,
    pub(crate) primary_loop_exact_fixpoint_rebind_elapsed_ns: u128,
    pub(crate) primary_loop_exact_fixpoint_rounds: usize,
    pub(crate) primary_loop_exact_finalize_elapsed_ns: u128,
    pub(crate) primary_loop_dead_temp_cleanup_elapsed_ns: u128,
    pub(crate) secondary_rewrite_elapsed_ns: u128,
    pub(crate) secondary_inline_elapsed_ns: u128,
    pub(crate) secondary_inline_branch_hoist_elapsed_ns: u128,
    pub(crate) secondary_inline_immediate_scalar_elapsed_ns: u128,
    pub(crate) secondary_inline_named_index_elapsed_ns: u128,
    pub(crate) secondary_inline_named_expr_elapsed_ns: u128,
    pub(crate) secondary_inline_scalar_region_elapsed_ns: u128,
    pub(crate) secondary_inline_immediate_index_elapsed_ns: u128,
    pub(crate) secondary_inline_adjacent_dedup_elapsed_ns: u128,
    pub(crate) secondary_exact_elapsed_ns: u128,
    pub(crate) secondary_helper_cleanup_elapsed_ns: u128,
    pub(crate) secondary_helper_wrapper_elapsed_ns: u128,
    pub(crate) secondary_helper_metric_elapsed_ns: u128,
    pub(crate) secondary_helper_alias_elapsed_ns: u128,
    pub(crate) secondary_helper_simple_expr_elapsed_ns: u128,
    pub(crate) secondary_helper_full_range_elapsed_ns: u128,
    pub(crate) secondary_helper_named_copy_elapsed_ns: u128,
    pub(crate) secondary_record_sroa_elapsed_ns: u128,
    pub(crate) secondary_finalize_cleanup_elapsed_ns: u128,
    pub(crate) secondary_finalize_bundle_elapsed_ns: u128,
    pub(crate) secondary_finalize_dead_temp_elapsed_ns: u128,
    pub(crate) secondary_finalize_dead_temp_facts_elapsed_ns: u128,
    pub(crate) secondary_finalize_dead_temp_mark_elapsed_ns: u128,
    pub(crate) secondary_finalize_dead_temp_reverse_elapsed_ns: u128,
    pub(crate) secondary_finalize_dead_temp_compact_elapsed_ns: u128,
    pub(crate) finalize_elapsed_ns: u128,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PeepholeOptions {
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) preserve_all_defs: bool,
    pub(crate) fast_dev: bool,
    pub(crate) opt_level: crate::compiler::OptLevel,
}

impl PeepholeOptions {
    pub(crate) fn new(direct_builtin_call_map: bool) -> Self {
        Self {
            direct_builtin_call_map,
            preserve_all_defs: false,
            fast_dev: false,
            opt_level: crate::compiler::OptLevel::O2,
        }
    }

    pub(crate) fn preserving_all_defs(mut self, preserve_all_defs: bool) -> Self {
        self.preserve_all_defs = preserve_all_defs;
        self
    }

    pub(crate) fn fast_dev(mut self, fast_dev: bool) -> Self {
        self.fast_dev = fast_dev;
        self
    }

    pub(crate) fn opt_level(mut self, opt_level: crate::compiler::OptLevel) -> Self {
        self.opt_level = opt_level;
        self
    }
}

pub(crate) fn optimize_emitted_r(code: &str, direct_builtin_call_map: bool) -> String {
    super::r_peephole::optimize_emitted_r(code, direct_builtin_call_map)
}

pub(crate) fn optimize_emitted_r_with_line_map(
    code: &str,
    direct_builtin_call_map: bool,
) -> (String, Vec<u32>) {
    super::r_peephole::optimize_emitted_r_with_line_map(code, direct_builtin_call_map)
}

pub(crate) fn optimize_emitted_r_with_context(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
) -> (String, Vec<u32>) {
    super::r_peephole::optimize_emitted_r_with_context(
        code,
        direct_builtin_call_map,
        pure_user_calls,
    )
}

pub(crate) fn optimize_emitted_r_with_context_and_fresh(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
) -> (String, Vec<u32>) {
    super::r_peephole::optimize_emitted_r_with_context_and_fresh(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        fresh_user_calls,
    )
}

pub(crate) fn optimize_emitted_r_with_context_and_fresh_with_options(
    code: &str,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    options: PeepholeOptions,
) -> (String, Vec<u32>) {
    super::r_peephole::optimize_emitted_r_with_context_and_fresh_with_options(
        code,
        pure_user_calls,
        fresh_user_calls,
        options,
    )
}

pub(crate) fn optimize_emitted_r_with_context_and_fresh_with_profile(
    code: &str,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    options: PeepholeOptions,
) -> ((String, Vec<u32>), PeepholeProfile) {
    super::r_peephole::optimize_emitted_r_with_context_and_fresh_with_profile(
        code,
        pure_user_calls,
        fresh_user_calls,
        options,
    )
}

pub(crate) fn rewrite_selected_simple_expr_helper_calls_in_text(
    code: &str,
    helper_names: &[&str],
) -> String {
    super::r_peephole::rewrite_selected_simple_expr_helper_calls_in_text(code, helper_names)
}

pub(crate) fn simplify_nested_index_vec_floor_calls_in_text(code: &str) -> String {
    super::r_peephole::simplify_nested_index_vec_floor_calls_in_text(code)
}
