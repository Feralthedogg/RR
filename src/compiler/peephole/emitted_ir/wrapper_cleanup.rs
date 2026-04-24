
fn has_trivial_scalar_clamp_wrapper_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let significant: Vec<&str> = lines[func.body_start..=func.end]
                .iter()
                .map(|line| line.trim())
                .filter(|trimmed| !trimmed.is_empty() && *trimmed != "{" && *trimmed != "}")
                .collect();
            if significant.len() != 6 {
                return false;
            }
            significant[0].contains(" <- ")
                && significant[1].starts_with("if ((")
                && significant[1].contains(" < ")
                && significant[2].contains(" <- ")
                && significant[3].starts_with("if ((")
                && significant[3].contains(" > ")
                && significant[4].contains(" <- ")
                && significant[5].starts_with("return(")
        })
}

fn apply_collapse_trivial_scalar_clamp_wrappers_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let significant: Vec<(usize, String)> = function
            .body
            .iter()
            .enumerate()
            .filter_map(|(idx, stmt)| {
                let trimmed = stmt.text.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    None
                } else {
                    Some((idx, trimmed.to_string()))
                }
            })
            .collect();
        if significant.len() != 6 {
            continue;
        }
        let Some((tmp, init_expr)) = significant[0]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim().to_string(), rhs.trim().to_string()))
        else {
            continue;
        };
        let Some((assign_lo_lhs, lo_expr)) = significant[2]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim().to_string(), rhs.trim().to_string()))
        else {
            continue;
        };
        let Some((assign_hi_lhs, hi_expr)) = significant[4]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim().to_string(), rhs.trim().to_string()))
        else {
            continue;
        };
        if assign_lo_lhs != tmp
            || assign_hi_lhs != tmp
            || significant[5].1 != format!("return({tmp})")
        {
            continue;
        }
        let first_guard_ok = significant[1].1 == format!("if (({init_expr} < {lo_expr})) {{")
            || significant[1].1 == format!("if (({tmp} < {lo_expr})) {{");
        let second_guard_ok = significant[3].1 == format!("if (({tmp} > {hi_expr})) {{");
        if !first_guard_ok || !second_guard_ok {
            continue;
        }

        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let open_idx = function
            .body
            .iter()
            .position(|stmt| stmt.text.trim() == "{")
            .unwrap_or(0);
        let indent = function.body[return_idx].indent();
        if open_idx < function.body.len() {
            function.body[open_idx].replace_text("{".to_string());
        }
        if open_idx + 1 < function.body.len() {
            function.body[open_idx + 1].replace_text(format!(
                "{indent}return(pmin(pmax({init_expr}, {lo_expr}), {hi_expr}))"
            ));
        }
        let clear_end = function.body.len().saturating_sub(1);
        for stmt in function
            .body
            .iter_mut()
            .skip(open_idx + 2)
            .take(clear_end.saturating_sub(open_idx + 2))
        {
            stmt.clear();
        }
    }
}

