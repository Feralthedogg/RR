use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

fn has_repeat_loop_candidates(lines: &[String]) -> bool {
    lines.iter().any(|line| line.trim() == "repeat {")
}

fn has_repeat_guard_break_candidates(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        let trimmed = line.trim();
        (trimmed.starts_with("if (!(") || trimmed.starts_with("if !("))
            && trimmed.ends_with("break")
    })
}

fn has_next_candidates(lines: &[String]) -> bool {
    lines.iter().any(|line| line.trim() == "next")
}

fn has_for_seq_len_candidates(lines: &[String]) -> bool {
    lines
        .iter()
        .any(|line| line.trim_start().starts_with("for (") && line.contains("seq_len("))
}

fn has_match_phi_candidates(lines: &[String]) -> bool {
    lines.iter().any(|line| line.contains(".phi_"))
        && lines.iter().any(|line| line.trim() == "} else {")
}

fn parse_constant_repeat_guard(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("if (!(")
        .or_else(|| trimmed.strip_prefix("if !("))?
        .strip_suffix(")) break")
        .or_else(|| {
            trimmed
                .strip_prefix("if (!(")
                .or_else(|| trimmed.strip_prefix("if !("))
                .and_then(|s| s.strip_suffix(") break"))
        })?
        .trim();
    for op in ["<=", "<"] {
        let needle = format!(" {op} ");
        let Some((lhs, rhs)) = inner.split_once(&needle) else {
            continue;
        };
        let lhs = lhs.trim().trim_matches(|ch| ch == '(' || ch == ')');
        let rhs = rhs.trim();
        if lhs.is_empty() || rhs.is_empty() {
            continue;
        }
        let numeric = lhs.trim_end_matches('L').trim_end_matches('l');
        if numeric.parse::<f64>().ok().is_some() {
            return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
        }
    }
    None
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RepeatBlockKind {
    Loop,
    Other,
}

fn count_repeat_loops(stack: &[RepeatBlockKind]) -> usize {
    stack
        .iter()
        .filter(|kind| matches!(kind, RepeatBlockKind::Loop))
        .count()
}

fn leading_close_count(line: &str) -> usize {
    line.chars().take_while(|ch| *ch == '}').count()
}

#[derive(Debug, Clone)]
struct RepeatLoopFacts {
    repeat_idx: usize,
    loop_end: usize,
    body_start: usize,
    significant_lines: Vec<usize>,
    next_lines: Vec<usize>,
    block_depth_before: Vec<usize>,
    loop_depth_before: Vec<usize>,
    assigned_counts: FxHashMap<String, usize>,
    first_ref_idx: FxHashMap<String, usize>,
    last_ref_idx: FxHashMap<String, usize>,
}

impl RepeatLoopFacts {
    fn rel(&self, line_idx: usize) -> usize {
        line_idx - self.body_start
    }

    fn mentions_ident(&self, ident: &str) -> bool {
        self.first_ref_idx.contains_key(ident)
    }

    fn assigned_count(&self, ident: &str) -> usize {
        self.assigned_counts.get(ident).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Default)]
pub(in super::super) struct RepeatLoopAnalysisCache {
    signature: Option<u64>,
    loops: Vec<RepeatLoopFacts>,
}

fn lines_signature(lines: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for line in lines {
        line.hash(&mut hasher);
    }
    hasher.finish()
}

fn collect_repeat_loop_facts(lines: &[String]) -> Vec<RepeatLoopFacts> {
    let mut out = Vec::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let Some(next_repeat) =
            (idx..lines.len()).find(|line_idx| lines[*line_idx].trim() == "repeat {")
        else {
            break;
        };
        if let Some(facts) = analyze_repeat_loop(lines, next_repeat) {
            idx = facts.loop_end + 1;
            out.push(facts);
        } else {
            idx = next_repeat + 1;
        }
    }
    out
}

fn cached_repeat_loop_facts<'a>(
    cache: &'a mut RepeatLoopAnalysisCache,
    lines: &[String],
) -> &'a [RepeatLoopFacts] {
    let signature = lines_signature(lines);
    if cache.signature != Some(signature) {
        cache.loops = collect_repeat_loop_facts(lines);
        cache.signature = Some(signature);
    }
    &cache.loops
}

fn clear_repeat_loop_facts(cache: &mut RepeatLoopAnalysisCache) {
    cache.signature = None;
    cache.loops.clear();
}

