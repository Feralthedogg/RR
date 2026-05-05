use super::*;
pub(crate) fn rewrite_named_scalar_pure_call_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_outer_parens_local(rhs).to_string();
        let max_uses = if rhs.starts_with("rr_wrap_index_vec_i(") {
            1usize
        } else if rhs.starts_with("rr_idx_cube_vec_i(") {
            2usize
        } else {
            continue;
        };
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents_local(rhs.as_str());
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
            if let Some((later_lhs, _later_rhs)) = parse_local_assign_line(line_trimmed) {
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
                if total_uses > max_uses {
                    break;
                }
            }
        }
        if total_uses == 0 || total_uses != max_uses {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }
        for use_idx in use_line_idxs {
            lines[use_idx] = replace_symbol_occurrences_local(&lines[use_idx], &lhs, rhs.as_str());
        }
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
