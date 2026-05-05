use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

pub(crate) fn has_repeat_loop_candidates(lines: &[String]) -> bool {
    lines.iter().any(|line| line.trim() == "repeat {")
}

pub(crate) fn has_repeat_guard_break_candidates(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        let trimmed = line.trim();
        (trimmed.starts_with("if (!(")
            || trimmed.starts_with("if !(")
            || trimmed.starts_with("if (!rr_truthy1("))
            && trimmed.ends_with("break")
    })
}

pub(crate) fn has_next_candidates(lines: &[String]) -> bool {
    lines.iter().any(|line| line.trim() == "next")
}

pub(crate) fn has_for_seq_len_candidates(lines: &[String]) -> bool {
    lines
        .iter()
        .any(|line| line.trim_start().starts_with("for (") && line.contains("seq_len("))
}

pub(crate) fn has_match_phi_candidates(lines: &[String]) -> bool {
    lines.iter().any(|line| line.contains(".phi_"))
        && lines.iter().any(|line| line.trim() == "} else {")
}

pub(crate) fn parse_constant_repeat_guard(line: &str) -> Option<(String, String, String)> {
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
pub(crate) enum RepeatBlockKind {
    Loop,
    Other,
}

pub(crate) fn count_repeat_loops(stack: &[RepeatBlockKind]) -> usize {
    stack
        .iter()
        .filter(|kind| matches!(kind, RepeatBlockKind::Loop))
        .count()
}

pub(crate) fn leading_close_count(line: &str) -> usize {
    line.chars().take_while(|ch| *ch == '}').count()
}

#[derive(Debug, Clone)]
pub(crate) struct RepeatLoopFacts {
    pub(crate) repeat_idx: usize,
    pub(crate) loop_end: usize,
    pub(crate) body_start: usize,
    pub(crate) significant_lines: Vec<usize>,
    pub(crate) next_lines: Vec<usize>,
    pub(crate) block_depth_before: Vec<usize>,
    pub(crate) loop_depth_before: Vec<usize>,
    pub(crate) assigned_counts: FxHashMap<String, usize>,
    pub(crate) first_ref_idx: FxHashMap<String, usize>,
    pub(crate) last_ref_idx: FxHashMap<String, usize>,
}

#[derive(Debug)]
pub(crate) struct RepeatBodyFacts {
    pub(crate) significant_lines: Vec<usize>,
    pub(crate) next_lines: Vec<usize>,
    pub(crate) block_depth_before: Vec<usize>,
    pub(crate) loop_depth_before: Vec<usize>,
    pub(crate) assigned_counts: FxHashMap<String, usize>,
    pub(crate) first_ref_idx: FxHashMap<String, usize>,
    pub(crate) last_ref_idx: FxHashMap<String, usize>,
}

impl RepeatLoopFacts {
    pub(crate) fn rel(&self, line_idx: usize) -> usize {
        line_idx - self.body_start
    }

    pub(crate) fn mentions_ident(&self, ident: &str) -> bool {
        self.first_ref_idx.contains_key(ident)
    }

    pub(crate) fn assigned_count(&self, ident: &str) -> usize {
        self.assigned_counts.get(ident).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RepeatLoopAnalysisCache {
    pub(crate) signature: Option<u64>,
    pub(crate) loops: Vec<RepeatLoopFacts>,
}

pub(crate) fn lines_signature(lines: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for line in lines {
        line.hash(&mut hasher);
    }
    hasher.finish()
}

pub(crate) fn collect_repeat_loop_facts(lines: &[String]) -> Vec<RepeatLoopFacts> {
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

pub(crate) fn cached_repeat_loop_facts<'a>(
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

pub(crate) fn clear_repeat_loop_facts(cache: &mut RepeatLoopAnalysisCache) {
    cache.signature = None;
    cache.loops.clear();
}

pub(crate) fn next_repeat_loop_fact_at_or_after(
    cache: &mut RepeatLoopAnalysisCache,
    lines: &[String],
    idx: usize,
) -> Option<RepeatLoopFacts> {
    cached_repeat_loop_facts(cache, lines)
        .iter()
        .find(|facts| facts.repeat_idx >= idx)
        .cloned()
}

pub(crate) fn analyze_repeat_loop(lines: &[String], repeat_idx: usize) -> Option<RepeatLoopFacts> {
    let loop_end = find_matching_block_end(lines, repeat_idx)?;
    let body_start = repeat_idx + 1;
    let body_facts = collect_repeat_body_facts(lines, body_start, loop_end);

    Some(RepeatLoopFacts {
        repeat_idx,
        loop_end,
        body_start,
        significant_lines: body_facts.significant_lines,
        next_lines: body_facts.next_lines,
        block_depth_before: body_facts.block_depth_before,
        loop_depth_before: body_facts.loop_depth_before,
        assigned_counts: body_facts.assigned_counts,
        first_ref_idx: body_facts.first_ref_idx,
        last_ref_idx: body_facts.last_ref_idx,
    })
}

pub(crate) fn collect_repeat_body_facts(
    lines: &[String],
    body_start: usize,
    loop_end: usize,
) -> RepeatBodyFacts {
    let mut significant_lines = Vec::new();
    let mut next_lines = Vec::new();
    let mut block_stack = vec![RepeatBlockKind::Loop];
    let mut block_depth_before = vec![0usize; loop_end.saturating_sub(body_start)];
    let mut loop_depth_before = vec![0usize; loop_end.saturating_sub(body_start)];
    let mut assigned_counts = FxHashMap::<String, usize>::default();
    let mut first_ref_idx = FxHashMap::<String, usize>::default();
    let mut last_ref_idx = FxHashMap::<String, usize>::default();

    for (line_idx, line) in lines.iter().enumerate().take(loop_end).skip(body_start) {
        let trimmed_start = line.trim_start();
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

    RepeatBodyFacts {
        significant_lines,
        next_lines,
        block_depth_before,
        loop_depth_before,
        assigned_counts,
        first_ref_idx,
        last_ref_idx,
    }
}

pub(crate) fn line_indent(line: &str) -> String {
    line.chars()
        .take_while(|ch| ch.is_ascii_whitespace())
        .collect()
}

pub(crate) fn find_repeat_guard_break(
    lines: &[String],
    loop_facts: &RepeatLoopFacts,
) -> Option<usize> {
    loop_facts
        .significant_lines
        .iter()
        .copied()
        .find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            (trimmed.starts_with("if (!(") || trimmed.starts_with("if !("))
                && trimmed.ends_with("break")
        })
}

pub(crate) fn find_wrapped_repeat_guard_break(
    lines: &[String],
    loop_facts: &RepeatLoopFacts,
) -> Option<usize> {
    loop_facts
        .significant_lines
        .iter()
        .copied()
        .find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            trimmed.starts_with("if (!(") && trimmed.ends_with(")) break")
        })
}

pub(crate) fn repeat_increment_literal(start_lit: &str) -> &'static str {
    if start_lit.contains('.') {
        "1.0"
    } else if start_lit.ends_with('L') || start_lit.ends_with('l') {
        "1L"
    } else {
        "1"
    }
}