fn next_repeat_loop_fact_at_or_after(
    cache: &mut RepeatLoopAnalysisCache,
    lines: &[String],
    idx: usize,
) -> Option<RepeatLoopFacts> {
    cached_repeat_loop_facts(cache, lines)
        .iter()
        .find(|facts| facts.repeat_idx >= idx)
        .cloned()
}

fn analyze_repeat_loop(lines: &[String], repeat_idx: usize) -> Option<RepeatLoopFacts> {
    let loop_end = find_matching_block_end(lines, repeat_idx)?;
    let body_start = repeat_idx + 1;
    let mut significant_lines = Vec::new();
    let mut next_lines = Vec::new();
    let mut block_stack = vec![RepeatBlockKind::Loop];
    let mut block_depth_before = vec![0usize; loop_end.saturating_sub(body_start)];
    let mut loop_depth_before = vec![0usize; loop_end.saturating_sub(body_start)];
    let mut assigned_counts = FxHashMap::<String, usize>::default();
    let mut first_ref_idx = FxHashMap::<String, usize>::default();
    let mut last_ref_idx = FxHashMap::<String, usize>::default();

    for line_idx in body_start..loop_end {
        let trimmed_start = lines[line_idx].trim_start();
        let close_count = leading_close_count(trimmed_start);
        for _ in 0..close_count {
            let _ = block_stack.pop();
        }
        let rel = line_idx - body_start;
        block_depth_before[rel] = block_stack.len();
        loop_depth_before[rel] = count_repeat_loops(&block_stack);

        let remainder = trimmed_start[close_count..].trim_start();
        if remainder.ends_with('{') {
            let kind = if remainder == "repeat {"
                || remainder.starts_with("for ")
                || remainder.starts_with("while ")
            {
                RepeatBlockKind::Loop
            } else {
                RepeatBlockKind::Other
            };
            block_stack.push(kind);
        }

        let trimmed = lines[line_idx].trim();
        if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
            continue;
        }
        significant_lines.push(line_idx);
        if trimmed == "next" {
            next_lines.push(line_idx);
        }
        if let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            *assigned_counts.entry(lhs.to_string()).or_insert(0) += 1;
        } else if let Some(caps) = indexed_store_base_re().and_then(|re| re.captures(trimmed)) {
            let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
            *assigned_counts.entry(base.to_string()).or_insert(0) += 1;
        }
        for ident in expr_idents(trimmed) {
            first_ref_idx.entry(ident.clone()).or_insert(line_idx);
            last_ref_idx.insert(ident, line_idx);
        }
    }

    Some(RepeatLoopFacts {
        repeat_idx,
        loop_end,
        body_start,
        significant_lines,
        next_lines,
        block_depth_before,
        loop_depth_before,
        assigned_counts,
        first_ref_idx,
        last_ref_idx,
    })
}

pub(super) fn normalize_repeat_loop_counters(lines: Vec<String>) -> Vec<String> {
    let mut cache = RepeatLoopAnalysisCache::default();
    normalize_repeat_loop_counters_with_cache(lines, &mut cache)
}

