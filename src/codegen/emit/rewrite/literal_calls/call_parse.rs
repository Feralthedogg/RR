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
