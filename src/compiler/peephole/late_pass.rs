use super::{
    FxHashMap, FxHashSet, IDENT_PATTERN, PeepholeAnalysisCache, assign_re,
    collapse_identical_if_else_tail_assignments_late_ir, compile_regex, count_unquoted_braces,
    expr_idents, latest_literal_assignment_before, parse_repeat_guard_cmp_line, plain_ident_re,
    strip_redundant_identical_pure_rebinds_ir, strip_unreachable_sym_helpers_ir,
};
use regex::{Captures, Regex};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub(crate) struct PureCallBinding {
    pub(crate) expr: String,
    pub(crate) var: String,
    pub(crate) deps: FxHashSet<String>,
}

pub(crate) fn extract_pure_call_binding(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> Option<PureCallBinding> {
    let rhs = rhs.trim();
    if !expr_has_only_pure_calls(rhs, pure_user_calls) {
        return None;
    }
    let (callee, rest) = rhs.split_once('(')?;
    if !pure_user_calls.contains(callee.trim()) || !rhs.ends_with(')') {
        return None;
    }
    let args = rest.strip_suffix(')')?.trim();
    let deps = expr_idents(args)
        .into_iter()
        .filter(|ident| ident != callee.trim())
        .collect();
    Some(PureCallBinding {
        expr: rhs.to_string(),
        var: lhs.to_string(),
        deps,
    })
}

pub(crate) fn rewrite_pure_call_reuse(expr: &str, bindings: &[PureCallBinding]) -> String {
    let mut out = expr.to_string();
    for binding in bindings {
        if out.contains(&binding.expr) {
            out = out.replace(&binding.expr, &binding.var);
        }
    }
    out
}

pub(crate) fn rewrite_return_expr_line(
    line: &str,
    last_rhs_for_var: &FxHashMap<String, String>,
) -> String {
    let trimmed = line.trim();
    let Some(inner) = trimmed
        .strip_prefix("return(")
        .and_then(|s| s.strip_suffix(')'))
    else {
        return line.to_string();
    };
    let inner = inner.trim();
    let Some((var, _rhs)) = last_rhs_for_var
        .iter()
        .find(|(_, rhs)| rhs.as_str() == inner)
    else {
        return line.to_string();
    };
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    format!("{indent}return({var})")
}

pub(crate) fn written_base_var(lhs: &str) -> Option<&str> {
    let lhs = lhs.trim();
    if let Some((base, _)) = lhs.split_once('[') {
        let base = base.trim();
        if plain_ident_re().is_some_and(|re| re.is_match(base)) {
            return Some(base);
        }
        return None;
    }
    if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
        Some(lhs)
    } else {
        None
    }
}

pub(crate) fn maybe_expand_fresh_alias_rhs(
    rhs: &str,
    fresh_expr_for_var: &FxHashMap<String, String>,
) -> Option<String> {
    let ident = rhs.trim();
    if !plain_ident_re().is_some_and(|re| re.is_match(ident)) {
        return None;
    }
    fresh_expr_for_var.get(ident).cloned()
}

#[derive(Debug)]
pub(crate) struct SharedTailAssignment {
    pub(crate) then_idx: usize,
    pub(crate) else_idx: usize,
    pub(crate) text: String,
}

pub(crate) fn collect_common_if_else_tail_assignments(
    lines: &[String],
    if_idx: usize,
    else_idx: usize,
    end_idx: usize,
) -> Vec<SharedTailAssignment> {
    let mut then_idx = else_idx;
    let mut else_tail_idx = end_idx;
    let mut shared = Vec::new();

    while let Some((cur_then_idx, then_assign)) = last_non_empty_assign_before(lines, then_idx) {
        let Some((cur_else_idx, else_assign)) = last_non_empty_assign_before(lines, else_tail_idx)
        else {
            break;
        };
        if cur_then_idx <= if_idx
            || cur_else_idx <= else_idx
            || then_assign.trim() != else_assign.trim()
        {
            break;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(then_assign.trim())) else {
            break;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
            break;
        }
        shared.push(SharedTailAssignment {
            then_idx: cur_then_idx,
            else_idx: cur_else_idx,
            text: then_assign.trim().to_string(),
        });
        then_idx = cur_then_idx;
        else_tail_idx = cur_else_idx;
    }

    shared.reverse();
    shared
}

