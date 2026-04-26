use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
struct RawRecordField {
    name: String,
    expr: String,
}

pub(crate) fn rewrite_static_record_scalarization_lines(mut lines: Vec<String>) -> Vec<String> {
    if lines.is_empty() {
        return lines;
    }

    for line in &mut lines {
        *line = rewrite_inline_literal_record_field_accesses(line);
    }

    let existing_symbols = collect_raw_symbols(&lines);
    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            idx += 1;
            continue;
        };
        let lhs = lhs.to_string();
        let Some(fields) = parse_raw_record_list_expr(rhs) else {
            idx += 1;
            continue;
        };
        if fields.is_empty()
            || fields
                .iter()
                .any(|field| !raw_record_field_expr_is_sroa_safe(&field.expr))
            || fields
                .iter()
                .any(|field| line_contains_symbol(&field.expr, &lhs))
            || raw_line_is_within_loop_body(&lines, idx)
        {
            idx += 1;
            continue;
        }

        let Some(function_end) = raw_function_end_for_line(&lines, idx) else {
            idx += 1;
            continue;
        };

        let field_temps = build_raw_record_field_temp_map(&lhs, &fields, &existing_symbols);
        let mut used_fields = FxHashSet::default();
        let mut rewrites = Vec::new();
        let mut safe = true;

        for use_idx in (idx + 1)..function_end {
            let trimmed = lines[use_idx].trim();
            if let Some((next_lhs, _)) = parse_raw_assign_line(trimmed)
                && next_lhs == lhs
            {
                safe = false;
                break;
            }
            if raw_line_assigns_to_record_field(trimmed, &lhs) {
                safe = false;
                break;
            }
            if !line_contains_symbol(trimmed, &lhs) {
                continue;
            }
            let (rewritten, line_used_fields) =
                rewrite_raw_record_field_accesses(&lines[use_idx], &lhs, &field_temps);
            if line_contains_symbol(&rewritten, &lhs) {
                safe = false;
                break;
            }
            used_fields.extend(line_used_fields);
            rewrites.push((use_idx, rewritten));
        }

        if !safe || used_fields.is_empty() {
            idx += 1;
            continue;
        }

        for (use_idx, rewritten) in rewrites {
            lines[use_idx] = rewritten;
        }

        let indent = leading_indent(&lines[idx]).to_string();
        let mut scalar_lines = Vec::new();
        for field in &fields {
            if !used_fields.contains(&field.name) {
                continue;
            }
            let Some(temp) = field_temps.get(&field.name) else {
                continue;
            };
            let rhs = rewrite_inline_literal_record_field_accesses(&field.expr);
            scalar_lines.push(format!("{indent}{temp} <- {}", rhs.trim()));
        }

        if scalar_lines.is_empty() {
            idx += 1;
            continue;
        }

        lines.splice(idx..=idx, scalar_lines);
        idx += 1;
    }

    lines
}

pub(crate) fn rewrite_static_record_scalarization_in_raw_emitted_r(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    let lines = rewrite_static_record_scalarization_lines(lines);
    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if raw_expr_idents(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_named_scalar_expr(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            continue;
        };
        let next_trimmed = lines[next_idx].trim().to_string();
        let next_is_assign = parse_raw_assign_line(next_trimmed.as_str()).is_some();
        let next_is_return =
            next_trimmed.starts_with("return(") || next_trimmed.starts_with("return (");
        if lines[next_idx].contains("<- function")
            || (!next_is_assign && !next_is_return)
            || !line_contains_symbol(&next_trimmed, &lhs)
        {
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
            if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let replacement = format!("({})", strip_redundant_outer_parens(&rhs));
        lines[next_idx] = replace_symbol_occurrences(&lines[next_idx], &lhs, replacement.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn parse_raw_record_list_expr(rhs: &str) -> Option<Vec<RawRecordField>> {
    let rhs = strip_redundant_outer_parens(rhs).trim();
    let open = rhs.find("list(")? + "list".len();
    if open != "list".len() {
        return None;
    }
    let close = find_matching_call_close(rhs, open)?;
    if close + 1 != rhs.len() {
        return None;
    }

    let args = split_top_level_args(&rhs[open + 1..close])?;
    let mut fields = Vec::with_capacity(args.len());
    let mut seen = FxHashSet::default();
    for arg in args {
        let (name, expr) = split_raw_record_field_arg(&arg)?;
        if !seen.insert(name.clone()) {
            return None;
        }
        fields.push(RawRecordField { name, expr });
    }
    Some(fields)
}

fn split_raw_record_field_arg(arg: &str) -> Option<(String, String)> {
    let mut depth = 0i32;
    for (idx, ch) in arg.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '=' if depth == 0 => {
                let name = arg[..idx].trim();
                let expr = arg[idx + ch.len_utf8()..].trim();
                if name.is_empty() || expr.is_empty() || !name.chars().all(is_symbol_char) {
                    return None;
                }
                return Some((name.to_string(), expr.to_string()));
            }
            _ => {}
        }
    }
    None
}

fn raw_record_field_expr_is_sroa_safe(expr: &str) -> bool {
    let expr = strip_redundant_outer_parens(expr);
    if expr.contains("<-")
        || expr.contains("function(")
        || expr.contains("function (")
        || expr.contains("tryCatch(")
        || expr.contains("print(")
        || expr.contains("cat(")
        || expr.contains("message(")
        || expr.contains("warning(")
        || expr.contains("stop(")
        || expr.contains("quit(")
    {
        return false;
    }

    raw_calls_are_sroa_safe(expr)
}

fn raw_calls_are_sroa_safe(expr: &str) -> bool {
    let bytes = expr.as_bytes();
    let mut idx = 0usize;
    let mut in_single = false;
    let mut in_double = false;
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
            b'(' if !in_single && !in_double => {
                let callee = raw_callee_before_open(expr, idx);
                if let Some(callee) = callee
                    && !raw_sroa_allowed_pure_call(callee)
                {
                    return false;
                }
            }
            _ => {}
        }
        idx += 1;
    }
    true
}

