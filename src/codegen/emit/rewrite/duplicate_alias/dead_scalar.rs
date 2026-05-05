use super::*;
pub(crate) fn strip_dead_simple_scalar_assigns(output: &mut String) {
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

pub(crate) fn strip_shadowed_simple_scalar_seed_assigns(output: &mut String) {
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
