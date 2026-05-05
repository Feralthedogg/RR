use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TrivialClampHelperSummary {
    pub(crate) x_slot: usize,
    pub(crate) lo_slot: usize,
    pub(crate) hi_slot: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawHelperExprReuseSummary {
    pub(crate) wrapper: String,
    pub(crate) inner_callee: String,
    pub(crate) temp_var: String,
    pub(crate) params: Vec<String>,
    pub(crate) return_expr: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RawDotProductHelperSummary {
    pub(crate) helper: String,
    pub(crate) lhs_param: String,
    pub(crate) rhs_param: String,
    pub(crate) len_param: String,
}

pub(crate) fn rewrite_trivial_clamp_helper_calls_in_raw_emitted_r(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let helpers = collect_trivial_clamp_helpers_in_emitted_r(&lines);
    if helpers.is_empty() {
        return output.to_string();
    }

    let mut helper_names: Vec<String> = helpers.keys().cloned().collect();
    helper_names.sort_by_key(|name| std::cmp::Reverse(name.len()));

    let mut rewritten = Vec::with_capacity(lines.len());
    for line in lines {
        rewritten.push(rewrite_trivial_clamp_calls_in_line(
            &line,
            &helpers,
            &helper_names,
        ));
    }

    let mut out = rewritten.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn extract_ifelse_range_expr(line: &str) -> Option<String> {
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

pub(crate) fn repair_missing_cse_range_aliases_in_raw_emitted_r(output: &str) -> String {
    let Some(floor_temp_re) = crate::compiler::peephole::patterns::compile_regex(
        r"rr_index_vec_floor\(\.__rr_cse_\d+\)".to_string(),
    ) else {
        return output.to_string();
    };
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }
    for line in &mut lines {
        if !line.contains("rr_ifelse_strict(") || !line.contains("rr_index_vec_floor(.__rr_cse_") {
            continue;
        }
        let Some(range) = extract_ifelse_range_expr(line.as_str()) else {
            continue;
        };
        *line = floor_temp_re
            .replace_all(line.as_str(), format!("rr_index_vec_floor({range})"))
            .to_string();
    }
    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn collect_trivial_clamp_helpers_in_emitted_r(
    lines: &[String],
) -> FxHashMap<String, TrivialClampHelperSummary> {
    let mut out = FxHashMap::default();
    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((name, params)) = parse_emitted_function_header(&lines[idx]) else {
            idx += 1;
            continue;
        };
        let Some(end) = emitted_function_scope_end(lines, idx) else {
            idx += 1;
            continue;
        };
        if let Some(summary) = match_trivial_clamp_helper(lines, idx, end, &params) {
            out.insert(name, summary);
        }
        idx = end + 1;
    }
    out
}

pub(crate) fn parse_emitted_function_header(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let (name, raw_params) = trimmed.split_once(" <- function(")?;
    let raw_params = raw_params.strip_suffix(')')?;
    let params = if raw_params.trim().is_empty() {
        Vec::new()
    } else {
        raw_params
            .split(',')
            .map(|param| param.trim().to_string())
            .collect()
    };
    Some((name.trim().to_string(), params))
}

pub(crate) fn emitted_function_scope_end(lines: &[String], start: usize) -> Option<usize> {
    let mut depth = 0isize;
    let mut saw_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    saw_open = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if saw_open && depth <= 0 {
            return Some(idx);
        }
    }
    None
}

pub(crate) fn match_trivial_clamp_helper(
    lines: &[String],
    start: usize,
    end: usize,
    params: &[String],
) -> Option<TrivialClampHelperSummary> {
    if params.len() != 3 {
        return None;
    }
    let body: Vec<&str> = lines
        .get(start + 1..=end)?
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    if body.len() != 15 || body[0] != "{" || body[7] != "} else {" || body[8] != "}" {
        return None;
    }
    if body[11] != "} else {" || body[12] != "}" || body[14] != "}" {
        return None;
    }

    let x = &params[0];
    let lo = &params[1];
    let hi = &params[2];
    let arg_x = format!(".arg_{x}");
    let arg_lo = format!(".arg_{lo}");
    let arg_hi = format!(".arg_{hi}");

    if body[1] != format!("{arg_x} <- {x}")
        || body[2] != format!("{arg_lo} <- {lo}")
        || body[3] != format!("{arg_hi} <- {hi}")
    {
        return None;
    }

    let (target, init_rhs) = body[4].split_once(" <- ")?;
    if init_rhs != arg_x && init_rhs != x {
        return None;
    }

    let lo_cmp_ok = body[5] == format!("if (({target} < {arg_lo})) {{")
        || body[5] == format!("if (({target} < {lo})) {{");
    let lo_assign_ok =
        body[6] == format!("{target} <- {arg_lo}") || body[6] == format!("{target} <- {lo}");
    let hi_cmp_ok = body[9] == format!("if (({target} > {arg_hi})) {{")
        || body[9] == format!("if (({target} > {hi})) {{");
    let hi_assign_ok =
        body[10] == format!("{target} <- {arg_hi}") || body[10] == format!("{target} <- {hi}");

    if !lo_cmp_ok || !lo_assign_ok || !hi_cmp_ok || !hi_assign_ok {
        return None;
    }
    if body[13] != format!("return({target})") {
        return None;
    }

    Some(TrivialClampHelperSummary {
        x_slot: 0,
        lo_slot: 1,
        hi_slot: 2,
    })
}

pub(crate) fn rewrite_trivial_clamp_calls_in_line(
    line: &str,
    helpers: &FxHashMap<String, TrivialClampHelperSummary>,
    helper_names: &[String],
) -> String {
    let mut out = String::with_capacity(line.len());
    let mut idx = 0usize;
    while idx < line.len() {
        let mut rewritten = false;
        for name in helper_names {
            let slice = &line[idx..];
            if !slice.starts_with(name) {
                continue;
            }
            if idx > 0 && line[..idx].chars().next_back().is_some_and(is_symbol_char) {
                continue;
            }
            let open = idx + name.len();
            if !line[open..].starts_with('(') {
                continue;
            }
            let Some(close) = find_matching_call_close(line, open) else {
                continue;
            };
            let Some(args) = split_top_level_args(&line[open + 1..close]) else {
                continue;
            };
            let Some(summary) = helpers.get(name) else {
                continue;
            };
            let max_slot = summary.x_slot.max(summary.lo_slot).max(summary.hi_slot);
            if args.len() <= max_slot {
                continue;
            }
            out.push_str(&format!(
                "(pmin(pmax({}, {}), {}))",
                args[summary.x_slot].trim(),
                args[summary.lo_slot].trim(),
                args[summary.hi_slot].trim()
            ));
            idx = close + 1;
            rewritten = true;
            break;
        }
        if rewritten {
            continue;
        }
        let Some(ch) = line[idx..].chars().next() else {
            break;
        };
        out.push(ch);
        idx += ch.len_utf8();
    }
    out
}

pub(crate) fn rewrite_branch_local_identical_alloc_rebinds_in_raw_emitted_r(
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
        let rhs_canonical = strip_redundant_outer_parens(rhs);
        if !is_raw_branch_rebind_candidate(rhs_canonical) {
            continue;
        }
        let Some(branch_start) = enclosing_raw_branch_start(&lines, idx) else {
            continue;
        };
        if branch_body_writes_symbol_before(&lines, branch_start + 1, idx, lhs) {
            continue;
        }
        let prev_assign = if raw_vec_fill_signature(rhs_canonical).is_some() {
            previous_outer_assign_before_branch_relaxed(&lines, branch_start, lhs)
        } else if is_raw_alloc_like_expr(rhs_canonical) {
            previous_outer_assign_before_branch(&lines, branch_start, lhs)
        } else {
            previous_outer_assign_before_branch_relaxed(&lines, branch_start, lhs)
        };
        let Some((prev_lhs, prev_rhs)) = prev_assign else {
            continue;
        };
        if prev_lhs == lhs && raw_branch_rebind_exprs_equivalent(prev_rhs, rhs_canonical) {
            lines[idx].clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn hoist_branch_local_pure_scalar_assigns_used_after_branch_in_raw_emitted_r(
    output: &str,
) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim().to_string();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            idx += 1;
            continue;
        }
        let guard_idents = raw_expr_idents(trimmed.as_str());
        let Some(end_idx) = find_raw_block_end(&lines, idx) else {
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
            let Some((lhs, rhs)) = parse_raw_assign_line(trimmed_line) else {
                break;
            };
            if lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_")
                || !is_inlineable_raw_scalar_index_rhs(rhs)
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
            let rhs_deps = raw_expr_idents(strip_redundant_outer_parens(&rhs));
            let dep_written_in_branch = lines
                .iter()
                .take(assign_idx)
                .skip(idx + 1)
                .filter_map(|line| parse_raw_assign_line(line.trim()))
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
                if let Some((later_lhs, _)) = parse_raw_assign_line(later_trimmed)
                    && later_lhs == lhs
                {
                    break;
                }
                if line_contains_symbol(later_trimmed, &lhs) {
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

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_single_use_scalar_index_aliases_in_raw_emitted_r(output: &str) -> String {
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
            || lhs.starts_with(".phi_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_scalar_index_rhs(&rhs)
        {
            continue;
        }

        let rhs_canonical = strip_redundant_outer_parens(&rhs).to_string();
        let rhs_deps = raw_expr_idents(rhs_canonical.as_str());

        let mut later_reassigned = false;
        for later_line in lines.iter().skip(idx + 1) {
            let later_trimmed = later_line.trim();
            if later_line.contains("<- function") {
                break;
            }
            if let Some((later_lhs, later_rhs)) = parse_raw_assign_line(later_trimmed)
                && later_lhs == lhs
            {
                if line_contains_symbol(later_rhs, &lhs) {
                    later_reassigned = true;
                }
                break;
            }
        }
        if later_reassigned {
            continue;
        }

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
                    // Allow up to two scalar uses for small straight-line
                    // hydrometeor-style locals such as `qc <- q_c[i]`.
                    if total_uses > 2 {
                        break;
                    }
                }
            }
        }
        if total_uses == 0 {
            lines[idx].clear();
            continue;
        }
        if total_uses > 2 {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }
        for use_idx in use_line_idxs {
            lines[use_idx] =
                replace_symbol_occurrences(&lines[use_idx], &lhs, rhs_canonical.as_str());
        }
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_small_multiuse_scalar_index_aliases_in_adjacent_assignments_in_raw_emitted_r(
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
        if lhs.starts_with(".arg_")
            || lhs.starts_with(".phi_")
            || lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_")
            || !is_inlineable_raw_scalar_index_rhs(&rhs)
            || raw_line_is_within_loop_body(&lines, idx)
        {
            continue;
        }

        let rhs_canonical = strip_redundant_outer_parens(&rhs).to_string();
        let rhs_deps = raw_expr_idents(rhs_canonical.as_str());

        let mut scan_start = idx + 1;
        while let Some(alias_idx) = (scan_start..lines.len()).find(|i| !lines[*i].trim().is_empty())
        {
            let trimmed = lines[alias_idx].trim();
            let Some((alias_lhs, alias_rhs)) = parse_raw_assign_line(trimmed) else {
                break;
            };
            if alias_lhs.starts_with(".arg_")
                || alias_lhs.starts_with(".__rr_cse_")
                || alias_lhs.starts_with(".tachyon_")
                || !is_inlineable_raw_scalar_index_rhs(alias_rhs)
            {
                break;
            }
            scan_start = alias_idx + 1;
        }

        let Some(next1_idx) = (scan_start..lines.len()).find(|i| !lines[*i].trim().is_empty())
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
        {
            continue;
        }

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
            if let Some((later_lhs, later_rhs)) = parse_raw_assign_line(line_trimmed) {
                if later_lhs == lhs {
                    if line_contains_symbol(later_rhs, &lhs) {
                        total_uses = usize::MAX;
                    }
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
                if total_uses > 6 {
                    break;
                }
            }
        }

        if total_uses == 0 || total_uses > 6 {
            continue;
        }
        if use_line_idxs
            .iter()
            .any(|line_no| *line_no != next1_idx && *line_no != next2_idx)
        {
            continue;
        }
        let Some(last_use_idx) = use_line_idxs.last().copied() else {
            continue;
        };
        if dep_write_idxs.iter().any(|dep_idx| *dep_idx < last_use_idx) {
            continue;
        }

        lines[next1_idx] =
            replace_symbol_occurrences(&lines[next1_idx], &lhs, rhs_canonical.as_str());
        lines[next2_idx] =
            replace_symbol_occurrences(&lines[next2_idx], &lhs, rhs_canonical.as_str());
        lines[idx].clear();
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}
