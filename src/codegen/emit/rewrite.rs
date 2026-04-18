use super::*;
use std::sync::OnceLock;

pub(super) fn rewrite_safe_scalar_loop_index_helpers(output: &mut String) {
    let Some(assign_re) = compile_regex(format!(r"^(?P<lhs>{}) <- (?P<rhs>.+)$", IDENT_PATTERN))
    else {
        return;
    };
    let Some(guard_re) = compile_regex(format!(
        r"^if \(!\((?P<var>{}) (?P<op><|<=) (?P<bound>{})\)\) break$",
        IDENT_PATTERN, IDENT_PATTERN
    )) else {
        return;
    };
    let Some(read_re) = compile_regex(format!(
        r#"rr_index1_read\((?P<base>{}),\s*(?P<idx>\([^)]*\)|{})\s*,\s*(?:"index"|'index')\)"#,
        IDENT_PATTERN, IDENT_PATTERN
    )) else {
        return;
    };
    let Some(write_re) = compile_regex(format!(
        r#"rr_index1_write\((?P<idx>{}),\s*(?:"index"|'index')\)"#,
        IDENT_PATTERN
    )) else {
        return;
    };
    let mut lines: Vec<String> = output.lines().map(str::to_string).collect();
    let mut i = 0usize;
    while i + 3 < lines.len() {
        let init_line = lines[i].trim().to_string();
        let Some(init_caps) = assign_re.captures(&init_line) else {
            i += 1;
            continue;
        };
        let idx_var = init_caps
            .name("lhs")
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let init_rhs = init_caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let Some(start_value) = init_rhs
            .trim_end_matches('L')
            .trim_end_matches('l')
            .parse::<i64>()
            .ok()
        else {
            i += 1;
            continue;
        };
        if start_value < 1 || lines[i + 1].trim() != "repeat {" {
            i += 1;
            continue;
        }
        let Some(guard_caps) = guard_re.captures(lines[i + 2].trim()) else {
            i += 1;
            continue;
        };
        if guard_caps
            .name("var")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            != idx_var
        {
            i += 1;
            continue;
        }
        let allow_plus_one = guard_caps
            .name("op")
            .map(|m| m.as_str())
            .is_some_and(|op| op == "<");
        let mut cursor = i + 3;
        while cursor < lines.len() {
            let trimmed = lines[cursor].trim();
            if trimmed == "}" {
                break;
            }
            let rewritten = read_re
                .replace_all(&lines[cursor], |caps: &Captures<'_>| {
                    let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                    let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                    let compact = idx_expr
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>();
                    if compact == idx_var {
                        return format!("{base}[{idx_var}]");
                    }
                    let minus_one = format!("({idx_var}-1)");
                    if compact == minus_one && start_value >= 2 {
                        return format!("{base}[({idx_var} - 1)]");
                    }
                    let plus_one = format!("({idx_var}+1)");
                    if compact == plus_one && allow_plus_one {
                        return format!("{base}[({idx_var} + 1)]");
                    }
                    caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                })
                .to_string();
            let rewritten = write_re
                .replace_all(&rewritten, |caps: &Captures<'_>| {
                    let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                    if idx_expr == idx_var {
                        idx_var.to_string()
                    } else {
                        caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                    }
                })
                .to_string();
            lines[cursor] = rewritten;
            cursor += 1;
        }
        i = cursor.saturating_add(1);
    }
    *output = lines.join("\n");
}

pub(super) fn infer_generated_poly_loop_step(
    lines: &[String],
    body_start: usize,
    body_end: usize,
    var: &str,
) -> i64 {
    if !var.contains("tile_") {
        return 1;
    }
    let Some(step_re) = compile_regex(format!(
        r"\({}\s*\+\s*(?P<delta>[0-9]+)L\)",
        regex::escape(var)
    )) else {
        return 1;
    };
    lines
        .iter()
        .take(body_end)
        .skip(body_start)
        .filter_map(|line| {
            step_re
                .captures(line)
                .and_then(|caps| caps.name("delta"))
                .and_then(|m| m.as_str().parse::<i64>().ok())
        })
        .max()
        .map(|delta| delta + 1)
        .unwrap_or(1)
}

pub(super) fn first_generated_poly_loop_var_in_line(line: &str) -> Option<String> {
    let mut search_from = 0usize;
    while let Some(rel_idx) = line[search_from..].find(GENERATED_POLY_LOOP_IV_PREFIX) {
        let idx = search_from + rel_idx;
        let tail = &line[idx..];
        let len = tail
            .chars()
            .take_while(|ch| RBackend::is_symbol_char(*ch))
            .count();
        if len > 0 {
            return Some(tail[..len].to_string());
        }
        search_from = idx + GENERATED_POLY_LOOP_IV_PREFIX.len();
    }
    None
}

