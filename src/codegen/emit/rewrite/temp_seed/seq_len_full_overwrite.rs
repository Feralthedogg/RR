use super::*;
pub(crate) fn rewrite_seq_len_full_overwrite_inits(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = crate::compiler::pipeline::strip_redundant_outer_parens(rhs).to_string();
        let Some(seq_inner) = rhs
            .strip_prefix("seq_len(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            idx += 1;
            continue;
        };

        let Some(iter_init_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let Some((iter_var, iter_start)) = parse_local_assign_line(lines[iter_init_idx].trim())
        else {
            idx += 1;
            continue;
        };
        if iter_start.trim() != "1" && iter_start.trim() != "1L" {
            idx += 1;
            continue;
        }

        let Some(repeat_idx) = ((iter_init_idx + 1)..lines.len()).find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            !trimmed.is_empty() && trimmed == "repeat {"
        }) else {
            idx += 1;
            continue;
        };
        let Some(loop_end) = find_block_end_local(&lines, repeat_idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) =
            ((repeat_idx + 1)..loop_end).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some((guard_iter, _op, guard_bound)) =
            parse_repeat_guard_cmp_line_local(lines[guard_idx].trim())
        else {
            idx += 1;
            continue;
        };
        if guard_iter != iter_var || guard_bound != seq_inner {
            idx += 1;
            continue;
        }

        let mut first_use_idx = None;
        let mut safe = true;
        let write_pat = format!("{lhs}[{iter_var}] <-");
        for (body_idx, line) in lines.iter().enumerate().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "next" {
                continue;
            }
            if line.contains("<- function") {
                safe = false;
                break;
            }
            if count_symbol_occurrences_local(trimmed, &lhs) > 0 {
                first_use_idx = Some(body_idx);
                if !trimmed.starts_with(&write_pat) {
                    safe = false;
                }
                break;
            }
        }
        if !safe || first_use_idx.is_none() {
            idx += 1;
            continue;
        }

        lines[idx] = format!(
            "{}{} <- rep.int(0, {})",
            lines[idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>(),
            lhs,
            seq_inner
        );
        idx = loop_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
