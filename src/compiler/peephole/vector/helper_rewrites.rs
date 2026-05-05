use super::super::patterns::{
    assign_re, expr_idents, next_generated_cse_index, split_top_level_args,
};
use super::*;
use crate::compiler::pipeline::{line_contains_symbol, replace_symbol_occurrences};
use rustc_hash::{FxHashMap, FxHashSet};

pub(crate) fn helper_call_prefixes() -> &'static [&'static str] {
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

pub(crate) fn reusable_vector_helper_names() -> &'static [&'static str] {
    &[
        "rr_index1_read_vec_floor",
        "rr_index1_read_vec",
        "rr_gather",
        "rr_index_vec_floor",
    ]
}

pub(crate) fn helper_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
}

pub(crate) fn match_balanced_call_span(
    expr: &str,
    start: usize,
    callee: &str,
) -> Option<(usize, usize)> {
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

pub(crate) fn text_might_mention_ident(text: &str, ident: &str) -> bool {
    if ident.is_empty() || !text.contains(ident) {
        return false;
    }
    let bytes = text.as_bytes();
    let ident_bytes = ident.as_bytes();
    let mut start = 0usize;
    while let Some(off) = text[start..].find(ident) {
        let pos = start + off;
        let before_ok = pos == 0 || !helper_ident_char(bytes[pos - 1] as char);
        let after = pos + ident_bytes.len();
        let after_ok = after >= bytes.len() || !helper_ident_char(bytes[after] as char);
        if before_ok && after_ok {
            return true;
        }
        start = pos + ident_bytes.len();
    }
    false
}

pub(crate) fn collect_repeated_vector_helper_calls_with_counts(expr: &str) -> Vec<(String, usize)> {
    let mut counts = FxHashMap::<String, usize>::default();
    for idx in expr.char_indices().map(|(idx, _)| idx) {
        for &callee in reusable_vector_helper_names() {
            let Some((start, end)) = match_balanced_call_span(expr, idx, callee) else {
                continue;
            };
            let call = expr[start..end].trim().to_string();
            *counts.entry(call).or_default() += 1;
        }
    }
    let mut repeated: Vec<(String, usize)> = counts.into_iter().collect();
    repeated.sort_by(|(lhs_call, lhs_count), (rhs_call, rhs_count)| {
        rhs_call
            .len()
            .cmp(&lhs_call.len())
            .then_with(|| rhs_count.cmp(lhs_count))
            .then_with(|| lhs_call.cmp(rhs_call))
    });
    repeated
}

pub(crate) fn collect_repeated_vector_helper_calls(expr: &str) -> Vec<String> {
    collect_repeated_vector_helper_calls_with_counts(expr)
        .into_iter()
        .filter(|(call, count)| *count > 2 && call.len() > 12)
        .map(|(call, _)| call)
        .collect()
}

pub(crate) fn estimated_cse_savings(indent: &str, call: &str, count: usize, temp: &str) -> isize {
    let old_bytes = count.saturating_mul(call.len()) as isize;
    let replacement_bytes = count.saturating_mul(temp.len()) as isize;
    let temp_assignment_bytes = indent.len() as isize
        + temp.len() as isize
        + call.len() as isize
        + " <- ".len() as isize
        + 1;
    old_bytes - replacement_bytes - temp_assignment_bytes
}

pub(crate) fn best_profitable_vector_helper_cse(
    expr: &str,
    indent: &str,
    temp: &str,
    min_count: usize,
    min_savings: isize,
) -> Option<String> {
    collect_repeated_vector_helper_calls_with_counts(expr)
        .into_iter()
        .filter(|(_, count)| *count >= min_count)
        .filter_map(|(call, count)| {
            let savings = estimated_cse_savings(indent, &call, count, temp);
            (savings >= min_savings).then_some((call, savings, count))
        })
        .max_by(
            |(lhs_call, lhs_savings, lhs_count), (rhs_call, rhs_savings, rhs_count)| {
                lhs_savings
                    .cmp(rhs_savings)
                    .then_with(|| lhs_call.len().cmp(&rhs_call.len()))
                    .then_with(|| lhs_count.cmp(rhs_count))
            },
        )
        .map(|(call, _, _)| call)
}

pub(crate) fn expr_is_exact_reusable_vector_helper(rhs: &str) -> bool {
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
    hoist_repeated_vector_helper_calls_with_options_and_map(lines, false, usize::MAX, 1, 0).0
}

pub(crate) fn hoist_aggressive_repeated_vector_helper_calls_within_lines(
    lines: Vec<String>,
) -> Vec<String> {
    hoist_aggressive_repeated_vector_helper_calls_within_lines_with_map(lines).0
}

pub(crate) fn hoist_aggressive_repeated_vector_helper_calls_within_lines_with_map(
    lines: Vec<String>,
) -> (Vec<String>, Vec<u32>) {
    hoist_repeated_vector_helper_calls_with_options_and_map(lines, true, 16, 2, 8)
}

pub(crate) fn hoist_o3_repeated_index_vec_floor_calls_across_lines(
    lines: Vec<String>,
) -> Vec<String> {
    hoist_o3_repeated_index_vec_floor_calls_across_lines_with_map(lines).0
}

pub(crate) fn hoist_o3_repeated_index_vec_floor_calls_across_lines_with_map(
    lines: Vec<String>,
) -> (Vec<String>, Vec<u32>) {
    let mut out = Vec::with_capacity(lines.len());
    let mut line_map = vec![0u32; lines.len()];
    let mut next_cse_idx = next_generated_cse_index(&lines);
    let mut cursor = 0usize;

    while cursor < lines.len() {
        let Some(region_indent) = straight_line_o3_index_cse_region_indent(&lines[cursor]) else {
            line_map[cursor] = (out.len() + 1) as u32;
            out.push(lines[cursor].clone());
            cursor += 1;
            continue;
        };

        let start = cursor;
        cursor += 1;
        while cursor < lines.len()
            && straight_line_o3_index_cse_region_indent(&lines[cursor]) == Some(region_indent)
        {
            cursor += 1;
        }

        let out_base = out.len();
        let (region, region_map, next_idx) =
            hoist_index_vec_floor_calls_in_region_with_map(&lines[start..cursor], next_cse_idx);
        next_cse_idx = next_idx;
        for (offset, mapped) in region_map.into_iter().enumerate() {
            line_map[start + offset] = (out_base as u32).saturating_add(mapped);
        }
        out.extend(region);
    }

    (out, line_map)
}

pub(crate) fn define_missing_o3_semantic_index_temps_with_map(
    lines: Vec<String>,
) -> (Vec<String>, Vec<u32>) {
    if !lines.iter().any(|line| line.contains("idx_")) {
        let line_map = (1..=lines.len() as u32).collect::<Vec<_>>();
        return (lines, line_map);
    }

    let mut out = Vec::with_capacity(lines.len());
    let mut line_map = vec![0u32; lines.len()];
    let mut defined = FxHashSet::<String>::default();
    let mut repair_active = false;
    let mut fn_depth = 0usize;
    let mut seen_fn_open = false;

    for (idx, line) in lines.into_iter().enumerate() {
        if let Some((_fn_name, fn_params)) = parse_o3_index_repair_function_header(&line) {
            defined = fn_params.iter().cloned().collect();
            repair_active = true;
            fn_depth = 0;
            seen_fn_open = false;
            line_map[idx] = (out.len() + 1) as u32;
            out.push(line);
            continue;
        }

        if repair_active {
            let lhs = assign_re()
                .and_then(|re| re.captures(line.trim()))
                .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()));
            let indent = line
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            let mut missing_gathers = expr_idents(&line)
                .into_iter()
                .filter(|ident| ident.starts_with("n_"))
                .filter(|ident| lhs.as_deref() != Some(ident.as_str()))
                .filter(|ident| !defined.contains(ident))
                .filter_map(|ident| {
                    semantic_gather_repair_for_missing_temp(&ident, &defined)
                        .map(|expr| (ident, expr))
                })
                .collect::<Vec<_>>();
            missing_gathers.sort();
            missing_gathers.dedup();
            for (ident, expr) in missing_gathers {
                out.push(format!("{indent}{ident} <- {expr}"));
                defined.insert(ident);
            }

            let mut missing = expr_idents(&line)
                .into_iter()
                .filter(|ident| ident.starts_with("idx_"))
                .filter(|ident| lhs.as_deref() != Some(ident.as_str()))
                .filter(|ident| !defined.contains(ident))
                .filter_map(|ident| {
                    semantic_index_repair_for_missing_temp(&ident, &defined)
                        .map(|expr| (ident, expr))
                })
                .collect::<Vec<_>>();
            missing.sort();
            missing.dedup();
            for (ident, expr) in missing {
                out.push(format!("{indent}{ident} <- rr_index_vec_floor({expr})"));
                defined.insert(ident);
            }
        }

        line_map[idx] = (out.len() + 1) as u32;
        if let Some(lhs) = assign_re()
            .and_then(|re| re.captures(line.trim()))
            .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()))
        {
            defined.insert(lhs);
        }
        let (opens, closes) = count_unquoted_braces_local(&line);
        if repair_active {
            if opens > 0 {
                seen_fn_open = true;
            }
            fn_depth = fn_depth.saturating_add(opens);
        }
        out.push(line);
        if repair_active {
            fn_depth = fn_depth.saturating_sub(closes);
            if seen_fn_open && fn_depth == 0 {
                repair_active = false;
                defined.clear();
            }
        }
    }

    (out, line_map)
}

