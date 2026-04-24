pub(in super::super) fn rewrite_dead_zero_loop_seeds_before_for_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_dead_zero_loop_seed_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_dead_zero_loop_seeds_before_for_ir(&mut program);
    program.into_lines()
}

fn apply_rewrite_dead_zero_loop_seeds_before_for_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut removable = vec![false; function.body.len()];
        for idx in 0..function.body.len() {
            let EmittedStmtKind::Assign { lhs, rhs } = &function.body[idx].kind else {
                continue;
            };
            let seed = rhs.trim();
            if seed != "0" && seed != "1" {
                continue;
            }
            let Some(for_idx) = ((idx + 1)..function.body.len()).take(12).find(|line_idx| {
                matches!(
                    &function.body[*line_idx].kind,
                    EmittedStmtKind::ForSeqLen { iter_var, .. } if iter_var == lhs
                )
            }) else {
                continue;
            };
            let used_before_for = function.body[(idx + 1)..for_idx]
                .iter()
                .any(|stmt| stmt.mentions_ident(lhs));
            if !used_before_for {
                removable[idx] = true;
            }
        }
        function.body = function
            .body
            .drain(..)
            .enumerate()
            .filter_map(|(idx, stmt)| (!removable[idx]).then_some(stmt))
            .collect();
    }
}

fn expr_is_trivial_passthrough_setup_rhs_ir(rhs: &str) -> bool {
    let rhs = rhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || expr_is_fresh_allocation_like(rhs, &FxHashSet::default())
        || rhs
            .strip_prefix("length(")
            .and_then(|s| s.strip_suffix(')'))
            .is_some_and(|inner| plain_ident_re().is_some_and(|re| re.is_match(inner.trim())))
}

fn apply_strip_arg_aliases_in_trivial_return_wrappers_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let return_line = function.body[return_idx].text.trim().to_string();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            continue;
        };

        let mut aliases = FxHashMap::default();
        let mut trivial = true;
        for stmt in function.body.iter().take(return_idx) {
            match &stmt.kind {
                EmittedStmtKind::Blank | EmittedStmtKind::BlockClose => continue,
                EmittedStmtKind::OtherOpen if stmt.text.trim() == "{" => continue,
                EmittedStmtKind::Assign { lhs, rhs } => {
                    if lhs.starts_with(".arg_")
                        && plain_ident_re().is_some_and(|re| re.is_match(rhs))
                    {
                        aliases.insert(lhs.clone(), rhs.clone());
                    } else {
                        trivial = false;
                        break;
                    }
                }
                _ => {
                    trivial = false;
                    break;
                }
            }
        }
        if !trivial || aliases.is_empty() {
            continue;
        }
        let rewritten = normalize_expr_with_aliases(inner, &aliases);
        if rewritten != inner {
            let indent = function.body[return_idx].indent();
            function.body[return_idx].replace_text(format!("{indent}return({rewritten})"));
            for stmt in function.body.iter_mut().take(return_idx) {
                if matches!(&stmt.kind, EmittedStmtKind::Assign { lhs, .. } if lhs.starts_with(".arg_"))
                {
                    stmt.clear();
                }
            }
        }
    }
}

pub(in super::super) fn strip_arg_aliases_in_trivial_return_wrappers_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    program.into_lines()
}

fn apply_collapse_trivial_passthrough_return_wrappers_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let return_line = function.body[return_idx].text.trim().to_string();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let mut last_assign_to_return: Option<(usize, String)> = None;
        let mut trivial = true;
        for (idx, stmt) in function.body.iter().enumerate().take(return_idx) {
            match &stmt.kind {
                EmittedStmtKind::Blank | EmittedStmtKind::BlockClose => continue,
                EmittedStmtKind::OtherOpen if stmt.text.trim() == "{" => continue,
                EmittedStmtKind::Assign { lhs, rhs } => {
                    if lhs == inner && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                        last_assign_to_return = Some((idx, rhs.clone()));
                    } else if !expr_is_trivial_passthrough_setup_rhs_ir(rhs) {
                        trivial = false;
                        break;
                    }
                }
                _ => {
                    trivial = false;
                    break;
                }
            }
        }
        let Some((assign_idx, passthrough_ident)) = last_assign_to_return else {
            continue;
        };
        if !trivial {
            continue;
        }

        let indent = function.body[return_idx].indent();
        function.body[return_idx].replace_text(format!("{indent}return({passthrough_ident})"));
        for stmt in function.body.iter_mut().take(return_idx) {
            match stmt.kind {
                EmittedStmtKind::Blank | EmittedStmtKind::BlockClose => {}
                EmittedStmtKind::OtherOpen if stmt.text.trim() == "{" => {}
                _ => stmt.clear(),
            }
        }
        function.body[assign_idx].clear();
    }
}