pub(crate) fn normalize_constant_repeat_guard_line(
    out: &mut Vec<String>,
    loop_facts: &RepeatLoopFacts,
    repeat_idx: usize,
    guard_idx: usize,
    loop_end: usize,
) -> (usize, usize, bool) {
    let Some((start_lit, cmp, bound)) = parse_constant_repeat_guard(&out[guard_idx]) else {
        return (guard_idx, loop_end, false);
    };

    let idx_var = ".__rr_i";
    if loop_facts.mentions_ident(idx_var) {
        return (guard_idx, loop_end, false);
    }

    let indent = line_indent(&out[guard_idx]);
    let repeat_indent = line_indent(&out[repeat_idx]);
    out.insert(
        repeat_idx,
        format!("{repeat_indent}{idx_var} <- {start_lit}"),
    );

    let guard_idx = guard_idx + 1;
    let mut loop_end = loop_end + 1;
    out[guard_idx] = if cmp == "<=" {
        format!("{indent}if (!({idx_var} <= {bound})) break")
    } else {
        format!("{indent}if (!({idx_var} < {bound})) break")
    };

    out.insert(
        loop_end,
        format!(
            "{indent}{idx_var} <- ({idx_var} + {})",
            repeat_increment_literal(&start_lit)
        ),
    );
    loop_end += 1;
    (guard_idx, loop_end, true)
}

