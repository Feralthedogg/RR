use super::*;
pub(crate) fn hoist_branch_local_pure_scalar_assigns_used_after_branch(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim().to_string();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            idx += 1;
            continue;
        }
        let guard_idents = raw_expr_idents_local(trimmed.as_str());
        let Some(end_idx) = find_block_end_local(&lines, idx) else {
            idx += 1;
            continue;
        };

        let mut trailing_assigns = Vec::new();
        let mut scan = end_idx;
        while scan > idx + 1 {
            scan -= 1;
            let trimmed_line = lines[scan].trim();
            if trimmed_line.is_empty() {
                continue;
            }
            let Some((lhs, rhs)) = parse_local_assign_line(trimmed_line) else {
                break;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || !is_branch_hoistable_named_scalar_rhs_local(rhs)
            {
                break;
            }
            trailing_assigns.push((scan, lhs.to_string(), rhs.to_string()));
        }
        if trailing_assigns.is_empty() {
            idx = end_idx + 1;
            continue;
        }
        trailing_assigns.reverse();

        let mut hoisted = Vec::new();
        for (assign_idx, lhs, rhs) in trailing_assigns {
            if guard_idents.iter().any(|ident| ident == &lhs) {
                continue;
            }
            let rhs_deps = raw_expr_idents_local(strip_outer_parens_local(&rhs));
            let dep_written_in_branch = lines
                .iter()
                .take(assign_idx)
                .skip(idx + 1)
                .filter_map(|line| parse_local_assign_line(line.trim()))
                .any(|(branch_lhs, _)| rhs_deps.iter().any(|dep| dep == branch_lhs));
            if dep_written_in_branch {
                continue;
            }

            let mut used_after = false;
            for later_line in lines.iter().skip(end_idx + 1) {
                let later_trimmed = later_line.trim();
                if later_line.contains("<- function") {
                    break;
                }
                if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, &lhs) > 0 {
                    used_after = true;
                    break;
                }
            }
            if used_after {
                hoisted.push(lines[assign_idx].clone());
                lines[assign_idx].clear();
            }
        }

        if !hoisted.is_empty() {
            for (offset, line) in hoisted.into_iter().enumerate() {
                lines.insert(idx + offset, line);
            }
            idx = end_idx + 1;
            continue;
        }
        idx = end_idx + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
