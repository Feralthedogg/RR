use super::*;
impl RBackend {
    pub(crate) fn rewrite_safe_scalar_loop_index_helpers(output: &mut String) {
        rewrite_emit::rewrite_safe_scalar_loop_index_helpers(output)
    }

    pub(crate) fn rewrite_branch_local_identical_alloc_rebinds(output: &mut String) {
        rewrite_emit::rewrite_branch_local_identical_alloc_rebinds(output)
    }

    pub(crate) fn hoist_branch_local_pure_scalar_assigns_used_after_branch(output: &mut String) {
        rewrite_emit::hoist_branch_local_pure_scalar_assigns_used_after_branch(output)
    }

    pub(crate) fn rewrite_single_use_scalar_index_aliases(output: &mut String) {
        rewrite_emit::rewrite_single_use_scalar_index_aliases(output)
    }

    pub(crate) fn rewrite_immediate_and_guard_named_scalar_exprs(output: &mut String) {
        rewrite_emit::rewrite_immediate_and_guard_named_scalar_exprs(output)
    }

    pub(crate) fn rewrite_two_use_named_scalar_exprs(output: &mut String) {
        rewrite_emit::rewrite_two_use_named_scalar_exprs(output)
    }

    pub(crate) fn rewrite_small_multiuse_scalar_index_aliases(output: &mut String) {
        rewrite_emit::rewrite_small_multiuse_scalar_index_aliases(output)
    }

    pub(crate) fn rewrite_one_or_two_use_named_scalar_index_reads_in_straight_line_region(
        output: &mut String,
    ) {
        rewrite_emit::rewrite_one_or_two_use_named_scalar_index_reads_in_straight_line_region(
            output,
        )
    }

    pub(crate) fn rewrite_named_scalar_pure_call_aliases(output: &mut String) {
        rewrite_emit::rewrite_named_scalar_pure_call_aliases(output)
    }

    pub(crate) fn rewrite_loop_index_alias_ii(output: &mut String) {
        rewrite_emit::rewrite_loop_index_alias_ii(output)
    }

    pub(crate) fn strip_dead_zero_seed_ii(output: &mut String) {
        rewrite_emit::strip_dead_zero_seed_ii(output)
    }

    pub(crate) fn rewrite_slice_bound_aliases(output: &mut String) {
        rewrite_emit::rewrite_slice_bound_aliases(output)
    }

    pub(crate) fn rewrite_particle_idx_alias(output: &mut String) {
        rewrite_emit::rewrite_particle_idx_alias(output)
    }

    pub(crate) fn rewrite_adjacent_duplicate_symbol_assignments(output: &mut String) {
        rewrite_emit::rewrite_adjacent_duplicate_symbol_assignments(output)
    }

    pub(crate) fn rewrite_duplicate_pure_call_assignments(
        output: &mut String,
        pure_user_calls: &FxHashSet<String>,
    ) {
        rewrite_emit::rewrite_duplicate_pure_call_assignments(output, pure_user_calls)
    }

    pub(crate) fn strip_noop_self_assignments(output: &mut String) {
        rewrite_emit::strip_noop_self_assignments(output)
    }

    pub(crate) fn rewrite_temp_uses_after_named_copy(output: &mut String) {
        rewrite_emit::rewrite_temp_uses_after_named_copy(output)
    }

    pub(crate) fn strip_noop_temp_copy_roundtrips(output: &mut String) {
        rewrite_emit::strip_noop_temp_copy_roundtrips(output)
    }

    pub(crate) fn strip_dead_simple_scalar_assigns(output: &mut String) {
        rewrite_emit::strip_dead_simple_scalar_assigns(output)
    }

    pub(crate) fn strip_shadowed_simple_scalar_seed_assigns(output: &mut String) {
        rewrite_emit::strip_shadowed_simple_scalar_seed_assigns(output)
    }

    pub(crate) fn strip_dead_seq_len_locals(output: &mut String) {
        rewrite_emit::strip_dead_seq_len_locals(output)
    }

    pub(crate) fn strip_redundant_branch_local_vec_fill_rebinds(output: &mut String) {
        rewrite_emit::strip_redundant_branch_local_vec_fill_rebinds(output)
    }

