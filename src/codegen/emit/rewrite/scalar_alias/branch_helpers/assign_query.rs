fn branch_body_writes_symbol_before_local(
    lines: &[String],
    start: usize,
    end_exclusive: usize,
    symbol: &str,
) -> bool {
    lines
        .iter()
        .take(end_exclusive)
        .skip(start)
        .filter_map(|line| parse_local_assign_line(line.trim()))
        .any(|(lhs, _)| lhs == symbol)
}

fn previous_outer_assign_before_branch_local<'a>(
    lines: &'a [String],
    branch_start: usize,
    lhs: &str,
    relaxed: bool,
) -> Option<(&'a str, &'a str)> {
    for prev_idx in (0..branch_start).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() || trimmed == "}" || trimmed == "{" {
            continue;
        }
        if !relaxed && trimmed.ends_with('{') {
            break;
        }
        if relaxed
            && (trimmed == "repeat {"
                || trimmed.starts_with("while ")
                || trimmed.starts_with("while(")
                || trimmed.starts_with("for ")
                || trimmed.starts_with("for(")
                || trimmed.contains("<- function"))
        {
            break;
        }
        let Some((prev_lhs, prev_rhs)) = parse_local_assign_line(trimmed) else {
            continue;
        };
        if prev_lhs == lhs {
            return Some((prev_lhs, prev_rhs));
        }
        if !relaxed && count_symbol_occurrences_local(trimmed, lhs) > 0 {
            break;
        }
    }
    None
}
