use super::*;
pub(crate) fn split_top_level_compare_local(expr: &str) -> Option<(&str, &str, &str)> {
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

pub(crate) fn extract_ifelse_range_expr_local(line: &str) -> Option<String> {
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

pub(crate) fn repair_missing_cse_range_aliases(output: &mut String) {
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
