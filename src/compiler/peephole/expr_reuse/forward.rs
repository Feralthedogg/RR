use super::super::{
    assign_re, collect_prologue_arg_aliases, expr_has_only_pure_calls, expr_idents, ident_re,
    is_peephole_temp, plain_ident_re, scalar_lit_re,
};
use crate::compiler::peephole::alias::normalize_expr_with_aliases;
use regex::Captures;
use rustc_hash::{FxHashMap, FxHashSet};

pub(in super::super) fn rewrite_forward_simple_alias_guards(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
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
        if is_peephole_temp(&lhs)
            || !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
            || !plain_ident_re().is_some_and(|re| re.is_match(&rhs))
            || lhs == rhs
        {
            continue;
        }

        let alias_map = FxHashMap::from_iter([(lhs.clone(), rhs.clone())]);
        let mut replaced_any = false;
        let mut unsafe_use_seen = false;

        let mut relative_depth = 0i32;
        for line in out.iter_mut().skip(idx + 1) {
            let line_trimmed = line.trim();
            let next_indent = line.len() - line.trim_start().len();
            if line.contains("<- function") {
                break;
            }
            if line_trimmed == "}" {
                if relative_depth == 0 {
                    break;
                }
                relative_depth -= 1;
                continue;
            }
            if line_trimmed.starts_with("} else") {
                if relative_depth == 0 {
                    break;
                }
                continue;
            }
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    unsafe_use_seen = true;
                    break;
                }
                if next_lhs == rhs {
                    break;
                }
                if expr_idents(next_rhs).iter().any(|ident| ident == &lhs) {
                    unsafe_use_seen = true;
                    break;
                }
                continue;
            }

            if line_trimmed.starts_with("if ") && line_trimmed.ends_with('{') {
                let rewritten = ident_re
                    .replace_all(line, |caps: &Captures<'_>| {
                        let ident = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                        alias_map
                            .get(ident)
                            .cloned()
                            .unwrap_or_else(|| ident.to_string())
                    })
                    .to_string();
                if rewritten != *line {
                    *line = rewritten;
                    replaced_any = true;
                }
                relative_depth += 1;
                continue;
            }
            if line_trimmed.ends_with('{') {
                relative_depth += 1;
            }

            if expr_idents(line_trimmed).iter().any(|ident| ident == &lhs) {
                unsafe_use_seen = true;
                break;
            }

            if line_trimmed == "return(NULL)"
                || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
            {
                break;
            }
        }

        if replaced_any && !unsafe_use_seen {
            out[idx].clear();
        }
    }
    out
}

pub(in super::super) fn rewrite_loop_index_alias_ii(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs != "ii" || rhs != "i" {
            continue;
        }

        let mut replaced_any = false;
        for line in out.iter_mut().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == "ii" || next_lhs == "i" {
                    break;
                }
            }
            if !expr_idents(line_trimmed).iter().any(|ident| ident == "ii") {
                continue;
            }
            let rewritten = ident_re
                .replace_all(line, |m: &Captures<'_>| {
                    let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                    if ident == "ii" {
                        "i".to_string()
                    } else {
                        ident.to_string()
                    }
                })
                .to_string();
            if rewritten != *line {
                *line = rewritten;
                replaced_any = true;
            }
        }
        if replaced_any {
            out[idx].clear();
        }
    }
    out
}

pub(in super::super) fn expr_is_exact_reusable_scalar(rhs: &str) -> bool {
    let rhs = rhs.trim();
    let has_runtime_helper = rhs.contains("rr_") && !rhs.contains(".__rr_cse_");
    if rhs.is_empty()
        || plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || has_runtime_helper
        || rhs.contains('[')
        || rhs.contains('"')
        || rhs.contains(',')
        || rhs.contains("Sym_")
    {
        return false;
    }
    true
}

pub(in super::super) fn rewrite_forward_exact_pure_call_reuse(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
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
        let deps: FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
        if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || deps.contains(&lhs)
            || !rhs.contains('(')
            || !expr_has_only_pure_calls(&rhs, pure_user_calls)
        {
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

pub(in super::super) fn rewrite_adjacent_duplicate_pure_call_assignments(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    if out.len() < 2 {
        return out;
    }

    for idx in 0..(out.len() - 1) {
        let first = out[idx].trim().to_string();
        let second = out[idx + 1].trim().to_string();
        let Some(caps0) = assign_re().and_then(|re| re.captures(&first)) else {
            continue;
        };
        let Some(caps1) = assign_re().and_then(|re| re.captures(&second)) else {
            continue;
        };
        let lhs0 = caps0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs0 = caps0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let lhs1 = caps1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs1 = caps1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs0))
            || !plain_ident_re().is_some_and(|re| re.is_match(lhs1))
            || lhs0.starts_with(".arg_")
            || lhs1.starts_with(".arg_")
            || lhs0.starts_with(".__rr_cse_")
            || lhs1.starts_with(".__rr_cse_")
            || lhs0 == lhs1
            || rhs0 != rhs1
            || !rhs0.contains('(')
            || !expr_has_only_pure_calls(rhs0, pure_user_calls)
        {
            continue;
        }

        let indent = out[idx + 1]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        out[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
    }

    out
}

pub(in super::super) fn rewrite_adjacent_duplicate_symbol_assignments(
    lines: Vec<String>,
) -> Vec<String> {
    let mut out = lines;
    if out.len() < 2 {
        return out;
    }

    for idx in 0..(out.len() - 1) {
        let first = out[idx].trim().to_string();
        let second = out[idx + 1].trim().to_string();
        let Some(caps0) = assign_re().and_then(|re| re.captures(&first)) else {
            continue;
        };
        let Some(caps1) = assign_re().and_then(|re| re.captures(&second)) else {
            continue;
        };
        let lhs0 = caps0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs0 = caps0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let lhs1 = caps1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs1 = caps1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs0))
            || !plain_ident_re().is_some_and(|re| re.is_match(lhs1))
            || !plain_ident_re().is_some_and(|re| re.is_match(rhs0))
            || lhs0.starts_with(".arg_")
            || lhs1.starts_with(".arg_")
            || rhs0.starts_with(".arg_")
            || lhs0.starts_with(".__rr_cse_")
            || lhs1.starts_with(".__rr_cse_")
            || lhs0 == lhs1
            || rhs0 != rhs1
        {
            continue;
        }

        let indent = out[idx + 1]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        out[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
    }

    out
}

