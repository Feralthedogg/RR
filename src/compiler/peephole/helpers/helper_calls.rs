use super::super::{
    PeepholeAnalysisCache, assign_re, build_function_text_index,
    collapse_trivial_passthrough_return_wrappers_ir, expr_idents, expr_is_fresh_allocation_like,
    helper_call_candidate_lines, plain_ident_re, rewrite_simple_expr_helper_calls_ir,
    scalar_lit_re, simplify_nested_index_vec_floor_calls_ir, split_top_level_args,
    strip_arg_aliases_in_trivial_return_wrappers_ir, strip_unused_helper_params_ir,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(crate) fn has_sym_call_candidates(lines: &[String]) -> bool {
    lines
        .iter()
        .any(|line| !line.contains("<- function") && line.contains("Sym_") && line.contains('('))
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

pub(crate) fn full_text_signature(lines: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for line in lines {
        line.hash(&mut hasher);
    }
    hasher.finish()
}

#[derive(Debug, Clone, Default)]
pub(crate) struct HelperAnalysisCache {
    pub(crate) passthrough_signature: Option<u64>,
    pub(crate) passthrough_helpers: FxHashMap<String, String>,
    pub(crate) simple_signature: Option<u64>,
    pub(crate) simple_helpers: FxHashMap<String, SimpleExprHelper>,
    pub(crate) trim_signature: Option<u64>,
    pub(crate) trims: FxHashMap<String, HelperTrim>,
}

#[derive(Debug, Clone)]
pub(crate) struct SimpleExprHelper {
    pub(crate) params: Vec<String>,
    pub(crate) expr: String,
}

#[derive(Debug, Clone)]
pub(crate) struct HelperTrim {
    pub(crate) original_len: usize,
    pub(crate) kept_indices: Vec<usize>,
    pub(crate) kept_params: Vec<String>,
}

pub(crate) fn expr_is_trivial_passthrough_setup_rhs(
    rhs: &str,
    fresh_user_calls: &FxHashSet<String>,
) -> bool {
    let rhs = rhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || expr_is_fresh_allocation_like(rhs, fresh_user_calls)
        || rhs
            .strip_prefix("length(")
            .and_then(|s| s.strip_suffix(')'))
            .is_some_and(|inner| plain_ident_re().is_some_and(|re| re.is_match(inner.trim())))
}

pub(crate) fn collect_passthrough_helpers(lines: &[String]) -> FxHashMap<String, String> {
    let mut out = FxHashMap::default();
    for func in build_function_text_index(lines, parse_function_header) {
        let Some(fn_name) = func.name.as_ref() else {
            continue;
        };
        let Some(return_idx) = func.return_idx else {
            continue;
        };
        let body_lines: Vec<&str> = lines
            .iter()
            .take(func.end)
            .skip(func.body_start)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && *s != "{" && *s != "}")
            .collect();
        if !body_lines.is_empty() {
            continue;
        }
        let return_line = lines[return_idx].trim();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };
        if plain_ident_re().is_some_and(|re| re.is_match(inner)) {
            out.insert(fn_name.clone(), inner.to_string());
        }
    }
    out
}

pub(crate) fn cached_passthrough_helpers(
    cache: &mut HelperAnalysisCache,
    lines: &[String],
) -> FxHashMap<String, String> {
    let signature = function_defs_signature(lines);
    if cache.passthrough_signature != Some(signature) {
        cache.passthrough_helpers = collect_passthrough_helpers(lines);
        cache.passthrough_signature = Some(signature);
    }
    cache.passthrough_helpers.clone()
}

pub(crate) fn rewrite_passthrough_helper_calls(lines: Vec<String>) -> Vec<String> {
    let mut cache = HelperAnalysisCache::default();
    let mut analysis_cache = PeepholeAnalysisCache::default();
    rewrite_passthrough_helper_calls_with_caches(lines, &mut cache, &mut analysis_cache)
}

pub(crate) fn rewrite_passthrough_helper_calls_with_cache(
    lines: Vec<String>,
    cache: &mut HelperAnalysisCache,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    rewrite_passthrough_helper_calls_with_caches(lines, cache, &mut analysis_cache)
}