pub(in super::super) fn normalize_repeat_loop_counters_with_cache(
    lines: Vec<String>,
    cache: &mut RepeatLoopAnalysisCache,
) -> Vec<String> {
    if !has_repeat_loop_candidates(&lines) || !has_repeat_guard_break_candidates(&lines) {
        return lines;
    }

    let mut out = lines;
    let mut idx = 0usize;
    while let Some(loop_facts) = next_repeat_loop_fact_at_or_after(cache, &out, idx) {
        let repeat_idx = loop_facts.repeat_idx;
        let mut loop_end = loop_facts.loop_end;
        let mut changed = false;
        let Some(mut guard_idx) = loop_facts
            .significant_lines
            .iter()
            .copied()
            .find(|line_idx| {
                let trimmed = out[*line_idx].trim();
                (trimmed.starts_with("if (!(") || trimmed.starts_with("if !("))
                    && trimmed.ends_with("break")
            })
        else {
            idx = loop_end + 1;
            continue;
        };

        if let Some((start_lit, cmp, bound)) = parse_constant_repeat_guard(&out[guard_idx]) {
            let idx_var = ".__rr_i";
            if !loop_facts.mentions_ident(idx_var) {
                let indent = out[guard_idx]
                    .chars()
                    .take_while(|ch| ch.is_ascii_whitespace())
                    .collect::<String>();
                let repeat_indent = out[repeat_idx]
                    .chars()
                    .take_while(|ch| ch.is_ascii_whitespace())
                    .collect::<String>();

                out.insert(
                    repeat_idx,
                    format!("{repeat_indent}{idx_var} <- {start_lit}"),
                );
                guard_idx += 1;
                loop_end += 1;
                out[guard_idx] = if cmp == "<=" {
                    format!("{indent}if (!({idx_var} <= {bound})) break")
                } else {
                    format!("{indent}if (!({idx_var} < {bound})) break")
                };

                let one = if start_lit.contains('.') {
                    "1.0"
                } else if start_lit.ends_with('L') || start_lit.ends_with('l') {
                    "1L"
                } else {
                    "1"
                };
                out.insert(
                    loop_end,
                    format!("{indent}{idx_var} <- ({idx_var} + {one})"),
                );
                loop_end += 1;
                changed = true;
            }
        }

        let Some((idx_var, op, _bound)) = parse_repeat_guard_cmp_line(out[guard_idx].trim()) else {
            idx = loop_end + 1;
            continue;
        };
        if op != "<=" || !plain_ident_re().is_some_and(|re| re.is_match(&idx_var)) {
            idx = loop_end + 1;
            continue;
        }

        if loop_facts.assigned_count(&idx_var) == 0 && loop_facts.mentions_ident(&idx_var) {
            let inner_indent = out[guard_idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            out.insert(
                loop_end,
                format!("{inner_indent}{idx_var} <- ({idx_var} + 1L)"),
            );
            loop_end += 1;
            changed = true;
        }

        if !loop_facts.next_lines.is_empty() {
            let mut insertions = Vec::new();
            for line_idx in loop_facts.next_lines.iter().copied() {
                if line_idx <= guard_idx || line_idx >= loop_end {
                    continue;
                }
                let rel = loop_facts.rel(line_idx);
                if loop_facts.loop_depth_before[rel] != 1 {
                    continue;
                }
                let branch_depth = loop_facts.block_depth_before[rel];
                let mut saw_idx_update = false;
                let mut scan = line_idx;
                while scan > guard_idx + 1 {
                    scan -= 1;
                    let trimmed = out[scan].trim();
                    if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
                        continue;
                    }
                    if loop_facts.block_depth_before[loop_facts.rel(scan)] < branch_depth {
                        break;
                    }
                    if trimmed.starts_with(&format!("{idx_var} <-")) {
                        saw_idx_update = true;
                        break;
                    }
                }
                if saw_idx_update {
                    continue;
                }

                let indent = out[line_idx]
                    .chars()
                    .take_while(|ch| ch.is_ascii_whitespace())
                    .collect::<String>();
                insertions.push((line_idx, format!("{indent}{idx_var} <- ({idx_var} + 1L)")));
            }

            if !insertions.is_empty() {
                for (line_idx, increment) in insertions.into_iter().rev() {
                    out.insert(line_idx, increment);
                    loop_end += 1;
                }
                changed = true;
            }
        }

        if changed {
            clear_repeat_loop_facts(cache);
        }
        idx = loop_end + 1;
    }

    out
}

pub(super) fn literal_one_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^(?:1L?|1(?:\.0+)?)$".to_string()))
        .as_ref()
}

pub(super) fn literal_positive_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^(?:[1-9]\d*)(?:\.0+)?L?$".to_string()))
        .as_ref()
}

pub(super) fn literal_integer_value(expr: &str) -> Option<i64> {
    let trimmed = expr.trim().trim_end_matches('L').trim_end_matches('l');
    if let Ok(value) = trimmed.parse::<i64>() {
        return Some(value);
    }
    let value = trimmed.parse::<f64>().ok()?;
    (value.fract() == 0.0).then_some(value as i64)
}

pub(super) fn latest_literal_assignment_before(
    lines: &[String],
    idx: usize,
    var: &str,
) -> Option<i64> {
    for i in (0..idx).rev() {
        let Some(caps) = assign_re().and_then(|re| re.captures(lines[i].trim())) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs != var {
            continue;
        }
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        return literal_integer_value(rhs);
    }
    None
}

pub(super) fn parse_repeat_guard_cmp_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("if !(")
        .or_else(|| trimmed.strip_prefix("if (!("))?;
    let inner = inner
        .strip_suffix(")) break")
        .or_else(|| inner.strip_suffix(") break"))?;
    if let Some((lhs, rhs)) = inner.split_once("<=") {
        return Some((
            lhs.trim().to_string(),
            "<=".to_string(),
            rhs.trim().to_string(),
        ));
    }
    if let Some((lhs, rhs)) = inner.split_once('<') {
        return Some((
            lhs.trim().to_string(),
            "<".to_string(),
            rhs.trim().to_string(),
        ));
    }
    None
}

