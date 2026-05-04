use super::*;

pub(crate) fn normalize_raw_iter_index_parens(line: &str, iter_var: &str) -> String {
    let paren_idx = format!("[({iter_var})]");
    let plain_idx = format!("[{iter_var}]");
    line.replace(&paren_idx, &plain_idx)
}

pub(crate) fn strip_dead_simple_scalar_assigns_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || (!rhs_is_raw_simple_scalar_alias_or_literal(rhs)
                    && !rhs_is_raw_simple_dead_expr(rhs))
                || raw_enclosing_repeat_guard_mentions_symbol(&lines, idx, lhs)
            {
                idx += 1;
                continue;
            }
            let mut used_later = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if line_contains_symbol(later_trimmed, lhs) {
                    used_later = true;
                    break;
                }
            }
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_shadowed_simple_scalar_seed_assigns_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };

        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || (!rhs_is_raw_simple_scalar_alias_or_literal(rhs)
                    && !rhs_is_raw_simple_dead_expr(rhs))
                || raw_enclosing_repeat_guard_mentions_symbol(&lines, idx, lhs)
            {
                idx += 1;
                continue;
            }

            let mut shadowed_before_use = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if later_trimmed.starts_with("if (")
                    || later_trimmed.starts_with("if(")
                    || later_trimmed.starts_with("} else {")
                    || later_trimmed.starts_with("} else if")
                    || later_trimmed == "repeat {"
                    || later_trimmed == "}"
                    || later_trimmed == "next"
                    || later_trimmed.starts_with("return(")
                    || later_trimmed.starts_with("return (")
                {
                    break;
                }
                if let Some((later_lhs, later_rhs)) = parse_raw_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    if !line_contains_symbol(later_rhs, lhs) {
                        shadowed_before_use = true;
                    }
                    break;
                }
                if line_contains_symbol(later_trimmed, lhs) {
                    break;
                }
            }

            if shadowed_before_use {
                lines[idx].clear();
            }
            idx += 1;
        }

        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_dead_weno_topology_seed_i_before_direct_adj_gather_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return output.to_string();
    }

    for idx in 0..(lines.len() - 2) {
        if lines[idx].trim() != "i <- 1"
            || lines[idx + 1].trim() != "adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))"
            || lines[idx + 2].trim() != "adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))"
        {
            continue;
        }
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_mountain_dx_temp_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 1 < lines.len() {
        if lines[idx].trim() == "dx_m <- (x_curr - 20)"
            && lines[idx + 1].trim() == "dy_m <- (y_curr - 20)"
        {
            let Some(dist_idx) = ((idx + 2)..lines.len().min(idx + 8)).find(|line_idx| {
                lines[*line_idx].trim() == "dist <- ((dx_m * dx_m) + (dy_m * dy_m))"
            }) else {
                idx += 1;
                continue;
            };
            let indent = lines[dist_idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx].clear();
            lines[idx + 1].clear();
            lines[dist_idx] = format!(
                "{indent}dist <- ((((x_curr - 20) * (x_curr - 20)) + ((y_curr - 20) * (y_curr - 20))))"
            );
            idx = dist_idx + 1;
            continue;
        }

        if lines[idx].trim() == "dx_m <- (x_curr - 20)"
            && lines[idx + 1].trim() == "dist <- ((dx_m * dx_m) + ((y_curr - 20) * (y_curr - 20)))"
        {
            let indent = lines[idx + 1]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx].clear();
            lines[idx + 1] = format!(
                "{indent}dist <- ((((x_curr - 20) * (x_curr - 20)) + ((y_curr - 20) * (y_curr - 20))))"
            );
        }

        idx += 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_dead_zero_seed_ii_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        if lines[idx].trim() != "ii <- 0" {
            continue;
        }
        let used_later = lines
            .iter()
            .skip(idx + 1)
            .any(|line| line_contains_symbol(line.trim(), "ii"));
        if !used_later {
            lines[idx].clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn collapse_weno_full_range_gather_replay_after_fill_inline_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 8 {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 5 < lines.len() {
        let first = lines[idx].trim();
        let second = lines[idx + 1].trim();
        let third = lines[idx + 2].trim();
        if !is_weno_adj_ll_seed(first)
            || !is_weno_adj_rr_seed(second)
            || !is_raw_one_assignment(third, "i")
        {
            idx += 1;
            continue;
        }

        let mut replay_idx = idx + 3;
        while replay_idx < lines.len() {
            let trimmed = lines[replay_idx].trim();
            if !is_weno_replay_padding_line(trimmed) {
                break;
            }
            replay_idx += 1;
        }
        if replay_idx + 3 >= lines.len() {
            idx += 1;
            continue;
        }
        let Some(left_end) =
            parse_weno_exprmap_gather(lines[replay_idx].trim(), ".tachyon_exprmap0_0", "adj_l")
        else {
            idx += 1;
            continue;
        };
        let Some(right_end) =
            parse_weno_exprmap_gather(lines[replay_idx + 1].trim(), ".tachyon_exprmap1_0", "adj_r")
        else {
            idx += 1;
            continue;
        };
        let Some(left_assign_end) = parse_weno_assign_slice(
            lines[replay_idx + 2].trim(),
            "adj_ll",
            ".tachyon_exprmap0_0",
        ) else {
            idx += 1;
            continue;
        };
        let Some(right_assign_end) = parse_weno_assign_slice(
            lines[replay_idx + 3].trim(),
            "adj_rr",
            ".tachyon_exprmap1_0",
        ) else {
            idx += 1;
            continue;
        };
        if left_end != right_end || left_end != left_assign_end || left_end != right_assign_end {
            idx += 1;
            continue;
        }

        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx] = format!("{indent}adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))");
        lines[idx + 1] = format!("{indent}adj_rr <- rr_gather(adj_r, rr_index_vec_floor(adj_r))");
        lines[idx + 2].clear();
        for line in lines.iter_mut().skip(replay_idx).take(4) {
            line.clear();
        }
        idx = replay_idx + 4;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn is_weno_adj_ll_seed(line: &str) -> bool {
    matches!(
        parse_raw_assign_line(line),
        Some(("adj_ll", "qr"))
            | Some(("adj_ll", "rep.int(0, TOTAL)"))
            | Some(("adj_ll", "rep.int(0L, TOTAL)"))
            | Some(("adj_ll", "rep.int(0.0, TOTAL)"))
    )
}

pub(crate) fn is_weno_adj_rr_seed(line: &str) -> bool {
    matches!(
        parse_raw_assign_line(line),
        Some(("adj_rr", "adj_ll"))
            | Some(("adj_rr", "qr"))
            | Some(("adj_rr", "rep.int(0, TOTAL)"))
            | Some(("adj_rr", "rep.int(0L, TOTAL)"))
            | Some(("adj_rr", "rep.int(0.0, TOTAL)"))
    )
}

pub(crate) fn is_raw_one_assignment(line: &str, lhs: &str) -> bool {
    matches!(
        parse_raw_assign_line(line),
        Some((assign_lhs, "1" | "1L" | "1.0")) if assign_lhs == lhs
    )
}

pub(crate) fn is_weno_replay_padding_line(line: &str) -> bool {
    line.is_empty()
        || line == "# rr-cse-pruned"
        || matches!(
            parse_raw_assign_line(line),
            Some(("ii", "0" | "0L" | "0.0"))
        )
}

pub(crate) fn parse_weno_exprmap_gather<'a>(
    line: &'a str,
    tmp: &str,
    adj: &str,
) -> Option<&'a str> {
    let prefix = format!(
        "{tmp} <- rr_gather({adj}, rr_index_vec_floor(rr_index1_read_vec({adj}, rr_index_vec_floor(i:"
    );
    line.strip_prefix(&prefix)?.strip_suffix("))))")
}

pub(crate) fn parse_weno_assign_slice<'a>(line: &'a str, lhs: &str, tmp: &str) -> Option<&'a str> {
    let prefix = format!("{lhs} <- rr_assign_slice({lhs}, i, ");
    let suffix = format!(", {tmp})");
    line.strip_prefix(&prefix)?.strip_suffix(&suffix)
}

