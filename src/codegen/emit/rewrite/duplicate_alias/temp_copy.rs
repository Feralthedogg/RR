use super::*;
pub(crate) fn strip_noop_temp_copy_roundtrips(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((tmp_lhs, tmp_rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let tmp_lhs = tmp_lhs.to_string();
        let tmp_rhs = tmp_rhs.to_string();
        if !(tmp_lhs.starts_with(".__pc_src_tmp") || tmp_lhs.starts_with(".__rr_cse_"))
            || !tmp_rhs.chars().all(RBackend::is_symbol_char)
        {
            idx += 1;
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            lines[idx].clear();
            break;
        };
        let Some((next_lhs, next_rhs)) = parse_local_assign_line(lines[next_idx].trim()) else {
            let used_later = lines
                .iter()
                .skip(next_idx)
                .any(|line| count_symbol_occurrences_local(line.trim(), &tmp_lhs) > 0);
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
            continue;
        };
        if next_lhs != tmp_rhs || next_rhs != tmp_lhs {
            let mut used_later = false;
            for later_line in lines.iter().skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if later_trimmed.contains("<- function") {
                    break;
                }
                if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == tmp_lhs
                {
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, &tmp_lhs) > 0 {
                    used_later = true;
                    break;
                }
            }
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
            continue;
        }

        lines[next_idx].clear();
        let used_later = lines
            .iter()
            .skip(next_idx + 1)
            .any(|line| count_symbol_occurrences_local(line.trim(), &tmp_lhs) > 0);
        if !used_later {
            lines[idx].clear();
        }
        idx = next_idx + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
