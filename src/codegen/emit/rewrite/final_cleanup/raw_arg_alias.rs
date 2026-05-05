use super::*;
pub(crate) fn strip_unused_raw_arg_aliases(output: &mut String) {
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
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(RBackend::is_symbol_char) {
                idx += 1;
                continue;
            }
            let used_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .any(|line| count_symbol_occurrences_local(line.trim(), lhs) > 0);
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(crate) fn rewrite_readonly_raw_arg_aliases(output: &mut String) {
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

        let mut aliases = Vec::new();
        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(RBackend::is_symbol_char) {
                continue;
            }
            let reassigned_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .filter_map(|line| parse_local_assign_line(line.trim()))
                .any(|(later_lhs, _)| later_lhs == rhs);
            if reassigned_later {
                continue;
            }
            aliases.push((idx, lhs.to_string(), rhs.to_string()));
        }

        for (alias_idx, alias, target) in aliases {
            for line in lines.iter_mut().take(fn_end + 1).skip(alias_idx + 1) {
                *line = replace_symbol_occurrences_local(line, &alias, &target);
            }
            lines[alias_idx].clear();
        }

        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
