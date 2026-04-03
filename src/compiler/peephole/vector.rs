use super::patterns::{
    assign_re, expr_idents, next_generated_cse_index, plain_ident_re, split_top_level_args,
};
use rustc_hash::FxHashMap;

fn helper_call_prefixes() -> &'static [&'static str] {
    &[
        "rr_parallel_vec_add_f64(",
        "rr_parallel_vec_sub_f64(",
        "rr_parallel_vec_mul_f64(",
        "rr_parallel_vec_div_f64(",
        "rr_parallel_vec_abs_f64(",
        "rr_parallel_vec_log_f64(",
        "rr_parallel_vec_sqrt_f64(",
        "rr_parallel_vec_pmax_f64(",
        "rr_parallel_vec_pmin_f64(",
        "rr_intrinsic_vec_add_f64(",
        "rr_intrinsic_vec_sub_f64(",
        "rr_intrinsic_vec_mul_f64(",
        "rr_intrinsic_vec_div_f64(",
        "rr_intrinsic_vec_abs_f64(",
        "rr_intrinsic_vec_log_f64(",
        "rr_intrinsic_vec_sqrt_f64(",
        "rr_intrinsic_vec_pmax_f64(",
        "rr_intrinsic_vec_pmin_f64(",
    ]
}

pub(crate) fn rewrite_direct_vec_helper_expr(expr: &str, enabled: bool) -> String {
    if !enabled {
        return expr.trim().to_string();
    }
    let expr = expr.trim();
    for prefix in [
        "rr_parallel_vec_add_f64(",
        "rr_parallel_vec_sub_f64(",
        "rr_parallel_vec_mul_f64(",
        "rr_parallel_vec_div_f64(",
        "rr_parallel_vec_pmax_f64(",
        "rr_parallel_vec_pmin_f64(",
        "rr_intrinsic_vec_add_f64(",
        "rr_intrinsic_vec_sub_f64(",
        "rr_intrinsic_vec_mul_f64(",
        "rr_intrinsic_vec_div_f64(",
        "rr_intrinsic_vec_pmax_f64(",
        "rr_intrinsic_vec_pmin_f64(",
    ] {
        if let Some(inner) = expr.strip_prefix(prefix).and_then(|s| s.strip_suffix(')'))
            && let Some(args) = split_top_level_args(inner)
            && args.len() == 2
        {
            let lhs = rewrite_direct_vec_helper_expr(&args[0], enabled);
            let rhs = rewrite_direct_vec_helper_expr(&args[1], enabled);
            return if prefix.contains("_pmax_") {
                format!("pmax({lhs}, {rhs})")
            } else if prefix.contains("_pmin_") {
                format!("pmin({lhs}, {rhs})")
            } else {
                let op = if prefix.contains("_add_") {
                    "+"
                } else if prefix.contains("_sub_") {
                    "-"
                } else if prefix.contains("_mul_") {
                    "*"
                } else {
                    "/"
                };
                format!("({lhs} {op} {rhs})")
            };
        }
    }

    for prefix in [
        "rr_parallel_vec_abs_f64(",
        "rr_parallel_vec_log_f64(",
        "rr_parallel_vec_sqrt_f64(",
        "rr_intrinsic_vec_abs_f64(",
        "rr_intrinsic_vec_log_f64(",
        "rr_intrinsic_vec_sqrt_f64(",
    ] {
        if let Some(inner) = expr.strip_prefix(prefix).and_then(|s| s.strip_suffix(')')) {
            let arg = rewrite_direct_vec_helper_expr(inner, enabled);
            let fun = if prefix.contains("_abs_") {
                "abs"
            } else if prefix.contains("_log_") {
                "log"
            } else {
                "sqrt"
            };
            return format!("{fun}({arg})");
        }
    }

    rewrite_helper_subcalls(expr, enabled)
}

