pub(super) fn rewrite_loop_index_alias_ii(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            continue;
        };
        if lhs != "ii" || rhs != "i" {
            continue;
        }

        let mut replaced_any = false;
        let mut stop_idx = lines.len();
        let mut stopped_on_i_reassign = false;
        for (scan_idx, line) in lines.iter_mut().enumerate().skip(idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                stop_idx = scan_idx;
                break;
            }
            if let Some((next_lhs, _)) = parse_local_assign_line(trimmed)
                && (next_lhs == "ii" || next_lhs == "i")
            {
                stop_idx = scan_idx;
                stopped_on_i_reassign = next_lhs == "i";
                break;
            }
            if count_symbol_occurrences_local(trimmed, "ii") == 0 {
                continue;
            }
            let rewritten = replace_symbol_occurrences_local(line, "ii", "i");
            if rewritten != *line {
                *line = rewritten;
                replaced_any = true;
            }
        }

        let mut keep_alias = false;
        if stopped_on_i_reassign {
            for line in lines.iter().skip(stop_idx + 1) {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                if line.contains("<- function") {
                    break;
                }
                if let Some((next_lhs, _)) = parse_local_assign_line(trimmed)
                    && next_lhs == "ii"
                {
                    break;
                }
                if count_symbol_occurrences_local(trimmed, "ii") > 0 {
                    keep_alias = true;
                    break;
                }
            }
        }

        if replaced_any && !keep_alias {
            lines[idx].clear();
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn strip_dead_zero_seed_ii(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        if lines[idx].trim() != "ii <- 0" {
            continue;
        }
        let used_later = lines
            .iter()
            .skip(idx + 1)
            .any(|line| count_symbol_occurrences_local(line.trim(), "ii") > 0);
        if !used_later {
            lines[idx].clear();
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_slice_bound_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx + 1 < lines.len() {
        let Some((start_lhs, start_rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if start_lhs != "start" {
            idx += 1;
            continue;
        }
        let start_rhs =
            crate::compiler::pipeline::strip_redundant_outer_parens(start_rhs).to_string();
        if !start_rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(end_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let Some((end_lhs, end_rhs)) = parse_local_assign_line(lines[end_idx].trim()) else {
            idx += 1;
            continue;
        };
        if end_lhs != "end" {
            idx += 1;
            continue;
        }
        let end_rhs = crate::compiler::pipeline::strip_redundant_outer_parens(end_rhs).to_string();
        if !end_rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let mut use_line_idxs = Vec::new();
        for (line_no, line) in lines.iter().enumerate().skip(end_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some((lhs, _)) = parse_local_assign_line(trimmed)
                && (lhs == "start" || lhs == "end")
            {
                break;
            }
            let uses_start = count_symbol_occurrences_local(trimmed, "start") > 0;
            let uses_end = count_symbol_occurrences_local(trimmed, "end") > 0;
            if uses_start || uses_end {
                if uses_start != uses_end || !trimmed.contains("neighbors[start:end] <-") {
                    use_line_idxs.clear();
                    break;
                }
                use_line_idxs.push(line_no);
                continue;
            }
            let is_control = trimmed == "}"
                || trimmed.starts_with("if (")
                || trimmed.starts_with("if(")
                || trimmed.starts_with("else")
                || trimmed.starts_with("} else");
            if !use_line_idxs.is_empty() && !is_control {
                break;
            }
        }
        if use_line_idxs.is_empty() {
            idx += 1;
            continue;
        }

        let slice_expr = format!("{start_rhs}:{end_rhs}");
        for use_idx in &use_line_idxs {
            lines[*use_idx] = lines[*use_idx].replace("start:end", &slice_expr);
        }
        lines[idx].clear();
        lines[end_idx].clear();
        idx = use_line_idxs.last().copied().unwrap_or(end_idx) + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_particle_idx_alias(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx + 2 < lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if lhs != "idx" {
            idx += 1;
            continue;
        }
        let rhs = crate::compiler::pipeline::strip_redundant_outer_parens(rhs).to_string();
        if !rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(next1_idx) =
            ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let Some(next2_idx) =
            ((next1_idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
        else {
            break;
        };
        let next1 = lines[next1_idx].trim().to_string();
        let next2 = lines[next2_idx].trim().to_string();
        let dx_ok = next1.contains("dx <-")
            && next1.contains("u[idx]")
            && next1.contains("* dt")
            && next1.contains("/ 400000");
        let dy_ok = next2.contains("dy <-")
            && next2.contains("v[idx]")
            && next2.contains("* dt")
            && next2.contains("/ 400000");
        if !dx_ok || !dy_ok {
            idx += 1;
            continue;
        }

        lines[next1_idx] = replace_symbol_occurrences_local(&lines[next1_idx], "idx", rhs.as_str());
        lines[next2_idx] = replace_symbol_occurrences_local(&lines[next2_idx], "idx", rhs.as_str());
        lines[idx].clear();
        idx = next2_idx + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

fn rhs_is_simple_scalar_alias_or_literal_local(rhs: &str) -> bool {
    let rhs = strip_outer_parens_local(rhs);
    rhs.chars().all(RBackend::is_symbol_char)
        || rhs.trim_end_matches('L').parse::<f64>().is_ok()
        || matches!(rhs, "TRUE" | "FALSE" | "NA" | "NULL")
}

fn rhs_is_simple_dead_expr_local(rhs: &str) -> bool {
    let rhs = strip_outer_parens_local(rhs);
    !rhs.is_empty()
        && !rhs.contains("<-")
        && !rhs.contains("function(")
        && !rhs.contains("function (")
        && !rhs.contains("tryCatch(")
        && !rhs.contains("print(")
        && !rhs.contains("cat(")
        && !rhs.contains("message(")
        && !rhs.contains("warning(")
        && !rhs.contains("stop(")
        && !rhs.contains("quit(")
        && !rhs.contains('"')
        && !rhs.contains(',')
}

fn parse_repeat_guard_cmp_line_local(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("if (!(")
        .or_else(|| trimmed.strip_prefix("if !("))?
        .strip_suffix(")) break")
        .or_else(|| {
            trimmed
                .strip_prefix("if (!(")
                .or_else(|| trimmed.strip_prefix("if !("))
                .and_then(|s| s.strip_suffix(") break"))
        })?
        .trim();
    for op in ["<=", "<"] {
        let needle = format!(" {op} ");
        let Some((lhs, rhs)) = inner.split_once(&needle) else {
            continue;
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if !lhs.is_empty() && !rhs.is_empty() {
            return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
        }
    }
    None
}

fn plain_ident_local(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(RBackend::is_symbol_char)
}

fn raw_enclosing_repeat_guard_mentions_symbol_local(
    lines: &[String],
    idx: usize,
    symbol: &str,
) -> bool {
    for start_idx in (0..idx).rev() {
        if lines[start_idx].trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = find_block_end_local(lines, start_idx) else {
            continue;
        };
        if idx >= end_idx {
            continue;
        }
        let Some(guard_idx) = ((start_idx + 1)..end_idx)
            .find(|line_idx| parse_repeat_guard_cmp_line_local(lines[*line_idx].trim()).is_some())
        else {
            continue;
        };
        if count_symbol_occurrences_local(lines[guard_idx].trim(), symbol) > 0 {
            return true;
        }
        break;
    }
    false
}

fn raw_enclosing_repeat_body_reads_symbol_before_local(
    lines: &[String],
    idx: usize,
    symbol: &str,
) -> bool {
    for start_idx in (0..idx).rev() {
        if lines[start_idx].trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = find_block_end_local(lines, start_idx) else {
            continue;
        };
        if idx >= end_idx {
            continue;
        }
        let Some(guard_idx) = ((start_idx + 1)..end_idx)
            .find(|line_idx| parse_repeat_guard_cmp_line_local(lines[*line_idx].trim()).is_some())
        else {
            continue;
        };
        for line in lines.iter().take(idx).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some((lhs, rhs)) = parse_local_assign_line(trimmed)
                && lhs == symbol
            {
                if count_symbol_occurrences_local(rhs, symbol) > 0 {
                    return true;
                }
                continue;
            }
            if count_symbol_occurrences_local(trimmed, symbol) > 0 {
                return true;
            }
        }
        break;
    }
    false
}

pub(super) fn rewrite_guard_scalar_literals(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_simple_scalar_alias_or_literal_local(&rhs)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            continue;
        };
        let mut guard_idx = next_idx;
        if lines[next_idx].trim() == "repeat {" {
            let Some(found_guard) =
                ((next_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
            else {
                continue;
            };
            guard_idx = found_guard;
        }
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if lines[guard_idx].contains("<- function")
            || !is_guard
            || count_symbol_occurrences_local(&guard_trimmed, &lhs) == 0
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
            if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if count_symbol_occurrences_local(later_trimmed, &lhs) > 0 {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[guard_idx] = replace_symbol_occurrences_local(
            &lines[guard_idx],
            &lhs,
            strip_outer_parens_local(&rhs),
        );
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_loop_guard_scalar_literals(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !rhs_is_simple_scalar_alias_or_literal_local(&rhs)
        {
            continue;
        }

        let Some(repeat_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        if lines[repeat_idx].trim() != "repeat {" {
            continue;
        }
        let Some(guard_idx) =
            ((repeat_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            continue;
        };
        let guard_trimmed = lines[guard_idx].trim().to_string();
        let is_guard = guard_trimmed.starts_with("if (") || guard_trimmed.starts_with("if(");
        if !is_guard || count_symbol_occurrences_local(&guard_trimmed, &lhs) == 0 {
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
            if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                used_after = true;
                break;
            }
            if count_symbol_occurrences_local(later_trimmed, &lhs) > 0 {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        lines[guard_idx] = replace_symbol_occurrences_local(
            &lines[guard_idx],
            &lhs,
            strip_outer_parens_local(&rhs),
        );
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn rewrite_named_scalar_pure_call_aliases(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_local_assign_line(&lines[idx]) else {
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = strip_outer_parens_local(rhs).to_string();
        let max_uses = if rhs.starts_with("rr_wrap_index_vec_i(") {
            1usize
        } else if rhs.starts_with("rr_idx_cube_vec_i(") {
            2usize
        } else {
            continue;
        };
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || raw_line_is_within_loop_body_local(&lines, idx)
        {
            continue;
        }

        let rhs_deps = raw_expr_idents_local(rhs.as_str());
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
            if let Some((later_lhs, _later_rhs)) = parse_local_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    break;
                }
                if rhs_deps.iter().any(|dep| dep == later_lhs) {
                    dep_write_idxs.push(line_no);
                }
            }
            let occurrences = count_symbol_occurrences_local(line_trimmed, &lhs);
            if occurrences > 0 {
                total_uses += occurrences;
                use_line_idxs.push(line_no);
                if total_uses > max_uses {
                    break;
                }
            }
        }
        if total_uses == 0 || total_uses != max_uses {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }
        for use_idx in use_line_idxs {
            lines[use_idx] = replace_symbol_occurrences_local(&lines[use_idx], &lhs, rhs.as_str());
        }
        lines[idx].clear();
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn hoist_branch_local_pure_scalar_assigns_used_after_branch(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim().to_string();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            idx += 1;
            continue;
        }
        let guard_idents = raw_expr_idents_local(trimmed.as_str());
        let Some(end_idx) = find_block_end_local(&lines, idx) else {
            idx += 1;
            continue;
        };

        let mut trailing_assigns = Vec::new();
        let mut scan = end_idx;
        while scan > idx + 1 {
            scan -= 1;
            let trimmed_line = lines[scan].trim();
            if trimmed_line.is_empty() {
                continue;
            }
            let Some((lhs, rhs)) = parse_local_assign_line(trimmed_line) else {
                break;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || !is_branch_hoistable_named_scalar_rhs_local(rhs)
            {
                break;
            }
            trailing_assigns.push((scan, lhs.to_string(), rhs.to_string()));
        }
        if trailing_assigns.is_empty() {
            idx = end_idx + 1;
            continue;
        }
        trailing_assigns.reverse();

        let mut hoisted = Vec::new();
        for (assign_idx, lhs, rhs) in trailing_assigns {
            if guard_idents.iter().any(|ident| ident == &lhs) {
                continue;
            }
            let rhs_deps = raw_expr_idents_local(strip_outer_parens_local(&rhs));
            let dep_written_in_branch = lines
                .iter()
                .take(assign_idx)
                .skip(idx + 1)
                .filter_map(|line| parse_local_assign_line(line.trim()))
                .any(|(branch_lhs, _)| rhs_deps.iter().any(|dep| dep == branch_lhs));
            if dep_written_in_branch {
                continue;
            }

            let mut used_after = false;
            for later_line in lines.iter().skip(end_idx + 1) {
                let later_trimmed = later_line.trim();
                if later_line.contains("<- function") {
                    break;
                }
                if let Some((later_lhs, _)) = parse_local_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    break;
                }
                if count_symbol_occurrences_local(later_trimmed, &lhs) > 0 {
                    used_after = true;
                    break;
                }
            }
            if used_after {
                hoisted.push(lines[assign_idx].clone());
                lines[assign_idx].clear();
            }
        }

        if !hoisted.is_empty() {
            for (offset, line) in hoisted.into_iter().enumerate() {
                lines.insert(idx + offset, line);
            }
            idx = end_idx + 1;
            continue;
        }
        idx = end_idx + 1;
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