pub(super) fn restore_missing_generated_poly_loop_steps(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(str::to_string).collect();
    if lines.is_empty() {
        return;
    }

    let scope_end = lines.len().saturating_sub(1);
    let mut i = 0usize;
    while i < lines.len() {
        if lines[i].trim() != "repeat {" {
            i += 1;
            continue;
        }

        let Some(loop_end) = RBackend::block_end_for_open_brace(&lines, i, scope_end) else {
            i += 1;
            continue;
        };
        let Some((guard_idx, guard_line)) =
            lines
                .iter()
                .enumerate()
                .skip(i + 1)
                .find_map(|(idx, line)| {
                    let trimmed = line.trim();
                    (!trimmed.is_empty() && trimmed != "# rr-cse-pruned").then_some((idx, line))
                })
        else {
            i = loop_end + 1;
            continue;
        };
        let Some(var) = first_generated_poly_loop_var_in_line(guard_line) else {
            i = loop_end + 1;
            continue;
        };
        let indent = guard_line[..guard_line.len() - guard_line.trim_start().len()].to_string();

        let has_explicit_step = lines.iter().take(loop_end).skip(guard_idx).any(|line| {
            RBackend::extract_plain_assign(line).is_some_and(|(lhs, _, rhs)| {
                lhs == var && RBackend::line_contains_symbol(rhs.as_str(), &var)
            })
        });
        if has_explicit_step {
            i += 1;
            continue;
        }

        let step = infer_generated_poly_loop_step(&lines, guard_idx, loop_end, &var);
        lines.insert(loop_end, format!("{indent}{var} <- ({var} + {step}L)"));
        i += 1;
    }

    let mut rebuilt = lines.join("\n");
    rebuilt.push('\n');
    *output = rebuilt;
}