pub(crate) fn is_plain_repeat_index_var(idx_var: &str) -> bool {
    plain_ident_re().is_some_and(|re| re.is_match(idx_var))
}

pub(crate) fn line_updates_var(line: &str, var: &str) -> bool {
    line.trim().starts_with(&format!("{var} <-"))
}

pub(crate) fn collect_missing_next_increments(
    lines: &[String],
    loop_facts: &RepeatLoopFacts,
    guard_idx: usize,
    loop_end: usize,
    idx_var: &str,
) -> Vec<(usize, String)> {
    let mut insertions = Vec::new();
    for line_idx in loop_facts.next_lines.iter().copied() {
        if line_idx <= guard_idx || line_idx >= loop_end {
            continue;
        }
        let rel = loop_facts.rel(line_idx);
        if loop_facts.loop_depth_before[rel] != 1 {
            continue;
        }
        if branch_updates_index_before_next(lines, loop_facts, guard_idx, line_idx, idx_var) {
            continue;
        }

        let indent = line_indent(&lines[line_idx]);
        insertions.push((line_idx, format!("{indent}{idx_var} <- ({idx_var} + 1L)")));
    }
    insertions
}

pub(crate) fn branch_updates_index_before_next(
    lines: &[String],
    loop_facts: &RepeatLoopFacts,
    guard_idx: usize,
    next_idx: usize,
    idx_var: &str,
) -> bool {
    let branch_depth = loop_facts.block_depth_before[loop_facts.rel(next_idx)];
    let mut scan = next_idx;
    while scan > guard_idx + 1 {
        scan -= 1;
        let trimmed = lines[scan].trim();
        if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
            continue;
        }
        if loop_facts.block_depth_before[loop_facts.rel(scan)] < branch_depth {
            break;
        }
        if line_updates_var(trimmed, idx_var) {
            return true;
        }
    }
    false
}

pub(crate) fn normalize_repeat_loop_counters(lines: Vec<String>) -> Vec<String> {
    let mut cache = RepeatLoopAnalysisCache::default();
    normalize_repeat_loop_counters_with_cache(lines, &mut cache)
}

pub(crate) fn normalize_repeat_loop_counters_with_cache(
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
        let Some(mut guard_idx) = find_repeat_guard_break(&out, &loop_facts) else {
            idx = loop_end + 1;
            continue;
        };

        let normalized = normalize_constant_repeat_guard_line(
            &mut out,
            &loop_facts,
            repeat_idx,
            guard_idx,
            loop_end,
        );
        guard_idx = normalized.0;
        loop_end = normalized.1;
        changed |= normalized.2;

        let Some((idx_var, op, _bound)) = parse_repeat_guard_cmp_line(out[guard_idx].trim()) else {
            idx = loop_end + 1;
            continue;
        };
        if op != "<=" || !is_plain_repeat_index_var(&idx_var) {
            idx = loop_end + 1;
            continue;
        }

        if loop_facts.assigned_count(&idx_var) == 0 && loop_facts.mentions_ident(&idx_var) {
            let inner_indent = line_indent(&out[guard_idx]);
            out.insert(
                loop_end,
                format!("{inner_indent}{idx_var} <- ({idx_var} + 1L)"),
            );
            loop_end += 1;
            changed = true;
        }

        if !loop_facts.next_lines.is_empty() {
            let insertions =
                collect_missing_next_increments(&out, &loop_facts, guard_idx, loop_end, &idx_var);

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

pub(crate) fn literal_one_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^(?:1L?|1(?:\.0+)?)$".to_string()))
        .as_ref()
}

pub(crate) fn literal_positive_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^(?:[1-9]\d*)(?:\.0+)?L?$".to_string()))
        .as_ref()
}

