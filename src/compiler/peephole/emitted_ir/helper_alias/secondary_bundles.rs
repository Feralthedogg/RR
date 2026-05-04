use super::*;
#[derive(Default)]
pub(crate) struct SecondaryMetricBundleProfile {
    pub(crate) post_wrapper_elapsed_ns: u128,
    pub(crate) metric_elapsed_ns: u128,
}

pub(crate) fn run_post_passthrough_metric_bundle_ir(
    lines: Vec<String>,
) -> (Vec<String>, SecondaryMetricBundleProfile) {
    let needs_arg_return_wrapper = has_arg_return_wrapper_candidates_ir(&lines);
    let needs_passthrough_return_wrapper = has_passthrough_return_wrapper_candidates_ir(&lines);
    let needs_dot_product_wrapper = has_trivial_dot_product_wrapper_candidates_ir(&lines);
    let needs_wrapper =
        needs_arg_return_wrapper || needs_passthrough_return_wrapper || needs_dot_product_wrapper;
    let maybe_passthrough_helpers = has_passthrough_helper_candidates_ir(&lines);
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let maybe_metric_helpers = has_metric_helper_candidates_ir(&lines);
    let helpers = if maybe_metric_helpers {
        collect_metric_helpers_ir(&lines)
    } else {
        FxHashMap::default()
    };
    let needs_metric = !helpers.is_empty();
    if !needs_wrapper && !maybe_passthrough_helpers && !needs_floor && !needs_metric {
        return (lines, SecondaryMetricBundleProfile::default());
    }

    let mut profile = SecondaryMetricBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);
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
    if maybe_passthrough_helpers {
        let passthrough = collect_passthrough_helpers_from_program_ir(&program);
        if !passthrough.is_empty() {
            apply_rewrite_passthrough_helper_calls_ir(&mut program, &passthrough);
        }
    }
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    profile.post_wrapper_elapsed_ns = started.elapsed().as_nanos();
    if needs_metric {
        let started = std::time::Instant::now();
        let mut stmt_temp_counter = 0usize;
        apply_rewrite_metric_helper_statement_calls_ir(
            &mut program,
            &helpers,
            &mut stmt_temp_counter,
        );
        let mut return_temp_counter = 0usize;
        apply_rewrite_metric_helper_return_calls_ir(
            &mut program,
            &helpers,
            &mut return_temp_counter,
        );
        profile.metric_elapsed_ns = started.elapsed().as_nanos();
    }
    (program.into_lines(), profile)
}

pub(crate) fn run_passthrough_secondary_bundle_ir(
    lines: Vec<String>,
) -> (Vec<String>, SecondaryMetricBundleProfile) {
    let needs_arg_return_wrapper = has_arg_return_wrapper_candidates_ir(&lines);
    let needs_passthrough_return_wrapper = has_passthrough_return_wrapper_candidates_ir(&lines);
    let needs_dot_product_wrapper = has_trivial_dot_product_wrapper_candidates_ir(&lines);
    let needs_wrapper =
        needs_arg_return_wrapper || needs_passthrough_return_wrapper || needs_dot_product_wrapper;
    let needs_passthrough_helpers = has_passthrough_helper_candidates_ir(&lines);
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let needs_copy = has_inlined_copy_vec_sequence_candidates_ir(&lines);
    let maybe_metric_helpers = has_metric_helper_candidates_ir(&lines);
    let helpers = if maybe_metric_helpers {
        collect_metric_helpers_ir(&lines)
    } else {
        FxHashMap::default()
    };
    let needs_metric = !helpers.is_empty();
    if !needs_wrapper && !needs_passthrough_helpers && !needs_floor && !needs_copy && !needs_metric
    {
        return (lines, SecondaryMetricBundleProfile::default());
    }

    let mut profile = SecondaryMetricBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);
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
            &helpers,
            &mut stmt_temp_counter,
        );
        let mut return_temp_counter = 0usize;
        apply_rewrite_metric_helper_return_calls_ir(
            &mut program,
            &helpers,
            &mut return_temp_counter,
        );
        profile.metric_elapsed_ns = started.elapsed().as_nanos();
    }
    (program.into_lines(), profile)
}

pub(crate) fn collapse_trivial_scalar_clamp_wrappers_ir(lines: Vec<String>) -> Vec<String> {
    if !has_trivial_scalar_clamp_wrapper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    program.into_lines()
}