pub(super) fn var_has_known_positive_progression_before(
    lines: &[String],
    idx: usize,
    var: &str,
) -> bool {
    let mut seen_assign = false;
    for line in lines.iter().take(idx) {
        let Some(caps) = assign_re().and_then(|re| re.captures(line.trim())) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs != var {
            continue;
        }
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let is_positive_reset = literal_integer_value(rhs).is_some_and(|value| value >= 1);
        let is_canonical_incr = rhs == format!("({var} + 1)")
            || rhs == format!("({var} + 1L)")
            || rhs == format!("({var} + 1.0)");
        if !is_positive_reset && !is_canonical_incr {
            return false;
        }
        seen_assign = true;
    }
    seen_assign
}

pub(super) fn positive_guard_for_var_before(
    lines: &[String],
    idx: usize,
    iter_var: &str,
    bound_var: &str,
) -> bool {
    (0..idx).rev().any(|i| {
        parse_repeat_guard_cmp_line(lines[i].trim()).is_some_and(|(iter, _op, bound)| {
            iter == iter_var
                && bound == bound_var
                && var_has_known_positive_progression_before(lines, i, iter_var)
        })
    })
}

pub(super) fn count_unquoted_braces(line: &str) -> (usize, usize) {
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && (in_single || in_double) {
            escaped = true;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }
        if in_single || in_double {
            continue;
        }
        match ch {
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
    }
    (opens, closes)
}

pub(super) fn restore_missing_scalar_loop_increments(lines: Vec<String>) -> Vec<String> {
    if !has_repeat_loop_candidates(&lines) || !has_repeat_guard_break_candidates(&lines) {
        return lines;
    }
    let mut out = lines;
    let mut idx = 0usize;
    while idx < out.len() {
        if out[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }

        let Some(loop_end) = find_matching_block_end(&out, idx) else {
            idx += 1;
            continue;
        };

        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = out[*line_idx].trim();
            trimmed.starts_with("if (!(") && trimmed.ends_with(")) break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let guard = out[guard_idx].trim();
        let Some(inner) = guard
            .strip_prefix("if (!(")
            .and_then(|s| s.strip_suffix(")) break"))
        else {
            idx = loop_end + 1;
            continue;
        };
        let Some((lhs, _rhs)) = inner.split_once("<=") else {
            idx = loop_end + 1;
            continue;
        };
        let idx_var = lhs.trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(idx_var)) {
            idx = loop_end + 1;
            continue;
        }

        let mut already_updates_idx = false;
        let mut has_body_use = false;
        for line in out.iter().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with(&format!("{idx_var} <-")) {
                already_updates_idx = true;
                break;
            }
            if trimmed.contains(&format!("[{idx_var}]"))
                || expr_idents(trimmed).iter().any(|ident| ident == idx_var)
            {
                has_body_use = true;
            }
        }
        if already_updates_idx || !has_body_use {
            idx = loop_end + 1;
            continue;
        }

        let inner_indent = out[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        out.insert(
            loop_end,
            format!("{inner_indent}{idx_var} <- ({idx_var} + 1L)"),
        );
        idx = loop_end + 2;
    }
    out
}

