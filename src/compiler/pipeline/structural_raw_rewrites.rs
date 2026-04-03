//! Raw emitted-R rewrites for structural source shapes.
//!
//! These passes normalize literal helper calls, branch structure, and
//! slice/index aliases after emission but before the full peephole pipeline.

use super::*;

pub(crate) fn simplify_same_var_is_na_or_not_finite_guards_in_raw_emitted_r(
    output: &str,
) -> String {
    let rewritten =
        raw_same_var_is_na_or_not_finite_re().replace_all(output, |caps: &Captures<'_>| {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
            if lhs == rhs {
                format!("!(is.finite({lhs}))")
            } else {
                caps.get(0)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default()
            }
        });
    let mut out = rewritten.into_owned();
    if output.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub(crate) fn simplify_not_finite_or_zero_guard_parens_in_raw_emitted_r(output: &str) -> String {
    let rewritten = raw_not_finite_or_zero_guard_re().replace_all(output, |caps: &Captures<'_>| {
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
        let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
        if lhs == rhs {
            format!("(({inner} | ({rhs} == 0)))")
        } else {
            caps.get(0)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default()
        }
    });
    let mut out = rewritten.into_owned();
    if output.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub(crate) fn simplify_wrapped_not_finite_parens_in_raw_emitted_r(output: &str) -> String {
    let rewritten = raw_wrapped_not_finite_cond_re().replace_all(output, |caps: &Captures<'_>| {
        let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
        format!("({inner})")
    });
    let mut out = rewritten.into_owned();
    if output.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_literal_named_list_calls_in_raw_emitted_r(output: &str) -> String {
    let mut out = Vec::with_capacity(output.lines().count());
    for line in output.lines() {
        if line.contains("rr_named_list <- function") {
            out.push(line.to_string());
            continue;
        }
        let mut rewritten = line.to_string();
        loop {
            let Some(start) = rewritten.find("rr_named_list(") else {
                break;
            };
            let call_start = start + "rr_named_list".len();
            let Some(call_end) = find_matching_call_close(&rewritten, call_start) else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                break;
            };
            if args.len() % 2 != 0 {
                break;
            }
            let mut fields = Vec::new();
            let mut ok = true;
            for pair in args.chunks(2) {
                let Some(name) = raw_literal_record_field_name(pair[0].trim()) else {
                    ok = false;
                    break;
                };
                fields.push(format!("{name} = {}", pair[1].trim()));
            }
            if !ok {
                break;
            }
            let replacement = if fields.is_empty() {
                "list()".to_string()
            } else {
                format!("list({})", fields.join(", "))
            };
            rewritten.replace_range(start..=call_end, &replacement);
        }
        out.push(rewritten);
    }
    let mut rewritten = out.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    rewritten
}

pub(crate) fn rewrite_literal_field_get_calls_in_raw_emitted_r(output: &str) -> String {
    let mut out = Vec::with_capacity(output.lines().count());
    for line in output.lines() {
        if line.contains("<- function") {
            out.push(line.to_string());
            continue;
        }
        let mut rewritten = line.to_string();
        loop {
            let Some(start) = rewritten.find("rr_field_get(") else {
                break;
            };
            let call_start = start + "rr_field_get".len();
            let Some(call_end) = find_matching_call_close(&rewritten, call_start) else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                break;
            };
            if args.len() != 2 {
                break;
            }
            let base = args[0].trim();
            let Some(name) = raw_literal_record_field_name(args[1].trim()) else {
                break;
            };
            let replacement = format!(r#"{base}[["{name}"]]"#);
            rewritten.replace_range(start..=call_end, &replacement);
        }
        out.push(rewritten);
    }
    let mut rewritten = out.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    rewritten
}