pub(in super::super) fn collapse_trivial_passthrough_return_wrappers_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_passthrough_return_wrapper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    program.into_lines()
}

pub(in super::super) fn run_return_wrapper_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_return_wrapper_candidates_ir(&lines)
        && !has_passthrough_return_wrapper_candidates_ir(&lines)
        && !has_trivial_dot_product_wrapper_candidates_ir(&lines)
    {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    program.into_lines()
}

fn collect_passthrough_helpers_from_program_ir(
    program: &EmittedProgram,
) -> FxHashMap<String, String> {
    let mut out = FxHashMap::default();
    for item in &program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, _params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let significant: Vec<&EmittedStmt> = function
            .body
            .iter()
            .filter(|stmt| {
                !matches!(
                    stmt.kind,
                    EmittedStmtKind::Blank | EmittedStmtKind::BlockClose
                )
            })
            .collect();
        if significant.len() != 1 {
            continue;
        }
        let stmt = significant[0];
        let trimmed = stmt.text.trim();
        let Some(inner) = trimmed
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };
        if plain_ident_re().is_some_and(|re| re.is_match(inner)) {
            out.insert(fn_name, inner.to_string());
        }
    }
    out
}

fn has_arg_return_wrapper_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let Some(return_idx) = func.return_idx else {
                return false;
            };
            let return_line = lines[return_idx].trim();
            let Some(inner) = return_line
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
            else {
                return false;
            };
            let mut aliases = FxHashMap::default();
            let mut saw_alias = false;
            for line in &lines[func.body_start..return_idx] {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    continue;
                }
                let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                    return false;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                if lhs.starts_with(".arg_") && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                    aliases.insert(lhs.to_string(), rhs.to_string());
                    saw_alias = true;
                } else if !expr_is_trivial_passthrough_setup_rhs_ir(rhs) {
                    return false;
                }
            }
            saw_alias && normalize_expr_with_aliases(inner, &aliases) != inner
        })
}

fn has_passthrough_return_wrapper_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let Some(return_idx) = func.return_idx else {
                return false;
            };
            let return_line = lines[return_idx].trim();
            let Some(inner) = return_line
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::trim)
            else {
                return false;
            };
            let mut last_assign_to_return = false;
            for line in &lines[func.body_start..return_idx] {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    continue;
                }
                let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                    return false;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                if lhs == inner && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                    last_assign_to_return = true;
                } else if !expr_is_trivial_passthrough_setup_rhs_ir(rhs) {
                    return false;
                }
            }
            last_assign_to_return
        })
}

fn has_trivial_dot_product_wrapper_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            func.params.len() == 3
                && lines[func.body_start..=func.end]
                    .iter()
                    .any(|line| line.trim() == "repeat {")
                && lines[func.body_start..=func.end]
                    .iter()
                    .any(|line| line.contains(" * "))
                && func
                    .return_idx
                    .and_then(|idx| lines.get(idx))
                    .is_some_and(|line| line.trim().starts_with("return("))
        })
}

fn has_passthrough_helper_definitions_with_calls_ir(lines: &[String]) -> bool {
    let candidate_names: Vec<String> = build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .filter_map(|func| {
            let fn_name = func.name?;
            let significant: Vec<&str> = lines[func.body_start..=func.end]
                .iter()
                .map(|line| line.trim())
                .filter(|trimmed| !trimmed.is_empty() && *trimmed != "{" && *trimmed != "}")
                .collect();
            if significant.len() != 1 {
                return None;
            }
            let inner = significant[0]
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::trim)?;
            plain_ident_re()
                .is_some_and(|re| re.is_match(inner))
                .then_some(fn_name)
        })
        .collect();

    !candidate_names.is_empty()
        && lines.iter().any(|line| {
            candidate_names
                .iter()
                .any(|name| line_has_helper_callsite_ir(line, name))
        })
}