pub(crate) fn literal_integer_value(expr: &str) -> Option<i64> {
    let trimmed = expr.trim().trim_end_matches('L').trim_end_matches('l');
    if let Ok(value) = trimmed.parse::<i64>() {
        return Some(value);
    }
    let value = trimmed.parse::<f64>().ok()?;
    (value.fract() == 0.0).then_some(value as i64)
}

pub(crate) fn latest_literal_assignment_before(
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

pub(crate) fn parse_repeat_guard_cmp_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = if let Some(inner) = trimmed
        .strip_prefix("if (!rr_truthy1(")
        .and_then(|s| s.strip_suffix(")) break"))
    {
        let args = split_top_level_args(inner)?;
        args.first()?.trim().to_string()
    } else {
        let inner = trimmed
            .strip_prefix("if !(")
            .or_else(|| trimmed.strip_prefix("if (!("))?;
        inner
            .strip_suffix(")) break")
            .or_else(|| inner.strip_suffix(") break"))?
            .trim()
            .to_string()
    };
    let inner = strip_redundant_outer_parens(&inner).trim();
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

pub(crate) fn var_has_known_positive_progression_before(
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

pub(crate) fn positive_guard_for_var_before(
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

pub(crate) fn count_unquoted_braces(line: &str) -> (usize, usize) {
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

pub(crate) fn restore_missing_scalar_loop_increments(lines: Vec<String>) -> Vec<String> {
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

pub(crate) fn restore_constant_one_guard_repeat_loop_counters(lines: Vec<String>) -> Vec<String> {
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
        let Some((start_lit, cmp, bound)) = parse_constant_repeat_guard(&out[guard_idx]) else {
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

        let indent = line_indent(&out[guard_idx]);
        let repeat_indent = line_indent(&out[idx]);

        out.insert(idx, format!("{repeat_indent}{idx_var} <- {start_lit}"));
        let guard_line = if cmp == "<=" {
            format!("{indent}if (!({idx_var} <= {bound})) break")
        } else {
            format!("{indent}if (!({idx_var} < {bound})) break")
        };
        out[guard_idx + 1] = guard_line;
        out.insert(
            loop_end + 1,
            format!(
                "{indent}{idx_var} <- ({idx_var} + {})",
                repeat_increment_literal(&start_lit)
            ),
        );
        idx = loop_end + 3;
    }

    out
}

pub(crate) fn restore_missing_scalar_loop_next_increments(lines: Vec<String>) -> Vec<String> {
    let mut cache = RepeatLoopAnalysisCache::default();
    restore_missing_scalar_loop_next_increments_with_cache(lines, &mut cache)
}

pub(crate) fn restore_missing_scalar_loop_next_increments_with_cache(
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

        let Some(guard_idx) = find_wrapped_repeat_guard_break(&out, &loop_facts) else {
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
        if !is_plain_repeat_index_var(idx_var) {
            idx = loop_end + 1;
            continue;
        }

        let insertions =
            collect_missing_next_increments(&out, &loop_facts, guard_idx, loop_end, idx_var);

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

pub(crate) fn rewrite_canonical_counted_repeat_loops_to_for(lines: Vec<String>) -> Vec<String> {
    let mut cache = RepeatLoopAnalysisCache::default();
    rewrite_canonical_counted_repeat_loops_to_for_with_cache(lines, &mut cache)
}
