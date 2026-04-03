use super::super::{
    assign_re, expr_idents, expr_is_fresh_allocation_like, find_matching_block_end, ident_re,
    nested_index_vec_floor_re, normalize_expr_with_aliases, plain_ident_re,
    previous_non_empty_line, scalar_lit_re, split_top_level_args, unquoted_sym_refs,
};
use regex::Captures;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone)]
pub(in super::super) struct SimpleExprHelper {
    pub(super) params: Vec<String>,
    pub(super) expr: String,
}

fn expr_is_trivial_passthrough_setup_rhs(rhs: &str, fresh_user_calls: &FxHashSet<String>) -> bool {
    let rhs = rhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || expr_is_fresh_allocation_like(rhs, fresh_user_calls)
        || rhs
            .strip_prefix("length(")
            .and_then(|s| s.strip_suffix(')'))
            .is_some_and(|inner| plain_ident_re().is_some_and(|re| re.is_match(inner.trim())))
}

fn collect_passthrough_helpers(lines: &[String]) -> FxHashMap<String, String> {
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
        let header = lines[fn_start].trim();
        let Some((fn_name, _)) = header.split_once("<- function") else {
            fn_start = fn_end + 1;
            continue;
        };
        let fn_name = fn_name.trim();
        let Some(return_idx) = previous_non_empty_line(lines, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body_lines: Vec<&str> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && *s != "{" && *s != "}")
            .collect();
        if !body_lines.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }
        let return_line = lines[return_idx].trim();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            fn_start = fn_end + 1;
            continue;
        };
        if plain_ident_re().is_some_and(|re| re.is_match(inner)) {
            out.insert(fn_name.to_string(), inner.to_string());
        }
        fn_start = fn_end + 1;
    }
    out
}

pub(in super::super) fn rewrite_passthrough_helper_calls(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let passthrough = collect_passthrough_helpers(&out);
    if passthrough.is_empty() {
        return out;
    }
    for line in &mut out {
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
        *line = format!("{indent}{lhs} <- {}", args[0].trim());
    }
    out
}

pub(in super::super) fn rewrite_simple_expr_helper_calls_filtered(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    allowed_helpers: Option<&FxHashSet<String>>,
) -> Vec<String> {
    let helpers = collect_simple_expr_helpers(&lines, pure_user_calls);
    if helpers.is_empty() {
        return lines;
    }
    let mut out = lines;
    for line in &mut out {
        if line.contains("<- function") {
            continue;
        }
        let mut rewritten = line.clone();
        loop {
            let mut changed = false;
            let mut next = String::with_capacity(rewritten.len());
            let mut idx = 0usize;
            while idx < rewritten.len() {
                let slice = &rewritten[idx..];
                let Some(caps) = ident_re().and_then(|re| re.captures(slice)) else {
                    next.push_str(slice);
                    break;
                };
                let Some(mat) = caps.get(0) else {
                    next.push_str(slice);
                    break;
                };
                let ident_start = idx + mat.start();
                let ident_end = idx + mat.end();
                next.push_str(&rewritten[idx..ident_start]);
                let ident = mat.as_str();
                let Some(helper) = helpers.get(ident) else {
                    next.push_str(ident);
                    idx = ident_end;
                    continue;
                };
                if allowed_helpers.is_some_and(|allowed| !allowed.contains(ident)) {
                    next.push_str(ident);
                    idx = ident_end;
                    continue;
                }
                if !rewritten[ident_end..].starts_with('(') {
                    next.push_str(ident);
                    idx = ident_end;
                    continue;
                }
                let mut depth = 0i32;
                let mut end = None;
                for (off, ch) in rewritten[ident_end..].char_indices() {
                    match ch {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(ident_end + off);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let Some(call_end) = end else {
                    next.push_str(ident);
                    idx = ident_end;
                    continue;
                };
                let args_inner = &rewritten[ident_end + 1..call_end];
                let Some(args) = split_top_level_args(args_inner) else {
                    next.push_str(&rewritten[ident_start..=call_end]);
                    idx = call_end + 1;
                    continue;
                };
                if args.len() != helper.params.len() {
                    next.push_str(&rewritten[ident_start..=call_end]);
                    idx = call_end + 1;
                    continue;
                }
                let subst = helper
                    .params
                    .iter()
                    .zip(args.iter())
                    .map(|(param, arg)| (param.clone(), arg.trim().to_string()))
                    .collect::<FxHashMap<_, _>>();
                let expanded = substitute_helper_expr(&helper.expr, &subst);
                next.push('(');
                next.push_str(&expanded);
                next.push(')');
                idx = call_end + 1;
                changed = true;
            }
            if !changed || next == rewritten {
                break;
            }
            rewritten = next;
        }
        *line = rewritten;
    }
    out
}

pub(in super::super) fn rewrite_simple_expr_helper_calls(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    rewrite_simple_expr_helper_calls_filtered(lines, pure_user_calls, None)
}

pub(in super::super) fn simplify_nested_index_vec_floor_calls(lines: Vec<String>) -> Vec<String> {
    let Some(re) = nested_index_vec_floor_re() else {
        return lines;
    };
    lines
        .into_iter()
        .map(|line| {
            let mut rewritten = line;
            loop {
                let next = re
                    .replace_all(&rewritten, |caps: &Captures<'_>| {
                        format!(
                            "rr_index_vec_floor({})",
                            caps.name("inner").map(|m| m.as_str()).unwrap_or("")
                        )
                    })
                    .to_string();
                if next == rewritten {
                    break rewritten;
                }
                rewritten = next;
            }
        })
        .collect()
}

pub(in super::super) fn rewrite_selected_simple_expr_helper_calls_in_text(
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

pub(in super::super) fn simplify_nested_index_vec_floor_calls_in_text(code: &str) -> String {
    let out_lines =
        simplify_nested_index_vec_floor_calls(code.lines().map(str::to_string).collect());
    let mut out = out_lines.join("\n");
    if code.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(in super::super) fn strip_arg_aliases_in_trivial_return_wrappers(
    lines: Vec<String>,
) -> Vec<String> {
    let mut out = lines;
    let mut fn_start = 0usize;
    while fn_start < out.len() {
        while fn_start < out.len() && !out[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= out.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&out, fn_start) else {
            break;
        };
        let body_start = fn_start + 1;
        let Some(return_idx) = previous_non_empty_line(&out, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let return_line = out[return_idx].trim().to_string();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            fn_start = fn_end + 1;
            continue;
        };

        let mut aliases = FxHashMap::default();
        let mut trivial = true;
        for line in out.iter().take(return_idx).skip(body_start) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                trivial = false;
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs.starts_with(".arg_") && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                aliases.insert(lhs.to_string(), rhs.to_string());
            } else {
                trivial = false;
                break;
            }
        }
        if !trivial || aliases.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }

        let rewritten = normalize_expr_with_aliases(inner, &aliases);
        if rewritten != inner {
            let indent_len = out[return_idx].len() - out[return_idx].trim_start().len();
            let indent = &out[return_idx][..indent_len];
            out[return_idx] = format!("{indent}return({rewritten})");
            for line in out.iter_mut().take(return_idx).skip(body_start) {
                if line.trim_start().starts_with(".arg_") {
                    line.clear();
                }
            }
        }

        fn_start = fn_end + 1;
    }
    out
}

