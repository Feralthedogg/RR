use super::*;
pub(crate) fn collect_passthrough_helpers_from_program_ir(
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

pub(crate) fn has_passthrough_helper_definitions_with_calls_ir(lines: &[String]) -> bool {
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

pub(crate) fn has_passthrough_helper_candidates_ir(lines: &[String]) -> bool {
    has_passthrough_helper_definitions_with_calls_ir(lines)
}

pub(crate) fn line_has_helper_callsite_ir(line: &str, helper_name: &str) -> bool {
    line.contains(&format!("{helper_name}("))
        && !line.contains(&format!("{helper_name} <- function("))
}

pub(crate) fn has_metric_helper_definitions_with_calls_ir(lines: &[String]) -> bool {
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

pub(crate) fn has_simple_expr_helper_definitions_with_calls_ir(lines: &[String]) -> bool {
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
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                if expr_idents(rhs).iter().any(|ident| ident == lhs) {
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

pub(crate) fn has_metric_helper_candidates_ir(lines: &[String]) -> bool {
    has_metric_helper_definitions_with_calls_ir(lines)
}

pub(crate) fn has_literal_field_get_candidates_ir(lines: &[String]) -> bool {
    let Some(re) = literal_field_get_re() else {
        return false;
    };
    lines.iter().any(|line| re.is_match(line))
}

pub(crate) fn has_literal_named_list_candidates_ir(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        line.contains("rr_named_list(")
            && !line.contains("rr_named_list <- function")
            && rewrite_literal_named_list_line_ir(line) != *line
    })
}

pub(crate) fn has_simple_expr_helper_candidates_ir(lines: &[String]) -> bool {
    has_simple_expr_helper_definitions_with_calls_ir(lines)
}

pub(crate) fn apply_rewrite_passthrough_helper_calls_ir(
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
