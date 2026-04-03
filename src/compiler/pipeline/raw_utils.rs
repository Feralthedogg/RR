//! Shared parser-style helpers for raw emitted-R rewrite passes.
//!
//! These routines sit on hot rewrite paths, so they intentionally stay small,
//! allocation-light, and free of policy decisions about when a rewrite should
//! fire.

use super::*;

pub(crate) fn parse_raw_assign_line(line: &str) -> Option<(&str, &str)> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let (lhs, rhs) = trimmed.split_once(" <- ")?;
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    if lhs.is_empty() || !lhs.chars().all(is_symbol_char) {
        return None;
    }
    Some((lhs, rhs))
}

pub(crate) fn parse_raw_function_header(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let (name, rest) = trimmed.split_once("<- function(")?;
    let name = name.trim();
    if name.is_empty() || !name.chars().all(is_symbol_char) {
        return None;
    }
    let args_inner = rest.strip_suffix(')')?.trim();
    let params = if args_inner.is_empty() {
        Vec::new()
    } else {
        split_top_level_args(args_inner)?
    };
    Some((name.to_string(), params))
}

pub(crate) fn parse_top_level_raw_call(rhs: &str) -> Option<(String, Vec<String>)> {
    let rhs = strip_redundant_outer_parens(rhs).trim();
    let open = rhs.find('(')?;
    let close = find_matching_call_close(rhs, open)?;
    if close + 1 != rhs.len() {
        return None;
    }
    let callee = rhs[..open].trim();
    if callee.is_empty() || !callee.chars().all(is_symbol_char) {
        return None;
    }
    let args = split_top_level_args(&rhs[open + 1..close])?;
    Some((callee.to_string(), args))
}

pub(crate) fn remap_source_map_lines(mut map: Vec<MapEntry>, line_map: &[u32]) -> Vec<MapEntry> {
    for entry in &mut map {
        let old_idx = entry.r_line.saturating_sub(1) as usize;
        if let Some(new_line) = line_map.get(old_idx) {
            entry.r_line = *new_line;
        }
    }
    map
}

pub(crate) fn rhs_is_raw_simple_scalar_alias_or_literal(rhs: &str) -> bool {
    let rhs = strip_redundant_outer_parens(rhs);
    rhs.chars().all(is_symbol_char)
        || is_raw_numeric_literal(rhs)
        || matches!(rhs, "TRUE" | "FALSE" | "NA" | "NULL")
}