pub(crate) fn rewrite_passthrough_helper_calls_with_caches(
    lines: Vec<String>,
    cache: &mut HelperAnalysisCache,
    analysis_cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    if !has_sym_call_candidates(&lines) {
        return lines;
    }
    let mut out = lines;
    let passthrough = cached_passthrough_helpers(cache, &out);
    if passthrough.is_empty() {
        return out;
    }
    let candidate_lines = helper_call_candidate_lines(analysis_cache, &out);
    for line_idx in candidate_lines {
        let line = out[line_idx].clone();
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
        if args.len() != 1 {
            continue;
        }
        if param_name.is_empty() {
            continue;
        }
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        out[line_idx] = format!("{indent}{lhs} <- {}", args[0].trim());
    }
    out
}

pub(crate) fn rewrite_simple_expr_helper_calls_filtered(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    allowed_helpers: Option<&FxHashSet<String>>,
) -> Vec<String> {
    let mut cache = HelperAnalysisCache::default();
    let mut analysis_cache = PeepholeAnalysisCache::default();
    rewrite_simple_expr_helper_calls_filtered_with_cache(
        lines,
        pure_user_calls,
        allowed_helpers,
        &mut cache,
        &mut analysis_cache,
    )
}

pub(crate) fn rewrite_simple_expr_helper_calls_filtered_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    allowed_helpers: Option<&FxHashSet<String>>,
    _cache: &mut HelperAnalysisCache,
    _analysis_cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    rewrite_simple_expr_helper_calls_ir(lines, pure_user_calls, allowed_helpers, false)
}

pub(crate) fn rewrite_simple_expr_helper_calls_filtered_size_controlled(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    allowed_helpers: Option<&FxHashSet<String>>,
) -> Vec<String> {
    rewrite_simple_expr_helper_calls_ir(lines, pure_user_calls, allowed_helpers, true)
}

pub(crate) fn rewrite_simple_expr_helper_calls(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    rewrite_simple_expr_helper_calls_filtered(lines, pure_user_calls, None)
}

pub(crate) fn rewrite_simple_expr_helper_calls_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut HelperAnalysisCache,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    rewrite_simple_expr_helper_calls_filtered_with_cache(
        lines,
        pure_user_calls,
        None,
        cache,
        &mut analysis_cache,
    )
}

pub(crate) fn rewrite_simple_expr_helper_calls_with_caches(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut HelperAnalysisCache,
    analysis_cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    rewrite_simple_expr_helper_calls_filtered_with_cache(
        lines,
        pure_user_calls,
        None,
        cache,
        analysis_cache,
    )
}

pub(crate) fn simplify_nested_index_vec_floor_calls(lines: Vec<String>) -> Vec<String> {
    simplify_nested_index_vec_floor_calls_ir(lines)
}