pub(crate) fn semantic_gather_repair_for_missing_temp(
    ident: &str,
    available: &FxHashSet<String>,
) -> Option<String> {
    let suffix = ident.strip_prefix("n_")?;
    let (base_suffix, index_suffix) = suffix.rsplit_once('_')?;
    if base_suffix.is_empty() || index_suffix.is_empty() {
        return None;
    }
    let base_param = format!("n_{base_suffix}");
    let index_param = format!("n_{index_suffix}");
    if !available.contains(&base_param) || !available.contains(&index_param) {
        return None;
    }
    Some(format!("rr_gather({base_param}, {index_param})"))
}

pub(crate) fn semantic_index_repair_for_missing_temp(
    ident: &str,
    available: &FxHashSet<String>,
) -> Option<String> {
    let suffix = ident.strip_prefix("idx_")?;
    for candidate_suffix in semantic_index_repair_suffix_candidates(suffix) {
        for candidate in [
            format!("n_{candidate_suffix}"),
            format!("adj_{candidate_suffix}"),
            candidate_suffix.clone(),
        ] {
            if available.contains(&candidate) {
                return Some(candidate);
            }
        }
    }
    None
}

pub(crate) fn semantic_index_repair_suffix_candidates(suffix: &str) -> Vec<String> {
    let mut out = vec![suffix.to_string()];
    let mut current = suffix;
    while let Some((base, numeric)) = current.rsplit_once('_') {
        if base.is_empty() || numeric.is_empty() || !numeric.chars().all(|ch| ch.is_ascii_digit()) {
            break;
        }
        out.push(base.to_string());
        current = base;
    }
    out
}

