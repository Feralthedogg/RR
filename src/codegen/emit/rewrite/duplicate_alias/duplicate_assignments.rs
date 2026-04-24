pub(super) fn rewrite_adjacent_duplicate_symbol_assignments(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return;
    }

    for idx in 0..(lines.len() - 1) {
        let first = lines[idx].trim().to_string();
        let second = lines[idx + 1].trim().to_string();
        let Some((lhs0, rhs0)) = parse_local_assign_line(&first) else {
            continue;
        };
        let Some((lhs1, rhs1)) = parse_local_assign_line(&second) else {
            continue;
        };
        let lhs0 = lhs0.trim();
        let lhs1 = lhs1.trim();
        let rhs0 = rhs0.trim();
        let rhs1 = rhs1.trim();
        if lhs0.is_empty()
            || lhs1.is_empty()
            || lhs0 == lhs1
            || lhs0.starts_with(".arg_")
            || lhs1.starts_with(".arg_")
            || lhs0.starts_with(".__rr_cse_")
            || lhs1.starts_with(".__rr_cse_")
            || !lhs0.chars().all(RBackend::is_symbol_char)
            || !lhs1.chars().all(RBackend::is_symbol_char)
            || rhs0 != rhs1
            || rhs0.starts_with(".arg_")
            || !rhs0.chars().all(RBackend::is_symbol_char)
        {
            continue;
        }

        let indent = lines[idx + 1]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_duplicate_pure_call_assignments(
    output: &mut String,
    pure_user_calls: &FxHashSet<String>,
) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let line_owned = lines[idx].clone();
        let trimmed = line_owned.trim();
        let candidate_indent = line_owned.len() - line_owned.trim_start().len();
        let Some((lhs, rhs)) = parse_local_assign_line(trimmed) else {
            continue;
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if lhs.is_empty()
            || !lhs.chars().all(RBackend::is_symbol_char)
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
        {
            continue;
        }
        let Some((callee, _args)) = parse_top_level_call_local(rhs) else {
            continue;
        };
        if !pure_user_calls.contains(&callee) {
            continue;
        }
        let deps: FxHashSet<String> = raw_expr_idents_local(rhs).into_iter().collect();

        for line in lines.iter_mut().skip(idx + 1) {
            let line_trimmed = line.trim().to_string();
            let next_indent = line.len() - line.trim_start().len();
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                break;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((next_lhs, next_rhs)) = parse_local_assign_line(&line_trimmed) {
                let next_lhs = next_lhs.trim();
                if next_lhs == lhs || deps.contains(next_lhs) {
                    break;
                }
                if next_rhs.trim() == rhs {
                    let indent = line
                        .chars()
                        .take_while(|ch| ch.is_ascii_whitespace())
                        .collect::<String>();
                    *line = format!("{indent}{next_lhs} <- {lhs}");
                }
            }
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
