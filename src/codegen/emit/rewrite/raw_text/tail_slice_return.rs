pub(super) fn strip_redundant_tail_assign_slice_return(output: &mut String) {
    let lines: Vec<String> = output.lines().map(str::to_string).collect();
    if lines.is_empty() || !lines.iter().any(|line| line.contains("rr_assign_slice(")) {
        return;
    }

    let funcs = local_function_spans(&lines);
    if funcs.is_empty() {
        return;
    }

    let mut remove = vec![false; lines.len()];
    for func in funcs {
        let body = &lines[(func.start + 1)..=func.end];
        let Some(return_rel_idx) = body
            .iter()
            .rposition(|line| line.trim().starts_with("return(") && line.trim().ends_with(')'))
        else {
            continue;
        };
        let return_idx = func.start + 1 + return_rel_idx;
        let Some(ret_var) = lines[return_idx]
            .trim()
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let Some(assign_idx) = (func.start + 1..return_idx).rev().find(|idx| {
            let trimmed = lines[*idx].trim();
            !trimmed.is_empty() && trimmed != "{" && trimmed != "}"
        }) else {
            continue;
        };
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[assign_idx]) else {
            continue;
        };
        if lhs != ret_var {
            continue;
        }

        let Some(assign_caps) = assign_slice_re_local().and_then(|re| re.captures(rhs)) else {
            continue;
        };
        let dest = assign_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let start = assign_caps
            .name("start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = assign_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let temp = assign_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if dest != ret_var
            || !literal_one_re_local().is_some_and(|re| re.is_match(start))
            || !plain_ident_re_local().is_some_and(|re| re.is_match(temp))
        {
            continue;
        }

        let mut fn_lines = Vec::new();
        fn_lines.extend(
            lines[func.start..=func.end]
                .iter()
                .filter(|line| line.trim() != "}"),
        );
        let fn_lines: Vec<String> = fn_lines.into_iter().cloned().collect();
        if function_has_non_empty_repeat_whole_assign_local(&fn_lines, ret_var, end, temp)
            || function_has_matching_exprmap_whole_assign_local(&fn_lines, ret_var, end, temp)
        {
            remove[assign_idx] = true;
        }
    }

    if !remove.iter().any(|flag| *flag) {
        return;
    }
    let kept: Vec<String> = lines
        .into_iter()
        .enumerate()
        .filter_map(|(idx, line)| (!remove[idx]).then_some(line))
        .collect();
    let mut rewritten = kept.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
