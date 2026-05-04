use super::*;
pub(crate) fn rewrite_temp_uses_after_named_copy(output: &mut String) {
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
        for idx in (fn_start + 1)..=fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            let lhs = lhs.to_string();
            let rhs = rhs.to_string();
            if !plain_ident_local(&lhs)
                || !(rhs.starts_with(".__pc_src_tmp") || rhs.starts_with(".__rr_cse_"))
            {
                continue;
            }
            let temp = rhs;
            let next_temp_def = ((idx + 1)..=fn_end)
                .find(|line_idx| {
                    parse_local_assign_line(lines[*line_idx].trim())
                        .is_some_and(|(later_lhs, _)| later_lhs == temp)
                })
                .unwrap_or(fn_end + 1);
            let next_lhs_def = ((idx + 1)..=fn_end)
                .find(|line_idx| {
                    parse_local_assign_line(lines[*line_idx].trim())
                        .is_some_and(|(later_lhs, _)| later_lhs == lhs)
                })
                .unwrap_or(fn_end + 1);
            let region_end = next_temp_def.min(next_lhs_def);
            for line in lines.iter_mut().take(region_end).skip(idx + 1) {
                let trimmed = line.trim();
                if trimmed.is_empty() || count_symbol_occurrences_local(trimmed, &temp) == 0 {
                    continue;
                }
                *line = replace_symbol_occurrences_local(line, &temp, &lhs);
            }
        }
        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
