use super::*;
pub(crate) fn rewrite_slice_bound_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < lines.len() {
        let Some((start_lhs, start_rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if start_lhs != "start" {
            idx += 1;
            continue;
        }
        let start_rhs =
            crate::compiler::pipeline::strip_redundant_outer_parens(start_rhs).to_string();
        if !start_rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(end_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let Some((end_lhs, end_rhs)) = parse_local_assign_line(lines[end_idx].trim()) else {
            idx += 1;
            continue;
        };
        if end_lhs != "end" {
            idx += 1;
            continue;
        }
        let end_rhs = crate::compiler::pipeline::strip_redundant_outer_parens(end_rhs).to_string();
        if !end_rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let mut use_line_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(end_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((lhs, _)) = parse_local_assign_line(trimmed)
                && (lhs == "start" || lhs == "end")
            {
                break;
            }
            let uses_start = count_symbol_occurrences_local(trimmed, "start") > 0;
            let uses_end = count_symbol_occurrences_local(trimmed, "end") > 0;
            if uses_start || uses_end {
                if uses_start != uses_end || !trimmed.contains("neighbors[start:end] <-") {
                    use_line_idxs.clear();
                    break;
                }
                use_line_idxs.push(line_no);
                continue;
            }
            let is_control = trimmed == "}"
                || trimmed.starts_with("if (")
                || trimmed.starts_with("if(")
                || trimmed.starts_with("else")
                || trimmed.starts_with("} else");
            if !use_line_idxs.is_empty() && !is_control {
                break;
            }
        }
        if use_line_idxs.is_empty() {
            idx += 1;
            continue;
        }

        let slice_expr = format!("{start_rhs}:{end_rhs}");
        for use_idx in &use_line_idxs {
            lines[*use_idx] = lines[*use_idx].replace("start:end", &slice_expr);
        }
        lines[idx].clear();
        lines[end_idx].clear();
        idx = use_line_idxs.last().copied().unwrap_or(end_idx) + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