fn has_passthrough_helper_candidates_ir(lines: &[String]) -> bool {
    has_passthrough_helper_definitions_with_calls_ir(lines)
}

fn build_block_end_map_ir(lines: &[String]) -> Vec<Option<usize>> {
    let mut out = vec![None; lines.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let (opens, closes) = count_unquoted_braces(line.trim());
        for _ in 0..closes {
            let Some(open_idx) = stack.pop() else {
                break;
            };
            out[open_idx] = Some(idx);
        }
        for _ in 0..opens {
            stack.push(idx);
        }
    }
    out
}

fn has_dead_zero_loop_seed_candidates_ir(lines: &[String]) -> bool {
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if rhs != "0" && rhs != "1" {
            continue;
        }
        let Some(for_idx) = ((idx + 1)..lines.len()).take(12).find(|line_idx| {
            parse_for_seq_len_header(lines[*line_idx].trim())
                .is_some_and(|(iter_var, _)| iter_var == lhs)
        }) else {
            continue;
        };
        let used_before_for = lines[(idx + 1)..for_idx]
            .iter()
            .any(|line| expr_idents(line.trim()).iter().any(|ident| ident == lhs));
        if !used_before_for {
            return true;
        }
    }
    false
}

fn has_terminal_repeat_next_candidates_ir(lines: &[String]) -> bool {
    let block_end_map = build_block_end_map_ir(lines);
    for (idx, line) in lines.iter().enumerate() {
        if line.trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = block_end_map.get(idx).and_then(|entry| *entry) else {
            continue;
        };
        let prev_non_empty = ((idx + 1)..end_idx).rev().find_map(|line_idx| {
            let text = lines[line_idx].trim();
            (!text.is_empty()).then_some(text)
        });
        if prev_non_empty == Some("next") {
            return true;
        }
    }
    false
}

fn has_identical_if_else_tail_assign_candidates_ir(lines: &[String]) -> bool {
    let block_end_map = build_block_end_map_ir(lines);
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            continue;
        }
        let Some(end_idx) = block_end_map.get(idx).and_then(|entry| *entry) else {
            continue;
        };
        let Some(else_idx) =
            ((idx + 1)..end_idx).find(|line_idx| lines[*line_idx].trim() == "} else {")
        else {
            continue;
        };
        let then_tail = ((idx + 1)..else_idx).rev().find_map(|line_idx| {
            let text = lines[line_idx].trim();
            (!text.is_empty()).then_some(text)
        });
        let else_tail = ((else_idx + 1)..end_idx).rev().find_map(|line_idx| {
            let text = lines[line_idx].trim();
            (!text.is_empty()).then_some(text)
        });
        let (Some(then_tail), Some(else_tail)) = (then_tail, else_tail) else {
            continue;
        };
        if then_tail == else_tail && assign_re().and_then(|re| re.captures(then_tail)).is_some() {
            return true;
        }
    }
    false
}

fn has_tail_assign_slice_return_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let Some(return_idx) = func.return_idx else {
                return false;
            };
            let ret_var = lines[return_idx]
                .trim()
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::trim);
            let Some(ret_var) = ret_var else {
                return false;
            };
            ((func.body_start)..return_idx)
                .rev()
                .find_map(|line_idx| {
                    let text = lines[line_idx].trim();
                    if text.is_empty() || text == "{" || text == "}" {
                        return None;
                    }
                    Some(text)
                })
                .is_some_and(|prev| {
                    assign_re()
                        .and_then(|re| re.captures(prev))
                        .is_some_and(|caps| {
                            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                            lhs == ret_var && rhs.contains("rr_assign_slice(")
                        })
                })
        })
}