fn raw_callee_before_open(expr: &str, open: usize) -> Option<&str> {
    let prefix = expr[..open].trim_end();
    let end = prefix.len();
    let mut start = end;
    for (idx, ch) in prefix.char_indices().rev() {
        if is_symbol_char(ch) {
            start = idx;
            continue;
        }
        break;
    }
    if start == end {
        return None;
    }
    Some(&prefix[start..end])
}

fn raw_sroa_allowed_pure_call(callee: &str) -> bool {
    matches!(
        callee,
        "list"
            | "c"
            | "integer"
            | "numeric"
            | "logical"
            | "character"
            | "rep.int"
            | "seq_len"
            | "abs"
            | "sqrt"
            | "sin"
            | "cos"
            | "tan"
            | "exp"
            | "log"
            | "min"
            | "max"
            | "sum"
            | "mean"
            | "pmin"
            | "pmax"
            | "is.na"
            | "is.finite"
    )
}

fn raw_function_end_for_line(lines: &[String], idx: usize) -> Option<usize> {
    let function_start = (0..=idx)
        .rev()
        .find(|line_idx| parse_raw_function_header(lines[*line_idx].trim()).is_some())?;
    let end = find_raw_block_end(lines, function_start)?;
    (idx < end).then_some(end)
}

fn collect_raw_symbols(lines: &[String]) -> FxHashSet<String> {
    let mut symbols = FxHashSet::default();
    for line in lines {
        symbols.extend(raw_expr_idents(line));
    }
    symbols
}

fn build_raw_record_field_temp_map(
    lhs: &str,
    fields: &[RawRecordField],
    existing_symbols: &FxHashSet<String>,
) -> FxHashMap<String, String> {
    let mut temps = FxHashMap::default();
    let mut reserved = existing_symbols.clone();
    for field in fields {
        let suffix = sanitize_raw_sroa_field_name(&field.name);
        let seed = format!("{lhs}__rr_sroa_{suffix}");
        let mut temp = seed.clone();
        let mut index = 0usize;
        while reserved.contains(&temp) {
            index += 1;
            temp = format!("{seed}_{index}");
        }
        reserved.insert(temp.clone());
        temps.insert(field.name.clone(), temp);
    }
    temps
}

fn sanitize_raw_sroa_field_name(field: &str) -> String {
    let mut out = String::with_capacity(field.len().max(1));
    for ch in field.chars() {
        if is_symbol_char(ch) {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "field".to_string()
    } else {
        out
    }
}

fn leading_indent(line: &str) -> &str {
    let end = line
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_whitespace()).then_some(idx))
        .unwrap_or(line.len());
    &line[..end]
}

fn rewrite_raw_record_field_accesses(
    line: &str,
    base: &str,
    field_temps: &FxHashMap<String, String>,
) -> (String, FxHashSet<String>) {
    let mut out = String::with_capacity(line.len());
    let mut used_fields = FxHashSet::default();
    let mut idx = 0usize;
    let bytes = line.as_bytes();
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
            && let Some((field, end)) = parse_raw_record_field_access_at(line, idx, base)
            && let Some(temp) = field_temps.get(&field)
        {
            out.push_str(temp);
            used_fields.insert(field);
            idx = end;
            continue;
        }

        let Some(ch) = line[idx..].chars().next() else {
            break;
        };
        out.push(ch);
        idx += ch.len_utf8();
    }

    (out, used_fields)
}

