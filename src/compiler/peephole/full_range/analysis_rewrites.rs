use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SecondaryFullRangePlan {
    pub(crate) needs_inline_slice_ops: bool,
    pub(crate) needs_contextual_gather_replays: bool,
}

impl SecondaryFullRangePlan {
    pub(crate) fn needs_any(self) -> bool {
        self.needs_inline_slice_ops || self.needs_contextual_gather_replays
    }
}

pub(crate) fn analyze_secondary_full_range_plan(lines: &[String]) -> SecondaryFullRangePlan {
    let mut plan = SecondaryFullRangePlan::default();
    for line in lines {
        if !plan.needs_inline_slice_ops
            && (line.contains("rr_assign_slice(") || line.contains("rr_call_map_slice_auto("))
        {
            plan.needs_inline_slice_ops = true;
        }
        if !plan.needs_contextual_gather_replays
            && line.contains("rr_gather(")
            && line.contains("rr_index1_read_vec")
            && line.contains("rr_assign_slice(")
        {
            plan.needs_contextual_gather_replays = true;
        }
        if plan.needs_inline_slice_ops && plan.needs_contextual_gather_replays {
            break;
        }
    }
    plan
}

pub(crate) fn run_secondary_full_range_bundle(
    lines: Vec<String>,
    direct_builtin_call_map: bool,
) -> Vec<String> {
    let plan = analyze_secondary_full_range_plan(&lines);
    if !plan.needs_any() {
        return lines;
    }

    let contextual_re = if plan.needs_contextual_gather_replays {
        compile_regex(format!(
            r"^(?P<lhs>{id}) <- rr_assign_slice\((?P<dest>{id}),\s*(?P<start>[^,]+),\s*(?P<end>[^,]+),\s*rr_gather\((?P<base>{id}),\s*rr_index_vec_floor\(rr_index1_read_vec(?:_floor)?\((?P<inner_base>{id}),\s*rr_index_vec_floor\((?P<inner_start>[^:]+):(?P<inner_end>[^\)]+)\)\)\)\)\)$",
            id = IDENT_PATTERN
        ))
    } else {
        None
    };

    let mut out = lines;
    let mut whole_range_index_aliases: FxHashMap<String, String> = FxHashMap::default();
    for idx in 0..out.len() {
        let original = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&original)) else {
            if is_control_flow_boundary(&original) || out[idx].contains("<- function") {
                whole_range_index_aliases.clear();
            }
            continue;
        };
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let mut rewritten_rhs = rhs.to_string();

        if plan.needs_inline_slice_ops {
            if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(&rewritten_rhs)) {
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
                if lhs == dest
                    && start_expr_is_one_in_context(&out, idx, start)
                    && end_expr_can_cover_full_range(&out, idx, end)
                {
                    let rewritten_rest = rewrite_inline_full_range_reads_with_aliases(
                        rest,
                        start,
                        end,
                        &whole_range_index_aliases,
                    )
                    .replace("rr_ifelse_strict(", "ifelse(");
                    rewritten_rhs = rewritten_rest;
                }
            }

            if let Some(call_caps) = call_map_slice_re().and_then(|re| re.captures(&rewritten_rhs))
            {
                let dest = call_caps
                    .name("dest")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let start = call_caps
                    .name("start")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let end = call_caps
                    .name("end")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let rest = call_caps
                    .name("rest")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if lhs == dest
                    && start_expr_is_one_in_context(&out, idx, start)
                    && end_expr_can_cover_full_range(&out, idx, end)
                {
                    let Some(parts) = split_top_level_args(rest) else {
                        whole_range_index_aliases.remove(lhs);
                        continue;
                    };
                    if parts.len() < 4 {
                        whole_range_index_aliases.remove(lhs);
                        continue;
                    }
                    let callee = parts[0].trim().trim_matches('"');
                    let slots = parts[2].trim();
                    let args: Vec<String> = parts[3..]
                        .iter()
                        .map(|arg| {
                            rewrite_inline_full_range_reads_with_aliases(
                                arg,
                                start,
                                end,
                                &whole_range_index_aliases,
                            )
                        })
                        .collect();
                    let mut call_map_rewritten = None;
                    if direct_builtin_call_map && (slots == "c(1L)" || slots == "c(1)") {
                        match (callee, args.as_slice()) {
                            ("pmax", [a, b]) | ("pmin", [a, b]) => {
                                call_map_rewritten = Some(format!("{callee}({a}, {b})"));
                            }
                            ("abs", [a]) | ("sqrt", [a]) | ("log", [a]) => {
                                call_map_rewritten = Some(format!("{callee}({a})"));
                            }
                            _ => {}
                        }
                    }
                    if let Some(new_rhs) = call_map_rewritten {
                        rewritten_rhs = new_rhs;
                    } else {
                        let joined = parts
                            .iter()
                            .take(3)
                            .cloned()
                            .chain(args)
                            .collect::<Vec<_>>()
                            .join(", ");
                        rewritten_rhs = format!("rr_call_map_whole_auto({dest}, {joined})");
                    }
                }
            }
        }

        if plan.needs_contextual_gather_replays {
            let candidate_line = format!("{lhs} <- {rewritten_rhs}");
            if let Some(caps) = contextual_re
                .as_ref()
                .and_then(|re| re.captures(&candidate_line))
            {
                let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
                let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
                let end = caps.name("end").map(|m| m.as_str()).unwrap_or("").trim();
                let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                let inner_base = caps
                    .name("inner_base")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let inner_start = caps
                    .name("inner_start")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let inner_end = caps
                    .name("inner_end")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if lhs == dest
                    && base == inner_base
                    && compact_expr(start) == compact_expr(inner_start)
                    && compact_expr(end) == compact_expr(inner_end)
                    && start_expr_is_one_in_context(&out, idx, start)
                    && end_expr_can_cover_full_range(&out, idx, end)
                {
                    rewritten_rhs = format!("rr_gather({base}, rr_index_vec_floor({base}))");
                }
            }
        }

        if lhs.starts_with('.')
            && (rewritten_rhs.contains(':') || rewritten_rhs.starts_with("rr_index_vec_floor("))
        {
            whole_range_index_aliases.insert(lhs.to_string(), rewritten_rhs.clone());
        } else {
            whole_range_index_aliases.remove(lhs);
        }

        if rewritten_rhs != rhs {
            out[idx] = format!("{indent}{lhs} <- {rewritten_rhs}");
        }
    }
    out
}

