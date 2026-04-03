use super::{
    assign_re, expr_has_only_pure_calls, expr_idents, find_matching_block_end, ident_re,
    is_control_flow_boundary, line_is_within_loop_body, plain_ident_re, scalar_lit_re,
};
use regex::Captures;
use rustc_hash::FxHashSet;

fn expr_is_simple_scalar_index_read(rhs: &str) -> bool {
    super::compile_regex(format!(r"^{}\[[^\],]+\]$", super::IDENT_PATTERN))
        .is_some_and(|re| re.is_match(rhs.trim()))
}

fn expr_is_inlineable_named_scalar_rhs(rhs: &str, pure_user_calls: &FxHashSet<String>) -> bool {
    let rhs = rhs.trim();
    expr_is_simple_scalar_index_read(rhs)
        || (rhs.starts_with("rr_")
            && rhs.contains('(')
            && !rhs.starts_with("rr_parallel_typed_vec_call(")
            && expr_has_only_pure_calls(rhs, pure_user_calls))
}

fn expr_is_inlineable_named_scalar_expr(rhs: &str, pure_user_calls: &FxHashSet<String>) -> bool {
    let rhs = rhs.trim();
    if rhs.is_empty()
        || plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || rhs.contains('"')
        || rhs.contains(',')
        || rhs.contains("Sym_")
        || rhs.starts_with("rr_parallel_typed_vec_call(")
    {
        return false;
    }
    !rhs.contains("rr_") || expr_has_only_pure_calls(rhs, pure_user_calls)
}

pub(super) fn rewrite_temp_uses_after_named_copy(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim();
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
        if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
            || !(rhs.starts_with(".__pc_src_tmp") || rhs.starts_with(".__rr_cse_"))
        {
            continue;
        }

        let temp = rhs;
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
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == temp {
                    break;
                }
                if next_lhs == lhs {
                    if next_rhs == temp {
                        continue;
                    }
                    break;
                }
            }
            if !line.contains(&temp)
                || !expr_idents(line_trimmed).iter().any(|ident| ident == &temp)
            {
                continue;
            }
            let rewritten = ident_re
                .replace_all(line, |m: &Captures<'_>| {
                    let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                    if ident == temp {
                        lhs.to_string()
                    } else {
                        ident.to_string()
                    }
                })
                .to_string();
            if rewritten != *line {
                *line = rewritten;
            }
        }
    }
    out
}