pub(crate) fn rewrite_helper_subcalls(expr: &str, enabled: bool) -> String {
    if !enabled {
        return expr.trim().to_string();
    }
    let mut out = expr.trim().to_string();
    loop {
        let mut changed = false;
        let mut i = 0usize;
        while i < out.len() {
            let slice = &out[i..];
            let Some(prefix) = helper_call_prefixes()
                .iter()
                .find(|prefix| slice.starts_with(**prefix))
            else {
                let Some(ch) = slice.chars().next() else {
                    break;
                };
                i += ch.len_utf8();
                continue;
            };
            let call_start = i;
            let mut depth = 0i32;
            let mut end = None;
            for (off, ch) in out[call_start..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(call_start + off + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(call_end) = end else {
                break;
            };
            let call = &out[call_start..call_end];
            let rewritten = rewrite_direct_vec_helper_expr(call, enabled);
            if rewritten != call {
                out = format!("{}{}{}", &out[..call_start], rewritten, &out[call_end..]);
                changed = true;
                break;
            }
            i = call_start + prefix.len();
        }
        if !changed {
            break;
        }
    }
    out
}

fn reusable_vector_helper_names() -> &'static [&'static str] {
    &[
        "rr_index1_read_vec_floor",
        "rr_index1_read_vec",
        "rr_gather",
        "rr_index_vec_floor",
    ]
}

fn helper_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
}

fn match_balanced_call_span(expr: &str, start: usize, callee: &str) -> Option<(usize, usize)> {
    if !expr.get(start..)?.starts_with(callee) {
        return None;
    }
    let prev_ok = expr[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !helper_ident_char(ch));
    if !prev_ok {
        return None;
    }
    let open = start + callee.len();
    if expr.as_bytes().get(open).copied() != Some(b'(') {
        return None;
    }
    let mut depth = 0i32;
    for (off, ch) in expr[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, open + off + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn collect_repeated_vector_helper_calls(expr: &str) -> Vec<String> {
    let mut counts = FxHashMap::<String, usize>::default();
    for idx in expr.char_indices().map(|(idx, _)| idx) {
        for &callee in reusable_vector_helper_names() {
            let Some((start, end)) = match_balanced_call_span(expr, idx, callee) else {
                continue;
            };
            if start != idx {
                continue;
            }
            let call = expr[start..end].trim().to_string();
            *counts.entry(call).or_default() += 1;
        }
    }
    let mut repeated: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(call, count)| *count > 2 && call.len() > 12)
        .collect();
    repeated.sort_by(|(lhs_call, lhs_count), (rhs_call, rhs_count)| {
        rhs_call
            .len()
            .cmp(&lhs_call.len())
            .then_with(|| rhs_count.cmp(lhs_count))
            .then_with(|| lhs_call.cmp(rhs_call))
    });
    repeated.into_iter().map(|(call, _)| call).collect()
}

fn expr_is_exact_reusable_vector_helper(rhs: &str) -> bool {
    let rhs = rhs.trim();
    if rhs.is_empty() || rhs.contains("<-") || rhs.contains("function(") || rhs.contains('"') {
        return false;
    }
    reusable_vector_helper_names().iter().any(|callee| {
        match_balanced_call_span(rhs, 0, callee)
            .is_some_and(|(start, end)| start == 0 && end == rhs.len())
    })
}

pub(crate) fn hoist_repeated_vector_helper_calls_within_lines(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut next_cse_idx = next_generated_cse_index(&lines);

    for line in lines {
        let Some(caps) = assign_re().and_then(|re| re.captures(line.trim_end())) else {
            out.push(line);
            continue;
        };
        if line.contains("<- function") {
            out.push(line);
            continue;
        }

        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let mut rhs = caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if lhs.starts_with(".__rr_cse_") || rhs.is_empty() || !lhs.starts_with(".tachyon_exprmap") {
            out.push(line);
            continue;
        }

        let mut prefix_lines = Vec::new();
        loop {
            let Some(candidate) = collect_repeated_vector_helper_calls(&rhs)
                .into_iter()
                .next()
            else {
                break;
            };
            let temp = format!(".__rr_cse_{}", next_cse_idx);
            next_cse_idx += 1;
            prefix_lines.push(format!("{indent}{temp} <- {candidate}"));
            rhs = rhs.replace(&candidate, &temp);
        }

        out.extend(prefix_lines);
        if rhs == caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim() {
            out.push(line);
        } else {
            out.push(format!("{indent}{lhs} <- {rhs}"));
        }
    }

    out
}

pub(crate) fn rewrite_forward_exact_vector_helper_reuse(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let len = out.len();
    for idx in 0..len {
        let line_owned = out[idx].clone();
        let trimmed = line_owned.trim();
        let candidate_indent = line_owned.len() - line_owned.trim_start().len();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let rhs = caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if !(plain_ident_re().is_some_and(|re| re.is_match(&lhs)) || lhs.starts_with(".__rr_cse_"))
            || !expr_is_exact_reusable_vector_helper(&rhs)
        {
            continue;
        }
        if !lhs.starts_with(".__rr_cse_") {
            continue;
        }

        let lhs_reassigned_later = (idx + 1..out.len()).any(|scan_idx| {
            let scan_trimmed = out[scan_idx].trim();
            let scan_indent = out[scan_idx].len() - out[scan_idx].trim_start().len();
            if !scan_trimmed.is_empty() && scan_indent < candidate_indent {
                return false;
            }
            if out[scan_idx].contains("<- function")
                || scan_trimmed == "repeat {"
                || scan_trimmed.starts_with("while")
                || scan_trimmed.starts_with("for")
            {
                return false;
            }
            assign_re()
                .and_then(|re| re.captures(scan_trimmed))
                .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()))
                .is_some_and(|scan_lhs| scan_lhs == lhs)
        });

        let deps: rustc_hash::FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
        let has_tachyon_consumer = out[(idx + 1)..]
            .iter()
            .take_while(|line| {
                let trimmed = line.trim();
                !(line.contains("<- function")
                    || trimmed == "repeat {"
                    || trimmed.starts_with("while")
                    || trimmed.starts_with("for"))
            })
            .any(|line| line.contains(".tachyon_exprmap") || line.contains("rr_assign_slice("));
        if !has_tachyon_consumer {
            continue;
        }
        let mut line_no = idx + 1;
        while line_no < out.len() {
            let line_trimmed = out[line_no].trim().to_string();
            let next_indent = out[line_no].len() - out[line_no].trim_start().len();
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                break;
            }
            if out[line_no].contains("<- function")
                || line_trimmed == "repeat {"
                || line_trimmed.starts_with("while")
                || line_trimmed.starts_with("for")
            {
                break;
            }

            if let Some(next_caps) = assign_re().and_then(|re| re.captures(&line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if next_lhs == lhs {
                    break;
                }
                if next_rhs.contains(&rhs) {
                    if lhs_reassigned_later {
                        line_no += 1;
                        continue;
                    }
                    out[line_no] = out[line_no].replacen(&rhs, &lhs, usize::MAX);
                }
                if deps.contains(&next_lhs) {
                    break;
                }
                line_no += 1;
                continue;
            }

            if line_trimmed.contains(&rhs) {
                out[line_no] = out[line_no].replacen(&rhs, &lhs, usize::MAX);
            }
            if line_trimmed == "return(NULL)"
                || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
            {
                break;
            }
            line_no += 1;
        }
    }
    out
}

