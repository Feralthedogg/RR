pub(super) fn rewrite_small_multiuse_scalar_index_aliases(output: &mut String) {
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
            || lhs.starts_with(".phi_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_scalar_index_rhs_local(&rhs)
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let rhs_canonical = strip_outer_parens_local(&rhs).to_string();
        let rhs_deps = raw_expr_idents_local(rhs_canonical.as_str());

        let mut scan_start = idx + 1;
        while let Some(alias_idx) = (scan_start..lines.len()).find(|i| !lines[*i].trim().is_empty())
        {
            let trimmed = lines[alias_idx].trim();
            let Some((alias_lhs, alias_rhs)) = parse_local_assign_line(trimmed) else {
                break;
            };
            if alias_lhs.starts_with(".arg_")
                || alias_lhs.starts_with(".__rr_cse_")
                || alias_lhs.starts_with(".tachyon_")
                || !is_inlineable_scalar_index_rhs_local(alias_rhs)
            {
                break;
            }
            scan_start = alias_idx + 1;
        }

        let Some(next1_idx) = (scan_start..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let Some(next2_idx) = ((next1_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let next1_trimmed = lines[next1_idx].trim().to_string();
        let next2_trimmed = lines[next2_idx].trim().to_string();
        if lines[next1_idx].contains("<- function")
            || lines[next2_idx].contains("<- function")
            || parse_local_assign_line(next1_trimmed.as_str()).is_none()
            || parse_local_assign_line(next2_trimmed.as_str()).is_none()
        {
            continue;
        }

        let mut use_line_idxs = Vec::new();
        let mut total_uses = 0usize;
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, later_rhs)) = parse_local_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    if count_symbol_occurrences_local(later_rhs, &lhs) > 0 {
                        total_uses = usize::MAX;
                    }
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences_local(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 6 {
                    break;
                }
            }
        }
        if total_uses == 0 || total_uses > 6 {
            continue;
        }
        if use_line_idxs
            .iter()
            .any(|line_no| *line_no != next1_idx && *line_no != next2_idx)
        {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }

        lines[next1_idx] =
            replace_symbol_occurrences_local(&lines[next1_idx], &lhs, rhs_canonical.as_str());
        lines[next2_idx] =
            replace_symbol_occurrences_local(&lines[next2_idx], &lhs, rhs_canonical.as_str());
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
