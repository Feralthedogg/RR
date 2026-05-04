use super::*;
pub(crate) fn rewrite_hoisted_loop_counter_aliases(output: &mut String) {
    fn extract_loop_counter_step(rhs: &str) -> Option<String> {
        let rhs = strip_outer_parens_local(rhs).trim();
        let caps = compile_regex(r"^([A-Za-z_][A-Za-z0-9_\.]*)\s*\+\s*(1L?|1\.0)$".to_string())?
            .captures(rhs)?;
        let var = caps.get(1)?.as_str();
        let step = caps.get(2)?.as_str();
        Some(format!("({var} + {step})"))
    }

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
            let lhs = lhs.to_string();
            let rhs = rhs.to_string();
            if !lhs.starts_with("licm_") {
                continue;
            }
            let Some(replacement) = extract_loop_counter_step(rhs.as_str()) else {
                continue;
            };
            let Some(var) = strip_outer_parens_local(rhs.as_str())
                .split('+')
                .next()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
            else {
                continue;
            };

            let mut use_lines = Vec::new();
            let mut valid = true;
            for (later_idx, line) in lines.iter().enumerate().take(fn_end + 1).skip(idx + 1) {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let occurrences = count_symbol_occurrences_local(trimmed, lhs.as_str());
                if occurrences == 0 {
                    continue;
                }
                let Some((later_lhs, later_rhs)) = parse_local_assign_line(trimmed) else {
                    valid = false;
                    break;
                };
                if later_lhs != var || strip_outer_parens_local(later_rhs).trim() != lhs {
                    valid = false;
                    break;
                }
                use_lines.push(later_idx);
            }
            if !valid || use_lines.is_empty() {
                continue;
            }

            for use_idx in use_lines {
                let indent = lines[use_idx]
                    .chars()
                    .take_while(|ch| ch.is_ascii_whitespace())
                    .collect::<String>();
                lines[use_idx] = format!("{indent}{var} <- {replacement}");
            }
            lines[idx].clear();
        }

        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
