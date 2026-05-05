use super::*;
pub(crate) fn rewrite_one_or_two_use_named_scalar_index_reads_in_straight_line_region(
    output: &mut String,
) {
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
            || !is_inlineable_scalar_index_rhs_local(&rhs)
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let rhs_canonical = strip_outer_parens_local(&rhs).to_string();
        let rhs_deps = raw_expr_idents_local(rhs_canonical.as_str());
        if rhs_deps.iter().any(|ident| ident == &lhs) {
            continue;
        }

        let next_def = ((idx + 1)..lines.len())
            .find(|line_idx| {
                parse_local_assign_line(lines[*line_idx].trim())
                    .is_some_and(|(later_lhs, _)| later_lhs == lhs)
            })
            .unwrap_or(lines.len());
        let region_end = straight_line_region_end_local(&lines, idx).min(next_def);
        if region_end <= idx + 1 {
            continue;
        }

        let mut use_line_idxs = Vec::new();
        let mut valid = true;
        for (line_no, line) in lines.iter().enumerate().take(region_end).skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if let Some((later_lhs, _)) = parse_local_assign_line(line_trimmed)
                && rhs_deps.iter().any(|dep| dep == later_lhs)
            {
                valid = false;
                break;
            }
            let occurrences = count_symbol_occurrences_local(line_trimmed, &lhs);
            if occurrences > 1 {
                valid = false;
                break;
            }
            if occurrences == 1 {
                use_line_idxs.push(line_no);
                if use_line_idxs.len() > 2 {
                    valid = false;
                    break;
                }
            }
        }
        if !valid || use_line_idxs.is_empty() {
            continue;
        }

        for use_idx in use_line_idxs {
            lines[use_idx] =
                replace_symbol_occurrences_local(&lines[use_idx], &lhs, rhs_canonical.as_str());
        }
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