pub(crate) fn rhs_is_raw_simple_dead_expr(rhs: &str) -> bool {
    let rhs = strip_redundant_outer_parens(rhs);
    !rhs.is_empty()
        && !rhs.contains("<-")
        && !rhs.contains("function(")
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

pub(crate) fn is_raw_numeric_literal(rhs: &str) -> bool {
    let rhs = rhs.trim();
    if rhs.is_empty() {
        return false;
    }
    let rhs = rhs.strip_suffix('L').unwrap_or(rhs);
    rhs.parse::<f64>().is_ok()
}

pub(crate) fn strip_redundant_outer_parens(expr: &str) -> &str {
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

pub(crate) fn is_inlineable_raw_scalar_index_rhs(rhs: &str) -> bool {
    let trimmed = strip_redundant_outer_parens(rhs);
    let open = trimmed.find('[');
    let Some(open) = open else {
        return false;
    };
    let close = matching_square_close(trimmed, open);
    let Some(close) = close else {
        return false;
    };
    if close + 1 != trimmed.len() || open == 0 || close <= open + 1 {
        return false;
    }
    let base = trimmed[..open].trim();
    base.chars().all(is_symbol_char)
}

fn matching_square_close(s: &str, open: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0usize;
    let mut idx = open;
    while idx < bytes.len() {
        match bytes[idx] {
            b'[' => depth += 1,
            b']' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
        idx += 1;
    }
    None
}

pub(crate) fn is_inlineable_raw_named_scalar_expr(rhs: &str) -> bool {
    let rhs = strip_redundant_outer_parens(rhs);
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

pub(crate) fn find_raw_block_end(lines: &[String], start_idx: usize) -> Option<usize> {
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

pub(crate) fn is_raw_alloc_like_expr(expr: &str) -> bool {
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

pub(crate) fn is_raw_branch_rebind_candidate(expr: &str) -> bool {
    is_raw_alloc_like_expr(expr)
        || expr.chars().all(is_symbol_char)
        || is_raw_numeric_literal(expr)
        || matches!(expr, "TRUE" | "FALSE" | "NA" | "NULL")
}

pub(crate) fn raw_branch_rebind_exprs_equivalent(prev_rhs: &str, rhs: &str) -> bool {
    let prev_rhs = strip_redundant_outer_parens(prev_rhs);
    let rhs = strip_redundant_outer_parens(rhs);
    if prev_rhs == rhs {
        return true;
    }
    raw_vec_fill_signature(prev_rhs)
        .zip(raw_vec_fill_signature(rhs))
        .is_some_and(|(lhs_sig, rhs_sig)| lhs_sig == rhs_sig)
}

pub(crate) fn raw_vec_fill_signature(expr: &str) -> Option<(String, String)> {
    let expr = strip_redundant_outer_parens(expr);
    if let Some(inner) = expr
        .strip_prefix("rep.int(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let args = split_top_level_args(inner)?;
        if args.len() == 2 {
            return Some((args[1].trim().to_string(), args[0].trim().to_string()));
        }
    }
    if let Some(inner) = expr
        .strip_prefix("Sym_17(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let args = split_top_level_args(inner)?;
        if args.len() == 2 {
            return Some((args[0].trim().to_string(), args[1].trim().to_string()));
        }
    }
    None
}

pub(crate) fn enclosing_raw_branch_start(lines: &[String], idx: usize) -> Option<usize> {
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

pub(crate) fn branch_body_writes_symbol_before(
    lines: &[String],
    start: usize,
    end_exclusive: usize,
    symbol: &str,
) -> bool {
    lines
        .iter()
        .take(end_exclusive)
        .skip(start)
        .filter_map(|line| parse_raw_assign_line(line.trim()))
        .any(|(lhs, _)| lhs == symbol)
}

pub(crate) fn previous_outer_assign_before_branch<'a>(
    lines: &'a [String],
    branch_start: usize,
    lhs: &str,
) -> Option<(&'a str, &'a str)> {
    for prev_idx in (0..branch_start).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "}" || trimmed == "{" {
            continue;
        }
        if trimmed.ends_with('{') {
            break;
        }
        let Some((prev_lhs, prev_rhs)) = parse_raw_assign_line(trimmed) else {
            continue;
        };
        if prev_lhs == lhs {
            return Some((prev_lhs, prev_rhs));
        }
        if line_contains_symbol(trimmed, lhs) {
            break;
        }
    }
    None
}

pub(crate) fn previous_outer_assign_before_branch_relaxed<'a>(
    lines: &'a [String],
    branch_start: usize,
    lhs: &str,
) -> Option<(&'a str, &'a str)> {
    for prev_idx in (0..branch_start).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "}" || trimmed == "{" {
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
            return Some((prev_lhs, prev_rhs));
        }
    }
    None
}

pub(crate) fn line_contains_symbol(line: &str, symbol: &str) -> bool {
    scan_symbol_occurrences(line, symbol).next().is_some()
}

pub(crate) fn count_symbol_occurrences(line: &str, symbol: &str) -> usize {
    scan_symbol_occurrences(line, symbol).count()
}

pub(crate) fn find_symbol_call(line: &str, symbol: &str, start_from: usize) -> Option<usize> {
    let mut search_from = start_from;
    while let Some(rel_idx) = line[search_from..].find(symbol) {
        let idx = search_from + rel_idx;
        let before = line[..idx].chars().next_back();
        let after = line[idx + symbol.len()..].chars().next();
        let boundary_ok = before.is_none_or(|ch| !is_symbol_char(ch)) && after == Some('(');
        if boundary_ok {
            return Some(idx);
        }
        search_from = idx + symbol.len();
    }
    None
}

pub(crate) fn find_matching_paren(line: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut saw_open = false;
    for (idx, ch) in line.char_indices().skip(open_idx) {
        match ch {
            '(' => {
                depth += 1;
                saw_open = true;
            }
            ')' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
                if saw_open && depth == 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn line_contains_unquoted_symbol_reference(line: &str, symbol: &str) -> bool {
    scan_symbol_occurrences(line, symbol).next().is_some()
}

pub(crate) fn replace_symbol_occurrences(line: &str, symbol: &str, replacement: &str) -> String {
    let hits: Vec<usize> = scan_symbol_occurrences(line, symbol).collect();
    if hits.is_empty() {
        return line.to_string();
    }
    let mut out = String::with_capacity(line.len());
    let mut cursor = 0usize;
    for hit in hits {
        out.push_str(&line[cursor..hit]);
        out.push_str(replacement);
        cursor = hit + symbol.len();
    }
    out.push_str(&line[cursor..]);
    out
}

fn scan_symbol_occurrences<'a>(line: &'a str, symbol: &'a str) -> impl Iterator<Item = usize> + 'a {
    let bytes = line.as_bytes();
    let symbol_bytes = symbol.as_bytes();
    let mut hits = Vec::new();
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
            _ => {}
        }

        if !in_single
            && !in_double
            && bytes[idx..].starts_with(symbol_bytes)
            && symbol_hit_is_rewritable(line, idx, symbol.len())
        {
            hits.push(idx);
        }

        idx += 1;
    }

    hits.into_iter()
}

fn symbol_hit_is_rewritable(line: &str, idx: usize, symbol_len: usize) -> bool {
    let before = line[..idx].chars().next_back();
    let after = line[idx + symbol_len..].chars().next();
    let boundary_ok =
        before.is_none_or(|ch| !is_symbol_char(ch)) && after.is_none_or(|ch| !is_symbol_char(ch));
    boundary_ok && !symbol_hit_is_named_label(line, idx + symbol_len)
}

fn symbol_hit_is_named_label(line: &str, after_idx: usize) -> bool {
    let rest = &line[after_idx..];
    for (off, ch) in rest.char_indices() {
        if ch.is_ascii_whitespace() {
            continue;
        }
        if ch != '=' {
            return false;
        }
        let tail = &rest[off + ch.len_utf8()..];
        let next_non_ws = tail.chars().find(|ch| !ch.is_ascii_whitespace());
        return next_non_ws != Some('=');
    }
    false
}

pub(crate) fn raw_expr_idents(expr: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = None;
    for (idx, ch) in expr.char_indices() {
        if is_symbol_char(ch) {
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

pub(crate) fn is_symbol_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')
}

pub(crate) fn find_matching_call_close(line: &str, open_idx: usize) -> Option<usize> {
    let mut depth = 0i32;
    for (off, ch) in line[open_idx..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_idx + off);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn split_top_level_args(expr: &str) -> Option<Vec<String>> {
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

pub(crate) fn raw_literal_record_field_name(expr: &str) -> Option<String> {
    let trimmed = expr.trim();
    let inner = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
        })?;
    inner.chars().all(is_symbol_char).then(|| inner.to_string())
}
