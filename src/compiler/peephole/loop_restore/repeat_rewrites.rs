use super::*;
pub(crate) fn rewrite_canonical_counted_repeat_loops_to_for_with_cache(
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

pub(crate) fn hoist_loop_invariant_pure_assignments_from_counted_repeat_loops(
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

pub(crate) fn hoist_loop_invariant_pure_assignments_from_counted_repeat_loops_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut RepeatLoopAnalysisCache,
) -> Vec<String> {
    struct HoistCandidate {
        pub(crate) line_idx: usize,
        pub(crate) lhs: String,
        pub(crate) deps: FxHashSet<String>,
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
        for (line_idx, line) in out
            .iter()
            .enumerate()
            .take(loop_end)
            .skip(loop_facts.body_start)
        {
            let trimmed = line.trim();
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

pub(crate) fn parse_singleton_list_match_cond(line: &str) -> Option<String> {
    let pattern = format!(
        r#"^if \(\(\(length\((?P<base>{})\) == 1L\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some(caps.name("base")?.as_str().to_string())
}

pub(crate) fn parse_single_field_record_match_cond(line: &str) -> Option<(String, String)> {
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

pub(crate) fn restore_empty_match_single_bind_arms(lines: Vec<String>) -> Vec<String> {
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

pub(crate) fn rewrite_dead_zero_loop_seeds_before_for(lines: Vec<String>) -> Vec<String> {
    rewrite_dead_zero_loop_seeds_before_for_ir(lines)
}
