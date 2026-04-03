use super::{
    assign_re, expr_has_only_pure_calls, expr_idents, find_matching_block_end,
    is_control_flow_boundary, is_dead_parenthesized_eval_line, is_dead_plain_ident_eval_line,
    is_loop_open_boundary, line_is_within_loop_body, plain_ident_re, scalar_lit_re,
};
use rustc_hash::{FxHashMap, FxHashSet};

fn is_dead_pure_expr_assignment_candidate(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let lhs = lhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(lhs))
        && expr_has_only_pure_calls(rhs, pure_user_calls)
}

fn is_dead_pure_call_assignment_candidate(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
        return false;
    }
    let Some((callee, _)) = rhs.split_once('(') else {
        return false;
    };
    let callee = callee.trim();
    pure_user_calls.contains(callee)
}

pub(super) fn strip_dead_temps(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, Vec<u32>) {
    fn loop_body_references_var_before(lines: &[String], idx: usize, var: &str) -> bool {
        (0..idx)
            .rev()
            .find_map(|start_idx| {
                is_loop_open_boundary(lines[start_idx].trim())
                    .then(|| find_matching_block_end(lines, start_idx).map(|end| (start_idx, end)))
                    .flatten()
                    .filter(|(_, end)| idx < *end)
            })
            .is_some_and(|(start, _end)| {
                lines
                    .iter()
                    .take(idx)
                    .skip(start + 1)
                    .any(|line| expr_idents(line).iter().any(|ident| ident == var))
            })
    }

    let overwritten_dead = mark_overwritten_dead_assignments(&lines, pure_user_calls);
    let branch_local_dead = mark_branch_local_dead_inits(&lines);
    let redundant_temp_reassign = mark_redundant_identical_temp_reassigns(&lines);
    let mut ever_read_per_line: Vec<FxHashSet<String>> = vec![FxHashSet::default(); lines.len()];
    let mut current_reads: FxHashSet<String> = FxHashSet::default();
    let mut current_indices: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if line.contains("<- function") {
            for &line_idx in &current_indices {
                ever_read_per_line[line_idx] = current_reads.clone();
            }
            current_reads.clear();
            current_indices.clear();
        }
        current_indices.push(idx);
        let trimmed = line.trim();
        if let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) {
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
            for ident in expr_idents(rhs) {
                current_reads.insert(ident);
            }
        } else {
            for ident in expr_idents(trimmed) {
                current_reads.insert(ident);
            }
        }
    }
    for &line_idx in &current_indices {
        ever_read_per_line[line_idx] = current_reads.clone();
    }

    let mut live: FxHashSet<String> = FxHashSet::default();
    let mut out = lines;
    let mut removed = vec![false; out.len()];
    for idx in (0..out.len()).rev() {
        let line = out[idx].clone();
        if line.trim().is_empty() {
            removed[idx] = true;
            out[idx] = String::new();
            continue;
        }
        if is_dead_plain_ident_eval_line(&line) {
            removed[idx] = true;
            out[idx] = String::new();
            continue;
        }
        if is_dead_parenthesized_eval_line(&line) {
            removed[idx] = true;
            out[idx] = String::new();
            continue;
        }
        if overwritten_dead[idx] || branch_local_dead[idx] || redundant_temp_reassign[idx] {
            removed[idx] = true;
            out[idx] = String::new();
            continue;
        }
        if line.trim() == "# rr-cse-pruned" {
            removed[idx] = true;
            out[idx] = String::new();
            continue;
        }
        if line.contains("<- function") {
            live.clear();
            continue;
        }
        let trimmed = line.trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            for ident in expr_idents(trimmed) {
                live.insert(ident);
            }
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
        let is_self_referential_update = expr_idents(rhs).iter().any(|ident| ident == lhs);
        let is_temp = lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_callmap_arg")
            || lhs.starts_with(".tachyon_exprmap")
            || lhs.starts_with("i_")
            || lhs.starts_with(".__rr_tmp_");
        let is_dead_helper_local = lhs.starts_with("licm_");
        let is_loop_carried_state_update =
            line_is_within_loop_body(&out, idx) && loop_body_references_var_before(&out, idx, lhs);
        let is_dead_simple_assign =
            is_dead_pure_expr_assignment_candidate(lhs, rhs, pure_user_calls)
                && !is_loop_carried_state_update
                && !ever_read_per_line[idx].contains(lhs);
        if ((is_temp || is_dead_helper_local)
            && !live.contains(lhs)
            && !(lhs.starts_with("i_") && is_self_referential_update))
            || is_dead_simple_assign
        {
            removed[idx] = true;
            out[idx] = String::new();
            continue;
        }
        live.remove(lhs);
        for ident in expr_idents(rhs) {
            live.insert(ident);
        }
    }
    let mut compacted = Vec::with_capacity(out.len());
    let mut line_map = vec![0u32; out.len()];
    let mut new_line = 0u32;
    for (idx, line) in out.into_iter().enumerate() {
        if removed[idx] {
            line_map[idx] = new_line.max(1);
            continue;
        }
        new_line += 1;
        line_map[idx] = new_line;
        compacted.push(line);
    }
    (compacted, line_map)
}