pub(super) fn restore_constant_one_guard_repeat_loop_counters(lines: Vec<String>) -> Vec<String> {
    fn parse_constant_guard(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim();
        let inner = trimmed
            .strip_prefix("if (!(")
            .or_else(|| trimmed.strip_prefix("if !("))?
            .strip_suffix(")) break")
            .or_else(|| {
                trimmed
                    .strip_prefix("if (!(")
                    .or_else(|| trimmed.strip_prefix("if !("))
                    .and_then(|s| s.strip_suffix(") break"))
            })?
            .trim();
        for op in ["<=", "<"] {
            let needle = format!(" {op} ");
            let Some((lhs, rhs)) = inner.split_once(&needle) else {
                continue;
            };
            let lhs = lhs.trim().trim_matches(|ch| ch == '(' || ch == ')');
            let rhs = rhs.trim();
            if lhs.is_empty() || rhs.is_empty() {
                continue;
            }
            let numeric = lhs.trim_end_matches('L').trim_end_matches('l');
            if numeric.parse::<f64>().ok().is_some() {
                return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
            }
        }
        None
    }

    if !has_repeat_loop_candidates(&lines) || !has_repeat_guard_break_candidates(&lines) {
        return lines;
    }
    let mut out = lines;
    let mut idx = 0usize;
    while idx < out.len() {
        if out[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }

        let Some(loop_end) = find_matching_block_end(&out, idx) else {
            idx += 1;
            continue;
        };

        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = out[*line_idx].trim();
            (trimmed.starts_with("if !(") || trimmed.starts_with("if (!("))
                && trimmed.ends_with("break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let Some((start_lit, cmp, bound)) = parse_constant_guard(&out[guard_idx]) else {
            idx = loop_end + 1;
            continue;
        };

        let idx_var = ".__rr_i";
        let body_mentions_idx = out.iter().take(loop_end).skip(guard_idx + 1).any(|line| {
            expr_idents(line.trim())
                .iter()
                .any(|ident| ident == idx_var)
        });
        if body_mentions_idx {
            idx = loop_end + 1;
            continue;
        }

        let indent = out[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let repeat_indent = out[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();

        out.insert(idx, format!("{repeat_indent}{idx_var} <- {start_lit}"));
        let guard_line = if cmp == "<=" {
            format!("{indent}if (!({idx_var} <= {bound})) break")
        } else {
            format!("{indent}if (!({idx_var} < {bound})) break")
        };
        out[guard_idx + 1] = guard_line;
        let one = if start_lit.contains('.') {
            "1.0"
        } else if start_lit.ends_with('L') || start_lit.ends_with('l') {
            "1L"
        } else {
            "1"
        };
        out.insert(
            loop_end + 1,
            format!("{indent}{idx_var} <- ({idx_var} + {one})"),
        );
        idx = loop_end + 3;
    }

    out
}

pub(super) fn restore_missing_scalar_loop_next_increments(lines: Vec<String>) -> Vec<String> {
    let mut cache = RepeatLoopAnalysisCache::default();
    restore_missing_scalar_loop_next_increments_with_cache(lines, &mut cache)
}

pub(in super::super) fn restore_missing_scalar_loop_next_increments_with_cache(
    lines: Vec<String>,
    cache: &mut RepeatLoopAnalysisCache,
) -> Vec<String> {
    if !has_repeat_loop_candidates(&lines)
        || !has_repeat_guard_break_candidates(&lines)
        || !has_next_candidates(&lines)
    {
        return lines;
    }
    let mut out = lines;
    let mut idx = 0usize;
    while let Some(loop_facts) = next_repeat_loop_fact_at_or_after(cache, &out, idx) {
        let loop_end = loop_facts.loop_end;

        let Some(guard_idx) = loop_facts
            .significant_lines
            .iter()
            .copied()
            .find(|line_idx| {
                let trimmed = out[*line_idx].trim();
                trimmed.starts_with("if (!(") && trimmed.ends_with(")) break")
            })
        else {
            idx = loop_end + 1;
            continue;
        };
        if loop_facts.next_lines.is_empty() {
            idx = loop_end + 1;
            continue;
        }
        let guard = out[guard_idx].trim();
        let Some(inner) = guard
            .strip_prefix("if (!(")
            .and_then(|s| s.strip_suffix(")) break"))
        else {
            idx = loop_end + 1;
            continue;
        };
        let Some((lhs, _rhs)) = inner.split_once("<=") else {
            idx = loop_end + 1;
            continue;
        };
        let idx_var = lhs.trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(idx_var)) {
            idx = loop_end + 1;
            continue;
        }

        let mut insertions = Vec::new();
        for line_idx in loop_facts.next_lines.iter().copied() {
            if line_idx <= guard_idx || line_idx >= loop_end {
                continue;
            }
            let rel = loop_facts.rel(line_idx);
            if loop_facts.loop_depth_before[rel] != 1 {
                continue;
            }

            let branch_depth = loop_facts.block_depth_before[rel];
            let mut saw_idx_update = false;
            let mut scan = line_idx;
            while scan > guard_idx + 1 {
                scan -= 1;
                let trimmed = out[scan].trim();
                if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
                    continue;
                }
                if loop_facts.block_depth_before[loop_facts.rel(scan)] < branch_depth {
                    break;
                }
                if trimmed.starts_with(&format!("{idx_var} <-")) {
                    saw_idx_update = true;
                    break;
                }
            }
            if saw_idx_update {
                continue;
            }

            let indent = out[line_idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            insertions.push((line_idx, format!("{indent}{idx_var} <- ({idx_var} + 1L)")));
        }

        if insertions.is_empty() {
            idx = loop_end + 1;
            continue;
        }

        for (line_idx, increment) in insertions.into_iter().rev() {
            out.insert(line_idx, increment);
        }
        clear_repeat_loop_facts(cache);
        idx = loop_end + 1;
    }
    out
}

pub(super) fn rewrite_canonical_counted_repeat_loops_to_for(lines: Vec<String>) -> Vec<String> {
    let mut cache = RepeatLoopAnalysisCache::default();
    rewrite_canonical_counted_repeat_loops_to_for_with_cache(lines, &mut cache)
}

pub(in super::super) fn rewrite_canonical_counted_repeat_loops_to_for_with_cache(
    lines: Vec<String>,
    cache: &mut RepeatLoopAnalysisCache,
) -> Vec<String> {
    if !has_repeat_loop_candidates(&lines) {
        return lines;
    }
    fn block_end(lines: &[String], start: usize) -> Option<usize> {
        let mut depth = 0usize;
        for (idx, line) in lines.iter().enumerate().skip(start) {
            for ch in line.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' if depth > 0 => depth -= 1,
                    _ => {}
                }
            }
            if depth == 0 {
                return Some(idx);
            }
        }
        None
    }

    fn next_significant_line(lines: &[String], start: usize, end: usize) -> Option<usize> {
        (start..end).find(|idx| {
            let trimmed = lines[*idx].trim();
            !trimmed.is_empty() && !trimmed.starts_with("rr_mark(")
        })
    }

    fn prev_significant_line(lines: &[String], start: usize, end: usize) -> Option<usize> {
        (start..end).rev().find(|idx| {
            let trimmed = lines[*idx].trim();
            !trimmed.is_empty() && !trimmed.starts_with("rr_mark(")
        })
    }

    fn is_canonical_increment(line: &str, idx_var: &str) -> bool {
        let trimmed = line.trim();
        trimmed == format!("{idx_var} <- ({idx_var} + 1)")
            || trimmed == format!("{idx_var} <- ({idx_var} + 1L)")
    }

    fn assigns_var(line: &str, var: &str) -> bool {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == var)
            || indexed_store_base_re()
                .and_then(|re| re.captures(line.trim()))
                .is_some_and(|caps| {
                    caps.name("base").map(|m| m.as_str()).unwrap_or("").trim() == var
                })
    }

    fn references_var(line: &str, var: &str) -> bool {
        line.contains(var) && expr_idents(line).iter().any(|ident| ident == var)
    }

    let mut out = lines;
    let mut idx = 0usize;
    while let Some(loop_facts) = next_repeat_loop_fact_at_or_after(cache, &out, idx) {
        let repeat_idx = loop_facts.repeat_idx;
        let loop_end = loop_facts.loop_end;

        let Some(guard_idx) = next_significant_line(&out, repeat_idx + 1, loop_end) else {
            idx = loop_end + 1;
            continue;
        };
        let Some((idx_var, end_expr)) = parse_break_guard(&out[guard_idx]) else {
            idx = loop_end + 1;
            continue;
        };
        if !plain_ident_re().is_some_and(|re| re.is_match(&idx_var)) {
            idx = loop_end + 1;
            continue;
        }

        let Some(mut last_sig_idx) = prev_significant_line(&out, repeat_idx + 1, loop_end) else {
            idx = loop_end + 1;
            continue;
        };
        let had_trailing_next = out[last_sig_idx].trim() == "next";
        if had_trailing_next {
            let Some(prev_idx) = prev_significant_line(&out, repeat_idx + 1, last_sig_idx) else {
                idx = loop_end + 1;
                continue;
            };
            last_sig_idx = prev_idx;
        }
        let incr_idx = last_sig_idx;
        if !is_canonical_increment(&out[incr_idx], &idx_var) {
            idx = loop_end + 1;
            continue;
        }

        let mut init_idx = None;
        let mut prefix_references_idx = false;
        let mut scan = repeat_idx;
        while scan > 0 {
            let prev_idx = scan - 1;
            let trimmed = out[prev_idx].trim();
            if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
                scan -= 1;
                continue;
            }
            if out[prev_idx].contains("<- function") || is_control_flow_boundary(trimmed) {
                break;
            }
            if assign_re()
                .and_then(|re| re.captures(trimmed))
                .is_some_and(|caps| {
                    caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == idx_var
                        && literal_one_re().is_some_and(|re| {
                            re.is_match(caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim())
                        })
                })
            {
                init_idx = Some(prev_idx);
                break;
            }
            if references_var(&out[prev_idx], &idx_var) {
                prefix_references_idx = true;
            }
            scan -= 1;
        }
        let Some(init_idx) = init_idx else {
            idx = loop_end + 1;
            continue;
        };
        if prefix_references_idx {
            idx = loop_end + 1;
            continue;
        }

        let end_expr_idents: FxHashSet<String> = expr_idents(&end_expr).into_iter().collect();
        let mut invalid = loop_facts
            .next_lines
            .iter()
            .any(|line_idx| *line_idx > guard_idx && *line_idx < incr_idx);
        if !invalid {
            for line_idx in loop_facts
                .significant_lines
                .iter()
                .copied()
                .filter(|line_idx| *line_idx > guard_idx && *line_idx < incr_idx)
            {
                let trimmed = out[line_idx].trim();
                let assigned = assign_re()
                    .and_then(|re| re.captures(trimmed))
                    .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()))
                    .or_else(|| {
                        indexed_store_base_re()
                            .and_then(|re| re.captures(trimmed))
                            .and_then(|caps| {
                                caps.name("base").map(|m| m.as_str().trim().to_string())
                            })
                    });
                let Some(assigned) = assigned else {
                    continue;
                };
                if assigned == idx_var || end_expr_idents.contains(&assigned) {
                    invalid = true;
                    break;
                }
            }
        }
        if invalid {
            idx = loop_end + 1;
            continue;
        }

        let mut invalid_after_loop = false;
        for line in out.iter().skip(loop_end + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
                continue;
            }
            if assigns_var(line, &idx_var) {
                break;
            }
            if references_var(line, &idx_var) {
                invalid_after_loop = true;
            }
            break;
        }
        if invalid_after_loop {
            idx = loop_end + 1;
            continue;
        }

        let indent = out[repeat_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let mut replacement = Vec::new();
        replacement.extend(out[(init_idx + 1)..repeat_idx].iter().cloned());
        replacement.push(format!("{indent}for ({idx_var} in seq_len({end_expr})) {{"));
        replacement.extend(out[(guard_idx + 1)..incr_idx].iter().cloned());
        replacement.push(format!("{indent}}}"));

        out.splice(init_idx..=loop_end, replacement);
        clear_repeat_loop_facts(cache);
        idx = init_idx + 1;
    }

    out
}

