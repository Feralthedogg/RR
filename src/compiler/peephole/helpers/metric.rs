use super::super::{
    build_function_text_index, rewrite_metric_helper_return_calls_ir,
    rewrite_metric_helper_statement_calls_ir,
};
use super::helper_calls::parse_function_header;
use rustc_hash::FxHashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Clone)]
pub(crate) struct MetricHelper {
    pub(crate) name_param: String,
    pub(crate) value_param: String,
    pub(crate) pre_name_lines: Vec<String>,
    pub(crate) pre_value_lines: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MetricHelperCache {
    pub(crate) signature: Option<u64>,
    pub(crate) helpers: FxHashMap<String, MetricHelper>,
}

pub(crate) fn has_metric_helper_candidates(lines: &[String]) -> bool {
    lines.iter().any(|line| line.contains("print("))
}

pub(crate) fn function_defs_signature(lines: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for func in build_function_text_index(lines, parse_function_header) {
        for line in lines.iter().take(func.end + 1).skip(func.start) {
            line.hash(&mut hasher);
        }
    }
    hasher.finish()
}

pub(crate) fn collect_metric_helpers(lines: &[String]) -> FxHashMap<String, MetricHelper> {
    let mut out = FxHashMap::default();
    for func in build_function_text_index(lines, parse_function_header) {
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
        let pre_name_lines = body_lines[..print_name_idx].to_vec();
        let pre_value_lines = body_lines[print_name_idx + 1..print_value_idx].to_vec();
        let helper = MetricHelper {
            name_param,
            value_param,
            pre_name_lines,
            pre_value_lines,
        };
        out.insert(fn_name.clone(), helper);
    }
    out
}

pub(crate) fn cached_metric_helpers(
    cache: &mut MetricHelperCache,
    lines: &[String],
) -> FxHashMap<String, MetricHelper> {
    let signature = function_defs_signature(lines);
    if cache.signature != Some(signature) {
        cache.helpers = collect_metric_helpers(lines);
        cache.signature = Some(signature);
    }
    cache.helpers.clone()
}

pub(crate) fn rewrite_metric_helper_return_calls(lines: Vec<String>) -> Vec<String> {
    rewrite_metric_helper_return_calls_ir(lines)
}

pub(crate) fn rewrite_metric_helper_return_calls_with_cache(
    lines: Vec<String>,
    _cache: &mut MetricHelperCache,
) -> Vec<String> {
    rewrite_metric_helper_return_calls_ir(lines)
}

pub(crate) fn rewrite_metric_helper_statement_calls(lines: Vec<String>) -> Vec<String> {
    rewrite_metric_helper_statement_calls_ir(lines)
}

pub(crate) fn rewrite_metric_helper_statement_calls_with_cache(
    lines: Vec<String>,
    _cache: &mut MetricHelperCache,
) -> Vec<String> {
    rewrite_metric_helper_statement_calls_ir(lines)
}
