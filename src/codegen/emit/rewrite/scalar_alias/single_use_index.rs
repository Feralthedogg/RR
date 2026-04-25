pub(super) fn rewrite_single_use_scalar_index_aliases(output: &mut String) {
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
        {
            continue;
        }

        let rhs_canonical = strip_outer_parens_local(&rhs).to_string();
        let rhs_deps = raw_expr_idents_local(rhs_canonical.as_str());

        let mut later_reassigned = false;
        for later_line in lines.iter().skip(idx + 1) {
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, later_rhs)) = parse_local_assign_line(later_line)
                && later_lhs == lhs
            {
                if count_symbol_occurrences_local(later_rhs, &lhs) > 0 {
                    later_reassigned = true;
                }
                break;
            }
        }
        if later_reassigned {
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
            if let Some((later_lhs, _)) = parse_local_assign_line(line_trimmed) {
                if later_lhs == lhs {
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
                if total_uses > 2 {
                    break;
                }
            }
        }
        if total_uses == 0 {
            lines[idx].clear();
            continue;
        }
        if total_uses > 2 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
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