pub(super) fn hoist_loop_invariant_pure_assignments_from_counted_repeat_loops(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut cache = RepeatLoopAnalysisCache::default();
    hoist_loop_invariant_pure_assignments_from_counted_repeat_loops_with_cache(
        lines,
        pure_user_calls,
        &mut cache,
    )
}

pub(in super::super) fn hoist_loop_invariant_pure_assignments_from_counted_repeat_loops_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut RepeatLoopAnalysisCache,
) -> Vec<String> {
    struct HoistCandidate {
        line_idx: usize,
        lhs: String,
        deps: FxHashSet<String>,
    }

    if !has_repeat_loop_candidates(&lines) || !has_repeat_guard_break_candidates(&lines) {
        return lines;
    }
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum BlockKind {
        Loop,
        Other,
    }

    fn count_loops(stack: &[BlockKind]) -> usize {
        stack
            .iter()
            .filter(|kind| matches!(kind, BlockKind::Loop))
            .count()
    }

    fn leading_close_count(line: &str) -> usize {
        line.chars().take_while(|ch| *ch == '}').count()
    }

    fn bound_literal_before(lines: &[String], idx: usize, bound: &str) -> Option<i64> {
        literal_integer_value(bound).or_else(|| {
            plain_ident_re()
                .is_some_and(|re| re.is_match(bound))
                .then(|| latest_literal_assignment_before(lines, idx, bound))
                .flatten()
        })
    }

    fn assigns_var(line: &str, var: &str) -> bool {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == var)
            || indexed_store_base_re()
                .and_then(|re| re.captures(line.trim()))
                .is_some_and(|caps| {
                    caps.name("base").map(|m| m.as_str()).unwrap_or("").trim() == var
                })
    }

    fn references_var(line: &str, var: &str) -> bool {
        expr_idents(line).iter().any(|ident| ident == var)
    }

    let mut out = lines;
    let mut repeat_idx = 0usize;
    while let Some(loop_facts) = next_repeat_loop_fact_at_or_after(cache, &out, repeat_idx) {
        let next_repeat = loop_facts.repeat_idx;
        let loop_end = loop_facts.loop_end;
        let Some(guard_idx) = (next_repeat + 1..loop_end).find(|idx| {
            let trimmed = out[*idx].trim();
            !trimmed.is_empty() && !trimmed.starts_with("rr_mark(")
        }) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some((iter_var, op, bound)) = parse_repeat_guard_cmp_line(out[guard_idx].trim()) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some(start_value) = latest_literal_assignment_before(&out, guard_idx, &iter_var) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some(bound_value) = bound_literal_before(&out, guard_idx, &bound) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let trip_count = if op == "<=" {
            bound_value - start_value + 1
        } else if op == "<" {
            bound_value - start_value
        } else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        if trip_count <= 0 || !plain_ident_re().is_some_and(|re| re.is_match(&iter_var)) {
            repeat_idx = next_repeat + 1;
            continue;
        }
        let mut candidates = Vec::<HoistCandidate>::new();
        for line_idx in loop_facts.body_start..loop_end {
            let trimmed = out[line_idx].trim();
            if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
                continue;
            }

            let rel = loop_facts.rel(line_idx);
            if loop_facts.loop_depth_before[rel] != 1 || loop_facts.block_depth_before[rel] != 1 {
                continue;
            }

            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                continue;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
                || !expr_has_only_pure_calls(rhs, pure_user_calls)
            {
                continue;
            }

            let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
            if deps.contains(&iter_var) || deps.contains(lhs) {
                continue;
            }

            candidates.push(HoistCandidate {
                line_idx,
                lhs: lhs.to_string(),
                deps,
            });
        }

        if candidates.is_empty() {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let mut hoists = Vec::<(usize, String)>::new();
        for candidate in candidates {
            if loop_facts.assigned_count(&candidate.lhs) > 1
                || candidate
                    .deps
                    .iter()
                    .any(|dep| loop_facts.assigned_count(dep) > 0)
            {
                continue;
            }

            let lhs_used_before = loop_facts
                .first_ref_idx
                .get(&candidate.lhs)
                .copied()
                .is_some_and(|first_idx| first_idx < candidate.line_idx);
            if lhs_used_before {
                continue;
            }

            let lhs_used_later = loop_facts
                .last_ref_idx
                .get(&candidate.lhs)
                .copied()
                .is_some_and(|last_idx| last_idx > candidate.line_idx);
            if !lhs_used_later {
                continue;
            }

            hoists.push((candidate.line_idx, out[candidate.line_idx].clone()));
        }

        if hoists.is_empty() {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let mut insert_at = next_repeat;
        for line_idx in hoists.iter().rev().map(|(line_idx, _)| *line_idx) {
            out.remove(line_idx);
        }
        for (_, line) in &hoists {
            out.insert(insert_at, line.clone());
            insert_at += 1;
        }
        clear_repeat_loop_facts(cache);
        repeat_idx = insert_at + 1;
    }

    out
}

