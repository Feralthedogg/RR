use super::*;
pub(crate) fn rewrite_particle_idx_alias(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx + 2 < lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if lhs != "idx" {
            idx += 1;
            continue;
        }
        let rhs = crate::compiler::pipeline::strip_redundant_outer_parens(rhs).to_string();
        if !rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(next1_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let Some(next2_idx) =
            ((next1_idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let next1 = lines[next1_idx].trim().to_string();
        let next2 = lines[next2_idx].trim().to_string();
        let dx_ok = next1.contains("dx <-")
            && next1.contains("u[idx]")
            && next1.contains("* dt")
            && next1.contains("/ 400000");
        let dy_ok = next2.contains("dy <-")
            && next2.contains("v[idx]")
            && next2.contains("* dt")
            && next2.contains("/ 400000");
        if !dx_ok || !dy_ok {
            idx += 1;
            continue;
        }

        lines[next1_idx] = replace_symbol_occurrences_local(&lines[next1_idx], "idx", rhs.as_str());
        lines[next2_idx] = replace_symbol_occurrences_local(&lines[next2_idx], "idx", rhs.as_str());
        lines[idx].clear();
        idx = next2_idx + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