pub(crate) fn move_shared_tail_assignments_after_if(
    lines: &mut Vec<String>,
    end_idx: usize,
    shared: &[SharedTailAssignment],
) -> usize {
    let indent_len = shared[0].text.len() - shared[0].text.trim_start().len();
    let indent = " ".repeat(indent_len);
    for item in shared {
        lines[item.then_idx].clear();
        lines[item.else_idx].clear();
    }

    let mut insert_at = end_idx + 1;
    for item in shared {
        lines.insert(insert_at, format!("{indent}{}", item.text));
        insert_at += 1;
    }
    insert_at
}

pub(crate) fn collapse_common_if_else_tail_assignments(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut i = 0usize;
    while i < out.len() {
        let trimmed = out[i].trim();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            i += 1;
            continue;
        }
        let Some((else_idx, end_idx)) = find_if_else_bounds(&out, i) else {
            i += 1;
            continue;
        };
        let shared = collect_common_if_else_tail_assignments(&out, i, else_idx, end_idx);
        if shared.is_empty() {
            i = end_idx + 1;
            continue;
        }
        i = move_shared_tail_assignments_after_if(&mut out, end_idx, &shared);
    }
    out
}

pub(crate) fn collapse_identical_if_else_tail_assignments_late(lines: Vec<String>) -> Vec<String> {
    collapse_identical_if_else_tail_assignments_late_ir(lines)
}