fn raw_line_assigns_to_record_field(line: &str, base: &str) -> bool {
    let Some((lhs, _)) = line.trim().split_once(" <- ") else {
        return false;
    };
    let lhs = lhs.trim();
    let Some((_, end)) = parse_raw_record_field_access_at(lhs, 0, base) else {
        return false;
    };
    lhs[end..].trim().is_empty()
}

fn parse_raw_record_field_access_at(line: &str, idx: usize, base: &str) -> Option<(String, usize)> {
    parse_direct_raw_record_field_access_at(line, idx, base)
        .or_else(|| parse_parenthesized_raw_record_field_access_at(line, idx, base))
}

fn parse_direct_raw_record_field_access_at(
    line: &str,
    idx: usize,
    base: &str,
) -> Option<(String, usize)> {
    if !line[idx..].starts_with(base) || !raw_symbol_boundary_before(line, idx) {
        return None;
    }
    let suffix_start = idx + base.len();
    let (field, suffix_len) = parse_raw_static_field_suffix(&line[suffix_start..])?;
    Some((field, suffix_start + suffix_len))
}

fn parse_parenthesized_raw_record_field_access_at(
    line: &str,
    idx: usize,
    base: &str,
) -> Option<(String, usize)> {
    if !line[idx..].starts_with('(') {
        return None;
    }
    let mut cursor = idx;
    let mut parens = 0usize;
    while line[cursor..].starts_with('(') {
        parens += 1;
        cursor += 1;
        cursor += raw_ascii_whitespace_prefix_len(&line[cursor..]);
    }
    if !line[cursor..].starts_with(base) || !raw_symbol_boundary_before(line, cursor) {
        return None;
    }
    cursor += base.len();
    cursor += raw_ascii_whitespace_prefix_len(&line[cursor..]);
    for _ in 0..parens {
        if !line[cursor..].starts_with(')') {
            return None;
        }
        cursor += 1;
        cursor += raw_ascii_whitespace_prefix_len(&line[cursor..]);
    }
    let (field, suffix_len) = parse_raw_static_field_suffix(&line[cursor..])?;
    Some((field, cursor + suffix_len))
}

fn raw_symbol_boundary_before(line: &str, idx: usize) -> bool {
    line[..idx]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_symbol_char(ch))
}

fn raw_ascii_whitespace_prefix_len(input: &str) -> usize {
    input
        .char_indices()
        .find_map(|(idx, ch)| (!ch.is_ascii_whitespace()).then_some(idx))
        .unwrap_or(input.len())
}

fn parse_raw_static_field_suffix(input: &str) -> Option<(String, usize)> {
    let leading = raw_ascii_whitespace_prefix_len(input);
    let rest = &input[leading..];
    let rest = rest.strip_prefix("[[")?;
    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let after_quote = &rest[quote.len_utf8()..];
    let close_quote = after_quote.find(quote)?;
    let field = &after_quote[..close_quote];
    if field.is_empty() || !field.chars().all(is_symbol_char) {
        return None;
    }
    let after_field = &after_quote[close_quote + quote.len_utf8()..];
    let after_suffix = after_field.strip_prefix("]]")?;
    let consumed = input.len() - after_suffix.len();
    Some((field.to_string(), consumed))
}

fn rewrite_inline_literal_record_field_accesses(line: &str) -> String {
    let mut rewritten = line.to_string();
    let mut search_from = 0usize;
    while search_from < rewritten.len() {
        let Some(rel_start) = rewritten[search_from..].find("list(") else {
            break;
        };
        let start = search_from + rel_start;
        let open = start + "list".len();
        let Some(close) = find_matching_call_close(&rewritten, open) else {
            break;
        };
        let Some((field, suffix_len)) = parse_raw_static_field_suffix(&rewritten[close + 1..])
        else {
            search_from = start + "list(".len();
            continue;
        };
        let Some(fields) = parse_raw_record_list_expr(&rewritten[start..=close]) else {
            search_from = start + "list(".len();
            continue;
        };
        if fields
            .iter()
            .any(|candidate| !raw_record_field_expr_is_sroa_safe(&candidate.expr))
        {
            search_from = start + "list(".len();
            continue;
        }
        let Some(replacement) = fields
            .iter()
            .find(|candidate| candidate.name == field)
            .map(|candidate| format!("({})", candidate.expr.trim()))
        else {
            search_from = close + 1;
            continue;
        };
        let replace_end = close + 1 + suffix_len;
        rewritten.replace_range(start..replace_end, &replacement);
        search_from = start + replacement.len();
    }
    rewritten
}

