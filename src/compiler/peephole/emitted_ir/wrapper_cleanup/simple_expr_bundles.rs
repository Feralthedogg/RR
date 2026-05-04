use super::*;
pub(crate) fn run_simple_expr_pre_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let needs_singleton = has_singleton_assign_slice_scalar_edit_candidates_ir(&lines);
    let needs_clamp = has_trivial_scalar_clamp_wrapper_candidates_ir(&lines);
    if !needs_singleton && !needs_clamp {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_singleton {
        apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    }
    if needs_clamp {
        apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    }
    program.into_lines()
}

#[derive(Clone, Copy)]
pub(crate) struct SimpleExprCleanupConfig<'a> {
    pub(crate) allowed_helpers: Option<&'a FxHashSet<String>>,
    pub(crate) rewrite_full_range_alias_reads: bool,
    pub(crate) size_controlled: bool,
}

pub(crate) fn run_simple_expr_cleanup_bundle_ir(
    mut lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    config: SimpleExprCleanupConfig<'_>,
) -> Vec<String> {
    lines = rewrite_index_access_patterns(lines);
    let needs_arg_alias_cleanup = has_arg_alias_cleanup_candidates_ir(&lines);
    let needs_singleton = has_singleton_assign_slice_scalar_edit_candidates_ir(&lines);
    let needs_clamp = has_trivial_scalar_clamp_wrapper_candidates_ir(&lines);
    let maybe_simple_expr_helpers = has_simple_expr_helper_candidates_ir(&lines);
    let needs_tail = has_identical_if_else_tail_assign_candidates_ir(&lines);
    let needs_literal_field_get = has_literal_field_get_candidates_ir(&lines);
    let needs_literal_named_list = has_literal_named_list_candidates_ir(&lines);
    let needs_helper_param_trim = has_unused_helper_param_candidates_ir(&lines);
    let needs_full_range_alias_reads = config.rewrite_full_range_alias_reads
        && has_one_based_full_range_index_alias_read_candidates(&lines);
    if !needs_arg_alias_cleanup
        && !needs_singleton
        && !needs_clamp
        && !maybe_simple_expr_helpers
        && !needs_tail
        && !needs_literal_field_get
        && !needs_literal_named_list
        && !needs_helper_param_trim
        && !needs_full_range_alias_reads
    {
        return lines;
    }
    if !needs_arg_alias_cleanup
        && !needs_singleton
        && !needs_clamp
        && !maybe_simple_expr_helpers
        && !needs_tail
        && !needs_literal_field_get
        && !needs_literal_named_list
        && !needs_helper_param_trim
    {
        return rewrite_one_based_full_range_index_alias_reads(lines);
    }
    let helpers = if maybe_simple_expr_helpers {
        collect_simple_expr_helpers_ir(&lines, pure_user_calls)
    } else {
        FxHashMap::default()
    };
    let helper_names: Vec<&str> = helpers.keys().map(String::as_str).collect();
    let needs_simple_expr = !helpers.is_empty();
    if !needs_arg_alias_cleanup
        && !needs_singleton
        && !needs_clamp
        && !needs_simple_expr
        && !needs_tail
        && !needs_literal_field_get
        && !needs_literal_named_list
        && !needs_helper_param_trim
    {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_arg_alias_cleanup {
        apply_strip_unused_arg_aliases_ir(&mut program);
        apply_rewrite_readonly_param_aliases_ir(&mut program);
        apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
        apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    }
    if needs_helper_param_trim {
        apply_strip_unused_helper_params_ir(&mut program);
    }
    if needs_literal_field_get {
        let Some(re) = literal_field_get_re() else {
            return lines;
        };
        for item in &mut program.items {
            match item {
                EmittedItem::Raw(line) => {
                    *line = re
                        .replace_all(line, |caps: &Captures<'_>| {
                            let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                            let name = caps.name("name").map(|m| m.as_str()).unwrap_or("").trim();
                            format!(r#"{base}[["{name}"]]"#)
                        })
                        .to_string();
                }
                EmittedItem::Function(function) => {
                    for stmt in &mut function.body {
                        let rewritten = re
                            .replace_all(&stmt.text, |caps: &Captures<'_>| {
                                let base =
                                    caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                                let name =
                                    caps.name("name").map(|m| m.as_str()).unwrap_or("").trim();
                                format!(r#"{base}[["{name}"]]"#)
                            })
                            .to_string();
                        if rewritten != stmt.text {
                            stmt.replace_text(rewritten);
                        }
                    }
                }
            }
        }
    }
    if needs_literal_named_list {
        for item in &mut program.items {
            match item {
                EmittedItem::Raw(line) => {
                    if !line.contains("rr_named_list <- function") {
                        *line = rewrite_literal_named_list_line_ir(line);
                    }
                }
                EmittedItem::Function(function) => {
                    for stmt in &mut function.body {
                        let rewritten = rewrite_literal_named_list_line_ir(&stmt.text);
                        if rewritten != stmt.text {
                            stmt.replace_text(rewritten);
                        }
                    }
                }
            }
        }
    }
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
            config.allowed_helpers,
            config.size_controlled,
        );
    }
    if needs_tail {
        apply_collapse_identical_if_else_tail_assignments_late_ir(&mut program);
    }
    let out = program.into_lines();
    if needs_full_range_alias_reads {
        rewrite_one_based_full_range_index_alias_reads(out)
    } else {
        out
    }
}
