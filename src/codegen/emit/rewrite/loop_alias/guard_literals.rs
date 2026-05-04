use super::*;
pub(crate) fn rewrite_guard_scalar_literals(output: &mut String) {
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
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_simple_scalar_alias_or_literal_local(&rhs)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            continue;
        };
        let mut guard_idx = next_idx;
        if lines[next_idx].trim() == "repeat {" {
            let Some(found_guard) =
                ((next_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
            else {
                continue;
            };
            guard_idx = found_guard;
        }
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if lines[guard_idx].contains("<- function")
            || !is_guard
            || count_symbol_occurrences_local(&guard_trimmed, &lhs) == 0
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(guard_idx + 1) {
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

        lines[guard_idx] = replace_symbol_occurrences_local(
            &lines[guard_idx],
            &lhs,
            strip_outer_parens_local(&rhs),
        );
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(crate) fn rewrite_loop_guard_scalar_literals(output: &mut String) {
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
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_simple_scalar_alias_or_literal_local(&rhs)
        {
            continue;
        }

        let Some(repeat_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        if lines[repeat_idx].trim() != "repeat {" {
            continue;
        }
        let Some(guard_idx) =
            ((repeat_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if !is_guard || count_symbol_occurrences_local(&guard_trimmed, &lhs) == 0 {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(guard_idx + 1) {
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

        lines[guard_idx] = replace_symbol_occurrences_local(
            &lines[guard_idx],
            &lhs,
            strip_outer_parens_local(&rhs),
        );
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
