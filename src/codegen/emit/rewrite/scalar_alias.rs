pub(super) fn rewrite_single_use_scalar_index_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".phi_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_scalar_index_rhs_local(&rhs)
        {
            continue;
        }

        let rhs_canonical = strip_outer_parens_local(&rhs).to_string();
        let rhs_deps = raw_expr_idents_local(rhs_canonical.as_str());

        let mut later_reassigned = false;
        for later_line in lines.iter().skip(idx + 1) {
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, later_rhs)) = parse_local_assign_line(later_line)
                && later_lhs == lhs
            {
                if count_symbol_occurrences_local(later_rhs, &lhs) > 0 {
                    later_reassigned = true;
                }
                break;
            }
        }
        if later_reassigned {
            continue;
        }

        let mut use_line_idxs = Vec::new();
        let mut total_uses = 0usize;
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_local_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences_local(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 2 {
                    break;
                }
            }
        }
        if total_uses == 0 {
            lines[idx].clear();
            continue;
        }
        if total_uses > 2 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }
        for use_idx in use_line_idxs {
            lines[use_idx] =
                replace_symbol_occurrences_local(&lines[use_idx], &lhs, rhs_canonical.as_str());
        }
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

fn is_raw_alloc_like_expr_local(expr: &str) -> bool {
    [
        "rep.int(",
        "numeric(",
        "integer(",
        "logical(",
        "character(",
        "vector(",
        "matrix(",
        "Sym_17(",
    ]
    .iter()
    .any(|prefix| expr.starts_with(prefix))
}

fn is_raw_branch_rebind_candidate_local(expr: &str) -> bool {
    is_raw_alloc_like_expr_local(expr)
        || expr.chars().all(RBackend::is_symbol_char)
        || expr.trim_end_matches('L').parse::<f64>().is_ok()
        || matches!(expr, "TRUE" | "FALSE" | "NA" | "NULL")
}

fn raw_vec_fill_signature_local(expr: &str) -> Option<(String, String)> {
    let expr = strip_outer_parens_local(expr);
    if let Some(inner) = expr
        .strip_prefix("rep.int(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let args = split_top_level_args_local(inner)?;
        if args.len() == 2 {
            return Some((args[1].trim().to_string(), args[0].trim().to_string()));
        }
    }
    if let Some(inner) = expr
        .strip_prefix("Sym_17(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let args = split_top_level_args_local(inner)?;
        if args.len() == 2 {
            return Some((args[0].trim().to_string(), args[1].trim().to_string()));
        }
    }
    None
}

fn raw_branch_rebind_exprs_equivalent_local(prev_rhs: &str, rhs: &str) -> bool {
    let prev_rhs = strip_outer_parens_local(prev_rhs);
    let rhs = strip_outer_parens_local(rhs);
    if prev_rhs == rhs {
        return true;
    }
    raw_vec_fill_signature_local(prev_rhs)
        .zip(raw_vec_fill_signature_local(rhs))
        .is_some_and(|(lhs_sig, rhs_sig)| lhs_sig == rhs_sig)
}

fn is_inlineable_named_scalar_expr_local(rhs: &str) -> bool {
    let rhs = strip_outer_parens_local(rhs);
    if rhs.is_empty()
        || rhs.contains('"')
        || rhs.contains(',')
        || rhs.contains("function(")
        || rhs.contains("function (")
    {
        return false;
    }
    true
}

fn enclosing_branch_start_local(lines: &[String], idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for prev_idx in (0..idx).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "}" {
            depth += 1;
            continue;
        }
        if trimmed.ends_with('{') {
            if depth == 0 {
                return (trimmed.starts_with("if ") || trimmed.starts_with("if("))
                    .then_some(prev_idx);
            }
            depth = depth.saturating_sub(1);
        }
    }
    None
}

fn find_block_end_local(lines: &[String], start_idx: usize) -> Option<usize> {
    let mut depth = 0isize;
    let mut saw_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    saw_open = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if saw_open && depth <= 0 {
            return Some(idx);
        }
    }
    None
}

fn raw_is_loop_open_boundary_local(trimmed: &str) -> bool {
    trimmed == "repeat {"
        || trimmed.starts_with("while ")
        || trimmed.starts_with("while(")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("for(")
}

fn is_control_flow_boundary_local(trimmed: &str) -> bool {
    let is_single_line_guard =
        trimmed.starts_with("if ") && (trimmed.ends_with(" break") || trimmed.ends_with(" next"));
    trimmed == "{"
        || trimmed == "}"
        || trimmed == "repeat {"
        || (trimmed.starts_with("if ") && !is_single_line_guard)
        || trimmed.starts_with("if(")
        || trimmed.starts_with("else")
        || trimmed.starts_with("} else")
        || trimmed.starts_with("while")
        || trimmed.starts_with("for")
        || trimmed == "break"
        || trimmed == "next"
}