pub(crate) fn rewrite_safe_loop_index_write_calls(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut repeat_idx = 0usize;
    while repeat_idx < out.len() {
        let Some(next_repeat) = (repeat_idx..out.len()).find(|idx| out[*idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_matching_block_end(&out, next_repeat) else {
            break;
        };
        let Some(guard_idx) = (next_repeat + 1..loop_end).find(|idx| {
            let trimmed = out[*idx].trim();
            trimmed.starts_with("if (!(") && trimmed.ends_with(")) break")
        }) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some((iter_var, _op, _bound)) = parse_repeat_guard_cmp_line(out[guard_idx].trim())
        else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let start_value = latest_literal_assignment_before(&out, guard_idx, &iter_var);
        if !plain_ident_re().is_some_and(|re| re.is_match(&iter_var))
            || start_value.is_none_or(|value| value < 1)
        {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let mut safe = true;
        for line in out.iter().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                continue;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs != iter_var {
                continue;
            }
            let canonical_incr = rhs == format!("({iter_var} + 1)")
                || rhs == format!("({iter_var} + 1L)")
                || rhs == format!("({iter_var} + 1.0)");
            if !canonical_incr {
                safe = false;
                break;
            }
        }
        if !safe {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let needle = format!("rr_index1_write({iter_var}, \"index\")");
        let needle_single = format!("rr_index1_write({iter_var}, 'index')");
        for line in out.iter_mut().take(loop_end).skip(guard_idx + 1) {
            if line.contains(&needle) {
                *line = line.replace(&needle, &iter_var);
            }
            if line.contains(&needle_single) {
                *line = line.replace(&needle_single, &iter_var);
            }
        }
        repeat_idx = next_repeat + 1;
    }
    out
}

pub(crate) fn rewrite_safe_loop_neighbor_read_calls(lines: Vec<String>) -> Vec<String> {
    #[derive(Clone, Copy)]
    struct NeighborReadPolicy {
        pub(crate) allow_prev: bool,
        pub(crate) allow_next: bool,
    }

    fn rewrite_loop_read_expr(expr: &str, iter_var: &str, policy: NeighborReadPolicy) -> String {
        let mut out = expr.to_string();
        let iter_esc = regex::escape(iter_var);
        let ctx = r#"(?:"index"|'index')"#;

        let direct_pat = format!(
            r#"rr_index1_read\((?P<base>{}),\s*{}\s*,\s*{}\)"#,
            IDENT_PATTERN, iter_esc, ctx
        );
        if let Some(re) = compile_regex(direct_pat) {
            out = re
                .replace_all(&out, |caps: &Captures<'_>| {
                    let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                    format!("{base}[{iter_var}]")
                })
                .to_string();
        }

        if policy.allow_prev {
            let prev_pat = format!(
                r#"rr_index1_read\((?P<base>{}),\s*\(\s*{}\s*-\s*1(?:L|\.0+)?\s*\)\s*,\s*{}\)"#,
                IDENT_PATTERN, iter_esc, ctx
            );
            if let Some(re) = compile_regex(prev_pat) {
                out = re
                    .replace_all(&out, |caps: &Captures<'_>| {
                        let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                        format!("{base}[({iter_var} - 1)]")
                    })
                    .to_string();
            }
        }

        if policy.allow_next {
            let next_pat = format!(
                r#"rr_index1_read\((?P<base>{}),\s*\(\s*{}\s*\+\s*1(?:L|\.0+)?\s*\)\s*,\s*{}\)"#,
                IDENT_PATTERN, iter_esc, ctx
            );
            if let Some(re) = compile_regex(next_pat) {
                out = re
                    .replace_all(&out, |caps: &Captures<'_>| {
                        let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                        format!("{base}[({iter_var} + 1)]")
                    })
                    .to_string();
            }
        }
        out
    }

    let mut out = lines;
    let mut repeat_idx = 0usize;
    while repeat_idx < out.len() {
        let Some(next_repeat) = (repeat_idx..out.len()).find(|idx| out[*idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_matching_block_end(&out, next_repeat) else {
            break;
        };
        let Some(guard_idx) = (next_repeat + 1..loop_end).find(|idx| {
            let trimmed = out[*idx].trim();
            trimmed.starts_with("if !(")
                || (trimmed.starts_with("if (!(") && trimmed.ends_with("break"))
        }) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some((iter_var, op, _bound)) = parse_repeat_guard_cmp_line(out[guard_idx].trim())
        else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let start_value = latest_literal_assignment_before(&out, guard_idx, &iter_var);
        if !plain_ident_re().is_some_and(|re| re.is_match(&iter_var))
            || start_value.is_none_or(|value| value < 1)
        {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let mut safe = true;
        for line in out.iter().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                continue;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs != iter_var {
                continue;
            }
            let canonical_incr = rhs == format!("({iter_var} + 1)")
                || rhs == format!("({iter_var} + 1L)")
                || rhs == format!("({iter_var} + 1.0)");
            if !canonical_incr {
                safe = false;
                break;
            }
        }
        if !safe {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let allow_prev = start_value.is_some_and(|value| value >= 2);
        let allow_next = op == "<";
        let policy = NeighborReadPolicy {
            allow_prev,
            allow_next,
        };
        for line in out.iter_mut().take(loop_end).skip(guard_idx + 1) {
            *line = rewrite_loop_read_expr(line, &iter_var, policy);
        }
        repeat_idx = next_repeat + 1;
    }
    out
}

pub(crate) fn is_branch_open_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    let is_single_line_guard =
        trimmed.starts_with("if ") && (trimmed.ends_with(" break") || trimmed.ends_with(" next"));
    (trimmed.starts_with("if ") && !is_single_line_guard)
        || trimmed.starts_with("if(")
        || trimmed.starts_with("else")
        || trimmed.starts_with("} else")
}

pub(crate) fn is_loop_open_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "repeat {" || trimmed.starts_with("while") || trimmed.starts_with("for")
}

pub(crate) fn line_is_within_loop_body(lines: &[String], idx: usize) -> bool {
    (0..idx).rev().any(|start_idx| {
        if !is_loop_open_boundary(lines[start_idx].trim()) {
            return false;
        }
        find_matching_block_end(lines, start_idx).is_some_and(|end_idx| idx < end_idx)
    })
}

pub(crate) fn strip_redundant_identical_pure_rebinds(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    strip_redundant_identical_pure_rebinds_ir(lines, pure_user_calls)
}

pub(crate) fn strip_redundant_identical_pure_rebinds_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    _cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    strip_redundant_identical_pure_rebinds_ir(lines, pure_user_calls)
}

pub(crate) fn find_if_else_bounds(lines: &[String], if_idx: usize) -> Option<(usize, usize)> {
    let mut depth = 1usize;
    let mut else_idx = None;
    for (idx, line) in lines.iter().enumerate().skip(if_idx + 1) {
        let trimmed = line.trim();
        if trimmed == "} else {" && depth == 1 {
            else_idx = Some(idx);
            continue;
        }
        if trimmed.ends_with('{') {
            depth += 1;
            continue;
        }
        if trimmed == "}" {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return else_idx.map(|else_idx| (else_idx, idx));
            }
        }
    }
    None
}