fn parse_accumulate_product_line_ir(line: &str, acc: &str) -> Option<(String, String, String)> {
    let pattern = format!(
        r"^(?P<lhs>{}) <- \({} \+ \((?P<a>{})\[(?P<idx_a>{})\] \* (?P<b>{})\[(?P<idx_b>{})\]\)\)$",
        IDENT_PATTERN,
        regex::escape(acc),
        IDENT_PATTERN,
        IDENT_PATTERN,
        IDENT_PATTERN,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    let lhs = caps.name("lhs")?.as_str().trim();
    let lhs_vec = caps.name("a")?.as_str().trim();
    let rhs_vec = caps.name("b")?.as_str().trim();
    let idx_a = caps.name("idx_a")?.as_str().trim();
    let idx_b = caps.name("idx_b")?.as_str().trim();
    if lhs != acc || idx_a != idx_b {
        return None;
    }
    Some((lhs_vec.to_string(), rhs_vec.to_string(), idx_a.to_string()))
}

fn apply_collapse_trivial_dot_product_wrappers_ir(program: &mut EmittedProgram) {
    fn is_zero_literal(expr: &str) -> bool {
        matches!(expr.trim(), "0" | "0L" | "0.0")
    }

    fn is_one_literal(expr: &str) -> bool {
        matches!(expr.trim(), "1" | "1L" | "1.0")
    }

    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((_name, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        if params.len() != 3 {
            continue;
        }

        let significant: Vec<(usize, String)> = function
            .body
            .iter()
            .enumerate()
            .filter_map(|(idx, stmt)| {
                let trimmed = stmt.text.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    None
                } else {
                    Some((idx, trimmed.to_string()))
                }
            })
            .collect();
        if significant.len() < 7 {
            continue;
        }

        let mut aliases: FxHashMap<String, String> = params
            .iter()
            .cloned()
            .map(|param| (param.clone(), param))
            .collect();
        let mut idx = 0usize;
        while idx < significant.len() {
            let Some((lhs, rhs)) = significant[idx]
                .1
                .split_once(" <- ")
                .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
            else {
                break;
            };
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                break;
            }
            if params.iter().any(|param| param == rhs) {
                aliases.insert(lhs.to_string(), rhs.to_string());
                idx += 1;
                continue;
            }
            break;
        }

        if idx + 6 >= significant.len() {
            continue;
        }
        let Some((acc, init_expr)) = significant[idx]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            continue;
        };
        let Some((iter_var, iter_init)) = significant[idx + 1]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            continue;
        };
        if !plain_ident_re().is_some_and(|re| re.is_match(acc))
            || !plain_ident_re().is_some_and(|re| re.is_match(iter_var))
            || !is_zero_literal(init_expr)
            || !is_one_literal(iter_init)
            || significant[idx + 2].1 != "repeat {"
        {
            continue;
        }

        let guard_line = format!("if (!({iter_var} <= {})) break", params[2]);
        let guard_line_with_alias = aliases.iter().find_map(|(alias, base)| {
            (base == &params[2] && alias != &params[2])
                .then(|| format!("if (!({iter_var} <= {alias})) break"))
        });
        if significant[idx + 3].1 != guard_line
            && guard_line_with_alias.as_deref() != Some(significant[idx + 3].1.as_str())
        {
            continue;
        }

        let mut product_idx = idx + 4;
        let mut index_ref = iter_var.to_string();
        if let Some((alias_lhs, alias_rhs)) = significant[product_idx]
            .1
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
            && alias_rhs == iter_var
            && plain_ident_re().is_some_and(|re| re.is_match(alias_lhs))
        {
            index_ref = alias_lhs.to_string();
            product_idx += 1;
        }
        if product_idx + 2 >= significant.len() {
            continue;
        }

        let Some((lhs_vec, rhs_vec, vec_index_ref)) =
            parse_accumulate_product_line_ir(&significant[product_idx].1, acc)
        else {
            continue;
        };
        let resolved_lhs = aliases
            .get(&lhs_vec)
            .map(String::as_str)
            .unwrap_or(lhs_vec.as_str());
        let resolved_rhs = aliases
            .get(&rhs_vec)
            .map(String::as_str)
            .unwrap_or(rhs_vec.as_str());
        if vec_index_ref != index_ref
            || resolved_lhs != params[0]
            || resolved_rhs != params[1]
            || !matches!(
                significant[product_idx + 1].1.as_str(),
                line if line == format!("{iter_var} <- ({iter_var} + 1)")
                    || line == format!("{iter_var} <- ({iter_var} + 1L)")
                    || line == format!("{iter_var} <- ({iter_var} + 1.0)")
            )
            || significant[product_idx + 2].1 != "next"
            || significant.last().map(|(_, line)| line.as_str()) != Some(&format!("return({acc})"))
            || significant.len() != product_idx + 4
        {
            continue;
        }

        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let open_idx = function
            .body
            .iter()
            .position(|stmt| stmt.text.trim() == "{")
            .unwrap_or(0);
        let indent = function.body[return_idx].indent();
        if open_idx < function.body.len() {
            function.body[open_idx].replace_text("{".to_string());
        }
        if open_idx + 1 < function.body.len() {
            function.body[open_idx + 1].replace_text(format!(
                "{indent}return(sum(({}[seq_len({})] * {}[seq_len({})])))",
                params[0], params[2], params[1], params[2]
            ));
        }
        let clear_end = function.body.len().saturating_sub(1);
        for stmt in function
            .body
            .iter_mut()
            .skip(open_idx + 2)
            .take(clear_end.saturating_sub(open_idx + 2))
        {
            stmt.clear();
        }
    }
}