pub(crate) fn rewrite_loop_index_reads_to_whole_expr(expr: &str, idx_var: &str) -> String {
    let pattern = format!(r"(?P<base>{})\[(?P<idx>[^\]]+)\]", IDENT_PATTERN);
    let Some(index_re) = compile_regex(pattern) else {
        return expr.to_string();
    };
    index_re
        .replace_all(expr, |caps: &Captures<'_>| {
            let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
            let idx = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
            if idx == idx_var {
                base.to_string()
            } else {
                caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
            }
        })
        .to_string()
}

pub(crate) fn parse_break_guard(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("if (!(")
        .and_then(|s| s.strip_suffix(")) break"))?;
    let (lhs, rhs) = inner.split_once("<=")?;
    Some((lhs.trim().to_string(), rhs.trim().to_string()))
}

pub(crate) fn has_one_based_full_range_index_alias_read_candidates(lines: &[String]) -> bool {
    let read_vec_re = compile_regex(format!(
        r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>{}|rr_index_vec_floor\([^\)]*\)|[^,\)]*:[^\)]*)\)",
        IDENT_PATTERN, IDENT_PATTERN
    ));
    let mut has_read = false;
    let mut has_range_alias = false;
    for line in lines {
        let trimmed = line.trim();
        if line.contains("<- function") {
            has_read = false;
            has_range_alias = false;
            continue;
        }
        if !has_read && trimmed.contains("rr_index1_read_vec") {
            has_read = true;
            if let Some(caps) = read_vec_re.as_ref().and_then(|re| re.captures(trimmed)) {
                let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                if expr_is_one_based_full_range_alias(idx_expr) {
                    return true;
                }
            }
        }
        if !has_range_alias && let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs.starts_with('.') && expr_is_one_based_full_range_alias(rhs) {
                has_range_alias = true;
            }
        }
        if has_read && has_range_alias {
            return true;
        }
    }
    false
}

pub(crate) fn parse_indexed_store_assign(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let (lhs, rhs) = trimmed.split_once("<-")?;
    let lhs = lhs.trim();
    let rhs = rhs.trim().to_string();
    let (base, idx) = lhs.split_once('[')?;
    let idx = idx.trim_end_matches(']').trim();
    Some((base.trim().to_string(), idx.to_string(), rhs))
}

