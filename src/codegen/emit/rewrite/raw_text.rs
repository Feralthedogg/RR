fn parse_local_assign_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (lhs, rhs) = trimmed.split_once(" <- ")?;
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    if lhs.is_empty() || !lhs.chars().all(RBackend::is_symbol_char) {
        return None;
    }
    Some((lhs, rhs))
}

fn assign_slice_re_local() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^rr_assign_slice\((?P<dest>{}),\s*(?P<start>.+?),\s*(?P<end>.+?),\s*(?P<rest>.+)\)$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn plain_ident_re_local() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(format!(r"^{}$", IDENT_PATTERN)))
        .as_ref()
}

fn literal_one_re_local() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^(?:1|1L|1l|1\.0)$".to_string()))
        .as_ref()
}

fn literal_positive_re_local() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(r"^(?:[1-9][0-9]*|[1-9][0-9]*L|[1-9][0-9]*l|[1-9][0-9]*\.0)$".to_string())
    })
    .as_ref()
}

fn strip_outer_parens_local(expr: &str) -> &str {
    let mut expr = expr.trim();
    loop {
        if !(expr.starts_with('(') && expr.ends_with(')')) {
            break;
        }
        let mut depth = 0i32;
        let mut wraps = true;
        for (idx, ch) in expr.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && idx + ch.len_utf8() < expr.len() {
                        wraps = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !wraps {
            break;
        }
        expr = expr[1..expr.len() - 1].trim();
    }
    expr
}

fn is_inlineable_scalar_index_rhs_local(rhs: &str) -> bool {
    let trimmed = strip_outer_parens_local(rhs);
    let Some(open) = trimmed.find('[') else {
        return false;
    };
    let mut depth = 0i32;
    let mut close = None;
    for (idx, ch) in trimmed.char_indices().skip(open) {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(close) = close else {
        return false;
    };
    if close + 1 != trimmed.len() || open == 0 || close <= open + 1 {
        return false;
    }
    let base = trimmed[..open].trim();
    base.chars().all(RBackend::is_symbol_char)
}

fn straight_line_region_end_local(lines: &[String], start_idx: usize) -> usize {
    for line_idx in start_idx + 1..lines.len() {
        let trimmed = lines[line_idx].trim();
        if lines[line_idx].contains("<- function")
            || (!trimmed.is_empty() && is_control_flow_boundary_local(trimmed))
        {
            return line_idx;
        }
    }
    lines.len()
}

fn is_branch_hoistable_named_scalar_rhs_local(rhs: &str) -> bool {
    let rhs = strip_outer_parens_local(rhs);
    is_inlineable_scalar_index_rhs_local(rhs)
        || rhs.starts_with("rr_wrap_index_vec_i(")
        || rhs.starts_with("rr_idx_cube_vec_i(")
}

fn raw_expr_idents_local(expr: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = None;
    for (idx, ch) in expr.char_indices() {
        if RBackend::is_symbol_char(ch) {
            if start.is_none() {
                start = Some(idx);
            }
        } else if let Some(begin) = start.take() {
            out.push(expr[begin..idx].to_string());
        }
    }
    if let Some(begin) = start {
        out.push(expr[begin..].to_string());
    }
    out
}

fn replace_symbol_occurrences_local(line: &str, symbol: &str, replacement: &str) -> String {
    if line.is_empty() || symbol.is_empty() || !line.contains(symbol) {
        return line.to_string();
    }
    let bytes = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut idx = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' if !in_double => {
                in_single = !in_single;
                out.push('\'');
                idx += 1;
                continue;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                out.push('"');
                idx += 1;
                continue;
            }
            _ => {}
        }
        if !in_single
            && !in_double
            && line[idx..].starts_with(symbol)
            && line[..idx]
                .chars()
                .next_back()
                .is_none_or(|ch| !RBackend::is_symbol_char(ch))
            && line[idx + symbol.len()..]
                .chars()
                .next()
                .is_none_or(|ch| !RBackend::is_symbol_char(ch))
        {
            out.push_str(replacement);
            idx += symbol.len();
            continue;
        }
        let Some(ch) = line[idx..].chars().next() else {
            break;
        };
        out.push(ch);
        idx += ch.len_utf8();
    }
    out
}