pub(in super::super) fn collapse_trivial_dot_product_wrappers_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_trivial_dot_product_wrapper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    program.into_lines()
}

fn scalar_rhs_from_singleton_rest_ir(rest: &str) -> Option<String> {
    let trimmed = rest.trim();
    if let Some(inner) = trimmed
        .strip_prefix("rep.int(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let args = split_top_level_args(inner)?;
        if args.len() == 2 && literal_one_re().is_some_and(|re| re.is_match(args[1].trim())) {
            return Some(args[0].trim().to_string());
        }
    }
    (scalar_lit_re().is_some_and(|re| re.is_match(trimmed))
        || plain_ident_re().is_some_and(|re| re.is_match(trimmed)))
    .then_some(trimmed.to_string())
}

fn collapse_singleton_assign_slice_scalar_stmt_text_ir(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let caps = assign_re().and_then(|re| re.captures(trimmed))?;
    let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
    let slice_caps = assign_slice_re().and_then(|re| re.captures(rhs))?;
    let dest = slice_caps
        .name("dest")
        .map(|m| m.as_str())
        .unwrap_or("")
        .trim();
    let start = slice_caps
        .name("start")
        .map(|m| m.as_str())
        .unwrap_or("")
        .trim();
    let end = slice_caps
        .name("end")
        .map(|m| m.as_str())
        .unwrap_or("")
        .trim();
    let rest = slice_caps
        .name("rest")
        .map(|m| m.as_str())
        .unwrap_or("")
        .trim();
    if lhs != dest || start != end {
        return None;
    }
    let scalar_rhs = scalar_rhs_from_singleton_rest_ir(rest)?;
    Some(format!(
        "{indent}{lhs} <- replace({dest}, {start}, {scalar_rhs})"
    ))
}

fn has_singleton_assign_slice_scalar_edit_candidates_ir(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        line.contains("rr_assign_slice(")
            && collapse_singleton_assign_slice_scalar_stmt_text_ir(line).is_some()
    })
}

pub(in super::super) fn collapse_singleton_assign_slice_scalar_edits_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_singleton_assign_slice_scalar_edit_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    program.into_lines()
}

fn apply_collapse_singleton_assign_slice_scalar_edits_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        match item {
            EmittedItem::Raw(line) => {
                if let Some(rewritten) = collapse_singleton_assign_slice_scalar_stmt_text_ir(line) {
                    *line = rewritten;
                }
            }
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    if let Some(rewritten) =
                        collapse_singleton_assign_slice_scalar_stmt_text_ir(&stmt.text)
                    {
                        stmt.replace_text(rewritten);
                    }
                }
            }
        }
    }
}

pub(in super::super) fn run_simple_expr_pre_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
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