pub(crate) struct FullRangeConditionalLoop {
    pub(crate) idx_var: String,
    pub(crate) end_expr: String,
    pub(crate) dest_base: String,
    pub(crate) cond_inner: String,
    pub(crate) then_rhs: String,
    pub(crate) else_rhs: String,
    pub(crate) end_idx: usize,
}

pub(crate) fn skip_blank_or_mark_lines(lines: &[String], mut cursor: usize) -> usize {
    while matches!(
        lines.get(cursor).map(|s| s.trim()),
        Some(line) if line.is_empty() || line.starts_with("rr_mark(")
    ) {
        cursor += 1;
    }
    cursor
}

pub(crate) fn parse_one_based_index_init(line: &str) -> Option<String> {
    let caps = assign_re().and_then(|re| re.captures(line.trim()))?;
    let idx_var = caps.name("lhs")?.as_str().trim();
    let init_rhs = caps.name("rhs")?.as_str().trim();
    (idx_var.starts_with("i_") && literal_one_re().is_some_and(|re| re.is_match(init_rhs)))
        .then(|| idx_var.to_string())
}

pub(crate) fn parse_if_condition_inner(line: &str) -> Option<&str> {
    line.strip_prefix("if ((")
        .and_then(|s| s.strip_suffix(")) {"))
        .or_else(|| {
            line.strip_prefix("if (")
                .and_then(|s| s.strip_suffix(") {"))
        })
}

pub(crate) fn parse_full_range_conditional_loop(
    lines: &[String],
    start: usize,
) -> Option<FullRangeConditionalLoop> {
    let idx_var = parse_one_based_index_init(&lines[start])?;
    (lines.get(start + 1).map(|s| s.trim())? == "repeat {").then_some(())?;

    let guard_cursor = skip_blank_or_mark_lines(lines, start + 2);
    let (guard_idx, end_expr) = lines.get(guard_cursor).and_then(|s| parse_break_guard(s))?;
    (guard_idx == idx_var).then_some(())?;

    let if_cursor = skip_blank_or_mark_lines(lines, guard_cursor + 1);
    let cond_inner = parse_if_condition_inner(lines.get(if_cursor).map(|s| s.trim())?)?
        .trim()
        .to_string();

    let then_cursor = skip_blank_or_mark_lines(lines, if_cursor + 1);
    let then_line = lines.get(then_cursor).map(|s| s.trim()).unwrap_or("");
    let else_header = lines.get(then_cursor + 1).map(|s| s.trim()).unwrap_or("");
    let else_cursor = skip_blank_or_mark_lines(lines, then_cursor + 2);
    let else_line = lines.get(else_cursor).map(|s| s.trim()).unwrap_or("");
    let close_line = lines.get(else_cursor + 1).map(|s| s.trim()).unwrap_or("");
    let incr_line = lines.get(else_cursor + 2).map(|s| s.trim()).unwrap_or("");
    let next_line = lines.get(else_cursor + 3).map(|s| s.trim()).unwrap_or("");
    let end_repeat = lines.get(else_cursor + 4).map(|s| s.trim()).unwrap_or("");
    if else_header != "} else {" || close_line != "}" || next_line != "next" || end_repeat != "}" {
        return None;
    }

    let (dest_base, then_idx, then_rhs) = parse_indexed_store_assign(then_line)?;
    let (else_base, else_idx, else_rhs) = parse_indexed_store_assign(else_line)?;
    if dest_base != else_base || then_idx != idx_var || else_idx != idx_var {
        return None;
    }

    let expected_incr = format!("{idx_var} <- ({idx_var} + 1L)");
    (incr_line == expected_incr).then_some(FullRangeConditionalLoop {
        idx_var,
        end_expr,
        dest_base,
        cond_inner,
        then_rhs,
        else_rhs,
        end_idx: else_cursor + 5,
    })
}

pub(crate) fn rewrite_full_range_conditional_loop_line(
    original_init_line: &str,
    plan: &FullRangeConditionalLoop,
) -> String {
    let cond_whole = rewrite_loop_index_reads_to_whole_expr(&plan.cond_inner, &plan.idx_var);
    let then_whole = rewrite_loop_index_reads_to_whole_expr(&plan.then_rhs, &plan.idx_var);
    let else_whole = rewrite_loop_index_reads_to_whole_expr(&plan.else_rhs, &plan.idx_var);
    let indent =
        &original_init_line[..original_init_line.len() - original_init_line.trim_start().len()];
    format!(
        "{indent}{} <- rr_assign_slice({}, 1L, {}, rr_ifelse_strict(({}), {}, {}))",
        plan.dest_base, plan.dest_base, plan.end_expr, cond_whole, then_whole, else_whole
    )
}

