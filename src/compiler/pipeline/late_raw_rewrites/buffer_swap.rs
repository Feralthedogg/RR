use super::*;

pub(crate) fn restore_buffer_swaps_after_temp_copy_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        let Some(base_var) = lhs.strip_prefix("tmp_") else {
            idx += 1;
            continue;
        };
        if rhs != base_var {
            idx += 1;
            continue;
        }

        let Some((loop_start, loop_end)) = (0..idx).rev().find_map(|line_idx| {
            (lines[line_idx].trim() == "repeat {")
                .then(|| find_raw_block_end(&lines, line_idx).map(|end| (line_idx, end)))
                .flatten()
                .filter(|(_, end)| idx < *end)
        }) else {
            idx += 1;
            continue;
        };

        let candidates = [format!("{base_var}_new"), format!("next_{base_var}")];
        let candidate = candidates.into_iter().find(|candidate| {
            lines
                .iter()
                .take(idx)
                .skip(loop_start + 1)
                .any(|line| raw_line_writes_symbol(line, candidate))
        });
        let Some(candidate) = candidate else {
            idx += 1;
            continue;
        };

        let has_base_swap = lines
            .iter()
            .take(loop_end)
            .skip(idx + 1)
            .any(|line| line.trim() == format!("{base_var} <- {candidate}"));
        let has_candidate_swap = lines
            .iter()
            .take(loop_end)
            .skip(idx + 1)
            .any(|line| line.trim() == format!("{candidate} <- {lhs}"));
        if has_base_swap || has_candidate_swap {
            idx += 1;
            continue;
        }

        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines.insert(idx + 1, format!("{indent}{base_var} <- {candidate}"));
        lines.insert(idx + 2, format!("{indent}{candidate} <- {lhs}"));
        idx += 3;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn raw_line_writes_symbol(line: &str, symbol: &str) -> bool {
    let trimmed = line.trim();
    parse_raw_assign_line(trimmed).is_some_and(|(lhs, _)| lhs == symbol)
        || trimmed.starts_with(&format!("{symbol}["))
        || trimmed.starts_with(&format!("({symbol}) <-"))
}