pub(in super::super) fn collapse_trivial_passthrough_return_wrappers(
    lines: Vec<String>,
) -> Vec<String> {
    let mut out = lines;
    let no_fresh_user_calls = FxHashSet::default();
    let mut fn_start = 0usize;
    while fn_start < out.len() {
        while fn_start < out.len() && !out[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= out.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&out, fn_start) else {
            break;
        };
        let body_start = fn_start + 1;
        let Some(return_idx) = previous_non_empty_line(&out, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let return_line = out[return_idx].trim().to_string();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            fn_start = fn_end + 1;
            continue;
        };

        let mut last_assign_to_return: Option<(usize, String)> = None;
        let mut trivial = true;
        for (idx, line) in out.iter().enumerate().take(return_idx).skip(body_start) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                trivial = false;
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs == inner && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                last_assign_to_return = Some((idx, rhs.to_string()));
            } else if !expr_is_trivial_passthrough_setup_rhs(rhs, &no_fresh_user_calls) {
                trivial = false;
                break;
            }
        }
        let Some((assign_idx, passthrough_ident)) = last_assign_to_return else {
            fn_start = fn_end + 1;
            continue;
        };
        if !trivial {
            fn_start = fn_end + 1;
            continue;
        }

        let indent_len = out[return_idx].len() - out[return_idx].trim_start().len();
        let indent = &out[return_idx][..indent_len];
        out[return_idx] = format!("{indent}return({passthrough_ident})");
        for line in out.iter_mut().take(return_idx).skip(body_start) {
            let trimmed = line.trim();
            if trimmed == "{" || trimmed == "}" {
                continue;
            }
            line.clear();
        }
        out[assign_idx].clear();
        fn_start = fn_end + 1;
    }
    out
}

