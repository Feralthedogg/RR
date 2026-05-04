use super::*;
#[derive(Clone, Copy)]
pub(crate) enum GuardScalarRewriteMode {
    AnyImmediateGuard,
    LoopGuard,
}

pub(crate) fn scalar_raw_lines_to_output(original: &str, lines: Vec<String>) -> String {
    let mut out = lines.join("\n");
    if original.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn next_non_pruned_raw_line(lines: &[String], start: usize) -> Option<usize> {
    (start..lines.len()).find(|idx| {
        let trimmed = lines[*idx].trim();
        !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
    })
}

pub(crate) fn raw_scalar_guard_candidate(lhs: &str, rhs: &str) -> bool {
    !lhs.starts_with(".arg_")
        && !lhs.starts_with(".__rr_cse_")
        && !lhs.starts_with(".tachyon_")
        && rhs_is_raw_simple_scalar_alias_or_literal(rhs)
}

pub(crate) fn scalar_guard_rewrite_target(
    lines: &[String],
    binding_idx: usize,
    lhs: &str,
    mode: GuardScalarRewriteMode,
) -> Option<usize> {
    let next_idx = next_non_pruned_raw_line(lines, binding_idx + 1)?;
    let guard_idx = match mode {
        GuardScalarRewriteMode::AnyImmediateGuard if lines[next_idx].trim() == "repeat {" => {
            next_non_pruned_raw_line(lines, next_idx + 1)?
        }
        GuardScalarRewriteMode::AnyImmediateGuard => next_idx,
        GuardScalarRewriteMode::LoopGuard => {
            (lines[next_idx].trim() == "repeat {").then_some(())?;
            next_non_pruned_raw_line(lines, next_idx + 1)?
        }
    };

    let guard_trimmed = lines[guard_idx].trim();
    let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
    (is_guard && line_contains_symbol(guard_trimmed, lhs)).then_some(guard_idx)
}

pub(crate) fn raw_symbol_used_after(lines: &[String], start: usize, lhs: &str) -> bool {
    for later_line in lines.iter().skip(start) {
        let later_trimmed = later_line.trim();
        if later_trimmed.is_empty() {
            continue;
        }
        if later_line.contains("<- function") {
            break;
        }
        if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
            && later_lhs == lhs
        {
            return true;
        }
        if line_contains_symbol(later_trimmed, lhs) {
            return true;
        }
    }
    false
}

pub(crate) fn rewrite_scalar_guard_literals(output: &str, mode: GuardScalarRewriteMode) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if !raw_scalar_guard_candidate(&lhs, &rhs) {
            continue;
        }

        let Some(guard_idx) = scalar_guard_rewrite_target(&lines, idx, &lhs, mode) else {
            continue;
        };
        if matches!(mode, GuardScalarRewriteMode::AnyImmediateGuard)
            && lines[guard_idx].contains("<- function")
        {
            continue;
        }
        if raw_symbol_used_after(&lines, guard_idx + 1, &lhs) {
            continue;
        }

        lines[guard_idx] =
            replace_symbol_occurrences(&lines[guard_idx], &lhs, strip_redundant_outer_parens(&rhs));
        lines[idx].clear();
    }

    scalar_raw_lines_to_output(output, lines)
}

pub(crate) fn rewrite_guard_only_scalar_literals_in_raw_emitted_r(output: &str) -> String {
    rewrite_scalar_guard_literals(output, GuardScalarRewriteMode::AnyImmediateGuard)
}

pub(crate) fn rewrite_loop_guard_scalar_literals_in_raw_emitted_r(output: &str) -> String {
    rewrite_scalar_guard_literals(output, GuardScalarRewriteMode::LoopGuard)
}

pub(crate) fn rewrite_single_use_named_scalar_pure_calls_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_redundant_outer_parens(rhs).to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs.starts_with("rr_wrap_index_vec_i(")
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents(rhs.as_str());
        let mut use_line_idxs = Vec::new();
        let mut total_uses = 0usize;
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _later_rhs)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > 1 {
                    break;
                }
            }
        }
        if total_uses != 1 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }

        let use_idx = use_line_idxs[0];
        lines[use_idx] = replace_symbol_occurrences(&lines[use_idx], &lhs, rhs.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}
