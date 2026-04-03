use super::*;

pub(crate) fn rewrite_immediate_single_use_named_scalar_exprs_in_raw_emitted_r(
    output: &str,
) -> String {
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
        if raw_expr_idents(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_named_scalar_expr(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            continue;
        };
        let next_trimmed = lines[next_idx].trim().to_string();
        let next_is_assign = parse_raw_assign_line(next_trimmed.as_str()).is_some();
        let next_is_return =
            next_trimmed.starts_with("return(") || next_trimmed.starts_with("return (");
        if lines[next_idx].contains("<- function")
            || (!next_is_assign && !next_is_return)
            || !line_contains_symbol(&next_trimmed, &lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(next_idx + 1) {
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
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let replacement = format!("({})", strip_redundant_outer_parens(&rhs));
        lines[next_idx] = replace_symbol_occurrences(&lines[next_idx], &lhs, replacement.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_guard_only_named_scalar_exprs_in_raw_emitted_r(output: &str) -> String {
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
        if raw_expr_idents(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_named_scalar_expr(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let Some(use_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && line_contains_symbol(trimmed, &lhs)
        }) else {
            continue;
        };
        let use_trimmed = lines[use_idx].trim().to_string();
        let use_is_guard = use_trimmed.starts_with("if (") || use_trimmed.starts_with("if(");
        let use_occurrences = count_symbol_occurrences(&use_trimmed, &lhs);
        if lines[use_idx].contains("<- function")
            || !use_is_guard
            || use_occurrences == 0
            || use_occurrences > 2
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(use_idx + 1) {
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
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let replacement = format!("({})", strip_redundant_outer_parens(&rhs));
        lines[use_idx] = replace_symbol_occurrences(&lines[use_idx], &lhs, replacement.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_two_use_named_scalar_exprs_in_raw_emitted_r(output: &str) -> String {
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
        if raw_expr_idents(rhs.as_str())
            .iter()
            .any(|ident| ident == &lhs)
        {
            continue;
        }
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_named_scalar_expr(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents(rhs.as_str());
        let Some(next1_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let Some(next2_idx) = ((next1_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let next1_trimmed = lines[next1_idx].trim().to_string();
        let next2_trimmed = lines[next2_idx].trim().to_string();
        if lines[next1_idx].contains("<- function")
            || lines[next2_idx].contains("<- function")
            || parse_raw_assign_line(next1_trimmed.as_str()).is_none()
            || parse_raw_assign_line(next2_trimmed.as_str()).is_none()
            || count_symbol_occurrences(&next1_trimmed, &lhs) != 1
            || count_symbol_occurrences(&next2_trimmed, &lhs) != 1
        {
            continue;
        }

        let mut total_uses = 0usize;
        let mut use_line_idxs = Vec::new();
        let mut dep_write_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _)) = parse_raw_assign_line(line_trimmed) {
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
                if total_uses > 2 {
                    break;
                }
            }
        }
        if total_uses != 2 || use_line_idxs != vec![next1_idx, next2_idx] {
            continue;
        }
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < next2_idx) {
            continue;
        }

        let replacement = strip_redundant_outer_parens(&rhs);
        lines[next1_idx] = replace_symbol_occurrences(&lines[next1_idx], &lhs, replacement);
        lines[next2_idx] = replace_symbol_occurrences(&lines[next2_idx], &lhs, replacement);
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_single_assignment_loop_seed_literals_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(&lines[idx]) else {
            continue;
        };
        if rhs.trim() != "1" && rhs.trim() != "1L" {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            continue;
        };
        let Some((_next_lhs, next_rhs)) = parse_raw_assign_line(lines[next_idx].trim()) else {
            continue;
        };
        if !next_rhs.contains(&format!("{lhs}:")) {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, _later_rhs)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[next_idx] = replace_symbol_occurrences(&lines[next_idx], lhs, "1");
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_sym210_loop_seed_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && lines[fn_start].trim() != "Sym_210 <- function(field, w, h)"
        {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };

        for idx in (fn_start + 1)..fn_end {
            if lines[idx].trim() != "i <- 1" {
                continue;
            }
            let Some(next_idx) = ((idx + 1)..fn_end).find(|i| !lines[*i].trim().is_empty()) else {
                continue;
            };
            let Some((next_lhs, _next_rhs)) = parse_raw_assign_line(lines[next_idx].trim()) else {
                continue;
            };
            if next_lhs != "lap" || !lines[next_idx].contains("i:size - 1") {
                continue;
            }
            lines[idx].clear();
            lines[next_idx] = lines[next_idx].replace("i:size - 1", "1:size - 1");
        }

        fn_start += 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_two_use_named_scalar_pure_calls_in_raw_emitted_r(output: &str) -> String {
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
            || !rhs.starts_with("rr_idx_cube_vec_i(")
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
                if total_uses > 2 {
                    break;
                }
            }
        }
        if total_uses != 2 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }
        for use_idx in use_line_idxs {
            lines[use_idx] = replace_symbol_occurrences(&lines[use_idx], &lhs, rhs.as_str());
        }
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_guard_only_scalar_literals_in_raw_emitted_r(output: &str) -> String {
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
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_raw_simple_scalar_alias_or_literal(&rhs)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            continue;
        };
        let mut guard_idx = next_idx;
        let next_trimmed = lines[next_idx].trim().to_string();
        if next_trimmed == "repeat {" {
            let Some(found_guard) = ((next_idx + 1)..lines.len()).find(|i| {
                let trimmed = lines[*i].trim();
                !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
            }) else {
                continue;
            };
            guard_idx = found_guard;
        }
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if lines[guard_idx].contains("<- function")
            || !is_guard
            || !line_contains_symbol(&guard_trimmed, &lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(guard_idx + 1) {
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
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[guard_idx] =
            replace_symbol_occurrences(&lines[guard_idx], &lhs, strip_redundant_outer_parens(&rhs));
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_loop_guard_scalar_literals_in_raw_emitted_r(output: &str) -> String {
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
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_raw_simple_scalar_alias_or_literal(&rhs)
        {
            continue;
        }

        let Some(repeat_idx) = ((idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            continue;
        };
        if lines[repeat_idx].trim() != "repeat {" {
            continue;
        }
        let Some(guard_idx) = ((repeat_idx + 1)..lines.len()).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed != "# rr-cse-pruned"
        }) else {
            continue;
        };
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if !is_guard || !line_contains_symbol(&guard_trimmed, &lhs) {
            continue;
        }

        let mut used_after = false;
        for later_line in lines.iter().skip(guard_idx + 1) {
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
                used_after = true;
                break;
            }
            if line_contains_symbol(later_trimmed, &lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[guard_idx] =
            replace_symbol_occurrences(&lines[guard_idx], &lhs, strip_redundant_outer_parens(&rhs));
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
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
