pub(super) fn strip_redundant_branch_local_vec_fill_rebinds(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            continue;
        };
        let Some(sig) = raw_vec_fill_signature_local(rhs) else {
            continue;
        };
        let Some(branch_start) = enclosing_branch_start_local(&lines, idx) else {
            continue;
        };
        if branch_body_writes_symbol_before_local(&lines, branch_start + 1, idx, lhs) {
            continue;
        }

        let mut prev_match = None;
        for prev_idx in (0..branch_start).rev() {
            let trimmed = lines[prev_idx].trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            if trimmed == "repeat {"
                || trimmed.starts_with("while ")
                || trimmed.starts_with("while(")
                || trimmed.starts_with("for ")
                || trimmed.starts_with("for(")
                || trimmed.contains("<- function")
            {
                break;
            }
            let Some((prev_lhs, prev_rhs)) = parse_local_assign_line(trimmed) else {
                continue;
            };
            if prev_lhs == lhs {
                prev_match = Some(prev_rhs.to_string());
                break;
            }
        }
        let Some(prev_rhs) = prev_match else {
            continue;
        };
        if raw_vec_fill_signature_local(prev_rhs.as_str()).is_some_and(|prev_sig| prev_sig == sig) {
            lines[idx].clear();
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