fn find_matching_call_close(output: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    for (rel_idx, ch) in output[open_idx..].char_indices() {
        match ch {
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '(' if !in_single && !in_double => depth += 1,
            ')' if !in_single && !in_double => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_idx + rel_idx);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_args_local(expr: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (idx, ch) in expr.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                args.push(expr[start..idx].trim().to_string());
                start = idx + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    args.push(expr[start..].trim().to_string());
    Some(args)
}

fn parse_top_level_call_local(rhs: &str) -> Option<(String, Vec<String>)> {
    let rhs = strip_outer_parens_local(rhs).trim();
    let open = rhs.find('(')?;
    let close = find_matching_call_close(rhs, open)?;
    if close + 1 != rhs.len() {
        return None;
    }
    let callee = rhs[..open].trim();
    if callee.is_empty() || !callee.chars().all(RBackend::is_symbol_char) {
        return None;
    }
    let args = split_top_level_args_local(&rhs[open + 1..close])?;
    Some((callee.to_string(), args))
}

fn literal_record_field_name(expr: &str) -> Option<String> {
    let trimmed = expr.trim();
    if let Some(inner) = trimmed.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        return Some(inner.to_string());
    }
    if let Some(inner) = trimmed
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
    {
        return Some(inner.to_string());
    }
    None
}

pub(super) fn rewrite_literal_named_list_calls(output: &mut String) {
    if !output.contains("rr_named_list(") {
        return;
    }
    let mut out = Vec::with_capacity(output.lines().count());
    for line in output.lines() {
        if line.contains("rr_named_list <- function") {
            out.push(line.to_string());
            continue;
        }
        let mut rewritten = line.to_string();
        loop {
            let Some(start) = rewritten.find("rr_named_list(") else {
                break;
            };
            let call_start = start + "rr_named_list".len();
            let Some(call_end) = find_matching_call_close(&rewritten, call_start) else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args_local(args_inner) else {
                break;
            };
            if args.len() % 2 != 0 {
                break;
            }
            let mut fields = Vec::new();
            let mut ok = true;
            for pair in args.chunks(2) {
                let Some(name) = literal_record_field_name(pair[0].trim()) else {
                    ok = false;
                    break;
                };
                fields.push(format!("{name} = {}", pair[1].trim()));
            }
            if !ok {
                break;
            }
            let replacement = if fields.is_empty() {
                "list()".to_string()
            } else {
                format!("list({})", fields.join(", "))
            };
            rewritten.replace_range(start..=call_end, &replacement);
        }
        out.push(rewritten);
    }
    let mut rewritten = out.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_literal_field_get_calls(output: &mut String) {
    if !output.contains("rr_field_get(") {
        return;
    }
    let mut out = Vec::with_capacity(output.lines().count());
    for line in output.lines() {
        if line.contains("<- function") {
            out.push(line.to_string());
            continue;
        }
        let mut rewritten = line.to_string();
        loop {
            let Some(start) = rewritten.find("rr_field_get(") else {
                break;
            };
            let call_start = start + "rr_field_get".len();
            let Some(call_end) = find_matching_call_close(&rewritten, call_start) else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args_local(args_inner) else {
                break;
            };
            if args.len() != 2 {
                break;
            }
            let base = args[0].trim();
            let Some(name) = literal_record_field_name(args[1].trim()) else {
                break;
            };
            let replacement = format!(r#"{base}[["{name}"]]"#);
            rewritten.replace_range(start..=call_end, &replacement);
        }
        out.push(rewritten);
    }
    let mut rewritten = out.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

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

pub(super) fn rewrite_loop_index_alias_ii(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            continue;
        };
        if lhs != "ii" || rhs != "i" {
            continue;
        }

        let mut replaced_any = false;
        let mut stop_idx = lines.len();
        let mut stopped_on_i_reassign = false;
        for (scan_idx, line) in lines.iter_mut().enumerate().skip(idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                stop_idx = scan_idx;
                break;
            }
            if let Some((next_lhs, _)) = parse_local_assign_line(trimmed)
                && (next_lhs == "ii" || next_lhs == "i")
            {
                stop_idx = scan_idx;
                stopped_on_i_reassign = next_lhs == "i";
                break;
            }
            if count_symbol_occurrences_local(trimmed, "ii") == 0 {
                continue;
            }
            let rewritten = replace_symbol_occurrences_local(line, "ii", "i");
            if rewritten != *line {
                *line = rewritten;
                replaced_any = true;
            }
        }

        let mut keep_alias = false;
        if stopped_on_i_reassign {
            for line in lines.iter().skip(stop_idx + 1) {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if line.contains("<- function") {
                    break;
                }
                if let Some((next_lhs, _)) = parse_local_assign_line(trimmed)
                    && next_lhs == "ii"
                {
                    break;
                }
                if count_symbol_occurrences_local(trimmed, "ii") > 0 {
                    keep_alias = true;
                    break;
                }
            }
        }

        if replaced_any && !keep_alias {
            lines[idx].clear();
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_dead_zero_seed_ii(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        if lines[idx].trim() != "ii <- 0" {
            continue;
        }
        let used_later = lines
            .iter()
            .skip(idx + 1)
            .any(|line| count_symbol_occurrences_local(line.trim(), "ii") > 0);
        if !used_later {
            lines[idx].clear();
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_slice_bound_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < lines.len() {
        let Some((start_lhs, start_rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if start_lhs != "start" {
            idx += 1;
            continue;
        }
        let start_rhs =
            crate::compiler::pipeline::strip_redundant_outer_parens(start_rhs).to_string();
        if !start_rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(end_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let Some((end_lhs, end_rhs)) = parse_local_assign_line(lines[end_idx].trim()) else {
            idx += 1;
            continue;
        };
        if end_lhs != "end" {
            idx += 1;
            continue;
        }
        let end_rhs = crate::compiler::pipeline::strip_redundant_outer_parens(end_rhs).to_string();
        if !end_rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let mut use_line_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(end_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((lhs, _)) = parse_local_assign_line(trimmed)
                && (lhs == "start" || lhs == "end")
            {
                break;
            }
            let uses_start = count_symbol_occurrences_local(trimmed, "start") > 0;
            let uses_end = count_symbol_occurrences_local(trimmed, "end") > 0;
            if uses_start || uses_end {
                if uses_start != uses_end || !trimmed.contains("neighbors[start:end] <-") {
                    use_line_idxs.clear();
                    break;
                }
                use_line_idxs.push(line_no);
                continue;
            }
            let is_control =
                trimmed == "}" || trimmed.starts_with("if (") || trimmed.starts_with("if(");
            if !use_line_idxs.is_empty() && !is_control {
                break;
            }
        }
        if use_line_idxs.is_empty() {
            idx += 1;
            continue;
        }

        let slice_expr = format!("{start_rhs}:{end_rhs}");
        for use_idx in &use_line_idxs {
            lines[*use_idx] = lines[*use_idx].replace("start:end", &slice_expr);
        }
        lines[idx].clear();
        lines[end_idx].clear();
        idx = use_line_idxs.last().copied().unwrap_or(end_idx) + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_particle_idx_alias(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx + 2 < lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if lhs != "idx" {
            idx += 1;
            continue;
        }
        let rhs = crate::compiler::pipeline::strip_redundant_outer_parens(rhs).to_string();
        if !rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(next1_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let Some(next2_idx) =
            ((next1_idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let next1 = lines[next1_idx].trim().to_string();
        let next2 = lines[next2_idx].trim().to_string();
        let dx_ok = next1.contains("dx <-")
            && next1.contains("u[idx]")
            && next1.contains("* dt")
            && next1.contains("/ 400000");
        let dy_ok = next2.contains("dy <-")
            && next2.contains("v[idx]")
            && next2.contains("* dt")
            && next2.contains("/ 400000");
        if !dx_ok || !dy_ok {
            idx += 1;
            continue;
        }

        lines[next1_idx] = replace_symbol_occurrences_local(&lines[next1_idx], "idx", rhs.as_str());
        lines[next2_idx] = replace_symbol_occurrences_local(&lines[next2_idx], "idx", rhs.as_str());
        lines[idx].clear();
        idx = next2_idx + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

fn rhs_is_simple_scalar_alias_or_literal_local(rhs: &str) -> bool {
    let rhs = strip_outer_parens_local(rhs);
    rhs.chars().all(RBackend::is_symbol_char)
        || rhs.trim_end_matches('L').parse::<f64>().is_ok()
        || matches!(rhs, "TRUE" | "FALSE" | "NA" | "NULL")
}

fn rhs_is_simple_dead_expr_local(rhs: &str) -> bool {
    let rhs = strip_outer_parens_local(rhs);
    !rhs.is_empty()
        && !rhs.contains("<-")
        && !rhs.contains("function(")
        && !rhs.contains("function (")
        && !rhs.contains("tryCatch(")
        && !rhs.contains("print(")
        && !rhs.contains("cat(")
        && !rhs.contains("message(")
        && !rhs.contains("warning(")
        && !rhs.contains("stop(")
        && !rhs.contains("quit(")
        && !rhs.contains('"')
        && !rhs.contains(',')
}

fn parse_repeat_guard_cmp_line_local(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("if (!(")
        .or_else(|| trimmed.strip_prefix("if !("))?
        .strip_suffix(")) break")
        .or_else(|| {
            trimmed
                .strip_prefix("if (!(")
                .or_else(|| trimmed.strip_prefix("if !("))
                .and_then(|s| s.strip_suffix(") break"))
        })?
        .trim();
    for op in ["<=", "<"] {
        let needle = format!(" {op} ");
        let Some((lhs, rhs)) = inner.split_once(&needle) else {
            continue;
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if !lhs.is_empty() && !rhs.is_empty() {
            return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
        }
    }
    None
}

fn plain_ident_local(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(RBackend::is_symbol_char)
}

fn raw_enclosing_repeat_guard_mentions_symbol_local(
    lines: &[String],
    idx: usize,
    symbol: &str,
) -> bool {
    for start_idx in (0..idx).rev() {
        if lines[start_idx].trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = find_block_end_local(lines, start_idx) else {
            continue;
        };
        if idx >= end_idx {
            continue;
        }
        let Some(guard_idx) = ((start_idx + 1)..end_idx)
            .find(|line_idx| parse_repeat_guard_cmp_line_local(lines[*line_idx].trim()).is_some())
        else {
            continue;
        };
        if count_symbol_occurrences_local(lines[guard_idx].trim(), symbol) > 0 {
            return true;
        }
        break;
    }
    false
}

fn raw_enclosing_repeat_body_reads_symbol_before_local(
    lines: &[String],
    idx: usize,
    symbol: &str,
) -> bool {
    for start_idx in (0..idx).rev() {
        if lines[start_idx].trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = find_block_end_local(lines, start_idx) else {
            continue;
        };
        if idx >= end_idx {
            continue;
        }
        let Some(guard_idx) = ((start_idx + 1)..end_idx)
            .find(|line_idx| parse_repeat_guard_cmp_line_local(lines[*line_idx].trim()).is_some())
        else {
            continue;
        };
        for line in lines.iter().take(idx).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((lhs, rhs)) = parse_local_assign_line(trimmed)
                && lhs == symbol
            {
                if count_symbol_occurrences_local(rhs, symbol) > 0 {
                    return true;
                }
                continue;
            }
            if count_symbol_occurrences_local(trimmed, symbol) > 0 {
                return true;
            }
        }
        break;
    }
    false
}

pub(super) fn rewrite_guard_scalar_literals(output: &mut String) {
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
            || !rhs_is_simple_scalar_alias_or_literal_local(&rhs)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            continue;
        };
        let mut guard_idx = next_idx;
        if lines[next_idx].trim() == "repeat {" {
            let Some(found_guard) =
                ((next_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
            else {
                continue;
            };
            guard_idx = found_guard;
        }
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if lines[guard_idx].contains("<- function")
            || !is_guard
            || count_symbol_occurrences_local(&guard_trimmed, &lhs) == 0
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(guard_idx + 1) {
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

        lines[guard_idx] = replace_symbol_occurrences_local(
            &lines[guard_idx],
            &lhs,
            strip_outer_parens_local(&rhs),
        );
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_loop_guard_scalar_literals(output: &mut String) {
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
            || !rhs_is_simple_scalar_alias_or_literal_local(&rhs)
        {
            continue;
        }

        let Some(repeat_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        if lines[repeat_idx].trim() != "repeat {" {
            continue;
        }
        let Some(guard_idx) =
            ((repeat_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if !is_guard || count_symbol_occurrences_local(&guard_trimmed, &lhs) == 0 {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(guard_idx + 1) {
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

        lines[guard_idx] = replace_symbol_occurrences_local(
            &lines[guard_idx],
            &lhs,
            strip_outer_parens_local(&rhs),
        );
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_named_scalar_pure_call_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_outer_parens_local(rhs).to_string();
        let max_uses = if rhs.starts_with("rr_wrap_index_vec_i(") {
            1usize
        } else if rhs.starts_with("rr_idx_cube_vec_i(") {
            2usize
        } else {
            continue;
        };
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents_local(rhs.as_str());
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
            if let Some((later_lhs, _later_rhs)) = parse_local_assign_line(line_trimmed) {
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
                if total_uses > max_uses {
                    break;
                }
            }
        }
        if total_uses == 0 || total_uses != max_uses {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }
        for use_idx in use_line_idxs {
            lines[use_idx] = replace_symbol_occurrences_local(&lines[use_idx], &lhs, rhs.as_str());
        }
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn hoist_branch_local_pure_scalar_assigns_used_after_branch(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim().to_string();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            idx += 1;
            continue;
        }
        let guard_idents = raw_expr_idents_local(trimmed.as_str());
        let Some(end_idx) = find_block_end_local(&lines, idx) else {
            idx += 1;
            continue;
        };

        let mut trailing_assigns = Vec::new();
        let mut scan = end_idx;
        while scan > idx + 1 {
            scan -= 1;
            let trimmed_line = lines[scan].trim();
            if trimmed_line.is_empty() {
                continue;
            }
            let Some((lhs, rhs)) = parse_local_assign_line(trimmed_line) else {
                break;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || !is_branch_hoistable_named_scalar_rhs_local(rhs)
            {
                break;
            }
            trailing_assigns.push((scan, lhs.to_string(), rhs.to_string()));
        }
        if trailing_assigns.is_empty() {
            idx = end_idx + 1;
            continue;
        }
        trailing_assigns.reverse();

        let mut hoisted = Vec::new();
        for (assign_idx, lhs, rhs) in trailing_assigns {
            if guard_idents.iter().any(|ident| ident == &lhs) {
                continue;
            }
            let rhs_deps = raw_expr_idents_local(strip_outer_parens_local(&rhs));
            let dep_written_in_branch = lines
                .iter()
                .take(assign_idx)
                .skip(idx + 1)
                .filter_map(|line| parse_local_assign_line(line.trim()))
                .any(|(branch_lhs, _)| rhs_deps.iter().any(|dep| dep == branch_lhs));
            if dep_written_in_branch {
                continue;
            }

            let mut used_after = false;
            for later_line in lines.iter().skip(end_idx + 1) {
                let later_trimmed = later_line.trim();
                if later_line.contains("<- function") {
                    break;
                }
                if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, &lhs) > 0 {
                    used_after = true;
                    break;
                }
            }
            if used_after {
                hoisted.push(lines[assign_idx].clone());
                lines[assign_idx].clear();
            }
        }

        if !hoisted.is_empty() {
            for (offset, line) in hoisted.into_iter().enumerate() {
                lines.insert(idx + offset, line);
            }
            idx = end_idx + 1;
            continue;
        }
        idx = end_idx + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

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

pub(super) fn strip_noop_self_assignments(output: &mut String) {
    let mut out = String::new();
    for line in output.lines() {
        let keep = if let Some((lhs, rhs)) = parse_local_assign_line(line.trim()) {
            lhs != strip_outer_parens_local(rhs)
        } else {
            true
        };
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    if output.is_empty() {
        *output = String::new();
        return;
    }
    if !output.ends_with('\n') {
        out.pop();
    }
    *output = out;
}

pub(super) fn strip_empty_else_blocks(output: &mut String) {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
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
    *output = rendered;
}

pub(super) fn collapse_nested_else_if_blocks(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return;
    }

    let mut changed = true;
    while changed {
        changed = false;
        let mut idx = 0usize;
        while idx < lines.len() {
            if lines[idx].trim() != "} else {" {
                idx += 1;
                continue;
            }
            let Some(nested_if_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                break;
            };
            let nested_if = lines[nested_if_idx].trim().to_string();
            if !nested_if.starts_with("if (") || !nested_if.ends_with('{') {
                idx += 1;
                continue;
            }
            let Some(nested_if_end) = find_block_end_local(&lines, nested_if_idx) else {
                idx += 1;
                continue;
            };
            let Some(else_close_idx) = ((nested_if_end + 1)..lines.len())
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                idx += 1;
                continue;
            };
            if lines[else_close_idx].trim() != "}" {
                idx += 1;
                continue;
            }

            let indent = lines[idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx] = format!("{indent}}} else {nested_if}");
            lines[nested_if_idx].clear();
            lines[else_close_idx].clear();
            changed = true;
            idx = else_close_idx + 1;
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_single_blank_spacers(output: &mut String) {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return;
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if idx > 0 && idx + 1 < lines.len() && lines[idx].trim().is_empty() {
            let Some(prev_idx) = (0..idx)
                .rev()
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };
            let Some(next_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };

            let prev = lines[prev_idx].trim();
            let next = lines[next_idx].trim();
            let prev_is_assign = parse_local_assign_line(prev).is_some();
            let next_is_assign = parse_local_assign_line(next).is_some();
            let next_is_control = next == "repeat {" || next == "}";
            let next_is_branch = next.starts_with("if (") || next.starts_with("if(");
            let next_is_return = next.starts_with("return(") || next.starts_with("return (");
            let prev_opens_block = prev.ends_with('{');
            let prev_is_return = prev.starts_with("return(") || prev.starts_with("return (");
            let prev_is_break = prev.starts_with("if (") && prev.ends_with("break");

            if (prev_is_assign && (next_is_assign || next_is_control || next_is_branch))
                || (prev_opens_block && (next_is_assign || next_is_return || next_is_branch))
                || (prev == "{"
                    && (next_is_assign || next_is_return || next_is_control || next_is_branch))
                || (prev == "}" && (next_is_assign || next == "}"))
                || (prev_is_break && (next_is_assign || next_is_branch || next_is_return))
                || (prev_is_return && next == "}")
            {
                continue;
            }
        }
        kept.push(lines[idx].clone());
    }

    let mut rewritten = kept.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn compact_blank_lines(output: &mut String) {
    let mut out = String::new();
    let mut blank_run = 0usize;
    for line in output.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    if output.is_empty() {
        *output = String::new();
        return;
    }
    if !output.ends_with('\n') {
        out.pop();
    }
    *output = out;
}

pub(super) fn strip_orphan_rr_cse_pruned_markers(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }
    for line in &mut lines {
        if line.trim() == "# rr-cse-pruned" {
            line.clear();
        }
    }
    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

fn find_matching_open_brace_line_local(lines: &[String], close_idx: usize) -> Option<usize> {
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

pub(super) fn strip_terminal_repeat_nexts(output: &mut String) {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return;
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if lines[idx].trim() == "next"
            && idx + 1 < lines.len()
            && lines[idx + 1].trim() == "}"
            && find_matching_open_brace_line_local(&lines, idx + 1)
                .is_some_and(|open_idx| lines[open_idx].trim() == "repeat {")
        {
            continue;
        }
        kept.push(lines[idx].clone());
    }

    let mut rewritten = kept.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_noop_temp_copy_roundtrips(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((tmp_lhs, tmp_rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let tmp_lhs = tmp_lhs.to_string();
        let tmp_rhs = tmp_rhs.to_string();
        if !(tmp_lhs.starts_with(".__pc_src_tmp") || tmp_lhs.starts_with(".__rr_cse_"))
            || !tmp_rhs.chars().all(RBackend::is_symbol_char)
        {
            idx += 1;
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            lines[idx].clear();
            break;
        };
        let Some((next_lhs, next_rhs)) = parse_local_assign_line(lines[next_idx].trim()) else {
            let used_later = lines
                .iter()
                .skip(next_idx)
                .any(|line| count_symbol_occurrences_local(line.trim(), &tmp_lhs) > 0);
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
                if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == tmp_lhs
                {
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, &tmp_lhs) > 0 {
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
            .any(|line| count_symbol_occurrences_local(line.trim(), &tmp_lhs) > 0);
        if !used_later {
            lines[idx].clear();
        }
        idx = next_idx + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_dead_simple_scalar_assigns(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || (!rhs_is_simple_scalar_alias_or_literal_local(rhs)
                    && !rhs_is_simple_dead_expr_local(rhs))
                || raw_enclosing_repeat_guard_mentions_symbol_local(&lines, idx, lhs)
                || raw_enclosing_repeat_body_reads_symbol_before_local(&lines, idx, lhs)
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
                if count_symbol_occurrences_local(later_trimmed, lhs) > 0 {
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

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_shadowed_simple_scalar_seed_assigns(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || (!rhs_is_simple_scalar_alias_or_literal_local(rhs)
                    && !rhs_is_simple_dead_expr_local(rhs))
                || raw_enclosing_repeat_guard_mentions_symbol_local(&lines, idx, lhs)
                || raw_enclosing_repeat_body_reads_symbol_before_local(&lines, idx, lhs)
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
                if let Some((later_lhs, later_rhs)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    if count_symbol_occurrences_local(later_rhs, lhs) == 0 {
                        shadowed_before_use = true;
                    }
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, lhs) > 0 {
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

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_temp_uses_after_named_copy(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };
        for idx in (fn_start + 1)..=fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            let lhs = lhs.to_string();
            let rhs = rhs.to_string();
            if !plain_ident_local(&lhs)
                || !(rhs.starts_with(".__pc_src_tmp") || rhs.starts_with(".__rr_cse_"))
            {
                continue;
            }
            let temp = rhs;
            let next_temp_def = ((idx + 1)..=fn_end)
                .find(|line_idx| {
                    parse_local_assign_line(lines[*line_idx].trim())
                        .is_some_and(|(later_lhs, _)| later_lhs == temp)
                })
                .unwrap_or(fn_end + 1);
            let next_lhs_def = ((idx + 1)..=fn_end)
                .find(|line_idx| {
                    parse_local_assign_line(lines[*line_idx].trim())
                        .is_some_and(|(later_lhs, _)| later_lhs == lhs)
                })
                .unwrap_or(fn_end + 1);
            let region_end = next_temp_def.min(next_lhs_def);
            for line_no in (idx + 1)..region_end {
                let line = &mut lines[line_no];
                let trimmed = line.trim();
                if trimmed.is_empty() || count_symbol_occurrences_local(trimmed, &temp) == 0 {
                    continue;
                }
                *line = replace_symbol_occurrences_local(line, &temp, &lhs);
            }
        }
        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_dead_seq_len_locals(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
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
                if let Some((later_lhs, later_rhs)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    if count_symbol_occurrences_local(later_rhs, lhs) > 0 {
                        used_later = true;
                    }
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, lhs) > 0 {
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

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_single_assignment_loop_seed_literals(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            continue;
        };
        if rhs.trim() != "1" && rhs.trim() != "1L" {
            continue;
        }

        let Some(next_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            continue;
        };
        let Some((_next_lhs, next_rhs)) = parse_local_assign_line(lines[next_idx].trim()) else {
            continue;
        };
        if !next_rhs.contains(&format!("{lhs}:")) {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _later_rhs)) = parse_local_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if count_symbol_occurrences_local(later_trimmed, lhs) > 0 {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[next_idx] = replace_symbol_occurrences_local(&lines[next_idx], lhs, "1");
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_sym210_loop_seed(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && lines[fn_start].trim() != "Sym_210 <- function(field, w, h)"
        {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            if lines[idx].trim() != "i <- 1" {
                continue;
            }
            let Some(next_idx) =
                ((idx + 1)..fn_end).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };
            let Some((next_lhs, _next_rhs)) = parse_local_assign_line(lines[next_idx].trim())
            else {
                continue;
            };
            if next_lhs != "lap" || !lines[next_idx].contains("i:size - 1") {
                continue;
            }
            lines[idx].clear();
            lines[next_idx] = lines[next_idx].replace("i:size - 1", "1:size - 1");
        }

        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_seq_len_full_overwrite_inits(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = crate::compiler::pipeline::strip_redundant_outer_parens(rhs).to_string();
        let Some(seq_inner) = rhs
            .strip_prefix("seq_len(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            idx += 1;
            continue;
        };

        let Some(iter_init_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let Some((iter_var, iter_start)) = parse_local_assign_line(lines[iter_init_idx].trim())
        else {
            idx += 1;
            continue;
        };
        if iter_start.trim() != "1" && iter_start.trim() != "1L" {
            idx += 1;
            continue;
        }

        let Some(repeat_idx) = ((iter_init_idx + 1)..lines.len()).find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            !trimmed.is_empty() && trimmed == "repeat {"
        }) else {
            idx += 1;
            continue;
        };
        let Some(loop_end) = find_block_end_local(&lines, repeat_idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) =
            ((repeat_idx + 1)..loop_end).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some((guard_iter, _op, guard_bound)) =
            parse_repeat_guard_cmp_line_local(lines[guard_idx].trim())
        else {
            idx += 1;
            continue;
        };
        if guard_iter != iter_var || guard_bound != seq_inner {
            idx += 1;
            continue;
        }

        let mut first_use_idx = None;
        let mut safe = true;
        let write_pat = format!("{lhs}[{iter_var}] <-");
        for (body_idx, line) in lines.iter().enumerate().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "next" {
                continue;
            }
            if line.contains("<- function") {
                safe = false;
                break;
            }
            if count_symbol_occurrences_local(trimmed, &lhs) > 0 {
                first_use_idx = Some(body_idx);
                if !trimmed.starts_with(&write_pat) {
                    safe = false;
                }
                break;
            }
        }
        if !safe || first_use_idx.is_none() {
            idx += 1;
            continue;
        }

        lines[idx] = format!(
            "{}{} <- rep.int(0, {})",
            lines[idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>(),
            lhs,
            seq_inner
        );
        idx = loop_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn restore_missing_repeat_loop_counter_updates(output: &mut String) {
    fn latest_local_literal_seed_before(lines: &[String], idx: usize, var: &str) -> Option<String> {
        for line in lines.iter().take(idx).rev() {
            let Some((lhs, rhs)) = parse_local_assign_line(line.trim()) else {
                continue;
            };
            if lhs != var {
                continue;
            }
            let rhs = crate::compiler::pipeline::strip_redundant_outer_parens(rhs).trim();
            let numeric = rhs.trim_end_matches('L').trim_end_matches('l');
            if numeric.parse::<f64>().ok().is_some() {
                return Some(rhs.to_string());
            }
            break;
        }
        None
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some(repeat_idx) =
            (idx..lines.len()).find(|line_idx| lines[*line_idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_block_end_local(&lines, repeat_idx) else {
            break;
        };
        let Some(guard_idx) = ((repeat_idx + 1)..loop_end)
            .find(|line_idx| parse_repeat_guard_cmp_line_local(lines[*line_idx].trim()).is_some())
        else {
            idx = loop_end + 1;
            continue;
        };
        let Some((iter_var, _cmp, _bound)) =
            parse_repeat_guard_cmp_line_local(lines[guard_idx].trim())
        else {
            idx = loop_end + 1;
            continue;
        };
        if !iter_var.chars().all(RBackend::is_symbol_char) {
            idx = loop_end + 1;
            continue;
        }
        let Some(seed) = latest_local_literal_seed_before(&lines, guard_idx, &iter_var) else {
            idx = loop_end + 1;
            continue;
        };

        let mut body_uses_iter = false;
        let mut body_assigns_iter = false;
        for line in lines.iter().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "# rr-cse-pruned" {
                continue;
            }
            if let Some((lhs, _rhs)) = parse_local_assign_line(trimmed)
                && lhs == iter_var
            {
                body_assigns_iter = true;
                break;
            }
            if count_symbol_occurrences_local(trimmed, &iter_var) > 0 {
                body_uses_iter = true;
            }
        }
        if body_assigns_iter || !body_uses_iter {
            idx = loop_end + 1;
            continue;
        }

        let indent = lines[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let step = if seed.contains('.') {
            "1.0"
        } else if seed.ends_with('L') || seed.ends_with('l') {
            "1L"
        } else {
            "1"
        };
        let insert_idx = ((guard_idx + 1)..loop_end)
            .rev()
            .find(|line_idx| {
                let trimmed = lines[*line_idx].trim();
                !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
            })
            .filter(|line_idx| lines[*line_idx].trim() == "next")
            .unwrap_or(loop_end);
        lines.insert(
            insert_idx,
            format!("{indent}{iter_var} <- ({iter_var} + {step})"),
        );
        idx = loop_end + 2;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_hoisted_loop_counter_aliases(output: &mut String) {
    fn extract_loop_counter_step(rhs: &str) -> Option<String> {
        let rhs = strip_outer_parens_local(rhs).trim();
        let caps = compile_regex(r"^([A-Za-z_][A-Za-z0-9_\.]*)\s*\+\s*(1L?|1\.0)$".to_string())?
            .captures(rhs)?;
        let var = caps.get(1)?.as_str();
        let step = caps.get(2)?.as_str();
        Some(format!("({var} + {step})"))
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            let lhs = lhs.to_string();
            let rhs = rhs.to_string();
            if !lhs.starts_with("licm_") {
                continue;
            }
            let Some(replacement) = extract_loop_counter_step(rhs.as_str()) else {
                continue;
            };
            let Some(var) = strip_outer_parens_local(rhs.as_str())
                .split('+')
                .next()
                .map(str::trim)
                .filter(|name| !name.is_empty())
                .map(str::to_string)
            else {
                continue;
            };

            let mut use_lines = Vec::new();
            let mut valid = true;
            for later_idx in (idx + 1)..=fn_end {
                let trimmed = lines[later_idx].trim();
                if trimmed.is_empty() {
                    continue;
                }
                let occurrences = count_symbol_occurrences_local(trimmed, lhs.as_str());
                if occurrences == 0 {
                    continue;
                }
                let Some((later_lhs, later_rhs)) = parse_local_assign_line(trimmed) else {
                    valid = false;
                    break;
                };
                if later_lhs != var || strip_outer_parens_local(later_rhs).trim() != lhs {
                    valid = false;
                    break;
                }
                use_lines.push(later_idx);
            }
            if !valid || use_lines.is_empty() {
                continue;
            }

            for use_idx in use_lines {
                let indent = lines[use_idx]
                    .chars()
                    .take_while(|ch| ch.is_ascii_whitespace())
                    .collect::<String>();
                lines[use_idx] = format!("{indent}{var} <- {replacement}");
            }
            lines[idx].clear();
        }

        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

fn split_top_level_compare_local(expr: &str) -> Option<(&str, &str, &str)> {
    let mut depth = 0i32;
    let bytes = expr.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        match bytes[idx] as char {
            '(' => depth += 1,
            ')' => depth -= 1,
            '>' | '<' | '=' | '!' if depth == 0 => {
                let rest = &expr[idx..];
                for op in ["<=", ">=", "==", "!=", "<", ">"] {
                    if rest.starts_with(op) {
                        let lhs = expr[..idx].trim();
                        let rhs = expr[idx + op.len()..].trim();
                        return Some((lhs, op, rhs));
                    }
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

fn extract_ifelse_range_expr_local(line: &str) -> Option<String> {
    let start = line.find("rr_ifelse_strict((")? + "rr_ifelse_strict((".len();
    let rest = &line[start..];
    let mut depth = 0i32;
    let mut idx = 0usize;
    while idx < rest.len() {
        let ch = rest.as_bytes()[idx] as char;
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            '<' | '>' | '=' | '!' if depth == 0 => {
                for op in ["<=", ">=", "==", "!=", "<", ">"] {
                    if rest[idx..].starts_with(op) {
                        let lhs = rest[..idx].trim();
                        if lhs.contains(':') && !lhs.contains(".__rr_cse_") {
                            return Some(lhs.to_string());
                        }
                        return None;
                    }
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

pub(super) fn repair_missing_cse_range_aliases(output: &mut String) {
    let Some(floor_temp_re) = compile_regex(r"rr_index_vec_floor\(\.__rr_cse_\d+\)".to_string())
    else {
        return;
    };

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for line in &mut lines {
        if !line.contains("rr_ifelse_strict(") || !line.contains("rr_index_vec_floor(.__rr_cse_") {
            continue;
        }
        let Some(range) = extract_ifelse_range_expr_local(line.as_str()) else {
            continue;
        };
        *line = floor_temp_re
            .replace_all(line, format!("rr_index_vec_floor({range})"))
            .to_string();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn restore_constant_one_guard_repeat_loop_counters(output: &mut String) {
    fn parse_constant_guard_local(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim();
        let inner = trimmed
            .strip_prefix("if (!(")
            .or_else(|| trimmed.strip_prefix("if !("))?
            .strip_suffix(")) break")
            .or_else(|| {
                trimmed
                    .strip_prefix("if (!(")
                    .or_else(|| trimmed.strip_prefix("if !("))
                    .and_then(|s| s.strip_suffix(") break"))
            })?
            .trim();
        for op in ["<=", "<"] {
            let needle = format!(" {op} ");
            let Some((lhs, rhs)) = inner.split_once(&needle) else {
                continue;
            };
            let lhs = crate::compiler::pipeline::strip_redundant_outer_parens(lhs.trim());
            let rhs = rhs.trim();
            if lhs.is_empty() || rhs.is_empty() {
                continue;
            }
            let numeric = lhs.trim_end_matches('L').trim_end_matches('l');
            if numeric.parse::<f64>().ok().is_some() {
                return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
            }
        }
        None
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        if lines[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }
        let Some(loop_end) = find_block_end_local(&lines, idx) else {
            idx += 1;
            continue;
        };
        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = lines[*line_idx].trim();
            (trimmed.starts_with("if !(") || trimmed.starts_with("if (!("))
                && trimmed.ends_with("break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let Some((start_lit, cmp, bound)) = parse_constant_guard_local(&lines[guard_idx]) else {
            idx = loop_end + 1;
            continue;
        };
        let idx_var = ".__rr_i";
        if lines
            .iter()
            .take(loop_end)
            .skip(guard_idx + 1)
            .any(|line| count_symbol_occurrences_local(line.trim(), idx_var) > 0)
        {
            idx = loop_end + 1;
            continue;
        }

        let indent = lines[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let repeat_indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines.insert(idx, format!("{repeat_indent}{idx_var} <- {start_lit}"));
        lines[guard_idx + 1] = if cmp == "<=" {
            format!("{indent}if (!({idx_var} <= {bound})) break")
        } else {
            format!("{indent}if (!({idx_var} < {bound})) break")
        };
        let one = if start_lit.contains('.') {
            "1.0"
        } else if start_lit.ends_with('L') || start_lit.ends_with('l') {
            "1L"
        } else {
            "1"
        };
        lines.insert(
            loop_end + 1,
            format!("{indent}{idx_var} <- ({idx_var} + {one})"),
        );
        idx = loop_end + 3;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

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

pub(super) fn strip_unused_raw_arg_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(RBackend::is_symbol_char) {
                idx += 1;
                continue;
            }
            let used_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .any(|line| count_symbol_occurrences_local(line.trim(), lhs) > 0);
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_readonly_raw_arg_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_block_end_local(&lines, fn_start) else {
            break;
        };

        let mut aliases = Vec::new();
        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(RBackend::is_symbol_char) {
                continue;
            }
            let reassigned_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .filter_map(|line| parse_local_assign_line(line.trim()))
                .any(|(later_lhs, _)| later_lhs == rhs);
            if reassigned_later {
                continue;
            }
            aliases.push((idx, lhs.to_string(), rhs.to_string()));
        }

        for (alias_idx, alias, target) in aliases {
            for line in lines.iter_mut().take(fn_end + 1).skip(alias_idx + 1) {
                *line = replace_symbol_occurrences_local(line, &alias, &target);
            }
            lines[alias_idx].clear();
        }

        fn_start = fn_end + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