pub(in super::super) fn strip_unused_helper_params(lines: Vec<String>) -> Vec<String> {
    #[derive(Clone)]
    struct HelperTrim {
        original_len: usize,
        kept_indices: Vec<usize>,
        kept_params: Vec<String>,
    }

    let mut trims = FxHashMap::<String, HelperTrim>::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&lines, fn_start) else {
            break;
        };
        let Some((fn_name, params)) = parse_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if !fn_name.starts_with("Sym_")
            || params.is_empty()
            || params.iter().any(|param| param.contains('='))
        {
            fn_start = fn_end + 1;
            continue;
        }
        let escaped = lines
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx < fn_start || *idx > fn_end)
            .any(|(_, line)| {
                unquoted_sym_refs(line).iter().any(|name| name == &fn_name)
                    && !line.contains(&format!("{fn_name}("))
            });
        if escaped {
            fn_start = fn_end + 1;
            continue;
        }
        let mut used_params = FxHashSet::default();
        for line in lines.iter().take(fn_end).skip(fn_start + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            for ident in expr_idents(trimmed) {
                used_params.insert(ident);
            }
        }
        let kept_indices: Vec<usize> = params
            .iter()
            .enumerate()
            .filter_map(|(idx, param)| used_params.contains(param).then_some(idx))
            .collect();
        if kept_indices.len() < params.len() {
            trims.insert(
                fn_name,
                HelperTrim {
                    original_len: params.len(),
                    kept_indices: kept_indices.clone(),
                    kept_params: kept_indices
                        .iter()
                        .map(|idx| params[*idx].clone())
                        .collect(),
                },
            );
        }
        fn_start = fn_end + 1;
    }

    if trims.is_empty() {
        return lines;
    }

    let mut out = lines;
    for line in &mut out {
        if line.contains("<- function") {
            if let Some((fn_name, _)) = parse_function_header(line)
                && let Some(trim) = trims.get(&fn_name)
            {
                *line = format!("{} <- function({})", fn_name, trim.kept_params.join(", "));
            }
            continue;
        }
        let mut rewritten = line.clone();
        loop {
            let mut changed = false;
            let mut next = String::with_capacity(rewritten.len());
            let mut idx = 0usize;
            while idx < rewritten.len() {
                let slice = &rewritten[idx..];
                let Some(caps) = ident_re().and_then(|re| re.captures(slice)) else {
                    next.push_str(slice);
                    break;
                };
                let Some(mat) = caps.get(0) else {
                    next.push_str(slice);
                    break;
                };
                let ident_start = idx + mat.start();
                let ident_end = idx + mat.end();
                next.push_str(&rewritten[idx..ident_start]);
                let ident = mat.as_str();
                let Some(trim) = trims.get(ident) else {
                    next.push_str(ident);
                    idx = ident_end;
                    continue;
                };
                if !rewritten[ident_end..].starts_with('(') {
                    next.push_str(ident);
                    idx = ident_end;
                    continue;
                }
                let mut depth = 0i32;
                let mut end = None;
                for (off, ch) in rewritten[ident_end..].char_indices() {
                    match ch {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(ident_end + off);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let Some(call_end) = end else {
                    next.push_str(ident);
                    idx = ident_end;
                    continue;
                };
                let args_inner = &rewritten[ident_end + 1..call_end];
                let Some(args) = split_top_level_args(args_inner) else {
                    next.push_str(&rewritten[ident_start..=call_end]);
                    idx = call_end + 1;
                    continue;
                };
                if args.len() != trim.original_len {
                    next.push_str(&rewritten[ident_start..=call_end]);
                    idx = call_end + 1;
                    continue;
                }
                next.push_str(ident);
                next.push('(');
                next.push_str(
                    &trim
                        .kept_indices
                        .iter()
                        .map(|idx| args[*idx].trim())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                next.push(')');
                idx = call_end + 1;
                changed = true;
            }
            if !changed || next == rewritten {
                break;
            }
            rewritten = next;
        }
        *line = rewritten;
    }
    out
}

pub(in super::super) fn parse_function_header(line: &str) -> Option<(String, Vec<String>)> {
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

pub(in super::super) fn substitute_helper_expr(
    expr: &str,
    bindings: &FxHashMap<String, String>,
) -> String {
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

fn helper_ident_is_start(expr: &str, idx: usize) -> bool {
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

fn helper_ident_end(expr: &str, start: usize) -> usize {
    let mut end = start;
    for (off, ch) in expr[start..].char_indices() {
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')) {
            break;
        }
        end = start + off + ch.len_utf8();
    }
    end
}

fn helper_ident_is_named_label(expr: &str, end: usize) -> bool {
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

pub(in super::super) fn collect_simple_expr_helpers(
    lines: &[String],
    _pure_user_calls: &FxHashSet<String>,
) -> FxHashMap<String, SimpleExprHelper> {
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
        let Some(return_idx) = previous_non_empty_line(lines, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let return_line = lines[return_idx].trim();
        let Some(return_expr) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            fn_start = fn_end + 1;
            continue;
        };

        let mut bindings: FxHashMap<String, String> = FxHashMap::default();
        let mut locals: FxHashSet<String> = FxHashSet::default();
        let mut simple = true;
        for line in lines.iter().take(return_idx).skip(fn_start + 1) {
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
            fn_start = fn_end + 1;
            continue;
        }

        let expanded_return = substitute_helper_expr(return_expr, &bindings);
        if expanded_return.contains(&format!("{fn_name}(")) {
            fn_start = fn_end + 1;
            continue;
        }
        if expr_idents(&expanded_return)
            .iter()
            .any(|ident| locals.contains(ident) && !params.iter().any(|param| param == ident))
        {
            fn_start = fn_end + 1;
            continue;
        }
        out.insert(
            fn_name,
            SimpleExprHelper {
                params,
                expr: expanded_return,
            },
        );
        fn_start = fn_end + 1;
    }
    out
}