fn parse_singleton_list_match_cond(line: &str) -> Option<String> {
    let pattern = format!(
        r#"^if \(\(\(length\((?P<base>{})\) == 1L\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some(caps.name("base")?.as_str().to_string())
}

fn parse_single_field_record_match_cond(line: &str) -> Option<(String, String)> {
    let pattern = format!(
        r#"^if \(\(\(TRUE & rr_field_exists\((?P<base>{}), "(?P<field>[^"]+)"\)\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some((
        caps.name("base")?.as_str().to_string(),
        caps.name("field")?.as_str().to_string(),
    ))
}

pub(super) fn restore_empty_match_single_bind_arms(lines: Vec<String>) -> Vec<String> {
    if !has_match_phi_candidates(&lines) {
        return lines;
    }
    let mut out = lines;
    let mut idx = 0usize;
    while idx + 3 < out.len() {
        if out[idx + 1].trim() != "} else {" || out[idx + 3].trim() != "}" {
            idx += 1;
            continue;
        }

        let Some(phi_caps) = assign_re().and_then(|re| re.captures(out[idx + 2].trim())) else {
            idx += 1;
            continue;
        };
        let phi_lhs = phi_caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if !phi_lhs.starts_with(".phi_") {
            idx += 1;
            continue;
        }

        let indent = out[idx + 2]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();

        if let Some(base) = parse_singleton_list_match_cond(&out[idx]) {
            out.insert(idx + 1, format!("{indent}{phi_lhs} <- {base}[1L]"));
            idx += 5;
            continue;
        }

        if let Some((base, field)) = parse_single_field_record_match_cond(&out[idx]) {
            out.insert(
                idx + 1,
                format!("{indent}{phi_lhs} <- {base}[[\"{field}\"]]"),
            );
            idx += 5;
            continue;
        }

        idx += 1;
    }
    out
}

pub(super) fn rewrite_dead_zero_loop_seeds_before_for(lines: Vec<String>) -> Vec<String> {
    rewrite_dead_zero_loop_seeds_before_for_ir(lines)
}