pub(crate) fn rewrite_forward_temp_aliases(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    for idx in 0..out.len() {
        let line_owned = out[idx].clone();
        let trimmed = line_owned.trim();
        let candidate_indent = line_owned.len() - line_owned.trim_start().len();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let rhs = caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if !lhs.starts_with(".__rr_cse_")
            || !plain_ident_re().is_some_and(|re| re.is_match(&rhs))
            || lhs == rhs
        {
            continue;
        }

        let mut line_no = idx + 1;
        while line_no < out.len() {
            let line_trimmed = out[line_no].trim().to_string();
            let next_indent = out[line_no].len() - out[line_no].trim_start().len();
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                break;
            }
            if out[line_no].contains("<- function")
                || line_trimmed == "repeat {"
                || line_trimmed.starts_with("while")
                || line_trimmed.starts_with("for")
            {
                break;
            }

            if let Some(next_caps) = assign_re().and_then(|re| re.captures(&line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if next_lhs == lhs || next_lhs == rhs {
                    break;
                }
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if expr_idents(&next_rhs).iter().any(|ident| ident == &lhs) {
                    out[line_no] = out[line_no].replacen(&lhs, &rhs, usize::MAX);
                }
                line_no += 1;
                continue;
            }

            if expr_idents(&line_trimmed).iter().any(|ident| ident == &lhs) {
                out[line_no] = out[line_no].replacen(&lhs, &rhs, usize::MAX);
            }
            if line_trimmed == "return(NULL)"
                || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
            {
                break;
            }
            line_no += 1;
        }
    }
    out
}
