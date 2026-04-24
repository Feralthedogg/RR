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