fn mark_redundant_identical_temp_reassigns(lines: &[String]) -> Vec<bool> {
    let mut removable = vec![false; lines.len()];
    for idx in 0..lines.len() {
        let trimmed = lines[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".__rr_cse_") {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let cur_indent = lines[idx].len() - lines[idx].trim_start().len();
        let mut j = idx;
        while j > 0 {
            j -= 1;
            let prev = lines[j].trim();
            if prev.is_empty() {
                continue;
            }
            if lines[j].contains("<- function")
                || prev == "repeat {"
                || prev.starts_with("while")
                || prev.starts_with("for")
            {
                break;
            }
            if let Some(prev_caps) = assign_re().and_then(|re| re.captures(prev)) {
                let prev_lhs = prev_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let prev_rhs = prev_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if prev_lhs == lhs {
                    let prev_indent = lines[j].len() - lines[j].trim_start().len();
                    if prev_rhs == rhs && prev_indent < cur_indent {
                        removable[idx] = true;
                    }
                    break;
                }
                if deps.contains(prev_lhs) {
                    break;
                }
            }
        }
    }
    removable
}

fn mark_overwritten_dead_assignments(
    lines: &[String],
    pure_user_calls: &FxHashSet<String>,
) -> Vec<bool> {
    let mut removable = vec![false; lines.len()];
    let mut pending: FxHashMap<String, usize> = FxHashMap::default();

    let clear_pending = |pending: &mut FxHashMap<String, usize>| {
        pending.clear();
    };

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if line.contains("<- function") || is_control_flow_boundary(line) {
            clear_pending(&mut pending);
            continue;
        }

        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            for ident in expr_idents(trimmed) {
                pending.remove(&ident);
            }
            continue;
        };

        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();

        for ident in expr_idents(rhs) {
            pending.remove(&ident);
        }

        if plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && !expr_idents(rhs).iter().any(|ident| ident == lhs)
            && let Some(prev_idx) = pending.remove(lhs)
        {
            removable[prev_idx] = true;
        }

        let candidate = is_dead_pure_expr_assignment_candidate(lhs, rhs, pure_user_calls)
            || is_dead_pure_call_assignment_candidate(lhs, rhs, pure_user_calls);
        if candidate {
            pending.insert(lhs.to_string(), idx);
        } else {
            pending.remove(lhs);
        }
    }

    removable
}

fn mark_branch_local_dead_inits(lines: &[String]) -> Vec<bool> {
    let mut removable = vec![false; lines.len()];

    for idx in 0..lines.len() {
        let trimmed = lines[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
            || !scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        {
            continue;
        }

        let mut next_idx = idx + 1;
        while next_idx < lines.len() {
            let next_trimmed = lines[next_idx].trim();
            if next_trimmed.is_empty() {
                next_idx += 1;
                continue;
            }
            let Some(next_caps) = assign_re().and_then(|re| re.captures(next_trimmed)) else {
                break;
            };
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
            if plain_ident_re().is_some_and(|re| re.is_match(next_lhs))
                && scalar_lit_re().is_some_and(|re| re.is_match(next_rhs))
            {
                next_idx += 1;
                continue;
            }
            break;
        }
        if next_idx >= lines.len() || lines[next_idx].trim() != "repeat {" {
            continue;
        }
        let Some(loop_end) = find_matching_block_end(lines, next_idx) else {
            continue;
        };

        let loop_lines = &lines[next_idx + 1..loop_end];
        let mut first_occurrence = None;
        for (off, line) in loop_lines.iter().enumerate() {
            let line_idx = next_idx + 1 + off;
            let trimmed = line.trim();
            let assigned_lhs = assign_re()
                .and_then(|re| re.captures(trimmed))
                .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()));
            let mentions = assigned_lhs.as_deref() == Some(lhs)
                || expr_idents(trimmed).iter().any(|ident| ident == lhs);
            if mentions {
                first_occurrence = Some((line_idx, assigned_lhs.as_deref() == Some(lhs)));
                break;
            }
        }
        let Some((first_line_idx, first_is_assign)) = first_occurrence else {
            continue;
        };
        if !first_is_assign {
            continue;
        }

        let Some(if_start) =
            lines[..=first_line_idx]
                .iter()
                .enumerate()
                .rev()
                .find_map(|(line_idx, line)| {
                    let trimmed = line.trim();
                    (trimmed.starts_with("if ") && trimmed.ends_with('{')).then_some(line_idx)
                })
        else {
            continue;
        };
        let Some(if_end) = find_matching_block_end(lines, if_start) else {
            continue;
        };
        if if_end > loop_end {
            continue;
        }

        let mut used_outside_if = false;
        for (line_pos, line) in lines.iter().enumerate().take(loop_end).skip(next_idx + 1) {
            let trimmed = line.trim();
            let assigned_lhs = assign_re()
                .and_then(|re| re.captures(trimmed))
                .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()));
            let mentions = assigned_lhs.as_deref() == Some(lhs)
                || expr_idents(trimmed).iter().any(|ident| ident == lhs);
            if !mentions {
                continue;
            }
            if line_pos < if_start || line_pos > if_end {
                used_outside_if = true;
                break;
            }
        }
        if !used_outside_if {
            for line in lines.iter().skip(loop_end + 1) {
                let trimmed = line.trim();
                if line.contains("<- function") {
                    break;
                }
                let assigned_lhs = assign_re()
                    .and_then(|re| re.captures(trimmed))
                    .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()));
                let mentions = assigned_lhs.as_deref() == Some(lhs)
                    || expr_idents(trimmed).iter().any(|ident| ident == lhs);
                if !mentions {
                    continue;
                }
                if assigned_lhs.as_deref() == Some(lhs) {
                    break;
                }
                used_outside_if = true;
                break;
            }
        }
        if !used_outside_if {
            removable[idx] = true;
        }
    }

    removable
}