fn raw_line_is_within_loop_body_local(lines: &[String], idx: usize) -> bool {
    (0..idx).rev().any(|start_idx| {
        if !raw_is_loop_open_boundary_local(lines[start_idx].trim()) {
            return false;
        }
        find_block_end_local(lines, start_idx).is_some_and(|end_idx| idx < end_idx)
    })
}

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

pub(super) fn rewrite_branch_local_identical_alloc_rebinds(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let rhs_canonical = strip_outer_parens_local(rhs);
        if !is_raw_branch_rebind_candidate_local(rhs_canonical) {
            continue;
        }
        let Some(branch_start) = enclosing_branch_start_local(&lines, idx) else {
            continue;
        };
        if branch_body_writes_symbol_before_local(&lines, branch_start + 1, idx, lhs) {
            continue;
        }
        let prev_assign = if raw_vec_fill_signature_local(rhs_canonical).is_some() {
            previous_outer_assign_before_branch_local(&lines, branch_start, lhs, true)
        } else if is_raw_alloc_like_expr_local(rhs_canonical) {
            previous_outer_assign_before_branch_local(&lines, branch_start, lhs, false)
        } else {
            previous_outer_assign_before_branch_local(&lines, branch_start, lhs, true)
        };
        let Some((prev_lhs, prev_rhs)) = prev_assign else {
            continue;
        };
        if prev_lhs == lhs && raw_branch_rebind_exprs_equivalent_local(prev_rhs, rhs_canonical) {
            lines[idx].clear();
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_immediate_and_guard_named_scalar_exprs(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if raw_expr_idents_local(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_named_scalar_expr_local(&rhs)
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let replacement = format!("({})", strip_outer_parens_local(&rhs));

        if let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) {
            let next_trimmed = lines[next_idx].trim().to_string();
            let next_is_assign = parse_local_assign_line(next_trimmed.as_str()).is_some();
            let next_is_return =
                next_trimmed.starts_with("return(") || next_trimmed.starts_with("return (");
            if !lines[next_idx].contains("<- function")
                && (next_is_assign || next_is_return)
                && count_symbol_occurrences_local(&next_trimmed, &lhs) > 0
            {
                let mut used_after = false;
                for later_line in lines.iter().skip(next_idx + 1) {
                    let later_trimmed = later_line.trim();
                    if later_trimmed.is_empty() {
                        continue;
                    }
                    if later_line.contains("<- function") {
                        break;
                    }
                    if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                        && later_lhs == lhs
                    {
                        used_after = true;
                        break;
                    }
                    if count_symbol_occurrences_local(later_trimmed, &lhs) > 0 {
                        used_after = true;
                        break;
                    }
                }
                if !used_after {
                    lines[next_idx] = replace_symbol_occurrences_local(
                        &lines[next_idx],
                        &lhs,
                        replacement.as_str(),
                    );
                    lines[idx].clear();
                    continue;
                }
            }
        }

        let Some(use_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && count_symbol_occurrences_local(trimmed, &lhs) > 0
        }) else {
            continue;
        };
        let use_trimmed = lines[use_idx].trim().to_string();
        let use_is_guard = use_trimmed.starts_with("if (") || use_trimmed.starts_with("if(");
        let use_occurrences = count_symbol_occurrences_local(&use_trimmed, &lhs);
        if lines[use_idx].contains("<- function")
            || !use_is_guard
            || use_occurrences == 0
            || use_occurrences > 2
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(use_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if count_symbol_occurrences_local(later_trimmed, &lhs) > 0 {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[use_idx] =
            replace_symbol_occurrences_local(&lines[use_idx], &lhs, replacement.as_str());
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_two_use_named_scalar_exprs(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if raw_expr_idents_local(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_named_scalar_expr_local(&rhs)
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents_local(rhs.as_str());
        let Some(next1_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let Some(next2_idx) = ((next1_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let next1_trimmed = lines[next1_idx].trim().to_string();
        let next2_trimmed = lines[next2_idx].trim().to_string();
        if lines[next1_idx].contains("<- function")
            || lines[next2_idx].contains("<- function")
            || parse_local_assign_line(next1_trimmed.as_str()).is_none()
            || parse_local_assign_line(next2_trimmed.as_str()).is_none()
            || count_symbol_occurrences_local(&next1_trimmed, &lhs) != 1
            || count_symbol_occurrences_local(&next2_trimmed, &lhs) != 1
        {
            continue;
        }

        let mut total_uses = 0usize;
        let mut use_line_idxs = Vec::new();
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_local_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences_local(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 2 {
                    break;
                }
            }
        }
        if total_uses != 2 || use_line_idxs != vec![next1_idx, next2_idx] {
            continue;
        }
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < next2_idx) {
            continue;
        }

        let replacement = strip_outer_parens_local(&rhs).to_string();
        lines[next1_idx] =
            replace_symbol_occurrences_local(&lines[next1_idx], &lhs, replacement.as_str());
        lines[next2_idx] =
            replace_symbol_occurrences_local(&lines[next2_idx], &lhs, replacement.as_str());
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_small_multiuse_scalar_index_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".phi_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_scalar_index_rhs_local(&rhs)
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let rhs_canonical = strip_outer_parens_local(&rhs).to_string();
        let rhs_deps = raw_expr_idents_local(rhs_canonical.as_str());

        let mut scan_start = idx + 1;
        while let Some(alias_idx) = (scan_start..lines.len()).find(|i| !lines[*i].trim().is_empty())
        {
            let trimmed = lines[alias_idx].trim();
            let Some((alias_lhs, alias_rhs)) = parse_local_assign_line(trimmed) else {
                break;
            };
            if alias_lhs.starts_with(".arg_")
                || alias_lhs.starts_with(".__rr_cse_")
                || alias_lhs.starts_with(".tachyon_")
                || !is_inlineable_scalar_index_rhs_local(alias_rhs)
            {
                break;
            }
            scan_start = alias_idx + 1;
        }

        let Some(next1_idx) = (scan_start..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let Some(next2_idx) = ((next1_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let next1_trimmed = lines[next1_idx].trim().to_string();
        let next2_trimmed = lines[next2_idx].trim().to_string();
        if lines[next1_idx].contains("<- function")
            || lines[next2_idx].contains("<- function")
            || parse_local_assign_line(next1_trimmed.as_str()).is_none()
            || parse_local_assign_line(next2_trimmed.as_str()).is_none()
        {
            continue;
        }

        let mut use_line_idxs = Vec::new();
        let mut total_uses = 0usize;
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, later_rhs)) = parse_local_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    if count_symbol_occurrences_local(later_rhs, &lhs) > 0 {
                        total_uses = usize::MAX;
                    }
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences_local(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 6 {
                    break;
                }
            }
        }
        if total_uses == 0 || total_uses > 6 {
            continue;
        }
        if use_line_idxs
            .iter()
            .any(|line_no| *line_no != next1_idx && *line_no != next2_idx)
        {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }

        lines[next1_idx] =
            replace_symbol_occurrences_local(&lines[next1_idx], &lhs, rhs_canonical.as_str());
        lines[next2_idx] =
            replace_symbol_occurrences_local(&lines[next2_idx], &lhs, rhs_canonical.as_str());
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_one_or_two_use_named_scalar_index_reads_in_straight_line_region(
    output: &mut String,
) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_scalar_index_rhs_local(&rhs)
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let rhs_canonical = strip_outer_parens_local(&rhs).to_string();
        let rhs_deps = raw_expr_idents_local(rhs_canonical.as_str());
        if rhs_deps.iter().any(|ident| ident == &lhs) {
            continue;
        }

        let next_def = ((idx + 1)..lines.len())
            .find(|line_idx| {
                parse_local_assign_line(lines[*line_idx].trim())
                    .is_some_and(|(later_lhs, _)| later_lhs == lhs)
            })
            .unwrap_or(lines.len());
        let region_end = straight_line_region_end_local(&lines, idx).min(next_def);
        if region_end <= idx + 1 {
            continue;
        }

        let mut use_line_idxs = Vec::new();
        let mut valid = true;
        for line_no in idx + 1..region_end {
            let line_trimmed = lines[line_no].trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if let Some((later_lhs, _)) = parse_local_assign_line(line_trimmed)
                && rhs_deps.iter().any(|dep| dep == later_lhs)
            {
                valid = false;
                break;
            }
            let occurrences = count_symbol_occurrences_local(line_trimmed, &lhs);
            if occurrences > 1 {
                valid = false;
                break;
            }
            if occurrences == 1 {
                use_line_idxs.push(line_no);
                if use_line_idxs.len() > 2 {
                    valid = false;
                    break;
                }
            }
        }
        if !valid || use_line_idxs.is_empty() {
            continue;
        }

        for use_idx in use_line_idxs {
            lines[use_idx] =
                replace_symbol_occurrences_local(&lines[use_idx], &lhs, rhs_canonical.as_str());
        }
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
