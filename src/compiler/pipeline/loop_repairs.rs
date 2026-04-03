use super::*;

pub(crate) fn parse_raw_clamp_guard_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = trimmed.strip_prefix("if ((")?.strip_suffix(")) {")?.trim();
    for op in ["<", ">"] {
        let needle = format!(" {op} ");
        let (lhs, rhs) = inner.split_once(&needle)?;
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if !lhs.is_empty() && !rhs.is_empty() {
            return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
        }
    }
    None
}

pub(crate) fn parse_raw_repeat_guard_cmp_line(line: &str) -> Option<(String, String, String)> {
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
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if !lhs.is_empty() && !rhs.is_empty() {
            return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
        }
    }
    None
}

pub(crate) fn latest_raw_literal_assignment_before(
    lines: &[String],
    idx: usize,
    var: &str,
) -> Option<i64> {
    for line in lines.iter().take(idx).rev() {
        let Some((lhs, rhs)) = parse_raw_assign_line(line.trim()) else {
            continue;
        };
        if lhs != var {
            continue;
        }
        let rhs = strip_redundant_outer_parens(rhs).trim_end_matches('L');
        if let Ok(value) = rhs.parse::<i64>() {
            return Some(value);
        }
        break;
    }
    None
}

pub(crate) fn restore_missing_repeat_loop_counter_updates_in_raw_emitted_r(output: &str) -> String {
    fn latest_raw_literal_seed_before(lines: &[String], idx: usize, var: &str) -> Option<String> {
        for line in lines.iter().take(idx).rev() {
            let Some((lhs, rhs)) = parse_raw_assign_line(line.trim()) else {
                continue;
            };
            if lhs != var {
                continue;
            }
            let rhs = strip_redundant_outer_parens(rhs).trim();
            let numeric = rhs.trim_end_matches('L').trim_end_matches('l');
            if numeric.parse::<f64>().ok().is_some() {
                return Some(rhs.to_string());
            }
            break;
        }
        None
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some(repeat_idx) =
            (idx..lines.len()).find(|line_idx| lines[*line_idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_raw_block_end(&lines, repeat_idx) else {
            break;
        };
        let Some(guard_idx) = ((repeat_idx + 1)..loop_end)
            .find(|line_idx| parse_raw_repeat_guard_cmp_line(lines[*line_idx].trim()).is_some())
        else {
            idx = loop_end + 1;
            continue;
        };
        let Some((iter_var, _cmp, _bound)) =
            parse_raw_repeat_guard_cmp_line(lines[guard_idx].trim())
        else {
            idx = loop_end + 1;
            continue;
        };
        if !iter_var.chars().all(is_symbol_char) {
            idx = loop_end + 1;
            continue;
        }
        let Some(seed) = latest_raw_literal_seed_before(&lines, guard_idx, &iter_var) else {
            idx = loop_end + 1;
            continue;
        };

        let mut body_uses_iter = false;
        let mut body_assigns_iter = false;
        for line in lines.iter().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "# rr-cse-pruned" {
                continue;
            }
            if let Some((lhs, _rhs)) = parse_raw_assign_line(trimmed)
                && lhs == iter_var
            {
                body_assigns_iter = true;
                break;
            }
            if line_contains_symbol(trimmed, &iter_var) {
                body_uses_iter = true;
            }
        }
        if body_assigns_iter || !body_uses_iter {
            idx = loop_end + 1;
            continue;
        }

        let indent = lines[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let step = if seed.contains('.') {
            "1.0"
        } else if seed.ends_with('L') || seed.ends_with('l') {
            "1L"
        } else {
            "1"
        };

        let insert_idx = ((guard_idx + 1)..loop_end)
            .rev()
            .find(|line_idx| {
                let trimmed = lines[*line_idx].trim();
                !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
            })
            .filter(|line_idx| lines[*line_idx].trim() == "next")
            .unwrap_or(loop_end);
        lines.insert(
            insert_idx,
            format!("{indent}{iter_var} <- ({iter_var} + {step})"),
        );
        idx = loop_end + 2;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn restore_constant_one_guard_repeat_loop_counters_in_raw_emitted_r(
    output: &str,
) -> String {
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
            let lhs = strip_redundant_outer_parens(lhs.trim());
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

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        if lines[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }
        let Some(loop_end) = find_raw_block_end(&lines, idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            (trimmed.starts_with("if !(") || trimmed.starts_with("if (!("))
                && trimmed.ends_with("break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let Some((start_lit, cmp, bound)) = parse_constant_guard(&lines[guard_idx]) else {
            idx = loop_end + 1;
            continue;
        };
        let idx_var = ".__rr_i";
        if lines
            .iter()
            .take(loop_end)
            .skip(guard_idx + 1)
            .any(|line| line_contains_symbol(line.trim(), idx_var))
        {
            idx = loop_end + 1;
            continue;
        }

        let indent = lines[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let repeat_indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines.insert(idx, format!("{repeat_indent}{idx_var} <- {start_lit}"));
        lines[guard_idx + 1] = if cmp == "<=" {
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
        lines.insert(
            loop_end + 1,
            format!("{indent}{idx_var} <- ({idx_var} + {one})"),
        );
        idx = loop_end + 3;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_exact_safe_loop_index_write_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut repeat_idx = 0usize;
    while repeat_idx < lines.len() {
        let Some(next_repeat) =
            (repeat_idx..lines.len()).find(|idx| lines[*idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_raw_block_end(&lines, next_repeat) else {
            break;
        };
        let Some(guard_idx) = (next_repeat + 1..loop_end).find(|idx| {
            let trimmed = lines[*idx].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some((iter_var, _op, _bound)) =
            parse_raw_repeat_guard_cmp_line(lines[guard_idx].trim())
        else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some(start_value) = latest_raw_literal_assignment_before(&lines, guard_idx, &iter_var)
        else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        if start_value < 1 || !iter_var.chars().all(is_symbol_char) {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let canonical_inc_1 = format!("{iter_var} <- ({iter_var} + 1)");
        let canonical_inc_1l = format!("{iter_var} <- ({iter_var} + 1L)");
        let canonical_inc_1f = format!("{iter_var} <- ({iter_var} + 1.0)");
        let mut safe = true;
        for line in lines.iter().take(loop_end).skip(guard_idx + 1) {
            let Some((lhs, _rhs)) = parse_raw_assign_line(line.trim()) else {
                continue;
            };
            if lhs != iter_var {
                continue;
            }
            let trimmed = line.trim();
            if trimmed != canonical_inc_1
                && trimmed != canonical_inc_1l
                && trimmed != canonical_inc_1f
            {
                safe = false;
                break;
            }
        }
        if !safe {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let needle_double = format!("rr_index1_write({iter_var}, \"index\")");
        let needle_single = format!("rr_index1_write({iter_var}, 'index')");
        for line in lines.iter_mut().take(loop_end).skip(guard_idx + 1) {
            if line.contains(&needle_double) {
                *line = line.replace(&needle_double, &iter_var);
            }
            if line.contains(&needle_single) {
                *line = line.replace(&needle_single, &iter_var);
            }
        }

        repeat_idx = next_repeat + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_seq_len_full_overwrite_inits_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_redundant_outer_parens(rhs).to_string();
        let Some(seq_inner) = rhs
            .strip_prefix("seq_len(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            idx += 1;
            continue;
        };

        let Some(iter_init_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            break;
        };
        let Some((iter_var, iter_start)) = parse_raw_assign_line(lines[iter_init_idx].trim())
        else {
            idx += 1;
            continue;
        };
        if iter_start.trim() != "1" && iter_start.trim() != "1L" {
            idx += 1;
            continue;
        }

        let Some(repeat_idx) = ((iter_init_idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed == "repeat {"
        }) else {
            idx += 1;
            continue;
        };
        let Some(loop_end) = find_raw_block_end(&lines, repeat_idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) = ((repeat_idx + 1)..loop_end).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some((guard_iter, _op, guard_bound)) =
            parse_raw_repeat_guard_cmp_line(lines[guard_idx].trim())
        else {
            idx += 1;
            continue;
        };
        if guard_iter != iter_var || guard_bound != seq_inner {
            idx += 1;
            continue;
        }

        let mut first_use_idx = None;
        let mut safe = true;
        let write_pat = format!("{lhs}[{iter_var}] <-");
        for (body_idx, line) in lines.iter().enumerate().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "next" {
                continue;
            }
            if line.contains("<- function") {
                safe = false;
                break;
            }
            if first_use_idx.is_none() && line_contains_symbol(trimmed, &lhs) {
                first_use_idx = Some(body_idx);
                if !trimmed.starts_with(&write_pat) {
                    safe = false;
                }
                break;
            }
        }
        if !safe || first_use_idx.is_none() {
            idx += 1;
            continue;
        }

        lines[idx] = format!(
            "{}{} <- rep.int(0, {})",
            lines[idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>(),
            lhs,
            seq_inner
        );
        idx = loop_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}