fn find_matching_open_brace_line_in_emitted(lines: &[String], close_idx: usize) -> Option<usize> {
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(close_idx + 1) {
        for ch in line.chars() {
            match ch {
                '{' => stack.push(idx),
                '}' => {
                    let open = stack.pop()?;
                    if idx == close_idx {
                        return Some(open);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

pub(in super::super) fn strip_terminal_repeat_nexts(lines: Vec<String>) -> Vec<String> {
    if lines.len() < 2 {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if lines[idx].trim() == "next"
            && idx + 1 < lines.len()
            && lines[idx + 1].trim() == "}"
            && find_matching_open_brace_line_in_emitted(&lines, idx + 1)
                .is_some_and(|open_idx| lines[open_idx].trim() == "repeat {")
        {
            continue;
        }
        out.push(lines[idx].clone());
    }
    out
}

pub(in super::super) fn rewrite_forward_exact_expr_reuse(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let len = out.len();
    let debug = std::env::var_os("RR_DEBUG_PEEPHOLE").is_some();
    for idx in 0..len {
        let prologue_arg_aliases = collect_prologue_arg_aliases(&out, idx);
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
        if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
            eprintln!(
                "RR_DEBUG_PEEPHOLE exact_expr_candidate line={} lhs={} rhs={}",
                idx + 1,
                lhs,
                rhs
            );
        }
        if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
            || !expr_is_exact_reusable_scalar(&rhs)
        {
            if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                eprintln!(
                    "RR_DEBUG_PEEPHOLE exact_expr_skip line={} lhs={} reusable={}",
                    idx + 1,
                    lhs,
                    expr_is_exact_reusable_scalar(&rhs)
                );
            }
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

        let deps: FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
        for line_no in idx + 1..out.len() {
            let line_trimmed = out[line_no].trim().to_string();
            let next_indent = out[line_no].len() - out[line_no].trim_start().len();
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                    eprintln!(
                        "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=indent_drop target_line={}",
                        idx + 1,
                        lhs,
                        line_trimmed
                    );
                }
                break;
            }
            if out[line_no].contains("<- function")
                || line_trimmed == "repeat {"
                || line_trimmed.starts_with("while")
                || line_trimmed.starts_with("for")
            {
                if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                    eprintln!(
                        "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=boundary",
                        idx + 1,
                        lhs
                    );
                }
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
                    if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                        eprintln!(
                            "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=same_lhs next_line={}",
                            idx + 1,
                            lhs,
                            line_trimmed
                        );
                    }
                    break;
                }
                if next_rhs.contains(&rhs) {
                    if lhs_reassigned_later {
                        continue;
                    }
                    if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                        eprintln!(
                            "RR_DEBUG_PEEPHOLE exact_expr_replace line={} lhs={} target_line={}",
                            idx + 1,
                            lhs,
                            line_trimmed
                        );
                    }
                    out[line_no] = out[line_no].replacen(&rhs, &lhs, usize::MAX);
                }
                if deps.contains(&next_lhs) {
                    let mut same_rhs_as_previous = false;
                    for prev_idx in (0..line_no).rev() {
                        let prev_trimmed = out[prev_idx].trim();
                        let Some(prev_caps) = assign_re().and_then(|re| re.captures(prev_trimmed))
                        else {
                            continue;
                        };
                        let prev_lhs = prev_caps
                            .name("lhs")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .trim();
                        if prev_lhs != next_lhs {
                            continue;
                        }
                        let prev_rhs = prev_caps
                            .name("rhs")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .trim();
                        let prev_norm =
                            normalize_expr_with_aliases(prev_rhs, &prologue_arg_aliases);
                        let next_norm =
                            normalize_expr_with_aliases(&next_rhs, &prologue_arg_aliases);
                        if prev_norm == next_norm {
                            same_rhs_as_previous = true;
                        }
                        break;
                    }
                    if same_rhs_as_previous {
                        continue;
                    }
                    if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                        eprintln!(
                            "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=dep_write dep={} target_line={}",
                            idx + 1,
                            lhs,
                            next_lhs,
                            line_trimmed
                        );
                    }
                    break;
                }
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
        }
    }
    out
}