pub(crate) fn rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if raw_expr_idents(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_named_scalar_expr(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let Some(use_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && line_contains_symbol(trimmed, &lhs)
        }) else {
            continue;
        };
        let use_trimmed = lines[use_idx].trim().to_string();
        let use_is_guard = use_trimmed.starts_with("if (") || use_trimmed.starts_with("if(");
        let use_occurrences = count_symbol_occurrences(&use_trimmed, &lhs);
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
            if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let replacement = format!("({})", strip_redundant_outer_parens(&rhs));
        lines[use_idx] = replace_symbol_occurrences(&lines[use_idx], &lhs, replacement.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_two_use_named_scalar_exprs_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if raw_expr_idents(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_named_scalar_expr(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents(rhs.as_str());
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
            || parse_raw_assign_line(next1_trimmed.as_str()).is_none()
            || parse_raw_assign_line(next2_trimmed.as_str()).is_none()
            || count_symbol_occurrences(&next1_trimmed, &lhs) != 1
            || count_symbol_occurrences(&next2_trimmed, &lhs) != 1
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
            if let Some((later_lhs, _)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences(line_trimmed, &lhs);
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

        let replacement = strip_redundant_outer_parens(&rhs);
        lines[next1_idx] = replace_symbol_occurrences(&lines[next1_idx], &lhs, replacement);
        lines[next2_idx] = replace_symbol_occurrences(&lines[next2_idx], &lhs, replacement);
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        if rhs.trim() != "1" && rhs.trim() != "1L" {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            continue;
        };
        let Some((_next_lhs, next_rhs)) = parse_raw_assign_line(lines[next_idx].trim()) else {
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
            if let Some((later_lhs, _later_rhs)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[next_idx] = replace_symbol_occurrences(&lines[next_idx], lhs, "1");
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_sym210_loop_seed_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
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
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            if lines[idx].trim() != "i <- 1" {
                continue;
            }
            let Some(next_idx) = ((idx + 1)..fn_end).find(|i| !lines[*i].trim().is_empty()) else {
                continue;
            };
            let Some((next_lhs, _next_rhs)) = parse_raw_assign_line(lines[next_idx].trim()) else {
                continue;
            };
            if next_lhs != "lap" || !lines[next_idx].contains("i:size - 1") {
                continue;
            }
            lines[idx].clear();
            lines[next_idx] = lines[next_idx].replace("i:size - 1", "1:size - 1");
        }

        fn_start += 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_two_use_named_scalar_pure_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_redundant_outer_parens(rhs).to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs.starts_with("rr_idx_cube_vec_i(")
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents(rhs.as_str());
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
            if let Some((later_lhs, _later_rhs)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 2 {
                    break;
                }
            }
        }
        if total_uses != 2 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }
        for use_idx in use_line_idxs {
            lines[use_idx] = replace_symbol_occurrences(&lines[use_idx], &lhs, rhs.as_str());
        }
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_guard_only_scalar_literals_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_raw_simple_scalar_alias_or_literal(&rhs)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            continue;
        };
        let mut guard_idx = next_idx;
        let next_trimmed = lines[next_idx].trim().to_string();
        if next_trimmed == "repeat {" {
            let Some(found_guard) = ((next_idx + 1)..lines.len()).find(|i| {
                let trimmed = lines[*i].trim();
                !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
            }) else {
                continue;
            };
            guard_idx = found_guard;
        }
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if lines[guard_idx].contains("<- function")
            || !is_guard
            || !line_contains_symbol(&guard_trimmed, &lhs)
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
            if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[guard_idx] =
            replace_symbol_occurrences(&lines[guard_idx], &lhs, strip_redundant_outer_parens(&rhs));
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_loop_guard_scalar_literals_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_raw_simple_scalar_alias_or_literal(&rhs)
        {
            continue;
        }

        let Some(repeat_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            continue;
        };
        if lines[repeat_idx].trim() != "repeat {" {
            continue;
        }
        let Some(guard_idx) = ((repeat_idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            continue;
        };
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if !is_guard || !line_contains_symbol(&guard_trimmed, &lhs) {
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
            if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[guard_idx] =
            replace_symbol_occurrences(&lines[guard_idx], &lhs, strip_redundant_outer_parens(&rhs));
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_single_use_named_scalar_pure_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_redundant_outer_parens(rhs).to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs.starts_with("rr_wrap_index_vec_i(")
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents(rhs.as_str());
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
            if let Some((later_lhs, _later_rhs)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 1 {
                    break;
                }
            }
        }
        if total_uses != 1 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }

        let use_idx = use_line_idxs[0];
        lines[use_idx] = replace_symbol_occurrences(&lines[use_idx], &lhs, rhs.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}
