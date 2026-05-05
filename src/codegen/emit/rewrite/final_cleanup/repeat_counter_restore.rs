use super::*;
pub(crate) fn restore_constant_one_guard_repeat_loop_counters(output: &mut String) {
    fn parse_constant_guard_local(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim();
        let inner = trimmed
            .strip_prefix("if (!(")
            .or_else(|| trimmed.strip_prefix("if !("))?
            .strip_suffix(")) break")
            .or_else(|| {
                trimmed
                    .strip_prefix("if (!(")
                    .or_else(|| trimmed.strip_prefix("if !("))
                    .and_then(|s| s.strip_suffix(") break"))
            })?
            .trim();
        for op in ["<=", "<"] {
            let needle = format!(" {op} ");
            let Some((lhs, rhs)) = inner.split_once(&needle) else {
                continue;
            };
            let lhs = crate::compiler::pipeline::strip_redundant_outer_parens(lhs.trim());
            let rhs = rhs.trim();
            if lhs.is_empty() || rhs.is_empty() {
                continue;
            }
            let numeric = lhs.trim_end_matches('L').trim_end_matches('l');
            if numeric.parse::<f64>().ok().is_some() {
                return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
            }
        }
        None
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        if lines[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }
        let Some(loop_end) = find_block_end_local(&lines, idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            (trimmed.starts_with("if !(") || trimmed.starts_with("if (!("))
                && trimmed.ends_with("break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let Some((start_lit, cmp, bound)) = parse_constant_guard_local(&lines[guard_idx]) else {
            idx = loop_end + 1;
            continue;
        };
        let idx_var = ".__rr_i";
        if lines
            .iter()
            .take(loop_end)
            .skip(guard_idx + 1)
            .any(|line| count_symbol_occurrences_local(line.trim(), idx_var) > 0)
        {
            idx = loop_end + 1;
            continue;
        }

        let indent = lines[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let repeat_indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines.insert(idx, format!("{repeat_indent}{idx_var} <- {start_lit}"));
        lines[guard_idx + 1] = if cmp == "<=" {
            format!("{indent}if (!({idx_var} <= {bound})) break")
        } else {
            format!("{indent}if (!({idx_var} < {bound})) break")
        };
        let one = if start_lit.contains('.') {
            "1.0"
        } else if start_lit.ends_with('L') || start_lit.ends_with('l') {
            "1L"
        } else {
            "1"
        };
        lines.insert(
            loop_end + 1,
            format!("{indent}{idx_var} <- ({idx_var} + {one})"),
        );
        idx = loop_end + 3;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
