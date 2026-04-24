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
