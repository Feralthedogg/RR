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