pub(crate) fn parse_o3_index_repair_function_header(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let (name, rest) = trimmed.split_once("<- function(")?;
    let args_inner = rest.split_once(')')?.0.trim();
    let params = if args_inner.is_empty() {
        Vec::new()
    } else {
        split_top_level_args(args_inner)?
            .into_iter()
            .map(|arg| arg.trim().to_string())
            .collect()
    };
    Some((name.trim().to_string(), params))
}

pub(crate) fn materialize_o3_large_repeated_arithmetic_subexpressions(
    lines: Vec<String>,
) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut next_temp_idx = next_generated_o3_arithmetic_temp_index(&lines);

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
        if rhs.len() < 420 {
            out.push(line);
            continue;
        }

        let original_rhs = rhs.clone();
        let mut prefix_lines = Vec::new();
        let use_expr_temps = lhs.starts_with(".__rr_cse_");
        while prefix_lines.len() < 32 && rhs.len() >= 700 {
            let temp = generated_o3_arithmetic_temp(use_expr_temps, next_temp_idx);
            let Some(candidate) = best_profitable_arithmetic_subexpr_cse(&rhs, indent, &temp)
            else {
                break;
            };
            next_temp_idx += 1;
            prefix_lines.push(format!("{indent}{temp} <- {candidate}"));
            rhs = rhs.replace(&candidate, &temp);
        }
        rhs = materialize_large_root_arithmetic_expr(
            rhs,
            indent,
            &mut next_temp_idx,
            &mut prefix_lines,
            use_expr_temps,
        );

        out.extend(prefix_lines);
        if rhs == original_rhs {
            out.push(line);
        } else {
            out.push(format!("{indent}{lhs} <- {rhs}"));
        }
    }

    out
}

pub(crate) fn materialize_large_root_arithmetic_expr(
    expr: String,
    indent: &str,
    next_temp_idx: &mut usize,
    hoists: &mut Vec<String>,
    use_expr_temps: bool,
) -> String {
    materialize_large_root_arithmetic_expr_inner(
        expr,
        indent,
        next_temp_idx,
        hoists,
        use_expr_temps,
        0,
    )
}

