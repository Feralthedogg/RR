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