fn has_unreachable_sym_helper_candidates_ir(lines: &[String]) -> bool {
    let functions = build_function_text_index(lines, parse_function_header_ir);
    let sym_funcs: FxHashMap<String, IndexedFunction> = functions
        .iter()
        .filter_map(|func| {
            let name = func.name.clone()?;
            name.starts_with("Sym_").then_some((name, func.clone()))
        })
        .collect();
    if sym_funcs.len() <= 1 {
        return false;
    }

    let mut in_function = vec![false; lines.len()];
    for func in &functions {
        for idx in func.start..=func.end {
            if idx < in_function.len() {
                in_function[idx] = true;
            }
        }
    }

    let mut roots = FxHashSet::default();
    if sym_funcs.contains_key("Sym_top_0") {
        roots.insert("Sym_top_0".to_string());
    }
    for (idx, line) in lines.iter().enumerate() {
        if in_function[idx] {
            continue;
        }
        for name in unquoted_sym_refs(line) {
            if sym_funcs.contains_key(&name) {
                roots.insert(name);
            }
        }
    }
    if roots.is_empty() {
        return false;
    }

    let sym_top_is_empty_entrypoint = |func: &IndexedFunction| {
        let Some(return_idx) = func.return_idx else {
            return false;
        };
        let mut saw_return_null = false;
        for line in &lines[func.body_start..=return_idx] {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            if trimmed == "return(NULL)" {
                saw_return_null = true;
                continue;
            }
            if !unquoted_sym_refs(trimmed).is_empty() {
                return false;
            }
            return false;
        }
        saw_return_null
    };
    if roots.len() == 1
        && roots.contains("Sym_top_0")
        && sym_funcs
            .get("Sym_top_0")
            .is_some_and(sym_top_is_empty_entrypoint)
    {
        return false;
    }

    let mut reachable = roots.clone();
    let mut work: Vec<String> = roots.into_iter().collect();
    while let Some(name) = work.pop() {
        let Some(func) = sym_funcs.get(&name) else {
            continue;
        };
        for line in &lines[func.body_start..=func.end] {
            for callee in unquoted_sym_refs(line) {
                if sym_funcs.contains_key(&callee) && reachable.insert(callee.clone()) {
                    work.push(callee);
                }
            }
        }
    }

    reachable.len() < sym_funcs.len()
}

fn line_has_helper_callsite_ir(line: &str, helper_name: &str) -> bool {
    line.contains(&format!("{helper_name}("))
        && !line.contains(&format!("{helper_name} <- function("))
}

fn has_metric_helper_definitions_with_calls_ir(lines: &[String]) -> bool {
    let candidate_names: Vec<String> = build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .filter_map(|func| {
            let fn_name = func.name?;
            if func.params.len() != 2 {
                return None;
            }
            let body_lines: Vec<String> = lines
                .iter()
                .take(func.end)
                .skip(func.body_start)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s != "{" && s != "}")
                .collect();
            if body_lines.len() < 3 || body_lines.len() > 5 {
                return None;
            }
            let name_param = func.params[0].clone();
            let value_param = func.params[1].clone();
            let return_line = body_lines.last()?;
            if return_line != &format!("return({value_param})") {
                return None;
            }
            let print_name_idx = body_lines
                .iter()
                .position(|line| line == &format!("print({name_param})"));
            let print_value_idx = body_lines
                .iter()
                .position(|line| line == &format!("print({value_param})"));
            let (Some(print_name_idx), Some(print_value_idx)) = (print_name_idx, print_value_idx)
            else {
                return None;
            };
            (print_name_idx < print_value_idx && print_value_idx + 1 == body_lines.len() - 1)
                .then_some(fn_name)
        })
        .collect();

    !candidate_names.is_empty()
        && lines.iter().any(|line| {
            candidate_names
                .iter()
                .any(|name| line_has_helper_callsite_ir(line, name))
        })
}