pub(crate) fn materialize_large_root_arithmetic_expr_inner(
    expr: String,
    indent: &str,
    next_temp_idx: &mut usize,
    hoists: &mut Vec<String>,
    use_expr_temps: bool,
    depth: usize,
) -> String {
    if expr.len() < 420 || depth >= 8 {
        return expr;
    }
    let trimmed = expr.trim();
    let inner = strip_balanced_outer_parens(trimmed).unwrap_or(trimmed);
    let Some((op_pos, op)) = find_top_level_arithmetic_op(inner) else {
        return expr;
    };
    let lhs = inner[..op_pos].trim();
    let rhs = inner[op_pos + op.len_utf8()..].trim();
    if lhs.is_empty() || rhs.is_empty() {
        return expr;
    }

    let lhs = materialize_large_arithmetic_side(
        lhs,
        indent,
        next_temp_idx,
        hoists,
        use_expr_temps,
        depth.saturating_add(1),
    );
    let rhs = materialize_large_arithmetic_side(
        rhs,
        indent,
        next_temp_idx,
        hoists,
        use_expr_temps,
        depth.saturating_add(1),
    );
    format!("({lhs} {op} {rhs})")
}

pub(crate) fn materialize_large_arithmetic_side(
    side: &str,
    indent: &str,
    next_temp_idx: &mut usize,
    hoists: &mut Vec<String>,
    use_expr_temps: bool,
    depth: usize,
) -> String {
    if side.len() < 180 {
        return side.to_string();
    }
    let reduced = materialize_large_root_arithmetic_expr_inner(
        side.to_string(),
        indent,
        next_temp_idx,
        hoists,
        use_expr_temps,
        depth,
    );
    let temp = generated_o3_arithmetic_temp(use_expr_temps, *next_temp_idx);
    *next_temp_idx += 1;
    hoists.push(format!("{indent}{temp} <- {reduced}"));
    temp
}

pub(crate) fn generated_o3_arithmetic_temp(use_expr_temps: bool, idx: usize) -> String {
    if use_expr_temps {
        format!(".__rr_expr_{}", idx)
    } else {
        format!(".__rr_cse_{}", idx)
    }
}

pub(crate) fn next_generated_o3_arithmetic_temp_index(lines: &[String]) -> usize {
    lines
        .iter()
        .flat_map(|line| expr_idents(line))
        .filter_map(|ident| {
            ident
                .strip_prefix(".__rr_cse_")
                .or_else(|| ident.strip_prefix(".__rr_expr_"))
                .and_then(|idx| idx.parse::<usize>().ok())
        })
        .max()
        .map_or(0, |idx| idx + 1)
}

pub(crate) fn strip_balanced_outer_parens(expr: &str) -> Option<&str> {
    if !expr.starts_with('(') || !expr.ends_with(')') {
        return None;
    }
    let mut depth = 0i32;
    let mut in_single = false;
    let mut in_double = false;
    for (idx, ch) in expr.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '(' if !in_single && !in_double => depth += 1,
            ')' if !in_single && !in_double => {
                depth -= 1;
                if depth == 0 && idx + ch.len_utf8() != expr.len() {
                    return None;
                }
            }
            _ => {}
        }
    }
    (depth == 0).then_some(expr[1..expr.len() - 1].trim())
}

pub(crate) fn find_top_level_arithmetic_op(expr: &str) -> Option<(usize, char)> {
    for ops in [['+', '-'], ['*', '/']] {
        let mut depth = 0i32;
        let mut in_single = false;
        let mut in_double = false;
        let mut found = None;
        for (idx, ch) in expr.char_indices() {
            match ch {
                '\'' if !in_double => in_single = !in_single,
                '"' if !in_single => in_double = !in_double,
                '(' if !in_single && !in_double => depth += 1,
                ')' if !in_single && !in_double => depth -= 1,
                _ if depth == 0 && !in_single && !in_double && ops.contains(&ch) => {
                    let before = expr[..idx].chars().next_back();
                    let after = expr[idx + ch.len_utf8()..].chars().next();
                    if before.is_some_and(|c| c.is_whitespace())
                        && after.is_some_and(|c| c.is_whitespace())
                    {
                        found = Some((idx, ch));
                    }
                }
                _ => {}
            }
        }
        if found.is_some() {
            return found;
        }
    }
    None
}

