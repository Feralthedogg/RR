use super::*;
#[derive(Default)]
pub(crate) struct SecondaryAliasSimpleExprBundleProfile {
    pub(crate) alias_elapsed_ns: u128,
    pub(crate) simple_expr_elapsed_ns: u128,
    pub(crate) tail_elapsed_ns: u128,
}

#[derive(Default)]
pub(crate) struct SecondaryHelperIrBundleProfile {
    pub(crate) post_wrapper_elapsed_ns: u128,
    pub(crate) metric_elapsed_ns: u128,
    pub(crate) alias_elapsed_ns: u128,
    pub(crate) simple_expr_elapsed_ns: u128,
    pub(crate) tail_elapsed_ns: u128,
}

pub(crate) fn run_secondary_helper_ir_bundle(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    size_controlled_simple_expr: bool,
) -> (Vec<String>, SecondaryHelperIrBundleProfile) {
    let needs_arg_return_wrapper = has_arg_return_wrapper_candidates_ir(&lines);
    let needs_passthrough_return_wrapper = has_passthrough_return_wrapper_candidates_ir(&lines);
    let needs_dot_product_wrapper = has_trivial_dot_product_wrapper_candidates_ir(&lines);
    let needs_wrapper =
        needs_arg_return_wrapper || needs_passthrough_return_wrapper || needs_dot_product_wrapper;
    let needs_passthrough_helpers = has_passthrough_helper_candidates_ir(&lines);
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let needs_copy = has_inlined_copy_vec_sequence_candidates_ir(&lines);
    let maybe_metric_helpers = has_metric_helper_candidates_ir(&lines);
    let maybe_simple_expr_helpers = has_simple_expr_helper_candidates_ir(&lines);
    let mut profile = SecondaryHelperIrBundleProfile::default();
    let needs_alias = has_arg_alias_cleanup_candidates_ir(&lines);
    let needs_helper_param_trim = has_unused_helper_param_candidates_ir(&lines);
    let needs_singleton = has_singleton_assign_slice_scalar_edit_candidates_ir(&lines);
    let needs_clamp = has_trivial_scalar_clamp_wrapper_candidates_ir(&lines);
    let needs_tail = has_identical_if_else_tail_assign_candidates_ir(&lines);
    if !needs_wrapper
        && !needs_passthrough_helpers
        && !needs_floor
        && !needs_copy
        && !needs_alias
        && !needs_helper_param_trim
        && !needs_singleton
        && !needs_clamp
        && !maybe_metric_helpers
        && !maybe_simple_expr_helpers
        && !needs_tail
    {
        return (lines, SecondaryHelperIrBundleProfile::default());
    }

    let mut program = EmittedProgram::parse(&lines);
    let metric_helpers = if maybe_metric_helpers {
        collect_metric_helpers_from_program_ir(&program)
    } else {
        FxHashMap::default()
    };
    let needs_metric = !metric_helpers.is_empty();
    let simple_helpers = if maybe_simple_expr_helpers {
        collect_simple_expr_helpers_from_program_ir(&program, pure_user_calls)
    } else {
        FxHashMap::default()
    };
    let simple_helper_names: Vec<&str> = simple_helpers.keys().map(String::as_str).collect();
    let needs_simple_expr = !simple_helpers.is_empty();

    let started = std::time::Instant::now();
    if needs_arg_return_wrapper {
        apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    }
    if needs_passthrough_return_wrapper {
        apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    }
    if needs_dot_product_wrapper {
        apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    }
    if needs_passthrough_helpers {
        let passthrough = collect_passthrough_helpers_from_program_ir(&program);
        if !passthrough.is_empty() {
            apply_rewrite_passthrough_helper_calls_ir(&mut program, &passthrough);
        }
    }
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    if needs_copy {
        apply_collapse_inlined_copy_vec_sequences_ir(&mut program);
    }
    profile.post_wrapper_elapsed_ns = started.elapsed().as_nanos();

    if needs_metric {
        let started = std::time::Instant::now();
        let mut stmt_temp_counter = 0usize;
        apply_rewrite_metric_helper_statement_calls_ir(
            &mut program,
            &metric_helpers,
            &mut stmt_temp_counter,
        );
        let mut return_temp_counter = 0usize;
        apply_rewrite_metric_helper_return_calls_ir(
            &mut program,
            &metric_helpers,
            &mut return_temp_counter,
        );
        profile.metric_elapsed_ns = started.elapsed().as_nanos();
    }

    let started = std::time::Instant::now();
    if needs_alias {
        apply_strip_unused_arg_aliases_ir(&mut program);
        apply_rewrite_readonly_param_aliases_ir(&mut program);
        apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
        apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    }
    if needs_helper_param_trim {
        apply_strip_unused_helper_params_ir(&mut program);
    }
    profile.alias_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    if needs_singleton {
        apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    }
    if needs_clamp {
        apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    }
    if needs_simple_expr {
        apply_rewrite_simple_expr_helper_calls_ir(
            &mut program,
            &simple_helpers,
            &simple_helper_names,
            None,
            size_controlled_simple_expr,
        );
    }
    profile.simple_expr_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    if needs_tail {
        apply_collapse_identical_if_else_tail_assignments_late_ir(&mut program);
    }
    profile.tail_elapsed_ns = started.elapsed().as_nanos();

    (program.into_lines(), profile)
}

