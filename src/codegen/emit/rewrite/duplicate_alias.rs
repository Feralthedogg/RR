pub(super) fn rewrite_adjacent_duplicate_symbol_assignments(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return;
    }

    for idx in 0..(lines.len() - 1) {
        let first = lines[idx].trim().to_string();
        let second = lines[idx + 1].trim().to_string();
        let Some((lhs0, rhs0)) = parse_local_assign_line(&first) else {
            continue;
        };
        let Some((lhs1, rhs1)) = parse_local_assign_line(&second) else {
            continue;
        };
        let lhs0 = lhs0.trim();
        let lhs1 = lhs1.trim();
        let rhs0 = rhs0.trim();
        let rhs1 = rhs1.trim();
        if lhs0.is_empty()
            || lhs1.is_empty()
            || lhs0 == lhs1
            || lhs0.starts_with(".arg_")
            || lhs1.starts_with(".arg_")
            || lhs0.starts_with(".__rr_cse_")
            || lhs1.starts_with(".__rr_cse_")
            || !lhs0.chars().all(RBackend::is_symbol_char)
            || !lhs1.chars().all(RBackend::is_symbol_char)
            || rhs0 != rhs1
            || rhs0.starts_with(".arg_")
            || !rhs0.chars().all(RBackend::is_symbol_char)
        {
            continue;
        }

        let indent = lines[idx + 1]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_duplicate_pure_call_assignments(
    output: &mut String,
    pure_user_calls: &FxHashSet<String>,
) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let line_owned = lines[idx].clone();
        let trimmed = line_owned.trim();
        let candidate_indent = line_owned.len() - line_owned.trim_start().len();
        let Some((lhs, rhs)) = parse_local_assign_line(trimmed) else {
            continue;
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if lhs.is_empty()
            || !lhs.chars().all(RBackend::is_symbol_char)
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
        {
            continue;
        }
        let Some((callee, _args)) = parse_top_level_call_local(rhs) else {
            continue;
        };
        if !pure_user_calls.contains(&callee) {
            continue;
        }
        let deps: FxHashSet<String> = raw_expr_idents_local(rhs).into_iter().collect();

        for line in lines.iter_mut().skip(idx + 1) {
            let line_trimmed = line.trim().to_string();
            let next_indent = line.len() - line.trim_start().len();
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                break;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((next_lhs, next_rhs)) = parse_local_assign_line(&line_trimmed) {
                let next_lhs = next_lhs.trim();
                if next_lhs == lhs || deps.contains(next_lhs) {
                    break;
                }
                if next_rhs.trim() == rhs {
                    let indent = line
                        .chars()
                        .take_while(|ch| ch.is_ascii_whitespace())
                        .collect::<String>();
                    *line = format!("{indent}{next_lhs} <- {lhs}");
                }
            }
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_noop_self_assignments(output: &mut String) {
    let mut out = String::new();
    for line in output.lines() {
        let keep = if let Some((lhs, rhs)) = parse_local_assign_line(line.trim()) {
            lhs != strip_outer_parens_local(rhs)
        } else {
            true
        };
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    if output.is_empty() {
        *output = String::new();
        return;
    }
    if !output.ends_with('\n') {
        out.pop();
    }
    *output = out;
}

pub(super) fn strip_empty_else_blocks(output: &mut String) {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim() == "} else {" {
            let mut close_idx = i + 1;
            while close_idx < lines.len() && lines[close_idx].trim().is_empty() {
                close_idx += 1;
            }
            if close_idx < lines.len() && lines[close_idx].trim() == "}" {
                let indent_len = line.len() - line.trim_start().len();
                let indent = &line[..indent_len];
                out.push(format!("{indent}}}"));
                i = close_idx + 1;
                continue;
            }
        }
        out.push(line.clone());
        i += 1;
    }

    let mut rendered = out.join("\n");
    if output.ends_with('\n') || !rendered.is_empty() {
        rendered.push('\n');
    }
    *output = rendered;
}

pub(super) fn collapse_nested_else_if_blocks(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return;
    }

    let mut changed = true;
    while changed {
        changed = false;
        let mut idx = 0usize;
        while idx < lines.len() {
            if lines[idx].trim() != "} else {" {
                idx += 1;
                continue;
            }
            let Some(nested_if_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                break;
            };
            let nested_if = lines[nested_if_idx].trim().to_string();
            if !nested_if.starts_with("if (") || !nested_if.ends_with('{') {
                idx += 1;
                continue;
            }
            let Some(nested_if_end) = find_block_end_local(&lines, nested_if_idx) else {
                idx += 1;
                continue;
            };
            let Some(else_close_idx) = ((nested_if_end + 1)..lines.len())
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                idx += 1;
                continue;
            };
            if lines[else_close_idx].trim() != "}" {
                idx += 1;
                continue;
            }

            let indent = lines[idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx] = format!("{indent}}} else {nested_if}");
            lines[nested_if_idx].clear();
            lines[else_close_idx].clear();
            changed = true;
            idx = else_close_idx + 1;
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_single_blank_spacers(output: &mut String) {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return;
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if idx > 0 && idx + 1 < lines.len() && lines[idx].trim().is_empty() {
            let Some(prev_idx) = (0..idx)
                .rev()
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };
            let Some(next_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };

            let prev = lines[prev_idx].trim();
            let next = lines[next_idx].trim();
            let prev_is_assign = parse_local_assign_line(prev).is_some();
            let next_is_assign = parse_local_assign_line(next).is_some();
            let next_is_control = next == "repeat {" || next == "}";
            let next_is_branch = next.starts_with("if (") || next.starts_with("if(");
            let next_is_return = next.starts_with("return(") || next.starts_with("return (");
            let prev_opens_block = prev.ends_with('{');
            let prev_is_return = prev.starts_with("return(") || prev.starts_with("return (");
            let prev_is_break = prev.starts_with("if (") && prev.ends_with("break");

            if (prev_is_assign && (next_is_assign || next_is_control || next_is_branch))
                || (prev_opens_block && (next_is_assign || next_is_return || next_is_branch))
                || (prev == "{"
                    && (next_is_assign || next_is_return || next_is_control || next_is_branch))
                || (prev == "}" && (next_is_assign || next == "}"))
                || (prev_is_break && (next_is_assign || next_is_branch || next_is_return))
                || (prev_is_return && next == "}")
            {
                continue;
            }
        }
        kept.push(lines[idx].clone());
    }

    let mut rewritten = kept.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn compact_blank_lines(output: &mut String) {
    let mut out = String::new();
    let mut blank_run = 0usize;
    for line in output.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    if output.is_empty() {
        *output = String::new();
        return;
    }
    if !output.ends_with('\n') {
        out.pop();
    }
    *output = out;
}

pub(super) fn strip_orphan_rr_cse_pruned_markers(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }
    for line in &mut lines {
        if line.trim() == "# rr-cse-pruned" {
            line.clear();
        }
    }
    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

fn find_matching_open_brace_line_local(lines: &[String], close_idx: usize) -> Option<usize> {
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(close_idx + 1) {
        for ch in line.chars() {
            match ch {
                '{' => stack.push(idx),
                '}' => {
                    let open = stack.pop()?;
                    if idx == close_idx {
                        return Some(open);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

pub(super) fn strip_terminal_repeat_nexts(output: &mut String) {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return;
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if lines[idx].trim() == "next"
            && idx + 1 < lines.len()
            && lines[idx + 1].trim() == "}"
            && find_matching_open_brace_line_local(&lines, idx + 1)
                .is_some_and(|open_idx| lines[open_idx].trim() == "repeat {")
        {
            continue;
        }
        kept.push(lines[idx].clone());
    }

    let mut rewritten = kept.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_noop_temp_copy_roundtrips(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((tmp_lhs, tmp_rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let tmp_lhs = tmp_lhs.to_string();
        let tmp_rhs = tmp_rhs.to_string();
        if !(tmp_lhs.starts_with(".__pc_src_tmp") || tmp_lhs.starts_with(".__rr_cse_"))
            || !tmp_rhs.chars().all(RBackend::is_symbol_char)
        {
            idx += 1;
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            lines[idx].clear();
            break;
        };
        let Some((next_lhs, next_rhs)) = parse_local_assign_line(lines[next_idx].trim()) else {
            let used_later = lines
                .iter()
                .skip(next_idx)
                .any(|line| count_symbol_occurrences_local(line.trim(), &tmp_lhs) > 0);
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
            continue;
        };
        if next_lhs != tmp_rhs || next_rhs != tmp_lhs {
            let mut used_later = false;
            for later_line in lines.iter().skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if later_trimmed.contains("<- function") {
                    break;
                }
                if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == tmp_lhs
                {
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, &tmp_lhs) > 0 {
                    used_later = true;
                    break;
                }
            }
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
            continue;
        }

        lines[next_idx].clear();
        let used_later = lines
            .iter()
            .skip(next_idx + 1)
            .any(|line| count_symbol_occurrences_local(line.trim(), &tmp_lhs) > 0);
        if !used_later {
            lines[idx].clear();
        }
        idx = next_idx + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_dead_simple_scalar_assigns(output: &mut String) {
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
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || (!rhs_is_simple_scalar_alias_or_literal_local(rhs)
                    && !rhs_is_simple_dead_expr_local(rhs))
                || raw_enclosing_repeat_guard_mentions_symbol_local(&lines, idx, lhs)
                || raw_enclosing_repeat_body_reads_symbol_before_local(&lines, idx, lhs)
            {
                idx += 1;
                continue;
            }
            let mut used_later = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if count_symbol_occurrences_local(later_trimmed, lhs) > 0 {
                    used_later = true;
                    break;
                }
            }
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_shadowed_simple_scalar_seed_assigns(output: &mut String) {
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
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || (!rhs_is_simple_scalar_alias_or_literal_local(rhs)
                    && !rhs_is_simple_dead_expr_local(rhs))
                || raw_enclosing_repeat_guard_mentions_symbol_local(&lines, idx, lhs)
                || raw_enclosing_repeat_body_reads_symbol_before_local(&lines, idx, lhs)
            {
                idx += 1;
                continue;
            }

            let mut shadowed_before_use = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if later_trimmed.starts_with("if (")
                    || later_trimmed.starts_with("if(")
                    || later_trimmed.starts_with("} else {")
                    || later_trimmed.starts_with("} else if")
                    || later_trimmed == "repeat {"
                    || later_trimmed == "}"
                    || later_trimmed == "next"
                    || later_trimmed.starts_with("return(")
                    || later_trimmed.starts_with("return (")
                {
                    break;
                }
                if let Some((later_lhs, later_rhs)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    if count_symbol_occurrences_local(later_rhs, lhs) == 0 {
                        shadowed_before_use = true;
                    }
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, lhs) > 0 {
                    break;
                }
            }

            if shadowed_before_use {
                lines[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