pub(crate) fn rewrite_full_range_conditional_scalar_loops(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        if let Some(plan) = parse_full_range_conditional_loop(&lines, i) {
            out.push(rewrite_full_range_conditional_loop_line(&lines[i], &plan));
            i = plan.end_idx;
        } else {
            out.push(lines[i].clone());
            i += 1;
        }
    }
    out
}

pub(crate) fn rewrite_inline_full_range_reads(expr: &str, start: &str, end: &str) -> String {
    rewrite_index1_read_vec_calls(expr, |base, idx_expr| {
        expr_is_full_range_index_alias(idx_expr, start, end).then(|| base.to_string())
    })
}

pub(crate) fn compact_expr(expr: &str) -> String {
    expr.chars().filter(|c| !c.is_whitespace()).collect()
}

pub(crate) fn expr_is_full_range_index_alias(expr: &str, start: &str, end: &str) -> bool {
    let expr = compact_expr(expr);
    let mut starts = vec![start.trim().to_string()];
    for one in ["1L", "1", "1.0", "1.0L"] {
        if !starts.iter().any(|s| s == one) {
            starts.push(one.to_string());
        }
    }
    starts.into_iter().any(|start_expr| {
        let direct = compact_expr(&format!("{}:{}", start_expr, end.trim()));
        let floor = compact_expr(&format!(
            "rr_index_vec_floor({}:{})",
            start_expr,
            end.trim()
        ));
        expr == direct || expr == floor
    })
}

pub(crate) fn expr_is_one_based_full_range_alias(expr: &str) -> bool {
    let expr = compact_expr(expr);
    let starts = ["1L", "1", "1.0", "1.0L"];
    starts.iter().any(|start_expr| {
        expr.starts_with(&format!("{}:", start_expr))
            || expr.starts_with(&format!("rr_index_vec_floor({}:", start_expr))
    })
}

pub(crate) fn call_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
}

pub(crate) fn match_balanced_call_span(
    expr: &str,
    start: usize,
    callee: &str,
) -> Option<(usize, usize)> {
    if !expr.get(start..)?.starts_with(callee) {
        return None;
    }
    if expr[..start]
        .chars()
        .next_back()
        .is_some_and(call_ident_char)
    {
        return None;
    }
    let open = start + callee.len();
    if expr.as_bytes().get(open).copied() != Some(b'(') {
        return None;
    }
    let mut depth = 0i32;
    for (off, ch) in expr[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, open + off + 1));
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn rewrite_index1_read_vec_calls<F>(expr: &str, mut replacement: F) -> String
where
    F: FnMut(&str, &str) -> Option<String>,
{
    if !expr.contains("rr_index1_read_vec") {
        return expr.to_string();
    }
    let callees = ["rr_index1_read_vec_floor", "rr_index1_read_vec"];
    let mut out = String::with_capacity(expr.len());
    let mut cursor = 0usize;
    let mut idx = 0usize;
    while idx < expr.len() {
        let mut matched = None;
        for callee in callees {
            if let Some((start, end)) = match_balanced_call_span(expr, idx, callee) {
                matched = Some((callee, start, end));
                break;
            }
        }
        let Some((callee, start, end)) = matched else {
            let Some(ch) = expr[idx..].chars().next() else {
                break;
            };
            idx += ch.len_utf8();
            continue;
        };
        out.push_str(&expr[cursor..start]);
        let args_inner_start = start + callee.len() + 1;
        let args_inner_end = end.saturating_sub(1);
        let original = &expr[start..end];
        let rewritten = split_top_level_args(&expr[args_inner_start..args_inner_end])
            .and_then(|args| {
                if args.len() == 2 {
                    replacement(args[0].trim(), args[1].trim())
                } else {
                    None
                }
            })
            .unwrap_or_else(|| original.to_string());
        out.push_str(&rewritten);
        cursor = end;
        idx = end;
    }
    out.push_str(&expr[cursor..]);
    out
}

