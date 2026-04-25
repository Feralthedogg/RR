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