pub(in super::super) fn run_simple_expr_cleanup_bundle_ir(
    mut lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    allowed_helpers: Option<&FxHashSet<String>>,
    rewrite_full_range_alias_reads: bool,
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
    let needs_full_range_alias_reads = rewrite_full_range_alias_reads
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
            allowed_helpers,
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

#[derive(Default)]
pub(in super::super) struct SecondaryAliasSimpleExprBundleProfile {
    pub(in super::super) alias_elapsed_ns: u128,
    pub(in super::super) simple_expr_elapsed_ns: u128,
    pub(in super::super) tail_elapsed_ns: u128,
}

#[derive(Default)]
pub(in super::super) struct SecondaryHelperIrBundleProfile {
    pub(in super::super) post_wrapper_elapsed_ns: u128,
    pub(in super::super) metric_elapsed_ns: u128,
    pub(in super::super) alias_elapsed_ns: u128,
    pub(in super::super) simple_expr_elapsed_ns: u128,
    pub(in super::super) tail_elapsed_ns: u128,
}

pub(in super::super) fn run_secondary_helper_ir_bundle(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
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

pub(in super::super) fn run_secondary_alias_simple_expr_bundle_ir(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
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
        apply_rewrite_simple_expr_helper_calls_ir(&mut program, &helpers, &helper_names, None);
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

fn apply_collapse_identical_if_else_tail_assignments_late_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut i = 0usize;
        while i < function.body.len() {
            if !matches!(function.body[i].kind, EmittedStmtKind::IfOpen) {
                i += 1;
                continue;
            }
            let Some((else_idx, end_idx)) = find_if_else_bounds_ir(&function.body, i) else {
                i += 1;
                continue;
            };

            let then_lines: Vec<usize> = ((i + 1)..else_idx)
                .filter(|idx| !function.body[*idx].text.trim().is_empty())
                .collect();
            let else_lines: Vec<usize> = ((else_idx + 1)..end_idx)
                .filter(|idx| !function.body[*idx].text.trim().is_empty())
                .collect();

            let mut t = then_lines.len();
            let mut e = else_lines.len();
            let mut shared = Vec::<(usize, usize, String)>::new();
            while t > 0 && e > 0 {
                let then_idx = then_lines[t - 1];
                let else_line_idx = else_lines[e - 1];
                let then_trimmed = function.body[then_idx].text.trim();
                let else_trimmed = function.body[else_line_idx].text.trim();
                if then_trimmed != else_trimmed {
                    break;
                }
                if assign_re()
                    .and_then(|re| re.captures(then_trimmed))
                    .is_none()
                {
                    break;
                }
                shared.push((then_idx, else_line_idx, then_trimmed.to_string()));
                t -= 1;
                e -= 1;
            }

            if shared.is_empty() {
                i = end_idx + 1;
                continue;
            }

            shared.reverse();
            let indent = function.body[i].indent();
            for (then_idx, else_idx_line, _) in &shared {
                function.body[*then_idx].clear();
                function.body[*else_idx_line].clear();
            }
            let mut insert_at = end_idx + 1;
            for (_, _, assign) in &shared {
                function
                    .body
                    .insert(insert_at, EmittedStmt::parse(&format!("{indent}{assign}")));
                insert_at += 1;
            }
            i = insert_at;
        }
    }
}

pub(in super::super) fn collapse_inlined_copy_vec_sequences_ir(lines: Vec<String>) -> Vec<String> {
    if !has_inlined_copy_vec_sequence_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let len = function.body.len();
        for idx in 0..len.saturating_sub(4) {
            let l0 = function.body[idx].text.trim().to_string();
            let l1 = function.body[idx + 1].text.trim().to_string();
            let l2 = function.body[idx + 2].text.trim().to_string();
            let l3 = function.body[idx + 3].text.trim().to_string();
            let l4 = function.body[idx + 4].text.trim().to_string();
            let Some(c0) = assign_re().and_then(|re| re.captures(&l0)) else {
                continue;
            };
            let Some(c1) = assign_re().and_then(|re| re.captures(&l1)) else {
                continue;
            };
            let Some(c2) = assign_re().and_then(|re| re.captures(&l2)) else {
                continue;
            };
            let Some(c3) = assign_re().and_then(|re| re.captures(&l3)) else {
                continue;
            };
            let Some(c4) = assign_re().and_then(|re| re.captures(&l4)) else {
                continue;
            };
            let n_var = c0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let n_rhs = c0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_var = c1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_rhs = c1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let i_var = c2.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let i_rhs = c2.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_replay_lhs = c3.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let src_rhs = c3.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let target_var = c4.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let target_rhs = c4.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let Some(src_var) = ({
                if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(src_rhs)) {
                    let dest = slice_caps
                        .name("dest")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim();
                    let start = slice_caps
                        .name("start")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim();
                    let end = slice_caps
                        .name("end")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim();
                    let rest = slice_caps
                        .name("rest")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim();
                    if dest == out_var
                        && start == i_var
                        && end == n_var
                        && plain_ident_re().is_some_and(|re| re.is_match(rest))
                    {
                        Some(rest.to_string())
                    } else {
                        None
                    }
                } else if plain_ident_re().is_some_and(|re| re.is_match(src_rhs)) {
                    Some(src_rhs.to_string())
                } else {
                    None
                }
            }) else {
                continue;
            };
            if !n_var.starts_with("inlined_")
                || !out_var.starts_with("inlined_")
                || !i_var.starts_with("inlined_")
                || out_replay_lhs != out_var
                || (target_rhs != out_var && target_rhs != src_var)
                || !literal_one_re().is_some_and(|re| re.is_match(i_rhs))
                || !n_rhs.starts_with("length(")
                || !out_rhs.starts_with("rep.int(0, ")
            {
                continue;
            }

            let mut final_assign_idx = None;
            for (search_idx, stmt) in function.body.iter().enumerate().skip(idx + 5) {
                let trimmed = stmt.text.trim();
                let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                    continue;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
                    continue;
                };
                let dest = slice_caps
                    .name("dest")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let start = slice_caps
                    .name("start")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let end = slice_caps
                    .name("end")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let rest = slice_caps
                    .name("rest")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if lhs == src_var
                    && dest == out_var
                    && start == i_var
                    && end == n_var
                    && rest == src_var
                {
                    final_assign_idx = Some(search_idx);
                    break;
                }
            }
            let Some(final_idx) = final_assign_idx else {
                continue;
            };
            let indent = function.body[idx + 4].indent();
            function.body[idx].clear();
            function.body[idx + 1].clear();
            function.body[idx + 2].clear();
            function.body[idx + 3].clear();
            function.body[idx + 4].replace_text(format!("{indent}{target_var} <- {src_var}"));
            let final_indent = function.body[final_idx].indent();
            function.body[final_idx]
                .replace_text(format!("{final_indent}{src_var} <- {target_var}"));
        }
    }
    program.into_lines()
}

