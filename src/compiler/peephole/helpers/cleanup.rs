use super::super::{
    IDENT_PATTERN, assign_re, assign_slice_re, compile_regex, expr_idents, find_matching_block_end,
    is_dead_parenthesized_eval_line, is_dead_plain_ident_eval_line, literal_one_re, plain_ident_re,
    previous_non_empty_line, scalar_lit_re, split_top_level_args,
};
use super::helper_calls::parse_function_header;
use rustc_hash::FxHashMap;

pub(in super::super) fn strip_empty_else_blocks(lines: Vec<String>) -> Vec<String> {
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
    out
}

pub(in super::super) fn strip_dead_simple_eval_lines(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| {
            !is_dead_plain_ident_eval_line(line) && !is_dead_parenthesized_eval_line(line)
        })
        .collect()
}

pub(in super::super) fn strip_noop_self_assignments(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| {
            let trimmed = line.trim();
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                return true;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            lhs != rhs
        })
        .collect()
}

pub(in super::super) fn strip_unused_arg_aliases(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut fn_start = 0usize;
    while fn_start < out.len() {
        while fn_start < out.len() && !out[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= out.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&out, fn_start) else {
            break;
        };
        let body_start = fn_start + 1;
        let mut idx = body_start;
        while idx < fn_end {
            let trimmed = out[idx].trim().to_string();
            if trimmed.is_empty() || trimmed == "{" {
                idx += 1;
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !lhs.starts_with(".arg_") || !plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                break;
            }

            let mut used = false;
            for later_line in out.iter().take(fn_end).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                    used = true;
                    break;
                }
            }
            if !used {
                out[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }
    out
}

pub(in super::super) fn collapse_trivial_scalar_clamp_wrappers(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut fn_start = 0usize;
    while fn_start < out.len() {
        while fn_start < out.len() && !out[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= out.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&out, fn_start) else {
            break;
        };
        let Some((_, _params)) = parse_function_header(&out[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body: Vec<String> = out
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "{" && line != "}")
            .collect();
        if body.len() != 6 {
            fn_start = fn_end + 1;
            continue;
        }
        let Some((tmp, init_expr)) = body[0]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some((assign_lo_lhs, lo_expr)) = body[2]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some((assign_hi_lhs, hi_expr)) = body[4]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        if assign_lo_lhs != tmp || assign_hi_lhs != tmp || body[5] != format!("return({tmp})") {
            fn_start = fn_end + 1;
            continue;
        }
        let first_guard_ok = body[1] == format!("if (({init_expr} < {lo_expr})) {{")
            || body[1] == format!("if (({tmp} < {lo_expr})) {{");
        let second_guard_ok = body[3] == format!("if (({tmp} > {hi_expr})) {{");
        if !first_guard_ok || !second_guard_ok {
            fn_start = fn_end + 1;
            continue;
        }

        let return_idx = previous_non_empty_line(&out, fn_end).unwrap_or(fn_end);
        let indent_len = out[return_idx].len() - out[return_idx].trim_start().len();
        let indent = out[return_idx][..indent_len].to_string();
        let open_idx = fn_start + 1;
        if open_idx < out.len() {
            out[open_idx] = "{".to_string();
        }
        if open_idx + 1 < out.len() {
            out[open_idx + 1] =
                format!("{indent}return(pmin(pmax({init_expr}, {lo_expr}), {hi_expr}))");
        }
        for line in out.iter_mut().take(fn_end).skip(open_idx + 2) {
            line.clear();
        }
        fn_start = fn_end + 1;
    }
    out
}

pub(in super::super) fn collapse_singleton_assign_slice_scalar_edits(
    lines: Vec<String>,
) -> Vec<String> {
    fn scalar_rhs_from_singleton_rest(rest: &str) -> Option<String> {
        let trimmed = rest.trim();
        if let Some(inner) = trimmed
            .strip_prefix("rep.int(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let args = split_top_level_args(inner)?;
            if args.len() == 2 && literal_one_re().is_some_and(|re| re.is_match(args[1].trim())) {
                return Some(args[0].trim().to_string());
            }
        }

        (scalar_lit_re().is_some_and(|re| re.is_match(trimmed))
            || plain_ident_re().is_some_and(|re| re.is_match(trimmed)))
        .then_some(trimmed.to_string())
    }

    let mut out = lines;
    for line in &mut out {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
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
        if lhs != dest || start != end {
            continue;
        }
        let Some(scalar_rhs) = scalar_rhs_from_singleton_rest(rest) else {
            continue;
        };
        *line = format!("{indent}{lhs} <- replace({dest}, {start}, {scalar_rhs})");
    }
    out
}

pub(in super::super) fn collapse_trivial_dot_product_wrappers(lines: Vec<String>) -> Vec<String> {
    fn is_zero_literal(expr: &str) -> bool {
        matches!(expr.trim(), "0" | "0L" | "0.0")
    }

    fn is_one_literal(expr: &str) -> bool {
        matches!(expr.trim(), "1" | "1L" | "1.0")
    }

    fn parse_accumulate_product_line(line: &str, acc: &str) -> Option<(String, String, String)> {
        let pattern = format!(
            r"^(?P<lhs>{}) <- \({} \+ \((?P<a>{})\[(?P<idx_a>{})\] \* (?P<b>{})\[(?P<idx_b>{})\]\)\)$",
            IDENT_PATTERN,
            regex::escape(acc),
            IDENT_PATTERN,
            IDENT_PATTERN,
            IDENT_PATTERN,
            IDENT_PATTERN
        );
        let caps = compile_regex(pattern)?.captures(line.trim())?;
        let lhs = caps.name("lhs")?.as_str().trim();
        let lhs_vec = caps.name("a")?.as_str().trim();
        let rhs_vec = caps.name("b")?.as_str().trim();
        let idx_a = caps.name("idx_a")?.as_str().trim();
        let idx_b = caps.name("idx_b")?.as_str().trim();
        if lhs != acc || idx_a != idx_b {
            return None;
        }
        Some((lhs_vec.to_string(), rhs_vec.to_string(), idx_a.to_string()))
    }

    let mut out = lines;
    let mut fn_start = 0usize;
    while fn_start < out.len() {
        while fn_start < out.len() && !out[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= out.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&out, fn_start) else {
            break;
        };
        let Some((_, params)) = parse_function_header(&out[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if params.len() != 3 {
            fn_start = fn_end + 1;
            continue;
        }

        let body: Vec<String> = out
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "{" && line != "}")
            .collect();
        if body.len() < 7 {
            fn_start = fn_end + 1;
            continue;
        }

        let mut aliases: FxHashMap<String, String> = params
            .iter()
            .cloned()
            .map(|param| (param.clone(), param))
            .collect();
        let mut idx = 0usize;
        while idx < body.len() {
            let Some((lhs, rhs)) = body[idx]
                .split_once(" <- ")
                .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
            else {
                break;
            };
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                break;
            }
            if params.iter().any(|param| param == rhs) {
                aliases.insert(lhs.to_string(), rhs.to_string());
                idx += 1;
                continue;
            }
            break;
        }

        if idx + 6 >= body.len() {
            fn_start = fn_end + 1;
            continue;
        }
        let Some((acc, init_expr)) = body[idx]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some((iter_var, iter_init)) = body[idx + 1]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        if !plain_ident_re().is_some_and(|re| re.is_match(acc))
            || !plain_ident_re().is_some_and(|re| re.is_match(iter_var))
            || !is_zero_literal(init_expr)
            || !is_one_literal(iter_init)
            || body[idx + 2] != "repeat {"
        {
            fn_start = fn_end + 1;
            continue;
        }

        let guard_line = format!("if (!({iter_var} <= {})) break", params[2]);
        let guard_line_with_alias = aliases.iter().find_map(|(alias, base)| {
            (base == &params[2] && alias != &params[2])
                .then(|| format!("if (!({iter_var} <= {alias})) break"))
        });
        if body[idx + 3] != guard_line
            && guard_line_with_alias.as_deref() != Some(body[idx + 3].as_str())
        {
            fn_start = fn_end + 1;
            continue;
        }

        let mut product_idx = idx + 4;
        let mut index_ref = iter_var.to_string();
        if let Some((alias_lhs, alias_rhs)) = body[product_idx]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
            && alias_rhs == iter_var
            && plain_ident_re().is_some_and(|re| re.is_match(alias_lhs))
        {
            index_ref = alias_lhs.to_string();
            product_idx += 1;
        }
        if product_idx + 2 >= body.len() {
            fn_start = fn_end + 1;
            continue;
        }
        let Some((lhs_vec, rhs_vec, vec_index_ref)) =
            parse_accumulate_product_line(&body[product_idx], acc)
        else {
            fn_start = fn_end + 1;
            continue;
        };
        let resolved_lhs = aliases
            .get(&lhs_vec)
            .map(String::as_str)
            .unwrap_or(lhs_vec.as_str());
        let resolved_rhs = aliases
            .get(&rhs_vec)
            .map(String::as_str)
            .unwrap_or(rhs_vec.as_str());
        if vec_index_ref != index_ref
            || resolved_lhs != params[0]
            || resolved_rhs != params[1]
            || !matches!(
                body[product_idx + 1].as_str(),
                line if line == format!("{iter_var} <- ({iter_var} + 1)")
                    || line == format!("{iter_var} <- ({iter_var} + 1L)")
                    || line == format!("{iter_var} <- ({iter_var} + 1.0)")
            )
            || body[product_idx + 2] != "next"
            || body.last().map(String::as_str) != Some(&format!("return({acc})"))
            || body.len() != product_idx + 4
        {
            fn_start = fn_end + 1;
            continue;
        }

        let return_idx = previous_non_empty_line(&out, fn_end).unwrap_or(fn_end);
        let indent_len = out[return_idx].len() - out[return_idx].trim_start().len();
        let indent = out[return_idx][..indent_len].to_string();
        let open_idx = fn_start + 1;
        if open_idx < out.len() {
            out[open_idx] = "{".to_string();
        }
        if open_idx + 1 < out.len() {
            out[open_idx + 1] = format!(
                "{indent}return(sum(({}[seq_len({})] * {}[seq_len({})])))",
                params[0], params[2], params[1], params[2]
            );
        }
        for line in out.iter_mut().take(fn_end).skip(open_idx + 2) {
            line.clear();
        }
        fn_start = fn_end + 1;
    }
    out
}

pub(in super::super) fn collapse_inlined_copy_vec_sequences(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let len = out.len();
    for idx in 0..len.saturating_sub(4) {
        let l0 = out[idx].trim().to_string();
        let l1 = out[idx + 1].trim().to_string();
        let l2 = out[idx + 2].trim().to_string();
        let l3 = out[idx + 3].trim().to_string();
        let l4 = out[idx + 4].trim().to_string();
        let Some(c0) = assign_re().and_then(|re| re.captures(&l0)) else {
            continue;
        };
        let Some(c1) = assign_re().and_then(|re| re.captures(&l1)) else {
            continue;
        };
        let Some(c2) = assign_re().and_then(|re| re.captures(&l2)) else {
            continue;
        };
        let Some(c3) = assign_re().and_then(|re| re.captures(&l3)) else {
            continue;
        };
        let Some(c4) = assign_re().and_then(|re| re.captures(&l4)) else {
            continue;
        };
        let n_var = c0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let n_rhs = c0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let out_var = c1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let out_rhs = c1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let i_var = c2.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let i_rhs = c2.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let out_replay_lhs = c3.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let src_rhs = c3.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let target_var = c4.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let target_rhs = c4.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(src_var) = ({
            if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(src_rhs)) {
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
                if dest == out_var
                    && start == i_var
                    && end == n_var
                    && plain_ident_re().is_some_and(|re| re.is_match(rest))
                {
                    Some(rest.to_string())
                } else {
                    None
                }
            } else if plain_ident_re().is_some_and(|re| re.is_match(src_rhs)) {
                Some(src_rhs.to_string())
            } else {
                None
            }
        }) else {
            continue;
        };
        if !n_var.starts_with("inlined_")
            || !out_var.starts_with("inlined_")
            || !i_var.starts_with("inlined_")
            || out_replay_lhs != out_var
            || (target_rhs != out_var && target_rhs != src_var)
            || !literal_one_re().is_some_and(|re| re.is_match(i_rhs))
            || !n_rhs.starts_with("length(")
            || !out_rhs.starts_with("rep.int(0, ")
        {
            continue;
        }

        let mut final_assign_idx = None;
        for (search_idx, line) in out.iter().enumerate().skip(idx + 5) {
            let trimmed = line.trim();
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                continue;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
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
            if lhs == src_var
                && dest == out_var
                && start == i_var
                && end == n_var
                && rest == src_var
            {
                final_assign_idx = Some(search_idx);
                break;
            }
        }
        let Some(final_idx) = final_assign_idx else {
            continue;
        };
        let indent_len = out[idx + 4].len() - out[idx + 4].trim_start().len();
        let indent = out[idx + 4][..indent_len].to_string();
        out[idx].clear();
        out[idx + 1].clear();
        out[idx + 2].clear();
        out[idx + 3].clear();
        out[idx + 4] = format!("{indent}{target_var} <- {src_var}");
        let final_indent_len = out[final_idx].len() - out[final_idx].trim_start().len();
        let final_indent = out[final_idx][..final_indent_len].to_string();
        out[final_idx] = format!("{final_indent}{src_var} <- {target_var}");
    }
    out
}