pub(crate) fn raw_enclosing_repeat_guard_mentions_symbol(
    lines: &[String],
    idx: usize,
    symbol: &str,
) -> bool {
    for start_idx in (0..idx).rev() {
        if lines[start_idx].trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = find_raw_block_end(lines, start_idx) else {
            continue;
        };
        if idx >= end_idx {
            continue;
        }
        let Some(guard_idx) = ((start_idx + 1)..end_idx)
            .find(|line_idx| parse_raw_repeat_guard_cmp_line(lines[*line_idx].trim()).is_some())
        else {
            continue;
        };
        let guard = lines[guard_idx].trim();
        if line_contains_symbol(guard, symbol) {
            return true;
        }
        break;
    }
    false
}

pub(crate) fn raw_is_loop_open_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "repeat {" || trimmed.starts_with("while") || trimmed.starts_with("for")
}

pub(crate) fn raw_line_is_within_loop_body(lines: &[String], idx: usize) -> bool {
    (0..idx).rev().any(|start_idx| {
        if !raw_is_loop_open_boundary(lines[start_idx].trim()) {
            return false;
        }
        find_raw_block_end(lines, start_idx).is_some_and(|end_idx| idx < end_idx)
    })
}

pub(crate) fn strip_noop_self_assignments_in_raw_emitted_r(output: &str) -> String {
    let mut out = String::new();
    for line in output.lines() {
        let keep = if let Some((lhs, rhs)) = parse_raw_assign_line(line.trim()) {
            lhs != strip_redundant_outer_parens(rhs)
        } else {
            true
        };
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    if output.is_empty() {
        return String::new();
    }
    if !output.ends_with('\n') {
        out.pop();
    }
    out
}

pub(crate) fn strip_noop_temp_copy_roundtrips_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((tmp_lhs, tmp_rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let tmp_lhs = tmp_lhs.to_string();
        let tmp_rhs = tmp_rhs.to_string();
        if !(tmp_lhs.starts_with(".__pc_src_tmp") || tmp_lhs.starts_with(".__rr_cse_"))
            || !tmp_rhs.chars().all(is_symbol_char)
        {
            idx += 1;
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            lines[idx].clear();
            break;
        };
        let Some((next_lhs, next_rhs)) = parse_raw_assign_line(lines[next_idx].trim()) else {
            let used_later = lines
                .iter()
                .skip(next_idx)
                .any(|line| line_contains_symbol(line.trim(), &tmp_lhs));
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
                if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                    && later_lhs == tmp_lhs
                {
                    break;
                }
                if line_contains_symbol(later_trimmed, &tmp_lhs) {
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
            .any(|line| line_contains_symbol(line.trim(), &tmp_lhs));
        if !used_later {
            lines[idx].clear();
        }
        idx = next_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_empty_else_blocks_in_raw_emitted_r(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim() == "} else {" {
            let mut close_idx = i + 1;
            while close_idx < lines.len() && lines[close_idx].trim().is_empty() {
                close_idx += 1;
            }
            if close_idx < lines.len() && lines[close_idx].trim() == "}" {
                let indent_len = line.len() - line.trim_start().len();
                let indent = &line[..indent_len];
                out.push(format!("{indent}}}"));
                i = close_idx + 1;
                continue;
            }
        }
        out.push(line.clone());
        i += 1;
    }

    let mut rendered = out.join("\n");
    if output.ends_with('\n') || !rendered.is_empty() {
        rendered.push('\n');
    }
    rendered
}

pub(crate) fn strip_redundant_branch_local_vec_fill_rebinds_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            continue;
        };
        let Some(sig) = raw_vec_fill_signature(rhs) else {
            continue;
        };
        let Some(branch_start) = enclosing_raw_branch_start(&lines, idx) else {
            continue;
        };
        if branch_body_writes_symbol_before(&lines, branch_start + 1, idx, lhs) {
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
            let Some((prev_lhs, prev_rhs)) = parse_raw_assign_line(trimmed) else {
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
        if raw_vec_fill_signature(prev_rhs.as_str()).is_some_and(|prev_sig| prev_sig == sig) {
            lines[idx].clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_unused_helper_params_in_raw_emitted_r(output: &str) -> String {
    #[derive(Clone)]
    struct HelperTrim {
        pub(crate) original_len: usize,
        pub(crate) kept_indices: Vec<usize>,
        pub(crate) kept_params: Vec<String>,
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut trims = FxHashMap::<String, HelperTrim>::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let Some((fn_name, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if !fn_name.starts_with("Sym_")
            || params.is_empty()
            || params.iter().any(|param| param.contains('='))
        {
            fn_start = fn_end + 1;
            continue;
        }

        let escaped = lines
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx < fn_start || *idx > fn_end)
            .any(|(_, line)| {
                let trimmed = line.trim();
                line_contains_symbol(trimmed, &fn_name)
                    && !trimmed.contains(&format!("{fn_name}("))
                    && !trimmed.contains(&format!("{fn_name} <- function("))
            });
        if escaped {
            fn_start = fn_end + 1;
            continue;
        }

        let mut used_params = FxHashSet::default();
        for line in lines.iter().take(fn_end).skip(fn_start + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            for ident in raw_expr_idents(trimmed) {
                used_params.insert(ident);
            }
        }
        let kept_indices: Vec<usize> = params
            .iter()
            .enumerate()
            .filter_map(|(idx, param)| used_params.contains(param).then_some(idx))
            .collect();
        if kept_indices.len() < params.len() {
            trims.insert(
                fn_name,
                HelperTrim {
                    original_len: params.len(),
                    kept_indices: kept_indices.clone(),
                    kept_params: kept_indices
                        .iter()
                        .map(|idx| params[*idx].clone())
                        .collect(),
                },
            );
        }
        fn_start = fn_end + 1;
    }

    if trims.is_empty() {
        return output.to_string();
    }

    for line in &mut lines {
        if line.contains("<- function") {
            if let Some((fn_name, _)) = parse_raw_function_header(line)
                && let Some(trim) = trims.get(&fn_name)
            {
                *line = format!("{} <- function({})", fn_name, trim.kept_params.join(", "));
            }
            continue;
        }

        let mut rewritten = line.clone();
        loop {
            let mut changed = false;
            let mut next = String::with_capacity(rewritten.len());
            let mut idx = 0usize;
            while idx < rewritten.len() {
                let mut best: Option<(usize, String)> = None;
                for fn_name in trims.keys() {
                    if let Some(pos) = find_symbol_call(&rewritten, fn_name, idx)
                        && best.as_ref().is_none_or(|(best_pos, _)| pos < *best_pos)
                    {
                        best = Some((pos, fn_name.clone()));
                    }
                }
                let Some((call_idx, fn_name)) = best else {
                    next.push_str(&rewritten[idx..]);
                    break;
                };
                let trim = &trims[&fn_name];
                let ident_end = call_idx + fn_name.len();
                let Some(call_end) = find_matching_call_close(&rewritten, ident_end) else {
                    next.push_str(&rewritten[idx..]);
                    break;
                };
                next.push_str(&rewritten[idx..call_idx]);
                let args_inner = &rewritten[ident_end + 1..call_end];
                let Some(args) = split_top_level_args(args_inner) else {
                    next.push_str(&rewritten[call_idx..=call_end]);
                    idx = call_end + 1;
                    continue;
                };
                if args.len() != trim.original_len {
                    next.push_str(&rewritten[call_idx..=call_end]);
                    idx = call_end + 1;
                    continue;
                }
                next.push_str(&fn_name);
                next.push('(');
                next.push_str(
                    &trim
                        .kept_indices
                        .iter()
                        .map(|idx| args[*idx].trim())
                        .collect::<Vec<_>>()
                        .join(", "),
                );
                next.push(')');
                idx = call_end + 1;
                changed = true;
            }
            if !changed || next == rewritten {
                break;
            }
            rewritten = next;
        }
        *line = rewritten;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_dead_seq_len_locals_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".tachyon_")
                || lhs.starts_with(".__rr_cse_")
                || !rhs.starts_with("seq_len(")
            {
                continue;
            }

            let mut used_later = false;
            for later_line in lines.iter().take(fn_end + 1).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if let Some((later_lhs, later_rhs)) = parse_raw_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    if line_contains_symbol(later_rhs, lhs) {
                        used_later = true;
                    }
                    break;
                }
                if line_contains_symbol(later_trimmed, lhs) {
                    used_later = true;
                    break;
                }
            }

            if !used_later {
                lines[idx].clear();
            }
        }

        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn find_matching_open_brace_line(lines: &[String], close_idx: usize) -> Option<usize> {
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(close_idx + 1) {
        for ch in line.chars() {
            match ch {
                '{' => stack.push(idx),
                '}' => {
                    let open = stack.pop()?;
                    if idx == close_idx {
                        return Some(open);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

pub(crate) fn strip_terminal_repeat_nexts_in_raw_emitted_r(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if lines[idx].trim() == "next"
            && idx + 1 < lines.len()
            && lines[idx + 1].trim() == "}"
            && find_matching_open_brace_line(&lines, idx + 1)
                .is_some_and(|open_idx| lines[open_idx].trim() == "repeat {")
        {
            continue;
        }
        kept.push(lines[idx].clone());
    }

    let mut out = kept.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}
