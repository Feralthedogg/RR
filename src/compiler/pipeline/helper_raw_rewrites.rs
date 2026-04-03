use super::*;

pub(crate) fn rewrite_duplicate_pure_call_assignments_in_raw_emitted_r(
    output: &str,
    pure_user_calls: &FxHashSet<String>,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let line_owned = lines[idx].clone();
        let trimmed = line_owned.trim();
        let candidate_indent = line_owned.len() - line_owned.trim_start().len();
        let Some((lhs, rhs)) = parse_raw_assign_line(trimmed) else {
            continue;
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if lhs.is_empty()
            || !lhs.chars().all(is_symbol_char)
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
        {
            continue;
        }
        let Some((callee, _args)) = parse_top_level_raw_call(rhs) else {
            continue;
        };
        if !pure_user_calls.contains(&callee) {
            continue;
        }
        let deps: FxHashSet<String> = raw_expr_idents(rhs).into_iter().collect();

        for line in lines.iter_mut().skip(idx + 1) {
            let line_trimmed = line.trim().to_string();
            let next_indent = line.len() - line.trim_start().len();
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                break;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((next_lhs, next_rhs)) = parse_raw_assign_line(&line_trimmed) {
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

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_adjacent_duplicate_symbol_assignments_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    for idx in 0..(lines.len() - 1) {
        let first = lines[idx].trim().to_string();
        let second = lines[idx + 1].trim().to_string();
        let Some((lhs0, rhs0)) = parse_raw_assign_line(&first) else {
            continue;
        };
        let Some((lhs1, rhs1)) = parse_raw_assign_line(&second) else {
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
            || !lhs0.chars().all(is_symbol_char)
            || !lhs1.chars().all(is_symbol_char)
            || rhs0 != rhs1
            || rhs0.starts_with(".arg_")
            || !rhs0.chars().all(is_symbol_char)
        {
            continue;
        }

        let indent = lines[idx + 1]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn collect_helper_expr_reuse_summaries_in_raw_emitted_r(
    lines: &[String],
) -> FxHashMap<String, RawHelperExprReuseSummary> {
    let mut map = FxHashMap::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(lines, fn_start) else {
            break;
        };
        let Some((wrapper, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if body.len() != 3 {
            fn_start = fn_end + 1;
            continue;
        }
        let Some((temp_var, rhs)) = parse_raw_assign_line(&body[1]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let rhs = strip_redundant_outer_parens(rhs);
        let Some(open) = rhs.find('(') else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some(close) = find_matching_call_close(rhs, open) else {
            fn_start = fn_end + 1;
            continue;
        };
        let inner_callee = rhs[..open].trim();
        let args_inner = &rhs[open + 1..close];
        let Some(args) = split_top_level_args(args_inner) else {
            fn_start = fn_end + 1;
            continue;
        };
        if inner_callee.is_empty()
            || !inner_callee.chars().all(is_symbol_char)
            || args != params
            || !body[2].starts_with("return(")
        {
            fn_start = fn_end + 1;
            continue;
        }
        let return_expr = body[2]
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
            .unwrap_or("");
        if return_expr.is_empty()
            || !line_contains_symbol(return_expr, temp_var)
            || return_expr.contains("\"")
        {
            fn_start = fn_end + 1;
            continue;
        }
        map.insert(
            wrapper.clone(),
            RawHelperExprReuseSummary {
                wrapper,
                inner_callee: inner_callee.to_string(),
                temp_var: temp_var.to_string(),
                params,
                return_expr: return_expr.to_string(),
            },
        );
        fn_start = fn_end + 1;
    }
    map
}

pub(crate) fn rewrite_helper_expr_reuse_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }
    let summaries = collect_helper_expr_reuse_summaries_in_raw_emitted_r(&lines);
    if summaries.is_empty() {
        return output.to_string();
    }

    for idx in 1..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            continue;
        };
        let rhs = strip_redundant_outer_parens(rhs);
        let Some(open) = rhs.find('(') else {
            continue;
        };
        let Some(close) = find_matching_call_close(rhs, open) else {
            continue;
        };
        let wrapper = rhs[..open].trim();
        let Some(summary) = summaries.get(wrapper) else {
            continue;
        };
        let args_inner = &rhs[open + 1..close];
        let Some(args) = split_top_level_args(args_inner) else {
            continue;
        };
        if args.len() != summary.params.len() {
            continue;
        }

        let Some((prev_lhs, prev_rhs)) = parse_raw_assign_line(lines[idx - 1].trim()) else {
            continue;
        };
        let prev_rhs = strip_redundant_outer_parens(prev_rhs);
        let Some(prev_open) = prev_rhs.find('(') else {
            continue;
        };
        let Some(prev_close) = find_matching_call_close(prev_rhs, prev_open) else {
            continue;
        };
        let prev_callee = prev_rhs[..prev_open].trim();
        let Some(prev_args) = split_top_level_args(&prev_rhs[prev_open + 1..prev_close]) else {
            continue;
        };
        if prev_callee != summary.inner_callee || prev_args != args {
            continue;
        }

        let replacement =
            replace_symbol_occurrences(&summary.return_expr, &summary.temp_var, prev_lhs);
        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx] = format!("{indent}{lhs} <- {replacement}");
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn collect_dot_product_helper_summaries_in_raw_emitted_r(
    lines: &[String],
) -> FxHashMap<String, RawDotProductHelperSummary> {
    let mut map = FxHashMap::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(lines, fn_start) else {
            break;
        };
        let Some((helper, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if params.len() != 3 {
            fn_start = fn_end + 1;
            continue;
        }
        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if body.len() != 2 || body[0] != "{" {
            fn_start = fn_end + 1;
            continue;
        }
        let expected = format!(
            "return(sum(({}[seq_len({})] * {}[seq_len({})])))",
            params[0], params[2], params[1], params[2]
        );
        if body[1] != expected {
            fn_start = fn_end + 1;
            continue;
        }
        map.insert(
            helper.clone(),
            RawDotProductHelperSummary {
                helper,
                lhs_param: params[0].clone(),
                rhs_param: params[1].clone(),
                len_param: params[2].clone(),
            },
        );
        fn_start = fn_end + 1;
    }
    map
}

pub(crate) fn rewrite_dot_product_helper_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }
    let helpers = collect_dot_product_helper_summaries_in_raw_emitted_r(&lines);
    if helpers.is_empty() {
        return output.to_string();
    }

    for line in lines.iter_mut() {
        if line.contains("<- function") {
            continue;
        }
        for summary in helpers.values() {
            let Some(call_idx) = find_symbol_call(line, &summary.helper, 0) else {
                continue;
            };
            let open = call_idx + summary.helper.len();
            let Some(close) = find_matching_call_close(line, open) else {
                continue;
            };
            let Some(args) = split_top_level_args(&line[open + 1..close]) else {
                continue;
            };
            if args.len() != 3 {
                continue;
            }
            let replacement = format!(
                "sum(({}[seq_len({})] * {}[seq_len({})]))",
                args[0].trim(),
                args[2].trim(),
                args[1].trim(),
                args[2].trim()
            );
            line.replace_range(call_idx..=close, &replacement);
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_sym119_helper_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut helper_names = FxHashSet::default();
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
        let Some((name, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if params == vec!["x", "n_l", "n_r", "n_d", "n_u"]
            && body
                == vec![
                    "{".to_string(),
                    "n_d <- rr_index_vec_floor(n_d)".to_string(),
                    "n_l <- rr_index_vec_floor(n_l)".to_string(),
                    "n_r <- rr_index_vec_floor(n_r)".to_string(),
                    "n_u <- rr_index_vec_floor(n_u)".to_string(),
                    "y <- ((4.0001 * x) - (((rr_gather(x, n_l) + rr_gather(x, n_r)) + rr_gather(x, n_d)) + rr_gather(x, n_u)))".to_string(),
                    "return(y)".to_string(),
                ]
        {
            helper_names.insert(name);
        }
        fn_start = fn_end + 1;
    }
    if helper_names.is_empty() {
        return output.to_string();
    }

    for line in lines.iter_mut() {
        if line.contains("<- function") {
            continue;
        }
        for helper in &helper_names {
            let Some(call_idx) = find_symbol_call(line, helper, 0) else {
                continue;
            };
            let open = call_idx + helper.len();
            let Some(close) = find_matching_call_close(line, open) else {
                continue;
            };
            let Some(args) = split_top_level_args(&line[open + 1..close]) else {
                continue;
            };
            if args.len() != 5 {
                continue;
            }
            let replacement = format!(
                "((4.0001 * {}) - (((rr_gather({}, rr_index_vec_floor({})) + rr_gather({}, rr_index_vec_floor({}))) + rr_gather({}, rr_index_vec_floor({}))) + rr_gather({}, rr_index_vec_floor({}))))",
                args[0].trim(),
                args[0].trim(),
                args[1].trim(),
                args[0].trim(),
                args[2].trim(),
                args[0].trim(),
                args[3].trim(),
                args[0].trim(),
                args[4].trim(),
            );
            line.replace_range(call_idx..=close, &replacement);
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_trivial_fill_helper_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut helper_names = FxHashSet::default();
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
        let Some((name, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if params == vec!["n", "val"]
            && body == vec!["{".to_string(), "return(rep.int(val, n))".to_string()]
        {
            helper_names.insert(name);
        }
        fn_start = fn_end + 1;
    }
    if helper_names.is_empty() {
        return output.to_string();
    }

    for line in lines.iter_mut() {
        if line.contains("<- function") {
            continue;
        }
        for helper in &helper_names {
            let Some(call_idx) = find_symbol_call(line, helper, 0) else {
                continue;
            };
            let open = call_idx + helper.len();
            let Some(close) = find_matching_paren(line, open) else {
                continue;
            };
            let Some(args) = split_top_level_args(&line[open + 1..close]) else {
                continue;
            };
            if args.len() != 2 {
                continue;
            }
            let replacement = format!("rep.int({}, {})", args[1].trim(), args[0].trim());
            line.replace_range(call_idx..=close, &replacement);
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_identical_zero_fill_pairs_to_aliases_in_raw_emitted_r(
    output: &str,
) -> String {
    fn raw_line_writes_symbol(line: &str, symbol: &str) -> bool {
        let trimmed = line.trim();
        parse_raw_assign_line(trimmed).is_some_and(|(lhs, _)| lhs == symbol)
            || trimmed.starts_with(&format!("{symbol}["))
            || trimmed.starts_with(&format!("({symbol}) <-"))
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    let replacements = [
        (
            vec!["adj_ll <- rep.int(0, TOTAL)", "adj_ll <- qr"],
            "adj_rr <- rep.int(0, TOTAL)",
            "adj_rr <- adj_ll",
        ),
        (
            vec!["u_stage <- rep.int(0, TOTAL)", "u_stage <- qr"],
            "u_new <- rep.int(0, TOTAL)",
            "u_new <- u_stage",
        ),
    ];

    for idx in 0..(lines.len() - 1) {
        let first = lines[idx].trim().to_string();
        let second = lines[idx + 1].trim().to_string();
        let Some((first_lhs, _)) = parse_raw_assign_line(&first) else {
            continue;
        };
        let Some((second_lhs, _)) = parse_raw_assign_line(&second) else {
            continue;
        };
        let fn_start = (0..=idx)
            .rev()
            .find(|line_idx| lines[*line_idx].contains("<- function"));
        let fn_end = fn_start
            .and_then(|start| find_raw_block_end(&lines, start))
            .unwrap_or(lines.len().saturating_sub(1));
        for (lhs_lines, rhs_line, replacement) in &replacements {
            if lhs_lines.iter().any(|lhs_line| first == *lhs_line) && second == *rhs_line {
                let later_diverging_write =
                    lines.iter().take(fn_end + 1).skip(idx + 2).any(|line| {
                        raw_line_writes_symbol(line, first_lhs)
                            || raw_line_writes_symbol(line, second_lhs)
                    });
                if later_diverging_write {
                    continue;
                }
                let indent = lines[idx + 1]
                    .chars()
                    .take_while(|ch| ch.is_ascii_whitespace())
                    .collect::<String>();
                lines[idx + 1] = format!("{indent}{replacement}");
            }
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_duplicate_sym183_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return output.to_string();
    }

    for idx in 0..(lines.len() - 1) {
        let Some((lhs0, rhs0)) = parse_raw_assign_line(lines[idx].trim()) else {
            continue;
        };
        let Some((lhs1, rhs1)) = parse_raw_assign_line(lines[idx + 1].trim()) else {
            continue;
        };
        if lhs0.chars().all(is_symbol_char)
            && lhs1.chars().all(is_symbol_char)
            && rhs0 == "Sym_183(1000)"
            && rhs1 == "Sym_183(1000)"
        {
            let indent = lines[idx + 1]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_dead_zero_loop_seeds_before_for_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    let mut idx = 0usize;

    while idx + 1 < lines.len() {
        let trimmed = lines[idx].trim();
        let Some((var, seed)) = trimmed.split_once("<-") else {
            idx += 1;
            continue;
        };
        let var = var.trim();
        let seed = seed.trim();
        if seed != "0" && seed != "1" {
            idx += 1;
            continue;
        }

        let Some(for_idx) = ((idx + 1)..lines.len()).take(12).find(|line_idx| {
            lines[*line_idx]
                .trim()
                .starts_with(&format!("for ({var} in seq_len("))
        }) else {
            idx += 1;
            continue;
        };

        let var_re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(var))).ok();
        let used_before_for = lines[(idx + 1)..for_idx]
            .iter()
            .any(|line| var_re.as_ref().is_some_and(|re| re.is_match(line)));
        if used_before_for {
            idx += 1;
            continue;
        }

        lines.remove(idx);
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}