fn unquoted_sym_refs_local(line: &str) -> Vec<String> {
    raw_expr_idents_local(line)
        .into_iter()
        .filter(|ident| ident.starts_with("Sym_"))
        .collect()
}

#[derive(Clone, Debug)]
struct LocalFunctionSpan {
    name: String,
    start: usize,
    end: usize,
}

fn local_function_spans(lines: &[String]) -> Vec<LocalFunctionSpan> {
    let mut funcs = Vec::new();
    let scope_end = lines.len().saturating_sub(1);
    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        let Some((name, _)) = trimmed.split_once(" <- function(") else {
            idx += 1;
            continue;
        };
        let open_idx = idx + 1;
        if open_idx >= lines.len() || lines[open_idx].trim() != "{" {
            idx += 1;
            continue;
        }
        let Some(end) = RBackend::block_end_for_open_brace(lines, open_idx, scope_end) else {
            idx += 1;
            continue;
        };
        funcs.push(LocalFunctionSpan {
            name: name.trim().to_string(),
            start: idx,
            end,
        });
        idx = end + 1;
    }
    funcs
}

pub(super) fn strip_unreachable_sym_helpers(output: &mut String) {
    let lines: Vec<String> = output.lines().map(str::to_string).collect();
    if lines.is_empty() {
        return;
    }

    let funcs = local_function_spans(&lines);
    let sym_funcs: FxHashMap<String, LocalFunctionSpan> = funcs
        .iter()
        .filter(|func| func.name.starts_with("Sym_"))
        .map(|func| (func.name.clone(), func.clone()))
        .collect();
    if sym_funcs.len() <= 1 {
        return;
    }

    let mut in_function = vec![false; lines.len()];
    for func in &funcs {
        for idx in func.start..=func.end {
            if idx < in_function.len() {
                in_function[idx] = true;
            }
        }
    }

    let sym_top_is_empty_entrypoint = |func: &LocalFunctionSpan| {
        let mut saw_return_null = false;
        for line in lines.iter().take(func.end + 1).skip(func.start + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            if trimmed == "return(NULL)" {
                saw_return_null = true;
                continue;
            }
            if !unquoted_sym_refs_local(trimmed).is_empty() {
                return false;
            }
            return false;
        }
        saw_return_null
    };

    let mut roots = FxHashSet::default();
    if sym_funcs.contains_key("Sym_top_0") {
        roots.insert("Sym_top_0".to_string());
    }
    for (idx, line) in lines.iter().enumerate() {
        if in_function[idx] {
            continue;
        }
        for name in unquoted_sym_refs_local(line) {
            if sym_funcs.contains_key(&name) {
                roots.insert(name);
            }
        }
    }
    if roots.is_empty() {
        return;
    }
    if roots.len() == 1
        && roots.contains("Sym_top_0")
        && sym_funcs
            .get("Sym_top_0")
            .is_some_and(sym_top_is_empty_entrypoint)
    {
        return;
    }

    let mut reachable = roots.clone();
    let mut work: Vec<String> = roots.into_iter().collect();
    while let Some(name) = work.pop() {
        let Some(func) = sym_funcs.get(&name) else {
            continue;
        };
        for line in lines.iter().take(func.end + 1).skip(func.start + 1) {
            for callee in unquoted_sym_refs_local(line) {
                if sym_funcs.contains_key(&callee) && reachable.insert(callee.clone()) {
                    work.push(callee);
                }
            }
        }
    }

    let mut kept = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some(func) = funcs.iter().find(|func| func.start == idx) {
            if !func.name.starts_with("Sym_") || reachable.contains(&func.name) {
                kept.extend(lines.iter().take(func.end + 1).skip(func.start).cloned());
            }
            idx = func.end + 1;
            continue;
        }
        kept.push(lines[idx].clone());
        idx += 1;
    }

    let mut rewritten = kept.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