    pub(crate) fn strip_unused_raw_arg_aliases(output: &mut String) {
        rewrite_emit::strip_unused_raw_arg_aliases(output)
    }

    pub(crate) fn rewrite_readonly_raw_arg_aliases(output: &mut String) {
        rewrite_emit::rewrite_readonly_raw_arg_aliases(output)
    }

    pub(crate) fn strip_empty_else_blocks(output: &mut String) {
        rewrite_emit::strip_empty_else_blocks(output)
    }

    pub(crate) fn collapse_nested_else_if_blocks(output: &mut String) {
        rewrite_emit::collapse_nested_else_if_blocks(output)
    }

    pub(crate) fn rewrite_guard_scalar_literals(output: &mut String) {
        rewrite_emit::rewrite_guard_scalar_literals(output)
    }

    pub(crate) fn rewrite_loop_guard_scalar_literals(output: &mut String) {
        rewrite_emit::rewrite_loop_guard_scalar_literals(output)
    }

    pub(crate) fn rewrite_single_assignment_loop_seed_literals(output: &mut String) {
        rewrite_emit::rewrite_single_assignment_loop_seed_literals(output)
    }

    pub(crate) fn rewrite_sym210_loop_seed(output: &mut String) {
        rewrite_emit::rewrite_sym210_loop_seed(output)
    }

    pub(crate) fn rewrite_seq_len_full_overwrite_inits(output: &mut String) {
        rewrite_emit::rewrite_seq_len_full_overwrite_inits(output)
    }

    pub(crate) fn restore_missing_repeat_loop_counter_updates(output: &mut String) {
        rewrite_emit::restore_missing_repeat_loop_counter_updates(output)
    }

    pub(crate) fn rewrite_hoisted_loop_counter_aliases(output: &mut String) {
        rewrite_emit::rewrite_hoisted_loop_counter_aliases(output)
    }

    pub(crate) fn repair_missing_cse_range_aliases(output: &mut String) {
        rewrite_emit::repair_missing_cse_range_aliases(output)
    }

    pub(crate) fn restore_constant_one_guard_repeat_loop_counters(output: &mut String) {
        rewrite_emit::restore_constant_one_guard_repeat_loop_counters(output)
    }

    pub(crate) fn rewrite_literal_named_list_calls(output: &mut String) {
        rewrite_emit::rewrite_literal_named_list_calls(output)
    }

    pub(crate) fn rewrite_literal_field_get_calls(output: &mut String) {
        rewrite_emit::rewrite_literal_field_get_calls(output)
    }

    pub(crate) fn strip_unreachable_sym_helpers(output: &mut String) {
        rewrite_emit::strip_unreachable_sym_helpers(output)
    }

    pub(crate) fn strip_redundant_tail_assign_slice_return(output: &mut String) {
        rewrite_emit::strip_redundant_tail_assign_slice_return(output)
    }

    pub(crate) fn strip_single_blank_spacers(output: &mut String) {
        rewrite_emit::strip_single_blank_spacers(output)
    }

    pub(crate) fn compact_blank_lines(output: &mut String) {
        rewrite_emit::compact_blank_lines(output)
    }

    pub(crate) fn strip_terminal_repeat_nexts(output: &mut String) {
        rewrite_emit::strip_terminal_repeat_nexts(output)
    }

    pub(crate) fn strip_orphan_rr_cse_pruned_markers(output: &mut String) {
        rewrite_emit::strip_orphan_rr_cse_pruned_markers(output)
    }

    pub(crate) fn infer_generated_poly_loop_step(
        lines: &[String],
        body_start: usize,
        body_end: usize,
        var: &str,
    ) -> i64 {
        rewrite_emit::infer_generated_poly_loop_step(lines, body_start, body_end, var)
    }

    pub(crate) fn first_generated_poly_loop_var_in_line(line: &str) -> Option<String> {
        rewrite_emit::first_generated_poly_loop_var_in_line(line)
    }

    pub(crate) fn restore_missing_generated_poly_loop_steps(output: &mut String) {
        rewrite_emit::restore_missing_generated_poly_loop_steps(output)
    }

    pub(crate) fn emit_mark(&mut self, span: Span, label: Option<&str>) {
        render_emit::emit_mark(self, span, label)
    }
}