pub(crate) fn rewrite_inline_full_range_reads_with_aliases(
    expr: &str,
    start: &str,
    end: &str,
    whole_range_index_aliases: &FxHashMap<String, String>,
) -> String {
    let mut out = rewrite_inline_full_range_reads(expr, start, end);
    for (alias, alias_rhs) in whole_range_index_aliases {
        if !expr_is_full_range_index_alias(alias_rhs, start, end) {
            continue;
        }
        let alias_compact = compact_expr(alias);
        out = rewrite_index1_read_vec_calls(&out, |base, idx_expr| {
            (compact_expr(idx_expr) == alias_compact).then(|| base.to_string())
        });
    }
    out
}

pub(crate) fn start_expr_is_one_in_context(lines: &[String], idx: usize, start: &str) -> bool {
    if literal_one_re().is_some_and(|re| re.is_match(start.trim())) {
        return true;
    }
    for prev_idx in (0..idx).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
            continue;
        }
        if lines[prev_idx].contains("<- function") || is_control_flow_boundary(trimmed) {
            break;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs == start.trim() {
            return literal_one_re().is_some_and(|re| re.is_match(rhs));
        }
    }
    false
}

pub(crate) fn end_expr_is_singleton(end: &str) -> bool {
    literal_one_re().is_some_and(|re| re.is_match(end.trim()))
}

