use super::*;
pub(crate) fn expr_is_trivial_passthrough_setup_rhs_ir(rhs: &str) -> bool {
    let rhs = rhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || expr_is_fresh_allocation_like(rhs, &FxHashSet::default())
        || rhs
            .strip_prefix("length(")
            .and_then(|s| s.strip_suffix(')'))
            .is_some_and(|inner| plain_ident_re().is_some_and(|re| re.is_match(inner.trim())))
}

pub(crate) fn apply_strip_arg_aliases_in_trivial_return_wrappers_ir(program: &mut EmittedProgram) {
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
                EmittedStmtKind::Assign { lhs, rhs }
                    if lhs.starts_with(".arg_")
                        && plain_ident_re().is_some_and(|re| re.is_match(rhs)) =>
                {
                    aliases.insert(lhs.clone(), rhs.clone());
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

pub(crate) fn strip_arg_aliases_in_trivial_return_wrappers_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    program.into_lines()
}

pub(crate) fn apply_collapse_trivial_passthrough_return_wrappers_ir(program: &mut EmittedProgram) {
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

pub(crate) fn collapse_trivial_passthrough_return_wrappers_ir(lines: Vec<String>) -> Vec<String> {
    if !has_passthrough_return_wrapper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    program.into_lines()
}

pub(crate) fn run_return_wrapper_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
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

pub(crate) fn has_arg_return_wrapper_candidates_ir(lines: &[String]) -> bool {
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

pub(crate) fn has_passthrough_return_wrapper_candidates_ir(lines: &[String]) -> bool {
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

pub(crate) fn has_trivial_dot_product_wrapper_candidates_ir(lines: &[String]) -> bool {
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