pub(super) fn inline_immediate_single_use_scalar_temps(lines: Vec<String>) -> Vec<String> {
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
        if !lhs.starts_with(".__rr_cse_") || !super::expr_is_exact_reusable_scalar(rhs) {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..out.len()).find(|i| !out[*i].trim().is_empty()) else {
            continue;
        };
        let next_trimmed = out[next_idx].trim().to_string();
        if out[next_idx].contains("<- function")
            || is_control_flow_boundary(&next_trimmed)
            || !expr_idents(&next_trimmed).iter().any(|ident| ident == lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in out.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let rewritten = ident_re
            .replace_all(&out[next_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[next_idx] {
            out[next_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}

pub(super) fn inline_single_use_named_scalar_index_reads_within_straight_line_region(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
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
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || !expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
        {
            continue;
        }
        let mut later_reassigned = false;
        for later_line in out.iter().skip(idx + 1) {
            let later_trimmed = later_line.trim();
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    later_reassigned = true;
                    break;
                }
            }
        }
        if later_reassigned {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let mut use_lines = Vec::new();
        let mut dep_write_lines = Vec::new();
        for (line_no, line) in out.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") || is_control_flow_boundary(line_trimmed) {
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
                    if expr_idents(next_rhs).iter().any(|ident| ident == lhs) {
                        dep_write_lines.push(line_no);
                    }
                    break;
                }
                if deps.contains(next_lhs) {
                    dep_write_lines.push(line_no);
                }
            }
            if expr_idents(line_trimmed).iter().any(|ident| ident == lhs) {
                use_lines.push(line_no);
                if use_lines.len() > 1 {
                    break;
                }
            }
        }
        if use_lines.len() != 1 {
            continue;
        }
        let use_idx = use_lines[0];
        if dep_write_lines.iter().any(|dep_idx| *dep_idx < use_idx) {
            continue;
        }
        let mut used_after_region = false;
        for later_line in out.iter().skip(use_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after_region = true;
                break;
            }
        }
        if used_after_region {
            continue;
        }
        let rewritten = ident_re
            .replace_all(&out[use_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[use_idx] {
            out[use_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}

pub(super) fn inline_immediate_single_use_named_scalar_exprs(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
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
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || !expr_is_inlineable_named_scalar_expr(rhs, pure_user_calls)
            || line_is_within_loop_body(&out, idx)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..out.len()).find(|i| !out[*i].trim().is_empty()) else {
            continue;
        };
        let next_trimmed = out[next_idx].trim().to_string();
        if out[next_idx].contains("<- function")
            || is_control_flow_boundary(&next_trimmed)
            || !expr_idents(&next_trimmed).iter().any(|ident| ident == lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in out.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let rewritten = ident_re
            .replace_all(&out[next_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[next_idx] {
            out[next_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}

pub(super) fn hoist_branch_local_named_scalar_assigns_used_after_branch(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    let mut idx = 0usize;
    while idx < out.len() {
        let trimmed = out[idx].trim();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            idx += 1;
            continue;
        }
        let guard_idents = expr_idents(trimmed);
        let Some(end_idx) = find_matching_block_end(&out, idx) else {
            break;
        };
        let mut trailing = Vec::new();
        let mut scan = end_idx;
        while scan > idx + 1 {
            scan -= 1;
            let trimmed_line = out[scan].trim();
            if trimmed_line.is_empty() {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed_line)) else {
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
                || lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || !expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
            {
                break;
            }
            trailing.push((scan, lhs.to_string(), rhs.to_string()));
        }
        if trailing.is_empty() {
            idx = end_idx + 1;
            continue;
        }
        trailing.reverse();

        let mut hoisted = Vec::new();
        for (assign_idx, lhs, rhs) in trailing {
            if guard_idents.iter().any(|ident| ident == &lhs) {
                continue;
            }
            let deps: FxHashSet<String> = expr_idents(rhs.as_str()).into_iter().collect();
            let dep_written_in_branch = out
                .iter()
                .take(assign_idx)
                .skip(idx + 1)
                .filter_map(|line| {
                    assign_re()
                        .and_then(|re| re.captures(line.trim()))
                        .map(|caps| {
                            caps.name("lhs")
                                .map(|m| m.as_str())
                                .unwrap_or("")
                                .trim()
                                .to_string()
                        })
                })
                .any(|branch_lhs| deps.contains(&branch_lhs));
            if dep_written_in_branch {
                continue;
            }

            let mut used_after = false;
            for later_line in out.iter().skip(end_idx + 1) {
                let later_trimmed = later_line.trim();
                if later_line.contains("<- function") {
                    break;
                }
                if let Some(caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                    let later_lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    if later_lhs == lhs {
                        break;
                    }
                }
                if expr_idents(later_trimmed).iter().any(|ident| ident == &lhs) {
                    used_after = true;
                    break;
                }
            }
            if used_after {
                hoisted.push(out[assign_idx].clone());
                out[assign_idx].clear();
            }
        }

        if !hoisted.is_empty() {
            for (offset, line) in hoisted.into_iter().enumerate() {
                out.insert(idx + offset, line);
            }
            idx = end_idx + 1;
            continue;
        }

        idx = end_idx + 1;
    }
    out
}

pub(super) fn inline_two_use_named_scalar_index_reads_within_straight_line_region(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
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
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || !expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
            || line_is_within_loop_body(&out, idx)
        {
            continue;
        }
        let mut later_reassigned = false;
        for later_line in out.iter().skip(idx + 1) {
            let later_trimmed = later_line.trim();
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    later_reassigned = true;
                    break;
                }
            }
        }
        if later_reassigned {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let mut use_lines = Vec::new();
        let mut dep_write_lines = Vec::new();
        for (line_no, line) in out.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") || is_control_flow_boundary(line_trimmed) {
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
                    if expr_idents(next_rhs).iter().any(|ident| ident == lhs) {
                        dep_write_lines.push(line_no);
                    }
                    break;
                }
                if deps.contains(next_lhs) {
                    dep_write_lines.push(line_no);
                }
            }
            if expr_idents(line_trimmed).iter().any(|ident| ident == lhs) {
                use_lines.push(line_no);
                if use_lines.len() > 2 {
                    break;
                }
            }
        }
        if use_lines.is_empty() || use_lines.len() > 2 {
            continue;
        }
        let last_use = *use_lines.last().unwrap_or(&idx);
        if dep_write_lines.iter().any(|dep_idx| *dep_idx < last_use) {
            continue;
        }
        let mut used_after_region = false;
        for later_line in out.iter().skip(last_use + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after_region = true;
                break;
            }
        }
        if used_after_region {
            continue;
        }
        let mut changed = false;
        for use_idx in use_lines {
            let rewritten = ident_re
                .replace_all(&out[use_idx], |m: &Captures<'_>| {
                    let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                    if ident == lhs {
                        rhs.to_string()
                    } else {
                        ident.to_string()
                    }
                })
                .to_string();
            if rewritten != out[use_idx] {
                out[use_idx] = rewritten;
                changed = true;
            }
        }
        if changed {
            out[idx].clear();
        }
    }
    out
}