pub(crate) fn strip_redundant_outer_parens(expr: &str) -> &str {
    let mut expr = expr.trim();
    loop {
        if !(expr.starts_with('(') && expr.ends_with(')')) {
            break;
        }
        let mut depth = 0i32;
        let mut wraps = true;
        for (idx, ch) in expr.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && idx + ch.len_utf8() < expr.len() {
                        wraps = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !wraps {
            break;
        }
        expr = expr[1..expr.len() - 1].trim();
    }
    expr
}

pub(crate) fn expr_has_top_level_arith(expr: &str) -> bool {
    let expr = strip_redundant_outer_parens(expr);
    let bytes = expr.as_bytes();
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] as char {
            '(' => depth += 1,
            ')' => depth -= 1,
            '+' | '-' | '*' | '/' if depth == 0 => return true,
            '%' if depth == 0 && i + 1 < bytes.len() && bytes[i + 1] as char == '%' => {
                return true;
            }
            '"' | '\'' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == quote {
                        break;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

pub(crate) fn end_expr_can_cover_full_range(lines: &[String], idx: usize, end: &str) -> bool {
    let end = end.trim();
    if end_expr_is_singleton(end) {
        return false;
    }
    if !expr_has_top_level_arith(end) {
        return true;
    }
    if end.starts_with("length(") {
        return !expr_has_top_level_arith(
            end.strip_prefix("length(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(end),
        );
    }
    for prev_idx in (0..idx).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
            continue;
        }
        if lines[prev_idx].contains("<- function") || is_control_flow_boundary(trimmed) {
            break;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs == end {
            return !expr_has_top_level_arith(rhs);
        }
    }
    false
}

pub(crate) fn has_inline_full_range_slice_op_candidates(lines: &[String]) -> bool {
    for idx in 0..lines.len() {
        let trimmed = lines[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) {
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
            if lhs == dest
                && start_expr_is_one_in_context(lines, idx, start)
                && end_expr_can_cover_full_range(lines, idx, end)
            {
                return true;
            }
        }
        if let Some(call_caps) = call_map_slice_re().and_then(|re| re.captures(rhs)) {
            let dest = call_caps
                .name("dest")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let start = call_caps
                .name("start")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let end = call_caps
                .name("end")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if lhs == dest
                && start_expr_is_one_in_context(lines, idx, start)
                && end_expr_can_cover_full_range(lines, idx, end)
            {
                return true;
            }
        }
    }
    false
}

pub(crate) fn rewrite_inline_full_range_slice_ops(
    lines: Vec<String>,
    direct_builtin_call_map: bool,
) -> Vec<String> {
    if !has_inline_full_range_slice_op_candidates(&lines) {
        return lines;
    }
    let mut out = lines;
    let mut whole_range_index_aliases: FxHashMap<String, String> = FxHashMap::default();
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();

        if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) {
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
            if lhs == dest
                && start_expr_is_one_in_context(&out, idx, start)
                && end_expr_can_cover_full_range(&out, idx, end)
            {
                let rewritten_rest = rewrite_inline_full_range_reads_with_aliases(
                    rest,
                    start,
                    end,
                    &whole_range_index_aliases,
                );
                let rewritten_rest = rewritten_rest.replace("rr_ifelse_strict(", "ifelse(");
                out[idx] = format!("{lhs} <- {rewritten_rest}");
            } else {
                whole_range_index_aliases.remove(lhs);
                continue;
            }
        }

        if let Some(call_caps) = call_map_slice_re().and_then(|re| re.captures(rhs)) {
            let dest = call_caps
                .name("dest")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let start = call_caps
                .name("start")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let end = call_caps
                .name("end")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let rest = call_caps
                .name("rest")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if lhs == dest
                && start_expr_is_one_in_context(&out, idx, start)
                && end_expr_can_cover_full_range(&out, idx, end)
            {
                let Some(parts) = split_top_level_args(rest) else {
                    continue;
                };
                if parts.len() < 4 {
                    continue;
                }
                let callee = parts[0].trim().trim_matches('"');
                let slots = parts[2].trim();
                let args: Vec<String> = parts[3..]
                    .iter()
                    .map(|arg| {
                        rewrite_inline_full_range_reads_with_aliases(
                            arg,
                            start,
                            end,
                            &whole_range_index_aliases,
                        )
                    })
                    .collect();
                if direct_builtin_call_map && (slots == "c(1L)" || slots == "c(1)") {
                    match (callee, args.as_slice()) {
                        ("pmax", [a, b]) | ("pmin", [a, b]) => {
                            out[idx] = format!("{lhs} <- {callee}({a}, {b})");
                        }
                        ("abs", [a]) | ("sqrt", [a]) | ("log", [a]) => {
                            out[idx] = format!("{lhs} <- {callee}({a})");
                        }
                        _ => {}
                    }
                }
                if out[idx].trim() == trimmed {
                    let joined = parts
                        .iter()
                        .take(3)
                        .cloned()
                        .chain(args)
                        .collect::<Vec<_>>()
                        .join(", ");
                    out[idx] = format!("{lhs} <- rr_call_map_whole_auto({dest}, {joined})");
                }
            } else {
                whole_range_index_aliases.remove(lhs);
                continue;
            }
        }

        let current_trimmed = out[idx].trim().to_string();
        if let Some(current_caps) = assign_re().and_then(|re| re.captures(&current_trimmed)) {
            let current_lhs = current_caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let current_rhs = current_caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if current_lhs.starts_with('.')
                && (current_rhs.contains(':') || current_rhs.starts_with("rr_index_vec_floor("))
            {
                whole_range_index_aliases.insert(current_lhs.to_string(), current_rhs.to_string());
            } else {
                whole_range_index_aliases.remove(current_lhs);
            }
        }
    }
    out
}

pub(crate) fn collapse_contextual_full_range_gather_replays(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| {
        line.contains("rr_gather(")
            && line.contains("rr_index1_read_vec")
            && line.contains("rr_assign_slice(")
    }) {
        return lines;
    }
    let mut out = lines;
    let Some(re) = compile_regex(format!(
        r"^(?P<indent>\s*)(?P<lhs>{id}) <- rr_assign_slice\((?P<dest>{id}),\s*(?P<start>[^,]+),\s*(?P<end>[^,]+),\s*rr_gather\((?P<base>{id}),\s*rr_index_vec_floor\(rr_index1_read_vec(?:_floor)?\((?P<inner_base>{id}),\s*rr_index_vec_floor\((?P<inner_start>[^:]+):(?P<inner_end>[^\)]+)\)\)\)\)\)$",
        id = IDENT_PATTERN
    )) else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = re.captures(&trimmed) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
        let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
        let end = caps.name("end").map(|m| m.as_str()).unwrap_or("").trim();
        let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
        let inner_base = caps
            .name("inner_base")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let inner_start = caps
            .name("inner_start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let inner_end = caps
            .name("inner_end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if lhs != dest
            || base != inner_base
            || compact_expr(start) != compact_expr(inner_start)
            || compact_expr(end) != compact_expr(inner_end)
            || !start_expr_is_one_in_context(&out, idx, start)
            || !end_expr_can_cover_full_range(&out, idx, end)
        {
            continue;
        }
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        out[idx] = format!("{indent}{lhs} <- rr_gather({base}, rr_index_vec_floor({base}))");
    }
    out
}