fn has_assignment_to_one_before_local(lines: &[String], idx: usize, var: &str) -> bool {
    (0..idx).rev().any(|i| {
        parse_local_assign_line(&lines[i]).is_some_and(|(lhs, rhs)| {
            lhs == var && literal_one_re_local().is_some_and(|re| re.is_match(rhs))
        })
    })
}

fn function_has_matching_exprmap_whole_assign_local(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    if !temp_var.starts_with(".tachyon_exprmap") {
        return false;
    }
    let Some(temp_idx) = lines
        .iter()
        .position(|line| parse_local_assign_line(line).is_some_and(|(lhs, _)| lhs == temp_var))
    else {
        return false;
    };
    let Some((_, temp_rhs)) = parse_local_assign_line(&lines[temp_idx]) else {
        return false;
    };

    for line in lines.iter().skip(temp_idx + 1) {
        let Some((lhs, rhs)) = parse_local_assign_line(line) else {
            continue;
        };
        let Some(slice_caps) = assign_slice_re_local().and_then(|re| re.captures(rhs)) else {
            continue;
        };
        let dest = slice_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = slice_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let rest = slice_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if lhs == dest_var
            && dest == dest_var
            && end == end_expr
            && (rest == temp_rhs || rest == temp_var)
        {
            return true;
        }
    }
    false
}

fn function_has_non_empty_repeat_whole_assign_local(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    let Some(temp_idx) = lines
        .iter()
        .position(|line| parse_local_assign_line(line).is_some_and(|(lhs, _)| lhs == temp_var))
    else {
        return false;
    };
    let Some((_, temp_rhs)) = parse_local_assign_line(&lines[temp_idx]) else {
        return false;
    };

    let mut assign_idx = None;
    for idx in temp_idx + 1..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let Some(slice_caps) = assign_slice_re_local().and_then(|re| re.captures(rhs)) else {
            continue;
        };
        let dest = slice_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let start = slice_caps
            .name("start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = slice_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let rest = slice_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if lhs == dest_var
            && dest == dest_var
            && end == end_expr
            && (rest == temp_rhs || rest == temp_var)
            && plain_ident_re_local().is_some_and(|re| re.is_match(start))
            && has_assignment_to_one_before_local(lines, idx, start)
        {
            assign_idx = Some(idx);
            break;
        }
    }
    let Some(assign_idx) = assign_idx else {
        return false;
    };

    let Some(repeat_idx) = (0..assign_idx)
        .rev()
        .find(|idx| lines[*idx].trim() == "repeat {")
    else {
        return false;
    };
    let Some(guard_idx) = (repeat_idx + 1..assign_idx).find(|idx| {
        lines[*idx].trim().starts_with("if !(") || lines[*idx].trim().starts_with("if (!(")
    }) else {
        return false;
    };
    let guard = lines[guard_idx].trim();
    let Some(inner) = guard
        .strip_prefix("if (!(")
        .and_then(|s| s.strip_suffix(")) break"))
    else {
        return false;
    };
    let Some((iter_var, bound)) = inner.split_once("<=") else {
        return false;
    };
    literal_positive_re_local().is_some_and(|re| re.is_match(bound.trim()))
        && has_assignment_to_one_before_local(lines, guard_idx, iter_var.trim())
}

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

fn count_symbol_occurrences_local(line: &str, symbol: &str) -> usize {
    if line.is_empty() || symbol.is_empty() || !line.contains(symbol) {
        return 0;
    }
    let bytes = line.as_bytes();
    let mut idx = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut count = 0usize;
    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' if !in_double => {
                in_single = !in_single;
                idx += 1;
                continue;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                idx += 1;
                continue;
            }
            _ => {}
        }
        if !in_single
            && !in_double
            && line[idx..].starts_with(symbol)
            && line[..idx]
                .chars()
                .next_back()
                .is_none_or(|ch| !RBackend::is_symbol_char(ch))
            && line[idx + symbol.len()..]
                .chars()
                .next()
                .is_none_or(|ch| !RBackend::is_symbol_char(ch))
        {
            count += 1;
            idx += symbol.len();
            continue;
        }
        let Some(ch) = line[idx..].chars().next() else {
            break;
        };
        idx += ch.len_utf8();
    }
    count
}
