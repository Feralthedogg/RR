use super::{
    FxHashMap, FxHashSet, IDENT_PATTERN, assign_re, compile_regex, count_unquoted_braces,
    expr_idents, indexed_store_base_re, latest_literal_assignment_before,
    parse_repeat_guard_cmp_line, plain_ident_re, strip_redundant_outer_parens,
};
use regex::{Captures, Regex};
use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub(super) struct PureCallBinding {
    pub(super) expr: String,
    pub(super) var: String,
    pub(super) deps: FxHashSet<String>,
}

pub(super) fn extract_pure_call_binding(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> Option<PureCallBinding> {
    let rhs = rhs.trim();
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

pub(super) fn rewrite_pure_call_reuse(expr: &str, bindings: &[PureCallBinding]) -> String {
    let mut out = expr.to_string();
    for binding in bindings {
        if out.contains(&binding.expr) {
            out = out.replace(&binding.expr, &binding.var);
        }
    }
    out
}

pub(super) fn rewrite_return_expr_line(
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

pub(super) fn written_base_var(lhs: &str) -> Option<&str> {
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

pub(super) fn maybe_expand_fresh_alias_rhs(
    rhs: &str,
    fresh_expr_for_var: &FxHashMap<String, String>,
) -> Option<String> {
    let ident = rhs.trim();
    if !plain_ident_re().is_some_and(|re| re.is_match(ident)) {
        return None;
    }
    fresh_expr_for_var.get(ident).cloned()
}

pub(super) fn collapse_common_if_else_tail_assignments(lines: Vec<String>) -> Vec<String> {
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
        let mut then_idx = else_idx;
        let mut else_tail_idx = end_idx;
        let mut shared = Vec::<(usize, usize, String)>::new();
        loop {
            let Some((cur_then_idx, then_assign)) = last_non_empty_assign_before(&out, then_idx)
            else {
                break;
            };
            let Some((cur_else_idx, else_assign)) =
                last_non_empty_assign_before(&out, else_tail_idx)
            else {
                break;
            };
            if cur_then_idx <= i || cur_else_idx <= else_idx {
                break;
            }
            if then_assign.trim() != else_assign.trim() {
                break;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(then_assign.trim())) else {
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                break;
            }
            shared.push((cur_then_idx, cur_else_idx, then_assign.trim().to_string()));
            then_idx = cur_then_idx;
            else_tail_idx = cur_else_idx;
        }
        if shared.is_empty() {
            i = end_idx + 1;
            continue;
        }
        shared.reverse();
        let indent_len = shared[0].2.len() - shared[0].2.trim_start().len();
        let indent = " ".repeat(indent_len);
        for (then_assign_idx, else_assign_idx, _) in &shared {
            out[*then_assign_idx].clear();
            out[*else_assign_idx].clear();
        }
        let mut insert_at = end_idx + 1;
        for (_, _, assign) in &shared {
            out.insert(insert_at, format!("{indent}{assign}"));
            insert_at += 1;
        }
        i = insert_at;
    }
    out
}

pub(super) fn collapse_identical_if_else_tail_assignments_late(lines: Vec<String>) -> Vec<String> {
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

        let then_lines: Vec<usize> = ((i + 1)..else_idx)
            .filter(|idx| {
                let t = out[*idx].trim();
                !t.is_empty()
            })
            .collect();
        let else_lines: Vec<usize> = ((else_idx + 1)..end_idx)
            .filter(|idx| {
                let t = out[*idx].trim();
                !t.is_empty()
            })
            .collect();

        let mut t = then_lines.len();
        let mut e = else_lines.len();
        let mut shared = Vec::<(usize, usize, String)>::new();
        while t > 0 && e > 0 {
            let then_idx = then_lines[t - 1];
            let else_line_idx = else_lines[e - 1];
            let then_trimmed = out[then_idx].trim();
            let else_trimmed = out[else_line_idx].trim();
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
        let indent_len = out[i].len() - out[i].trim_start().len();
        let indent = " ".repeat(indent_len);
        for (then_idx, else_idx_line, _) in &shared {
            out[*then_idx].clear();
            out[*else_idx_line].clear();
        }
        let mut insert_at = end_idx + 1;
        for (_, _, assign) in &shared {
            out.insert(insert_at, format!("{indent}{assign}"));
            insert_at += 1;
        }
        i = insert_at;
    }
    out
}

pub(super) fn rewrite_safe_loop_index_write_calls(lines: Vec<String>) -> Vec<String> {
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

pub(super) fn rewrite_safe_loop_neighbor_read_calls(lines: Vec<String>) -> Vec<String> {
    fn rewrite_loop_read_expr(
        expr: &str,
        iter_var: &str,
        allow_prev: bool,
        allow_next: bool,
    ) -> String {
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

        if allow_prev {
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

        if allow_next {
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
        for line in out.iter_mut().take(loop_end).skip(guard_idx + 1) {
            *line = rewrite_loop_read_expr(line, &iter_var, allow_prev, allow_next);
        }
        repeat_idx = next_repeat + 1;
    }
    out
}

fn is_branch_open_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    let is_single_line_guard =
        trimmed.starts_with("if ") && (trimmed.ends_with(" break") || trimmed.ends_with(" next"));
    (trimmed.starts_with("if ") && !is_single_line_guard)
        || trimmed.starts_with("if(")
        || trimmed.starts_with("else")
        || trimmed.starts_with("} else")
}

pub(super) fn is_loop_open_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "repeat {" || trimmed.starts_with("while") || trimmed.starts_with("for")
}

pub(super) fn line_is_within_loop_body(lines: &[String], idx: usize) -> bool {
    (0..idx).rev().any(|start_idx| {
        if !is_loop_open_boundary(lines[start_idx].trim()) {
            return false;
        }
        find_matching_block_end(lines, start_idx).is_some_and(|end_idx| idx < end_idx)
    })
}

fn is_identical_pure_rebind_candidate(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(lhs))
        && !lhs.starts_with(".arg_")
        && !lhs.starts_with(".__rr_cse_")
        && rhs.contains('(')
        && expr_has_only_pure_calls(rhs, pure_user_calls)
}

pub(super) fn strip_redundant_identical_pure_rebinds(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !is_identical_pure_rebind_candidate(lhs, rhs, pure_user_calls) {
            continue;
        }
        let rhs_canonical = strip_redundant_outer_parens(rhs);
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let cur_indent = out[idx].len() - out[idx].trim_start().len();
        let mut depth = 0usize;
        let mut crossed_enclosing_if_boundary = false;
        let mut removable = false;
        for prev_idx in (0..idx).rev() {
            let prev_line = out[prev_idx].as_str();
            let prev_trimmed = prev_line.trim();
            if prev_trimmed.is_empty() {
                continue;
            }
            if prev_line.contains("<- function") {
                break;
            }
            if prev_trimmed == "}" {
                depth += 1;
                continue;
            }
            if is_branch_open_boundary(prev_trimmed) || is_loop_open_boundary(prev_trimmed) {
                if depth > 0 {
                    depth -= 1;
                    if depth == 0 && is_branch_open_boundary(prev_trimmed) {
                        break;
                    }
                    continue;
                }
                if is_branch_open_boundary(prev_trimmed) {
                    let can_cross_current_if = !crossed_enclosing_if_boundary
                        && (prev_trimmed.starts_with("if ") || prev_trimmed.starts_with("if("));
                    if can_cross_current_if {
                        crossed_enclosing_if_boundary = true;
                        continue;
                    }
                    break;
                }
                continue;
            }
            if let Some(base) = indexed_store_base_re()
                .and_then(|re| re.captures(prev_trimmed))
                .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
            {
                if base == lhs || deps.contains(&base) {
                    break;
                }
                continue;
            }
            let Some(prev_caps) = assign_re().and_then(|re| re.captures(prev_trimmed)) else {
                continue;
            };
            let prev_lhs = prev_caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let prev_rhs = prev_caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if prev_lhs == lhs {
                if depth == 0 {
                    let prev_indent = out[prev_idx].len() - out[prev_idx].trim_start().len();
                    let same_scope_rebind = prev_indent == cur_indent;
                    let enclosing_if_rebind =
                        crossed_enclosing_if_boundary && prev_indent < cur_indent;
                    if strip_redundant_outer_parens(prev_rhs) == rhs_canonical
                        && (same_scope_rebind || enclosing_if_rebind)
                    {
                        removable = true;
                    }
                }
                break;
            }
            if deps.contains(prev_lhs) {
                break;
            }
        }
        if removable {
            out[idx].clear();
        }
    }
    out
}

fn find_if_else_bounds(lines: &[String], if_idx: usize) -> Option<(usize, usize)> {
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

fn last_non_empty_assign_before(lines: &[String], end_exclusive: usize) -> Option<(usize, &str)> {
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

pub(super) fn expr_is_fresh_allocation_like(
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

pub(super) fn expr_has_only_pure_calls(expr: &str, pure_user_calls: &FxHashSet<String>) -> bool {
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
    let Some(re) = compile_regex(format!(r"(?P<callee>{})\s*\(", IDENT_PATTERN)) else {
        return false;
    };
    re.captures_iter(expr).all(|caps| {
        let callee = caps.name("callee").map(|m| m.as_str()).unwrap_or("").trim();
        PURE_CALLS.contains(&callee) || pure_user_calls.contains(callee)
    })
}

pub(super) fn find_matching_block_end(lines: &[String], start_idx: usize) -> Option<usize> {
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

fn sym_ref_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"\b(?P<name>Sym_[A-Za-z0-9_]+)\b".to_string()))
        .as_ref()
}

pub(super) fn unquoted_sym_refs(line: &str) -> Vec<String> {
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

pub(super) fn strip_unreachable_sym_helpers(lines: Vec<String>) -> Vec<String> {
    #[derive(Clone)]
    struct FnRange {
        name: String,
        start: usize,
        end: usize,
    }

    let mut ranges = Vec::<FnRange>::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        if trimmed.starts_with("Sym_") && trimmed.contains("<- function") {
            let Some((name, _)) = trimmed.split_once("<- function") else {
                idx += 1;
                continue;
            };
            let Some(end) = find_matching_block_end(&lines, idx) else {
                break;
            };
            ranges.push(FnRange {
                name: name.trim().to_string(),
                start: idx,
                end,
            });
            idx = end + 1;
            continue;
        }
        idx += 1;
    }
    if ranges.is_empty() {
        return lines;
    }

    let mut name_to_range = FxHashMap::default();
    for range in &ranges {
        name_to_range.insert(range.name.clone(), range.clone());
    }

    let sym_top_is_empty_entrypoint = |range: &FnRange| {
        let mut saw_return_null = false;
        for line in lines.iter().take(range.end).skip(range.start + 1) {
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

    let mut roots = FxHashSet::default();
    if name_to_range.contains_key("Sym_top_0") {
        roots.insert("Sym_top_0".to_string());
    }
    let mut line_idx = 0usize;
    while line_idx < lines.len() {
        if let Some(range) = ranges.iter().find(|range| range.start == line_idx) {
            line_idx = range.end + 1;
            continue;
        }
        for name in unquoted_sym_refs(lines[line_idx].as_str()) {
            if name_to_range.contains_key(&name) {
                roots.insert(name);
            }
        }
        line_idx += 1;
    }
    if roots.is_empty() {
        return lines;
    }
    if roots.len() == 1
        && roots.contains("Sym_top_0")
        && name_to_range
            .get("Sym_top_0")
            .is_some_and(sym_top_is_empty_entrypoint)
    {
        return lines;
    }

    let mut reachable = roots.clone();
    let mut work: Vec<String> = roots.into_iter().collect();
    while let Some(name) = work.pop() {
        let Some(range) = name_to_range.get(&name) else {
            continue;
        };
        for line in lines.iter().take(range.end).skip(range.start + 1) {
            for callee in unquoted_sym_refs(line.as_str()) {
                if name_to_range.contains_key(&callee) && reachable.insert(callee.clone()) {
                    work.push(callee);
                }
            }
        }
    }

    let mut out = Vec::with_capacity(lines.len());
    let mut cursor = 0usize;
    for range in ranges {
        while cursor < range.start {
            out.push(lines[cursor].clone());
            cursor += 1;
        }
        if reachable.contains(&range.name) {
            while cursor <= range.end {
                out.push(lines[cursor].clone());
                cursor += 1;
            }
        } else {
            cursor = range.end + 1;
        }
    }
    while cursor < lines.len() {
        out.push(lines[cursor].clone());
        cursor += 1;
    }
    out
}
