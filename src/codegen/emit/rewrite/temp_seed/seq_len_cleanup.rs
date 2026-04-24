pub(super) fn strip_dead_seq_len_locals(output: &mut String) {
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

        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".tachyon_")
                || lhs.starts_with(".__rr_cse_")
                || !rhs.starts_with("seq_len(")
            {
                continue;
            }

            let mut used_later = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if let Some((later_lhs, later_rhs)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    if count_symbol_occurrences_local(later_rhs, lhs) > 0 {
                        used_later = true;
                    }
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, lhs) > 0 {
                    used_later = true;
                    break;
                }
            }

            if !used_later {
                lines[idx].clear();
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
