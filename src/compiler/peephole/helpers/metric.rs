use super::super::{find_matching_block_end, split_top_level_args};
use super::helper_calls::parse_function_header;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub(in super::super) struct MetricHelper {
    pub(super) name_param: String,
    pub(super) value_param: String,
    pub(super) pre_name_lines: Vec<String>,
    pub(super) pre_value_lines: Vec<String>,
}

pub(in super::super) fn collect_metric_helpers(
    lines: &[String],
) -> FxHashMap<String, MetricHelper> {
    let mut out = FxHashMap::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(lines, fn_start) else {
            break;
        };
        let Some((fn_name, params)) = parse_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if params.len() != 2 {
            fn_start = fn_end + 1;
            continue;
        }
        let body_lines: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "{" && s != "}")
            .collect();
        if body_lines.len() < 3 || body_lines.len() > 5 {
            fn_start = fn_end + 1;
            continue;
        }
        let name_param = params[0].clone();
        let value_param = params[1].clone();
        let Some(return_line) = body_lines.last() else {
            fn_start = fn_end + 1;
            continue;
        };
        if return_line != &format!("return({value_param})") {
            fn_start = fn_end + 1;
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
            fn_start = fn_end + 1;
            continue;
        };
        if print_name_idx >= print_value_idx || print_value_idx + 1 != body_lines.len() - 1 {
            fn_start = fn_end + 1;
            continue;
        }
        let pre_name_lines = body_lines[..print_name_idx].to_vec();
        let pre_value_lines = body_lines[print_name_idx + 1..print_value_idx].to_vec();
        let helper = MetricHelper {
            name_param,
            value_param,
            pre_name_lines,
            pre_value_lines,
        };
        out.insert(fn_name, helper);
        fn_start = fn_end + 1;
    }
    out
}

pub(in super::super) fn rewrite_metric_helper_return_calls(lines: Vec<String>) -> Vec<String> {
    let helpers = collect_metric_helpers(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut temp_counter = 0usize;
    for line in lines {
        let trimmed = line.trim();
        let Some(inner) = trimmed
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            out.push(line);
            continue;
        };
        let Some((callee, args_str)) = inner.split_once('(') else {
            out.push(line);
            continue;
        };
        let Some(args_inner) = args_str.strip_suffix(')') else {
            out.push(line);
            continue;
        };
        let Some(helper) = helpers.get(callee.trim()) else {
            out.push(line);
            continue;
        };
        let Some(args) = split_top_level_args(args_inner) else {
            out.push(line);
            continue;
        };
        if args.len() != 2 {
            out.push(line);
            continue;
        }
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let metric_name = args[0].trim();
        let metric_value = args[1].trim();
        for pre in &helper.pre_name_lines {
            out.push(format!("{indent}{pre}"));
        }
        out.push(format!("{indent}print({metric_name})"));
        let temp_name = format!(".__rr_inline_metric_{temp_counter}");
        temp_counter += 1;
        out.push(format!("{indent}{temp_name} <- {metric_value}"));
        for pre in &helper.pre_value_lines {
            out.push(format!("{indent}{pre}"));
        }
        out.push(format!("{indent}print({temp_name})"));
        out.push(format!("{indent}return({temp_name})"));
    }
    out
}

pub(in super::super) fn rewrite_metric_helper_statement_calls(lines: Vec<String>) -> Vec<String> {
    let helpers = collect_metric_helpers(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut temp_counter = 0usize;
    for line in lines {
        let trimmed = line.trim();
        let Some((callee, args_str)) = trimmed.split_once('(') else {
            out.push(line);
            continue;
        };
        if trimmed.contains("<-") || trimmed.starts_with("return(") {
            out.push(line);
            continue;
        }
        let Some(args_inner) = args_str.strip_suffix(')') else {
            out.push(line);
            continue;
        };
        let Some(helper) = helpers.get(callee.trim()) else {
            out.push(line);
            continue;
        };
        let Some(args) = split_top_level_args(args_inner) else {
            out.push(line);
            continue;
        };
        if args.len() != 2 {
            out.push(line);
            continue;
        }
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let metric_name = args[0].trim();
        let metric_value = args[1].trim();
        for pre in &helper.pre_name_lines {
            out.push(format!("{indent}{pre}"));
        }
        out.push(format!("{indent}print({metric_name})"));
        let temp_name = format!(".__rr_inline_metric_{temp_counter}");
        temp_counter += 1;
        out.push(format!("{indent}{temp_name} <- {metric_value}"));
        for pre in &helper.pre_value_lines {
            out.push(format!("{indent}{pre}"));
        }
        out.push(format!("{indent}print({temp_name})"));
    }
    out
}
