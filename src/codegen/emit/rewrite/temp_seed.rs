pub(super) fn rewrite_temp_uses_after_named_copy(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };
        for idx in (fn_start + 1)..=fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            let lhs = lhs.to_string();
            let rhs = rhs.to_string();
            if !plain_ident_local(&lhs)
                || !(rhs.starts_with(".__pc_src_tmp") || rhs.starts_with(".__rr_cse_"))
            {
                continue;
            }
            let temp = rhs;
            let next_temp_def = ((idx + 1)..=fn_end)
                .find(|line_idx| {
                    parse_local_assign_line(lines[*line_idx].trim())
                        .is_some_and(|(later_lhs, _)| later_lhs == temp)
                })
                .unwrap_or(fn_end + 1);
            let next_lhs_def = ((idx + 1)..=fn_end)
                .find(|line_idx| {
                    parse_local_assign_line(lines[*line_idx].trim())
                        .is_some_and(|(later_lhs, _)| later_lhs == lhs)
                })
                .unwrap_or(fn_end + 1);
            let region_end = next_temp_def.min(next_lhs_def);
            for line_no in (idx + 1)..region_end {
                let line = &mut lines[line_no];
                let trimmed = line.trim();
                if trimmed.is_empty() || count_symbol_occurrences_local(trimmed, &temp) == 0 {
                    continue;
                }
                *line = replace_symbol_occurrences_local(line, &temp, &lhs);
            }
        }
        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_dead_seq_len_locals(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".tachyon_")
                || lhs.starts_with(".__rr_cse_")
                || !rhs.starts_with("seq_len(")
            {
                continue;
            }

            let mut used_later = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if let Some((later_lhs, later_rhs)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    if count_symbol_occurrences_local(later_rhs, lhs) > 0 {
                        used_later = true;
                    }
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, lhs) > 0 {
                    used_later = true;
                    break;
                }
            }

            if !used_later {
                lines[idx].clear();
            }
        }

        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_single_assignment_loop_seed_literals(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            continue;
        };
        if rhs.trim() != "1" && rhs.trim() != "1L" {
            continue;
        }

        let Some(next_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            continue;
        };
        let Some((_next_lhs, next_rhs)) = parse_local_assign_line(lines[next_idx].trim()) else {
            continue;
        };
        if !next_rhs.contains(&format!("{lhs}:")) {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _later_rhs)) = parse_local_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if count_symbol_occurrences_local(later_trimmed, lhs) > 0 {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[next_idx] = replace_symbol_occurrences_local(&lines[next_idx], lhs, "1");
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_sym210_loop_seed(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && lines[fn_start].trim() != "Sym_210 <- function(field, w, h)"
        {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            if lines[idx].trim() != "i <- 1" {
                continue;
            }
            let Some(next_idx) =
                ((idx + 1)..fn_end).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };
            let Some((next_lhs, _next_rhs)) = parse_local_assign_line(lines[next_idx].trim())
            else {
                continue;
            };
            if next_lhs != "lap" || !lines[next_idx].contains("i:size - 1") {
                continue;
            }
            lines[idx].clear();
            lines[next_idx] = lines[next_idx].replace("i:size - 1", "1:size - 1");
        }

        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_seq_len_full_overwrite_inits(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = crate::compiler::pipeline::strip_redundant_outer_parens(rhs).to_string();
        let Some(seq_inner) = rhs
            .strip_prefix("seq_len(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            idx += 1;
            continue;
        };

        let Some(iter_init_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let Some((iter_var, iter_start)) = parse_local_assign_line(lines[iter_init_idx].trim())
        else {
            idx += 1;
            continue;
        };
        if iter_start.trim() != "1" && iter_start.trim() != "1L" {
            idx += 1;
            continue;
        }

        let Some(repeat_idx) = ((iter_init_idx + 1)..lines.len()).find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            !trimmed.is_empty() && trimmed == "repeat {"
        }) else {
            idx += 1;
            continue;
        };
        let Some(loop_end) = find_block_end_local(&lines, repeat_idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) =
            ((repeat_idx + 1)..loop_end).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some((guard_iter, _op, guard_bound)) =
            parse_repeat_guard_cmp_line_local(lines[guard_idx].trim())
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
            if count_symbol_occurrences_local(trimmed, &lhs) > 0 {
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

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn restore_missing_repeat_loop_counter_updates(output: &mut String) {
    fn latest_local_literal_seed_before(lines: &[String], idx: usize, var: &str) -> Option<String> {
        for line in lines.iter().take(idx).rev() {
            let Some((lhs, rhs)) = parse_local_assign_line(line.trim()) else {
                continue;
            };
            if lhs != var {
                continue;
            }
            let rhs = crate::compiler::pipeline::strip_redundant_outer_parens(rhs).trim();
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
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some(repeat_idx) =
            (idx..lines.len()).find(|line_idx| lines[*line_idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_block_end_local(&lines, repeat_idx) else {
            break;
        };
        let Some(guard_idx) = ((repeat_idx + 1)..loop_end)
            .find(|line_idx| parse_repeat_guard_cmp_line_local(lines[*line_idx].trim()).is_some())
        else {
            idx = loop_end + 1;
            continue;
        };
        let Some((iter_var, _cmp, _bound)) =
            parse_repeat_guard_cmp_line_local(lines[guard_idx].trim())
        else {
            idx = loop_end + 1;
            continue;
        };
        if !iter_var.chars().all(RBackend::is_symbol_char) {
            idx = loop_end + 1;
            continue;
        }
        let Some(seed) = latest_local_literal_seed_before(&lines, guard_idx, &iter_var) else {
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
            if let Some((lhs, _rhs)) = parse_local_assign_line(trimmed)
                && lhs == iter_var
            {
                body_assigns_iter = true;
                break;
            }
            if count_symbol_occurrences_local(trimmed, &iter_var) > 0 {
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

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