pub(in super::super) fn strip_unreachable_sym_helpers_ir(lines: Vec<String>) -> Vec<String> {
    if !has_unreachable_sym_helper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unreachable_sym_helpers_ir(&mut program);
    program.into_lines()
}

fn previous_non_empty_stmt(body: &[EmittedStmt], idx: usize) -> Option<usize> {
    (0..idx).rev().find(|i| !body[*i].text.trim().is_empty())
}

fn find_if_else_bounds_ir(body: &[EmittedStmt], if_idx: usize) -> Option<(usize, usize)> {
    let mut depth = 1usize;
    let mut else_idx = None;
    for (idx, stmt) in body.iter().enumerate().skip(if_idx + 1) {
        match stmt.kind {
            EmittedStmtKind::ElseOpen if depth == 1 => {
                else_idx = Some(idx);
            }
            EmittedStmtKind::IfOpen
            | EmittedStmtKind::RepeatOpen
            | EmittedStmtKind::ForSeqLen { .. }
            | EmittedStmtKind::ForOpen
            | EmittedStmtKind::WhileOpen
            | EmittedStmtKind::OtherOpen => {
                depth += 1;
            }
            EmittedStmtKind::BlockClose => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return else_idx.map(|else_idx| (else_idx, idx));
                }
            }
            _ => {}
        }
    }
    None
}

include!("wrapper_tail_cleanup.rs");