pub(crate) fn materialize_o3_large_ifelse_branches(lines: Vec<String>) -> Vec<String> {
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
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if rhs.len() < 420 {
            out.push(line);
            continue;
        }
        let Some(inner) = rhs
            .strip_prefix("ifelse(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            out.push(line);
            continue;
        };
        let Some(args) = split_top_level_args(inner) else {
            out.push(line);
            continue;
        };
        if args.len() != 3 {
            out.push(line);
            continue;
        }

        let mut next_args = Vec::with_capacity(3);
        next_args.push(args[0].trim().to_string());
        for branch in args.iter().skip(1) {
            let branch = branch.trim();
            if branch.len() >= 180 {
                let temp = format!(".__rr_cse_{}", next_cse_idx);
                next_cse_idx += 1;
                out.push(format!("{indent}{temp} <- {branch}"));
                next_args.push(temp);
            } else {
                next_args.push(branch.to_string());
            }
        }
        out.push(format!(
            "{indent}{lhs} <- ifelse({}, {}, {})",
            next_args[0], next_args[1], next_args[2]
        ));
    }

    out
}

pub(crate) fn semanticize_o3_vector_cse_temp_names(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut aliases = FxHashMap::<String, String>::default();
    let mut used_names: FxHashSet<String> = lines
        .iter()
        .flat_map(|line| expr_idents(line.trim()))
        .collect();

    for line in lines {
        let mut rewritten = line;
        let mut alias_pairs = aliases
            .iter()
            .map(|(from, to)| (from.clone(), to.clone()))
            .collect::<Vec<_>>();
        alias_pairs.sort_by_key(|(lhs, _)| std::cmp::Reverse(lhs.len()));
        for (from, to) in alias_pairs {
            if line_contains_symbol(&rewritten, &from) {
                rewritten = replace_symbol_occurrences(&rewritten, &from, &to);
            }
        }

        let Some(caps) = assign_re().and_then(|re| re.captures(rewritten.trim_end())) else {
            out.push(rewritten);
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".__rr_cse_") {
            out.push(rewritten);
            continue;
        }
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(base_name) = semantic_o3_cse_name_for_rhs(rhs) else {
            out.push(rewritten);
            continue;
        };
        let semantic = unique_semantic_temp_name(&base_name, &mut used_names);
        aliases.insert(lhs.to_string(), semantic.clone());
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        out.push(format!("{indent}{semantic} <- {rhs}"));
    }

    out
}

pub(crate) fn materialize_o3_semantic_gather_subexpressions(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut used_names: FxHashSet<String> = lines
        .iter()
        .flat_map(|line| expr_idents(line.trim()))
        .collect();

    for line in lines {
        if line.contains("<- function") {
            out.push(line);
            continue;
        }

        if let Some(caps) = assign_re().and_then(|re| re.captures(line.trim_end())) {
            let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if lhs.starts_with(".__rr_cse_") || rhs.len() < 180 || !rhs.contains("rr_gather(") {
                out.push(line);
                continue;
            }

            let (hoists, rhs) = materialize_semantic_gather_rhs(rhs, indent, &mut used_names);
            out.extend(hoists);
            out.push(format!("{indent}{lhs} <- {rhs}"));
            continue;
        }

        let Some((indent, rhs)) = parse_return_expr_line(&line) else {
            out.push(line);
            continue;
        };
        if rhs.len() < 180 || !rhs.contains("rr_gather(") {
            out.push(line);
            continue;
        }
        let (hoists, rhs) = materialize_semantic_gather_rhs(rhs, indent, &mut used_names);
        out.extend(hoists);
        out.push(format!("{indent}return({rhs})"));
    }

    out
}

pub(crate) fn materialize_semantic_gather_rhs(
    mut rhs: String,
    indent: &str,
    used_names: &mut FxHashSet<String>,
) -> (Vec<String>, String) {
    let mut hoists = Vec::new();
    let original_len = rhs.len();
    let mut candidates = collect_vector_calls_by_name(&rhs, "rr_gather");
    candidates.sort_by(|lhs, rhs| rhs.len().cmp(&lhs.len()).then_with(|| lhs.cmp(rhs)));
    candidates.dedup();
    for candidate in candidates {
        if hoists.len() >= 12 || !rhs.contains(&candidate) {
            continue;
        }
        let count = rhs.matches(&candidate).count();
        if count < 2 && candidate.len() < 24 && original_len < 700 {
            continue;
        }
        let Some(base_name) = semantic_o3_cse_name_for_rhs(&candidate) else {
            continue;
        };
        let temp = unique_semantic_temp_name(&base_name, used_names);
        hoists.push(format!("{indent}{temp} <- {candidate}"));
        rhs = rhs.replace(&candidate, &temp);
    }
    (hoists, rhs)
}