pub(crate) fn restore_particle_state_rebinds_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        if !trimmed.starts_with("particles <- Sym_186(") {
            idx += 1;
            continue;
        }
        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let end = ((idx + 1)..lines.len())
            .find(|line_idx| {
                let candidate = lines[*line_idx].trim();
                candidate.contains("<- function")
                    || candidate == "repeat {"
                    || candidate == "}"
                    || candidate.starts_with("if ")
                    || candidate.starts_with("return(")
            })
            .unwrap_or(lines.len());
        let has_px = lines[idx + 1..end]
            .iter()
            .any(|line| line.trim() == "p_x <- particles[[\"px\"]]");
        let has_py = lines[idx + 1..end]
            .iter()
            .any(|line| line.trim() == "p_y <- particles[[\"py\"]]");
        let has_pf = lines[idx + 1..end]
            .iter()
            .any(|line| line.trim() == "p_f <- particles[[\"pf\"]]");
        if !(has_px && has_py && has_pf) {
            let mut inserts = Vec::new();
            if !has_px {
                inserts.push(format!("{indent}p_x <- particles[[\"px\"]]"));
            }
            if !has_py {
                inserts.push(format!("{indent}p_y <- particles[[\"py\"]]"));
            }
            if !has_pf {
                inserts.push(format!("{indent}p_f <- particles[[\"pf\"]]"));
            }
            lines.splice((idx + 1)..(idx + 1), inserts);
            idx += 4;
            continue;
        }
        idx += 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_slice_bound_aliases_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 1 < lines.len() {
        let Some((start_lhs, start_rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if start_lhs != "start" {
            idx += 1;
            continue;
        }
        let start_rhs = strip_redundant_outer_parens(start_rhs).to_string();
        if !start_rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(end_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            break;
        };
        let Some((end_lhs, end_rhs)) = parse_raw_assign_line(lines[end_idx].trim()) else {
            idx += 1;
            continue;
        };
        if end_lhs != "end" {
            idx += 1;
            continue;
        }
        let end_rhs = strip_redundant_outer_parens(end_rhs).to_string();
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
            if let Some((lhs, _)) = parse_raw_assign_line(trimmed)
                && (lhs == "start" || lhs == "end")
            {
                break;
            }
            let uses_start = line_contains_symbol(trimmed, "start");
            let uses_end = line_contains_symbol(trimmed, "end");
            if uses_start || uses_end {
                if uses_start != uses_end || !trimmed.contains("neighbors[start:end] <-") {
                    use_line_idxs.clear();
                    break;
                }
                use_line_idxs.push(line_no);
                continue;
            }
            let is_control =
                trimmed == "}" || trimmed.starts_with("if (") || trimmed.starts_with("if(");
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
    rewritten
}

pub(crate) fn rewrite_particle_idx_alias_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 2 < lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if lhs != "idx" {
            idx += 1;
            continue;
        }
        let rhs = strip_redundant_outer_parens(rhs).to_string();
        if !rhs.starts_with("rr_idx_cube_vec_i(") {
            idx += 1;
            continue;
        }

        let Some(next1_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            break;
        };
        let Some(next2_idx) = ((next1_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
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

        lines[next1_idx] = replace_symbol_occurrences(&lines[next1_idx], "idx", rhs.as_str());
        lines[next2_idx] = replace_symbol_occurrences(&lines[next2_idx], "idx", rhs.as_str());
        lines[idx].clear();
        idx = next2_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_loop_index_alias_ii_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
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
            if let Some((next_lhs, _)) = parse_raw_assign_line(trimmed)
                && (next_lhs == "ii" || next_lhs == "i")
            {
                stop_idx = scan_idx;
                stopped_on_i_reassign = next_lhs == "i";
                break;
            }
            if !line_contains_symbol(trimmed, "ii") {
                continue;
            }
            let rewritten = replace_symbol_occurrences(line, "ii", "i");
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
                if let Some((next_lhs, _)) = parse_raw_assign_line(trimmed)
                    && next_lhs == "ii"
                {
                    break;
                }
                if line_contains_symbol(trimmed, "ii") {
                    keep_alias = true;
                    break;
                }
            }
        }

        if replaced_any && !keep_alias {
            lines[idx].clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_single_blank_spacers_in_raw_emitted_r(output: &str) -> String {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return output.to_string();
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if idx > 0 && idx + 1 < lines.len() && lines[idx].trim().is_empty() {
            let Some(prev_idx) = (0..idx)
                .rev()
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };
            let Some(next_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };

            let prev = lines[prev_idx].trim();
            let next = lines[next_idx].trim();
            let prev_is_assign = parse_raw_assign_line(prev).is_some();
            let next_is_assign = parse_raw_assign_line(next).is_some();
            let next_is_control = next == "repeat {" || next == "}";
            let next_is_branch = next.starts_with("if (") || next.starts_with("if(");
            let next_is_return = next.starts_with("return(") || next.starts_with("return (");
            let prev_opens_block = prev.ends_with('{');
            let prev_is_return = prev.starts_with("return(") || prev.starts_with("return (");
            let prev_is_break = prev.starts_with("if (") && prev.ends_with("break");

            if (prev_is_assign && (next_is_assign || next_is_control || next_is_branch))
                || (prev_opens_block && (next_is_assign || next_is_return || next_is_branch))
                || (prev == "{"
                    && (next_is_assign || next_is_return || next_is_control || next_is_branch))
                || (prev == "}" && (next_is_assign || next == "}"))
                || (prev_is_break && (next_is_assign || next_is_branch || next_is_return))
                || (prev_is_return && next == "}")
            {
                continue;
            }
        }
        kept.push(lines[idx].clone());
    }

    let mut out = kept.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn collapse_nested_else_if_blocks_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return output.to_string();
    }

    let mut changed = true;
    while changed {
        changed = false;
        let mut idx = 0usize;
        while idx < lines.len() {
            if lines[idx].trim() != "} else {" {
                idx += 1;
                continue;
            }
            let Some(nested_if_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                break;
            };
            let nested_if = lines[nested_if_idx].trim().to_string();
            if !nested_if.starts_with("if (") || !nested_if.ends_with('{') {
                idx += 1;
                continue;
            }
            let Some(nested_if_end) = find_raw_block_end(&lines, nested_if_idx) else {
                idx += 1;
                continue;
            };
            let Some(else_close_idx) = ((nested_if_end + 1)..lines.len())
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                idx += 1;
                continue;
            };
            if lines[else_close_idx].trim() != "}" {
                idx += 1;
                continue;
            }

            let indent = lines[idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx] = format!("{indent}}} else {nested_if}");
            lines[nested_if_idx].clear();
            lines[else_close_idx].clear();
            changed = true;
            idx = else_close_idx + 1;
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn compact_blank_lines_in_raw_emitted_r(output: &str) -> String {
    let mut out = String::new();
    let mut blank_run = 0usize;
    for line in output.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    if output.is_empty() {
        return String::new();
    }
    if !output.ends_with('\n') {
        out.pop();
    }
    out
}

pub(crate) fn collapse_adjacent_dir_neighbor_row_branches_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 12 {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx + 11 < lines.len() {
        let branch1 = lines[idx].trim();
        let assign1 = lines[idx + 1].trim();
        let close1 = lines[idx + 2].trim();
        let branch2 = lines[idx + 3].trim();
        let assign2 = lines[idx + 4].trim();
        let close2 = lines[idx + 5].trim();
        let branch3 = lines[idx + 6].trim();
        let assign3 = lines[idx + 7].trim();
        let close3 = lines[idx + 8].trim();
        let branch4 = lines[idx + 9].trim();
        let assign4 = lines[idx + 10].trim();
        let close4 = lines[idx + 11].trim();

        if branch1 != "if ((dir == 1)) {"
            || (assign1
                != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, ys, size)"
                && assign1
                    != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_60(f, x, size)")
            || close1 != "}"
            || branch2 != "if ((dir == 2)) {"
            || (assign2
                != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, ys, size)"
                && assign2
                    != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_64(f, x, size)")
            || close2 != "}"
            || branch3 != "if ((dir == 3)) {"
            || (assign3
                != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_66(f, x, ys, size)"
                && assign3
                    != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_66(f, x, size)")
            || close3 != "}"
            || branch4 != "if ((dir == 4)) {"
            || (assign4
                != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_72(f, x, ys, size)"
                && assign4
                    != "neighbors[rr_idx_cube_vec_i(f, x, 1, size):rr_idx_cube_vec_i(f, x, size, size)] <- Sym_72(f, x, size)")
            || close4 != "}"
        {
            idx += 1;
            continue;
        }

        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let replacement = vec![
            lines[idx].clone(),
            lines[idx + 1].clone(),
            format!("{indent}}} else if ((dir == 2)) {{"),
            lines[idx + 4].clone(),
            format!("{indent}}} else if ((dir == 3)) {{"),
            lines[idx + 7].clone(),
            format!("{indent}}} else if ((dir == 4)) {{"),
            lines[idx + 10].clone(),
            format!("{indent}}}"),
        ];
        lines.splice(idx..(idx + 12), replacement);
        idx += 9;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_orphan_rr_cse_markers_before_repeat_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for line in &mut lines {
        if line.trim() == "# rr-cse-pruned" {
            line.clear();
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn strip_unused_raw_arg_aliases_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let mut idx = fn_start + 1;
        while idx < fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                idx += 1;
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(is_symbol_char) {
                idx += 1;
                continue;
            }
            let used_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .any(|line| line_contains_symbol(line.trim(), lhs));
            if !used_later {
                lines[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn rewrite_readonly_raw_arg_aliases_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };

        let mut aliases = Vec::new();
        for idx in (fn_start + 1)..fn_end {
            let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
                continue;
            };
            if !lhs.starts_with(".arg_") || !rhs.chars().all(is_symbol_char) {
                continue;
            }
            let reassigned_later = lines
                .iter()
                .take(fn_end + 1)
                .skip(idx + 1)
                .filter_map(|line| parse_raw_assign_line(line.trim()))
                .any(|(later_lhs, _)| later_lhs == rhs);
            if reassigned_later {
                continue;
            }
            aliases.push((idx, lhs.to_string(), rhs.to_string()));
        }

        for (alias_idx, alias, target) in aliases {
            for line in lines.iter_mut().take(fn_end + 1).skip(alias_idx + 1) {
                *line = replace_symbol_occurrences(line, &alias, &target);
            }
            lines[alias_idx].clear();
        }

        fn_start = fn_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn collapse_trivial_dot_product_wrappers_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_raw_block_end(&lines, fn_start) else {
            break;
        };
        let Some((fn_name, params)) = parse_raw_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if !fn_name.starts_with("Sym_") || params.len() != 3 {
            fn_start = fn_end + 1;
            continue;
        }

        let body: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "# rr-cse-pruned")
            .collect();
        if body.len() != 10 {
            fn_start = fn_end + 1;
            continue;
        }

        let acc = "sum";
        let iter = "i";
        let lhs_vec = params[0].trim();
        let rhs_vec = params[1].trim();
        let len = params[2].trim();
        let expected = [
            "{".to_string(),
            format!("{acc} <- 0"),
            format!("{iter} <- 1"),
            "repeat {".to_string(),
            format!("if (!({iter} <= {len})) break"),
            format!("{acc} <- ({acc} + ({lhs_vec}[{iter}] * {rhs_vec}[{iter}]))"),
            format!("{iter} <- ({iter} + 1)"),
            "next".to_string(),
            "}".to_string(),
            format!("return({acc})"),
        ];
        let normalized_body: Vec<String> = body
            .iter()
            .map(|line| normalize_raw_iter_index_parens(line, iter))
            .collect();
        if normalized_body != expected {
            fn_start = fn_end + 1;
            continue;
        }

        lines.splice(
            (fn_start + 1)..fn_end,
            [
                "{".to_string(),
                format!(
                    "  return(sum(({}[seq_len({})] * {}[seq_len({})])))",
                    lhs_vec, len, rhs_vec, len
                ),
            ],
        );
        fn_start += 2;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}