fn has_simple_expr_helper_definitions_with_calls_ir(lines: &[String]) -> bool {
    let candidate_names: Vec<String> = build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .filter_map(|func| {
            let fn_name = func.name?;
            if !fn_name.starts_with("Sym_") {
                return None;
            }
            let return_idx = func.return_idx?;
            let return_line = lines[return_idx].trim();
            let return_expr = return_line
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::trim)?;
            let mut simple = true;
            for line in lines.iter().take(return_idx).skip(func.body_start) {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    continue;
                }
                let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                    simple = false;
                    break;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                    simple = false;
                    break;
                }
            }
            (simple && !return_expr.contains(&format!("{fn_name}("))).then_some(fn_name)
        })
        .collect();

    !candidate_names.is_empty()
        && lines.iter().any(|line| {
            candidate_names
                .iter()
                .any(|name| line_has_helper_callsite_ir(line, name))
        })
}

fn has_metric_helper_candidates_ir(lines: &[String]) -> bool {
    has_metric_helper_definitions_with_calls_ir(lines)
}

fn has_literal_field_get_candidates_ir(lines: &[String]) -> bool {
    let Some(re) = literal_field_get_re() else {
        return false;
    };
    lines.iter().any(|line| re.is_match(line))
}

fn has_literal_named_list_candidates_ir(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        line.contains("rr_named_list(")
            && !line.contains("rr_named_list <- function")
            && rewrite_literal_named_list_line_ir(line) != *line
    })
}

fn has_simple_expr_helper_candidates_ir(lines: &[String]) -> bool {
    has_simple_expr_helper_definitions_with_calls_ir(lines)
}

fn apply_rewrite_passthrough_helper_calls_ir(
    program: &mut EmittedProgram,
    passthrough: &FxHashMap<String, String>,
) {
    if passthrough.is_empty() {
        return;
    }
    for item in &mut program.items {
        match item {
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    let trimmed = stmt.text.trim().to_string();
                    let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
                        continue;
                    };
                    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let Some((callee, args_str)) = rhs.split_once('(') else {
                        continue;
                    };
                    let Some(args_inner) = args_str.strip_suffix(')') else {
                        continue;
                    };
                    let Some(param_name) = passthrough.get(callee.trim()) else {
                        continue;
                    };
                    let Some(args) = split_top_level_args(args_inner) else {
                        continue;
                    };
                    if args.len() != 1 || param_name.is_empty() {
                        continue;
                    }
                    let indent = stmt.indent();
                    stmt.replace_text(format!("{indent}{lhs} <- {}", args[0].trim()));
                }
            }
            EmittedItem::Raw(line) => {
                let trimmed = line.trim().to_string();
                let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
                    continue;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                let Some((callee, args_str)) = rhs.split_once('(') else {
                    continue;
                };
                let Some(args_inner) = args_str.strip_suffix(')') else {
                    continue;
                };
                let Some(param_name) = passthrough.get(callee.trim()) else {
                    continue;
                };
                let Some(args) = split_top_level_args(args_inner) else {
                    continue;
                };
                if args.len() != 1 || param_name.is_empty() {
                    continue;
                }
                let indent_len = line.len().saturating_sub(line.trim_start().len());
                let indent = &line[..indent_len];
                *line = format!("{indent}{lhs} <- {}", args[0].trim());
            }
        }
    }
}

fn apply_collapse_inlined_copy_vec_sequences_ir(program: &mut EmittedProgram) {
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
                    (dest == out_var
                        && start == i_var
                        && end == n_var
                        && rest.starts_with("rr_index1_read_vec("))
                    .then_some(rest.to_string())
                } else {
                    None
                }
            }) else {
                continue;
            };
            if out_var != out_replay_lhs
                || target_var != out_var
                || n_rhs != format!("length({out_rhs})")
                || i_rhs != "1"
                || !target_rhs.starts_with("rr_assign_slice(")
                || !src_var.contains(out_rhs)
            {
                continue;
            }
            function.body[idx].clear();
            function.body[idx + 2].clear();
            function.body[idx + 3].clear();
        }
    }
}

fn has_inlined_copy_vec_sequence_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let body = &lines[func.body_start..=func.end];
            body.iter().any(|line| line.contains("inlined_"))
                && body.iter().any(|line| line.contains("rr_assign_slice("))
                && body.iter().any(|line| line.contains("length("))
                && body.iter().any(|line| line.contains("rep.int("))
        })
}
