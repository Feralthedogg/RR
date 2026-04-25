pub(super) fn rewrite_immediate_and_guard_named_scalar_exprs(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if raw_expr_idents_local(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_named_scalar_expr_local(&rhs)
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let replacement = format!("({})", strip_outer_parens_local(&rhs));

        if let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) {
            let next_trimmed = lines[next_idx].trim().to_string();
            let next_is_assign = parse_local_assign_line(next_trimmed.as_str()).is_some();
            let next_is_return =
                next_trimmed.starts_with("return(") || next_trimmed.starts_with("return (");
            if !lines[next_idx].contains("<- function")
                && (next_is_assign || next_is_return)
                && count_symbol_occurrences_local(&next_trimmed, &lhs) > 0
            {
                let mut used_after = false;
                for later_line in lines.iter().skip(next_idx + 1) {
                    let later_trimmed = later_line.trim();
                    if later_trimmed.is_empty() {
                        continue;
                    }
                    if later_line.contains("<- function") {
                        break;
                    }
                    if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                        && later_lhs == lhs
                    {
                        used_after = true;
                        break;
                    }
                    if count_symbol_occurrences_local(later_trimmed, &lhs) > 0 {
                        used_after = true;
                        break;
                    }
                }
                if !used_after {
                    lines[next_idx] = replace_symbol_occurrences_local(
                        &lines[next_idx],
                        &lhs,
                        replacement.as_str(),
                    );
                    lines[idx].clear();
                    continue;
                }
            }
        }

        let Some(use_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && count_symbol_occurrences_local(trimmed, &lhs) > 0
        }) else {
            continue;
        };
        let use_trimmed = lines[use_idx].trim().to_string();
        let use_is_guard = use_trimmed.starts_with("if (") || use_trimmed.starts_with("if(");
        let use_occurrences = count_symbol_occurrences_local(&use_trimmed, &lhs);
        if lines[use_idx].contains("<- function")
            || !use_is_guard
            || use_occurrences == 0
            || use_occurrences > 2
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(use_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if count_symbol_occurrences_local(later_trimmed, &lhs) > 0 {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[use_idx] =
            replace_symbol_occurrences_local(&lines[use_idx], &lhs, replacement.as_str());
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