pub(crate) fn last_non_empty_assign_before(
    lines: &[String],
    end_exclusive: usize,
) -> Option<(usize, &str)> {
    for idx in (0..end_exclusive).rev() {
        let trimmed = lines[idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        return assign_re()
            .and_then(|re| re.captures(trimmed))
            .map(|_| (idx, lines[idx].as_str()));
    }
    None
}

pub(crate) fn expr_is_fresh_allocation_like(
    expr: &str,
    fresh_user_calls: &FxHashSet<String>,
) -> bool {
    const FRESH_BUILTINS: &[&str] = &[
        "rep.int",
        "numeric",
        "integer",
        "logical",
        "character",
        "vector",
        "matrix",
        "c",
        "seq_len",
        "seq_along",
        "rr_named_list",
    ];
    let rhs = expr.trim();
    let Some((callee, _rest)) = rhs.split_once('(') else {
        return false;
    };
    let callee = callee.trim();
    FRESH_BUILTINS.contains(&callee) || fresh_user_calls.contains(callee)
}

pub(crate) fn expr_has_only_pure_calls(expr: &str, pure_user_calls: &FxHashSet<String>) -> bool {
    const PURE_CALLS: &[&str] = &[
        "abs",
        "sqrt",
        "log",
        "log10",
        "log2",
        "exp",
        "sign",
        "floor",
        "ceiling",
        "trunc",
        "sin",
        "cos",
        "tan",
        "asin",
        "acos",
        "atan",
        "atan2",
        "sinh",
        "cosh",
        "tanh",
        "length",
        "seq_len",
        "seq_along",
        "mean",
        "sum",
        "min",
        "max",
        "pmin",
        "pmax",
        "is.na",
        "is.finite",
        "rep.int",
        "numeric",
        "vector",
        "matrix",
        "c",
        "ifelse",
        "rr_index1_read",
        "rr_index1_read_vec",
        "rr_index1_read_vec_floor",
        "rr_index_vec_floor",
        "rr_gather",
        "rr_wrap_index_vec_i",
        "rr_idx_cube_vec_i",
        "rr_parallel_typed_vec_call",
        "rr_field_get",
        "rr_field_exists",
        "rr_list_pattern_matchable",
        "rr_named_list",
        "replace",
    ];
    let expr = expr.trim();
    if expr.contains("<-")
        || expr.contains("function(")
        || expr.contains("tryCatch(")
        || expr.contains("dyn.load(")
        || expr.contains("print(")
        || expr.contains("cat(")
        || expr.contains("message(")
        || expr.contains("warning(")
        || expr.contains("stop(")
        || expr.contains("quit(")
    {
        return false;
    }
    fn pure_call_ident_re() -> Option<&'static Regex> {
        static RE: OnceLock<Option<Regex>> = OnceLock::new();
        RE.get_or_init(|| compile_regex(format!(r"(?P<callee>{})\s*\(", IDENT_PATTERN)))
            .as_ref()
    }
    let Some(re) = pure_call_ident_re() else {
        return false;
    };
    re.captures_iter(expr).all(|caps| {
        let callee = caps.name("callee").map(|m| m.as_str()).unwrap_or("").trim();
        PURE_CALLS.contains(&callee) || pure_user_calls.contains(callee)
    })
}

pub(crate) fn find_matching_block_end(lines: &[String], start_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
        let (opens, closes) = count_unquoted_braces(line);
        depth += opens;
        if closes > 0 {
            depth = depth.saturating_sub(closes);
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

pub(crate) fn sym_ref_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"\b(?P<name>Sym_[A-Za-z0-9_]+)\b".to_string()))
        .as_ref()
}

pub(crate) fn unquoted_sym_refs(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Some(re) = sym_ref_re() else {
        return out;
    };
    for caps in re.captures_iter(line) {
        let Some(mat) = caps.name("name") else {
            continue;
        };
        let start = mat.start();
        let end = mat.end();
        let prev = line[..start].chars().next_back();
        let next = line[end..].chars().next();
        if matches!(prev, Some('"') | Some('\'')) && matches!(next, Some('"') | Some('\'')) {
            continue;
        }
        out.push(mat.as_str().to_string());
    }
    out
}

pub(crate) fn strip_unreachable_sym_helpers(lines: Vec<String>) -> Vec<String> {
    strip_unreachable_sym_helpers_ir(lines)
}
