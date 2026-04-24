pub(super) fn rewrite_loop_index_alias_ii(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            continue;
        };
        if lhs != "ii" || rhs != "i" {
            continue;
        }

        let mut replaced_any = false;
        let mut stop_idx = lines.len();
        let mut stopped_on_i_reassign = false;
        for (scan_idx, line) in lines.iter_mut().enumerate().skip(idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                stop_idx = scan_idx;
                break;
            }
            if let Some((next_lhs, _)) = parse_local_assign_line(trimmed)
                && (next_lhs == "ii" || next_lhs == "i")
            {
                stop_idx = scan_idx;
                stopped_on_i_reassign = next_lhs == "i";
                break;
            }
            if count_symbol_occurrences_local(trimmed, "ii") == 0 {
                continue;
            }
            let rewritten = replace_symbol_occurrences_local(line, "ii", "i");
            if rewritten != *line {
                *line = rewritten;
                replaced_any = true;
            }
        }

        let mut keep_alias = false;
        if stopped_on_i_reassign {
            for line in lines.iter().skip(stop_idx + 1) {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if line.contains("<- function") {
                    break;
                }
                if let Some((next_lhs, _)) = parse_local_assign_line(trimmed)
                    && next_lhs == "ii"
                {
                    break;
                }
                if count_symbol_occurrences_local(trimmed, "ii") > 0 {
                    keep_alias = true;
                    break;
                }
            }
        }

        if replaced_any && !keep_alias {
            lines[idx].clear();
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_dead_zero_seed_ii(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        if lines[idx].trim() != "ii <- 0" {
            continue;
        }
        let used_later = lines
            .iter()
            .skip(idx + 1)
            .any(|line| count_symbol_occurrences_local(line.trim(), "ii") > 0);
        if !used_later {
            lines[idx].clear();
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