pub(crate) fn rewrite_selected_simple_expr_helper_calls_in_text(
    code: &str,
    helper_names: &[&str],
) -> String {
    let allowed_helpers: FxHashSet<String> = helper_names
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    let out_lines = rewrite_simple_expr_helper_calls_filtered(
        code.lines().map(str::to_string).collect(),
        &FxHashSet::default(),
        Some(&allowed_helpers),
    );
    let mut out = out_lines.join("\n");
    if code.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn simplify_nested_index_vec_floor_calls_in_text(code: &str) -> String {
    let out_lines =
        simplify_nested_index_vec_floor_calls(code.lines().map(str::to_string).collect());
    let mut out = out_lines.join("\n");
    if code.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_arg_aliases_in_trivial_return_wrappers(lines: Vec<String>) -> Vec<String> {
    strip_arg_aliases_in_trivial_return_wrappers_ir(lines)
}

pub(crate) fn collapse_trivial_passthrough_return_wrappers(lines: Vec<String>) -> Vec<String> {
    collapse_trivial_passthrough_return_wrappers_ir(lines)
}

pub(crate) fn strip_unused_helper_params(lines: Vec<String>) -> Vec<String> {
    strip_unused_helper_params_ir(lines)
}

pub(crate) fn strip_unused_helper_params_with_cache(
    lines: Vec<String>,
    _cache: &mut HelperAnalysisCache,
) -> Vec<String> {
    strip_unused_helper_params_ir(lines)
}

pub(crate) fn strip_unused_helper_params_with_caches(
    lines: Vec<String>,
    _cache: &mut HelperAnalysisCache,
    _analysis_cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    strip_unused_helper_params_ir(lines)
}

pub(crate) fn parse_function_header(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let (name, rest) = trimmed.split_once("<- function(")?;
    let args_inner = rest.split_once(')')?.0.trim();
    let params = if args_inner.is_empty() {
        Vec::new()
    } else {
        split_top_level_args(args_inner)?
            .into_iter()
            .map(|arg| arg.trim().to_string())
            .collect()
    };
    Some((name.trim().to_string(), params))
}

pub(crate) fn substitute_helper_expr(expr: &str, bindings: &FxHashMap<String, String>) -> String {
    let mut out = String::with_capacity(expr.len());
    let bytes = expr.as_bytes();
    let mut idx = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' if !in_double => {
                in_single = !in_single;
                out.push('\'');
                idx += 1;
                continue;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                out.push('"');
                idx += 1;
                continue;
            }
            _ => {}
        }

        if !in_single && !in_double && helper_ident_is_start(expr, idx) {
            let end = helper_ident_end(expr, idx);
            let ident = &expr[idx..end];
            if !helper_ident_is_named_label(expr, end)
                && let Some(replacement) = bindings.get(ident)
            {
                out.push_str(replacement);
            } else {
                out.push_str(ident);
            }
            idx = end;
            continue;
        }

        out.push(bytes[idx] as char);
        idx += 1;
    }

    out
}

pub(crate) fn helper_ident_is_start(expr: &str, idx: usize) -> bool {
    let rest = &expr[idx..];
    let mut chars = rest.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_ascii_alphabetic() || first == '_' {
        return true;
    }
    first == '.'
        && chars
            .next()
            .is_some_and(|next| next.is_ascii_alphabetic() || next == '_')
}

pub(crate) fn helper_ident_end(expr: &str, start: usize) -> usize {
    let mut end = start;
    for (off, ch) in expr[start..].char_indices() {
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')) {
            break;
        }
        end = start + off + ch.len_utf8();
    }
    end
}

pub(crate) fn helper_ident_is_named_label(expr: &str, end: usize) -> bool {
    let rest = &expr[end..];
    for (off, ch) in rest.char_indices() {
        if ch.is_ascii_whitespace() {
            continue;
        }
        if ch != '=' {
            return false;
        }
        let tail = &rest[off + ch.len_utf8()..];
        let next_non_ws = tail.chars().find(|ch| !ch.is_ascii_whitespace());
        return next_non_ws != Some('=');
    }
    false
}

pub(crate) fn collect_simple_expr_helpers(
    lines: &[String],
    _pure_user_calls: &FxHashSet<String>,
) -> FxHashMap<String, SimpleExprHelper> {
    let mut out = FxHashMap::default();
    for func in build_function_text_index(lines, parse_function_header) {
        let Some(fn_name) = func.name.as_ref() else {
            continue;
        };
        let params = &func.params;
        let Some(return_idx) = func.return_idx else {
            continue;
        };
        let return_line = lines[return_idx].trim();
        let Some(return_expr) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let mut bindings: FxHashMap<String, String> = FxHashMap::default();
        let mut locals: FxHashSet<String> = FxHashSet::default();
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
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                simple = false;
                break;
            }
            let expanded = substitute_helper_expr(rhs, &bindings);
            bindings.insert(lhs.to_string(), expanded);
            locals.insert(lhs.to_string());
        }
        if !simple {
            continue;
        }

        let expanded_return = substitute_helper_expr(return_expr, &bindings);
        if expanded_return.contains(&format!("{fn_name}(")) {
            continue;
        }
        if expr_idents(&expanded_return)
            .iter()
            .any(|ident| locals.contains(ident) && !params.iter().any(|param| param == ident))
        {
            continue;
        }
        out.insert(
            fn_name.clone(),
            SimpleExprHelper {
                params: params.clone(),
                expr: expanded_return,
            },
        );
    }
    out
}