pub(super) fn inline_single_use_scalar_temps_within_straight_line_region(
    lines: Vec<String>,
) -> Vec<String> {
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
        if !lhs.starts_with(".__rr_cse_") || !super::expr_is_exact_reusable_scalar(rhs) {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let mut use_lines = Vec::new();
        let mut blocked = false;
        for (line_no, line) in out.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") || is_control_flow_boundary(line_trimmed) {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    break;
                }
                if deps.contains(next_lhs) {
                    blocked = true;
                    break;
                }
            }
            if expr_idents(line_trimmed).iter().any(|ident| ident == lhs) {
                use_lines.push(line_no);
                if use_lines.len() > 1 {
                    break;
                }
            }
        }
        if blocked || use_lines.len() != 1 {
            continue;
        }
        let use_idx = use_lines[0];
        let mut used_after_region = false;
        for later_line in out.iter().skip(use_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after_region = true;
                break;
            }
        }
        if used_after_region {
            continue;
        }
        let rewritten = ident_re
            .replace_all(&out[use_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[use_idx] {
            out[use_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}

pub(super) fn inline_two_use_scalar_temps_within_straight_line_region(
    lines: Vec<String>,
) -> Vec<String> {
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
        if !lhs.starts_with(".__rr_cse_") || !super::expr_is_exact_reusable_scalar(rhs) {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let mut use_lines = Vec::new();
        let mut blocked = false;
        for (line_no, line) in out.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") || is_control_flow_boundary(line_trimmed) {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    break;
                }
                if deps.contains(next_lhs) {
                    blocked = true;
                    break;
                }
            }
            if expr_idents(line_trimmed).iter().any(|ident| ident == lhs) {
                use_lines.push(line_no);
                if use_lines.len() > 2 {
                    break;
                }
            }
        }
        if blocked || use_lines.is_empty() || use_lines.len() > 2 {
            continue;
        }
        let last_use = *use_lines.last().unwrap_or(&idx);
        let mut used_after_region = false;
        for later_line in out.iter().skip(last_use + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after_region = true;
                break;
            }
        }
        if used_after_region {
            continue;
        }
        let mut changed = false;
        for use_idx in use_lines {
            let rewritten = ident_re
                .replace_all(&out[use_idx], |m: &Captures<'_>| {
                    let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                    if ident == lhs {
                        rhs.to_string()
                    } else {
                        ident.to_string()
                    }
                })
                .to_string();
            if rewritten != out[use_idx] {
                out[use_idx] = rewritten;
                changed = true;
            }
        }
        if changed {
            out[idx].clear();
        }
    }
    out
}

pub(super) fn inline_immediate_single_use_index_temps(lines: Vec<String>) -> Vec<String> {
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
        if !lhs.starts_with(".__rr_cse_") || !rhs.starts_with("rr_index_vec_floor(") {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..out.len()).find(|i| !out[*i].trim().is_empty()) else {
            continue;
        };
        let next_trimmed = out[next_idx].trim().to_string();
        if out[next_idx].contains("<- function")
            || is_control_flow_boundary(&next_trimmed)
            || !expr_idents(&next_trimmed).iter().any(|ident| ident == lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in out.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let rewritten = ident_re
            .replace_all(&out[next_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[next_idx] {
            out[next_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}
