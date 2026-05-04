use super::*;
pub(crate) fn collect_metric_helpers_ir(lines: &[String]) -> FxHashMap<String, MetricHelperIr> {
    let mut out = FxHashMap::default();
    for func in build_function_text_index(lines, parse_function_header_ir) {
        let Some(fn_name) = func.name.as_ref() else {
            continue;
        };
        let params = &func.params;
        if params.len() != 2 {
            continue;
        }
        let body_lines: Vec<String> = lines
            .iter()
            .take(func.end)
            .skip(func.body_start)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "{" && s != "}")
            .collect();
        if body_lines.len() < 3 || body_lines.len() > 5 {
            continue;
        }
        let name_param = params[0].clone();
        let value_param = params[1].clone();
        let Some(return_line) = body_lines.last() else {
            continue;
        };
        if return_line != &format!("return({value_param})") {
            continue;
        }
        let print_name_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({name_param})"));
        let print_value_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({value_param})"));
        let (Some(print_name_idx), Some(print_value_idx)) = (print_name_idx, print_value_idx)
        else {
            continue;
        };
        if print_name_idx >= print_value_idx || print_value_idx + 1 != body_lines.len() - 1 {
            continue;
        }
        out.insert(
            fn_name.clone(),
            MetricHelperIr {
                name_param,
                value_param,
                pre_name_lines: body_lines[..print_name_idx].to_vec(),
                pre_value_lines: body_lines[print_name_idx + 1..print_value_idx].to_vec(),
            },
        );
    }
    out
}

pub(crate) fn collect_metric_helpers_from_program_ir(
    program: &EmittedProgram,
) -> FxHashMap<String, MetricHelperIr> {
    let mut out = FxHashMap::default();
    for item in &program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        if params.len() != 2 {
            continue;
        }
        let body_lines: Vec<String> = function
            .body
            .iter()
            .map(|stmt| stmt.text.trim().to_string())
            .filter(|s| !s.is_empty() && s != "{" && s != "}")
            .collect();
        if body_lines.len() < 3 || body_lines.len() > 5 {
            continue;
        }
        let name_param = params[0].clone();
        let value_param = params[1].clone();
        let Some(return_line) = body_lines.last() else {
            continue;
        };
        if return_line != &format!("return({value_param})") {
            continue;
        }
        let print_name_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({name_param})"));
        let print_value_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({value_param})"));
        let (Some(print_name_idx), Some(print_value_idx)) = (print_name_idx, print_value_idx)
        else {
            continue;
        };
        if print_name_idx >= print_value_idx || print_value_idx + 1 != body_lines.len() - 1 {
            continue;
        }
        out.insert(
            fn_name,
            MetricHelperIr {
                name_param,
                value_param,
                pre_name_lines: body_lines[..print_name_idx].to_vec(),
                pre_value_lines: body_lines[print_name_idx + 1..print_value_idx].to_vec(),
            },
        );
    }
    out
}

pub(crate) fn rewrite_metric_helper_return_calls_ir(lines: Vec<String>) -> Vec<String> {
    if !has_metric_helper_candidates_ir(&lines) {
        return lines;
    }
    let helpers = collect_metric_helpers_ir(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    let mut temp_counter = 0usize;
    apply_rewrite_metric_helper_return_calls_ir(&mut program, &helpers, &mut temp_counter);
    program.into_lines()
}

pub(crate) fn rewrite_metric_helper_statement_calls_ir(lines: Vec<String>) -> Vec<String> {
    if !has_metric_helper_candidates_ir(&lines) {
        return lines;
    }
    let helpers = collect_metric_helpers_ir(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    let mut temp_counter = 0usize;
    apply_rewrite_metric_helper_statement_calls_ir(&mut program, &helpers, &mut temp_counter);
    program.into_lines()
}

pub(crate) fn apply_rewrite_metric_helper_return_calls_ir(
    program: &mut EmittedProgram,
    helpers: &FxHashMap<String, MetricHelperIr>,
    temp_counter: &mut usize,
) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        for stmt in function.body.drain(..) {
            let trimmed = stmt.text.trim().to_string();
            let Some(inner) = trimmed
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
            else {
                out.push(stmt);
                continue;
            };
            let Some((callee, args_str)) = inner.split_once('(') else {
                out.push(stmt);
                continue;
            };
            let Some(args_inner) = args_str.strip_suffix(')') else {
                out.push(stmt);
                continue;
            };
            let Some(helper) = helpers.get(callee.trim()) else {
                out.push(stmt);
                continue;
            };
            let Some(args) = split_top_level_args(args_inner) else {
                out.push(stmt);
                continue;
            };
            if args.len() != 2 {
                out.push(stmt);
                continue;
            }
            let indent = stmt.indent();
            let metric_name = args[0].trim();
            let metric_value = args[1].trim();
            for pre in &helper.pre_name_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({metric_name})")));
            let temp_name = format!(".__rr_inline_metric_{}", *temp_counter);
            *temp_counter += 1;
            out.push(EmittedStmt::parse(&format!(
                "{indent}{temp_name} <- {metric_value}"
            )));
            for pre in &helper.pre_value_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({temp_name})")));
            out.push(EmittedStmt::parse(&format!("{indent}return({temp_name})")));
        }
        function.body = out;
    }
}

pub(crate) fn apply_rewrite_metric_helper_statement_calls_ir(
    program: &mut EmittedProgram,
    helpers: &FxHashMap<String, MetricHelperIr>,
    temp_counter: &mut usize,
) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        for stmt in function.body.drain(..) {
            let trimmed = stmt.text.trim().to_string();
            let Some((callee, args_str)) = trimmed.split_once('(') else {
                out.push(stmt);
                continue;
            };
            if trimmed.contains("<-") || trimmed.starts_with("return(") {
                out.push(stmt);
                continue;
            }
            let Some(args_inner) = args_str.strip_suffix(')') else {
                out.push(stmt);
                continue;
            };
            let Some(helper) = helpers.get(callee.trim()) else {
                out.push(stmt);
                continue;
            };
            let Some(args) = split_top_level_args(args_inner) else {
                out.push(stmt);
                continue;
            };
            if args.len() != 2 {
                out.push(stmt);
                continue;
            }
            let indent = stmt.indent();
            let metric_name = args[0].trim();
            let metric_value = args[1].trim();
            for pre in &helper.pre_name_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({metric_name})")));
            let temp_name = format!(".__rr_inline_metric_{}", *temp_counter);
            *temp_counter += 1;
            out.push(EmittedStmt::parse(&format!(
                "{indent}{temp_name} <- {metric_value}"
            )));
            for pre in &helper.pre_value_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({temp_name})")));
        }
        function.body = out;
    }
}