pub(crate) fn run_secondary_alias_simple_expr_bundle_ir(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    size_controlled_simple_expr: bool,
) -> (Vec<String>, SecondaryAliasSimpleExprBundleProfile) {
    let needs_alias = has_arg_alias_cleanup_candidates_ir(&lines);
    let needs_singleton = has_singleton_assign_slice_scalar_edit_candidates_ir(&lines);
    let needs_clamp = has_trivial_scalar_clamp_wrapper_candidates_ir(&lines);
    let maybe_simple_expr_helpers = has_simple_expr_helper_candidates_ir(&lines);
    let needs_tail = has_identical_if_else_tail_assign_candidates_ir(&lines);
    if !needs_alias && !needs_singleton && !needs_clamp && !maybe_simple_expr_helpers && !needs_tail
    {
        return (lines, SecondaryAliasSimpleExprBundleProfile::default());
    }
    let helpers = if maybe_simple_expr_helpers {
        collect_simple_expr_helpers_ir(&lines, pure_user_calls)
    } else {
        FxHashMap::default()
    };
    let helper_names: Vec<&str> = helpers.keys().map(String::as_str).collect();
    let needs_simple_expr = !helpers.is_empty();
    if !needs_alias && !needs_singleton && !needs_clamp && !needs_simple_expr && !needs_tail {
        return (lines, SecondaryAliasSimpleExprBundleProfile::default());
    }

    let mut profile = SecondaryAliasSimpleExprBundleProfile::default();
    let bundle_started = std::time::Instant::now();
    let mut program = EmittedProgram::parse(&lines);
    let alias_started = std::time::Instant::now();
    if needs_alias {
        apply_strip_unused_arg_aliases_ir(&mut program);
        apply_rewrite_readonly_param_aliases_ir(&mut program);
        apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
        apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    }
    profile.alias_elapsed_ns = alias_started.elapsed().as_nanos();
    let simple_started = std::time::Instant::now();
    if needs_singleton {
        apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    }
    if needs_clamp {
        apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    }
    if needs_simple_expr {
        apply_rewrite_simple_expr_helper_calls_ir(
            &mut program,
            &helpers,
            &helper_names,
            None,
            size_controlled_simple_expr,
        );
    }
    profile.simple_expr_elapsed_ns = simple_started.elapsed().as_nanos();
    let tail_started = std::time::Instant::now();
    if needs_tail {
        apply_collapse_identical_if_else_tail_assignments_late_ir(&mut program);
    }
    profile.tail_elapsed_ns = tail_started.elapsed().as_nanos();
    let parse_overhead = bundle_started.elapsed().as_nanos().saturating_sub(
        profile.alias_elapsed_ns + profile.simple_expr_elapsed_ns + profile.tail_elapsed_ns,
    );
    profile.alias_elapsed_ns += parse_overhead;
    (program.into_lines(), profile)
}
