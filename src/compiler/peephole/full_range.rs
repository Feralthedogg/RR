use super::*;

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct SecondaryFullRangePlan {
    pub(super) needs_inline_slice_ops: bool,
    pub(super) needs_contextual_gather_replays: bool,
}

impl SecondaryFullRangePlan {
    pub(super) fn needs_any(self) -> bool {
        self.needs_inline_slice_ops || self.needs_contextual_gather_replays
    }
}

pub(super) fn analyze_secondary_full_range_plan(lines: &[String]) -> SecondaryFullRangePlan {
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

pub(super) fn run_secondary_full_range_bundle(
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
                            .chain(args.into_iter())
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

fn rewrite_loop_index_reads_to_whole_expr(expr: &str, idx_var: &str) -> String {
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

pub(super) fn parse_break_guard(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("if (!(")
        .and_then(|s| s.strip_suffix(")) break"))?;
    let (lhs, rhs) = inner.split_once("<=")?;
    Some((lhs.trim().to_string(), rhs.trim().to_string()))
}

pub(super) fn has_one_based_full_range_index_alias_read_candidates(lines: &[String]) -> bool {
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

fn parse_indexed_store_assign(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let (lhs, rhs) = trimmed.split_once("<-")?;
    let lhs = lhs.trim();
    let rhs = rhs.trim().to_string();
    let (base, idx) = lhs.split_once('[')?;
    let idx = idx.trim_end_matches(']').trim();
    Some((base.trim().to_string(), idx.to_string(), rhs))
}

pub(super) fn rewrite_full_range_conditional_scalar_loops(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].trim();
        let Some(init_caps) = assign_re().and_then(|re| re.captures(line)) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        let idx_var = init_caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let init_rhs = init_caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if !idx_var.starts_with("i_") || !literal_one_re().is_some_and(|re| re.is_match(init_rhs)) {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }

        let Some(repeat_line) = lines.get(i + 1).map(|s| s.trim()) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        if repeat_line != "repeat {" {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }
        let mut cursor = i + 2;
        while matches!(lines.get(cursor).map(|s| s.trim()), Some(line) if line.is_empty() || line.starts_with("rr_mark("))
        {
            cursor += 1;
        }
        let Some((guard_idx, end_expr)) = lines.get(cursor).and_then(|s| parse_break_guard(s))
        else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        if guard_idx != idx_var {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }

        cursor += 1;
        while matches!(lines.get(cursor).map(|s| s.trim()), Some(line) if line.is_empty() || line.starts_with("rr_mark("))
        {
            cursor += 1;
        }
        let Some(if_line) = lines.get(cursor).map(|s| s.trim()) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        let Some(cond_inner) = if_line
            .strip_prefix("if ((")
            .and_then(|s| s.strip_suffix(")) {"))
            .or_else(|| {
                if_line
                    .strip_prefix("if (")
                    .and_then(|s| s.strip_suffix(") {"))
            })
        else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        let mut then_cursor = cursor + 1;
        while matches!(lines.get(then_cursor).map(|s| s.trim()), Some(line) if line.is_empty() || line.starts_with("rr_mark("))
        {
            then_cursor += 1;
        }
        let then_line = lines.get(then_cursor).map(|s| s.trim()).unwrap_or("");
        let else_header = lines.get(then_cursor + 1).map(|s| s.trim()).unwrap_or("");
        let mut else_cursor = then_cursor + 2;
        while matches!(lines.get(else_cursor).map(|s| s.trim()), Some(line) if line.is_empty() || line.starts_with("rr_mark("))
        {
            else_cursor += 1;
        }
        let else_line = lines.get(else_cursor).map(|s| s.trim()).unwrap_or("");
        let close_line = lines.get(else_cursor + 1).map(|s| s.trim()).unwrap_or("");
        let incr_line = lines.get(else_cursor + 2).map(|s| s.trim()).unwrap_or("");
        let next_line = lines.get(else_cursor + 3).map(|s| s.trim()).unwrap_or("");
        let end_repeat = lines.get(else_cursor + 4).map(|s| s.trim()).unwrap_or("");
        if else_header != "} else {"
            || close_line != "}"
            || next_line != "next"
            || end_repeat != "}"
        {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }
        let Some((dest_base, then_idx, then_rhs)) = parse_indexed_store_assign(then_line) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        let Some((else_base, else_idx, else_rhs)) = parse_indexed_store_assign(else_line) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        if dest_base != else_base || then_idx != idx_var || else_idx != idx_var {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }
        let expected_incr = format!("{idx_var} <- ({idx_var} + 1L)");
        if incr_line != expected_incr {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }

        let cond_whole = rewrite_loop_index_reads_to_whole_expr(cond_inner.trim(), &idx_var);
        let then_whole = rewrite_loop_index_reads_to_whole_expr(&then_rhs, &idx_var);
        let else_whole = rewrite_loop_index_reads_to_whole_expr(&else_rhs, &idx_var);
        let indent = &lines[i][..lines[i].len() - lines[i].trim_start().len()];
        out.push(format!(
            "{indent}{dest_base} <- rr_assign_slice({dest_base}, 1L, {end_expr}, rr_ifelse_strict(({cond_whole}), {then_whole}, {else_whole}))"
        ));
        i = else_cursor + 5;
    }
    out
}

fn rewrite_inline_full_range_reads(expr: &str, start: &str, end: &str) -> String {
    let mut out = expr.to_string();
    let start_pat = regex::escape(start.trim());
    let end_pat = regex::escape(end.trim());
    let direct_pat = format!(
        r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*:\s*{}\)",
        IDENT_PATTERN, start_pat, end_pat
    );
    if let Some(re) = compile_regex(direct_pat) {
        out = re
            .replace_all(&out, |caps: &Captures<'_>| {
                caps.name("base")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .to_string();
    }
    let floor_pat = format!(
        r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*rr_index_vec_floor\(\s*{}\s*:\s*{}\s*\)\)",
        IDENT_PATTERN, start_pat, end_pat
    );
    if let Some(re) = compile_regex(floor_pat) {
        out = re
            .replace_all(&out, |caps: &Captures<'_>| {
                caps.name("base")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .to_string();
    }
    out
}

pub(super) fn compact_expr(expr: &str) -> String {
    expr.chars().filter(|c| !c.is_whitespace()).collect()
}

fn expr_is_full_range_index_alias(expr: &str, start: &str, end: &str) -> bool {
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

fn expr_is_one_based_full_range_alias(expr: &str) -> bool {
    let expr = compact_expr(expr);
    let starts = ["1L", "1", "1.0", "1.0L"];
    starts.iter().any(|start_expr| {
        expr.starts_with(&format!("{}:", start_expr))
            || expr.starts_with(&format!("rr_index_vec_floor({}:", start_expr))
    })
}

fn rewrite_inline_full_range_reads_with_aliases(
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
        let pattern = format!(
            r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*\)",
            IDENT_PATTERN,
            regex::escape(alias),
        );
        let Some(re) = compile_regex(pattern) else {
            continue;
        };
        out = re
            .replace_all(&out, |caps: &Captures<'_>| {
                caps.name("base")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .to_string();
    }
    out
}

fn start_expr_is_one_in_context(lines: &[String], idx: usize, start: &str) -> bool {
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

fn end_expr_is_singleton(end: &str) -> bool {
    literal_one_re().is_some_and(|re| re.is_match(end.trim()))
}

pub(super) fn strip_redundant_outer_parens(expr: &str) -> &str {
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

fn expr_has_top_level_arith(expr: &str) -> bool {
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

fn end_expr_can_cover_full_range(lines: &[String], idx: usize, end: &str) -> bool {
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

fn has_inline_full_range_slice_op_candidates(lines: &[String]) -> bool {
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

pub(super) fn rewrite_inline_full_range_slice_ops(
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
                        .chain(args.into_iter())
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

pub(super) fn collapse_contextual_full_range_gather_replays(lines: Vec<String>) -> Vec<String> {
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

pub(super) fn rewrite_one_based_full_range_index_alias_reads(lines: Vec<String>) -> Vec<String> {
    if !has_one_based_full_range_index_alias_read_candidates(&lines) {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut whole_range_index_aliases: FxHashMap<String, String> = FxHashMap::default();
    let read_vec_re = compile_regex(format!(
        r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>{}|rr_index_vec_floor\([^\)]*\)|[^,\)]*:[^\)]*)\)",
        IDENT_PATTERN, IDENT_PATTERN
    ));
    for line in lines {
        let trimmed = line.trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            if is_control_flow_boundary(&trimmed) || line.contains("<- function") {
                whole_range_index_aliases.clear();
            }
            out.push(line);
            continue;
        };
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let mut rewritten_rhs = rhs.to_string();
        if rewritten_rhs.contains("rr_index1_read_vec")
            && let Some(re) = read_vec_re.as_ref()
        {
            rewritten_rhs = re
                .replace_all(&rewritten_rhs, |caps: &Captures<'_>| {
                    let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                    if expr_is_one_based_full_range_alias(idx_expr) {
                        caps.name("base")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                    }
                })
                .to_string();
        }
        for (alias, alias_rhs) in &whole_range_index_aliases {
            if !expr_is_one_based_full_range_alias(alias_rhs) {
                continue;
            }
            let pattern = format!(
                r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*\)",
                IDENT_PATTERN,
                regex::escape(alias),
            );
            let Some(re) = compile_regex(pattern) else {
                continue;
            };
            rewritten_rhs = re
                .replace_all(&rewritten_rhs, |caps: &Captures<'_>| {
                    caps.name("base")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .to_string()
                })
                .to_string();
        }
        if rewritten_rhs != rhs {
            rewritten_rhs = rewritten_rhs.replace("rr_ifelse_strict(", "ifelse(");
        }
        if lhs.starts_with('.') && expr_is_one_based_full_range_alias(&rewritten_rhs) {
            whole_range_index_aliases.insert(lhs.to_string(), rewritten_rhs.clone());
        } else {
            whole_range_index_aliases.remove(lhs);
        }
        out.push(format!("{indent}{lhs} <- {rewritten_rhs}"));
    }
    out
}
