use super::*;

pub(crate) fn prune_unreachable_raw_helper_definitions(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    loop {
        let mut changed = false;
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
            let Some((name, _params)) = parse_raw_function_header(&lines[fn_start]) else {
                fn_start = fn_end + 1;
                continue;
            };
            if !name.starts_with("Sym_") || name.starts_with("Sym_top_") {
                fn_start = fn_end + 1;
                continue;
            }

            let mut reachable = false;
            for (line_idx, line) in lines.iter().enumerate() {
                if line_idx >= fn_start && line_idx <= fn_end {
                    continue;
                }
                if find_symbol_call(line, &name, 0).is_some()
                    || line_contains_unquoted_symbol_reference(line, &name)
                {
                    reachable = true;
                    break;
                }
            }
            if reachable {
                fn_start = fn_end + 1;
                continue;
            }

            for line in lines.iter_mut().take(fn_end + 1).skip(fn_start) {
                line.clear();
            }
            changed = true;
            break;
        }
        if !changed {
            break;
        }
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((gx_lhs, gx_rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let gx_rhs = gx_rhs.to_string();
        if gx_lhs != "gx" {
            idx += 1;
            continue;
        }
        let Some(gy_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            break;
        };
        let Some((gy_lhs, gy_rhs)) = parse_raw_assign_line(lines[gy_idx].trim()) else {
            idx += 1;
            continue;
        };
        let gy_rhs = gy_rhs.to_string();
        if gy_lhs != "gy" {
            idx += 1;
            continue;
        }

        let seq = [
            ((gy_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 2)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 3)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 4)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 5)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 6)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 7)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 8)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 9)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 10)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 11)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
            ((gy_idx + 12)..lines.len()).find(|i| !lines[*i].trim().is_empty()),
        ];
        if seq.iter().any(|idx| idx.is_none()) {
            idx += 1;
            continue;
        }
        let indices: Vec<usize> = seq.into_iter().flatten().collect();
        let Ok(
            [
                gx_lt_guard_idx,
                gx_lt_assign_idx,
                gx_lt_close_idx,
                gx_gt_guard_idx,
                gx_gt_assign_idx,
                gx_gt_close_idx,
                gy_lt_guard_idx,
                gy_lt_assign_idx,
                gy_lt_close_idx,
                gy_gt_guard_idx,
                gy_gt_assign_idx,
                gy_gt_close_idx,
            ],
        ) = <[usize; 12]>::try_from(indices)
        else {
            idx += 1;
            continue;
        };

        let gx_lt_guard = lines[gx_lt_guard_idx].trim();
        let gx_lt_assign = lines[gx_lt_assign_idx].trim();
        let gx_lt_close = lines[gx_lt_close_idx].trim();
        let gx_gt_guard = lines[gx_gt_guard_idx].trim();
        let gx_gt_assign = lines[gx_gt_assign_idx].trim();
        let gx_gt_close = lines[gx_gt_close_idx].trim();
        let gy_lt_guard = lines[gy_lt_guard_idx].trim();
        let gy_lt_assign = lines[gy_lt_assign_idx].trim();
        let gy_lt_close = lines[gy_lt_close_idx].trim();
        let gy_gt_guard = lines[gy_gt_guard_idx].trim();
        let gy_gt_assign = lines[gy_gt_assign_idx].trim();
        let gy_gt_close = lines[gy_gt_close_idx].trim();

        if gx_lt_guard != "if ((gx < 1)) {"
            || gx_lt_assign != "gx <- 1"
            || gx_lt_close != "}"
            || gx_gt_guard != "if ((gx > N)) {"
            || gx_gt_assign != "gx <- N"
            || gx_gt_close != "}"
            || gy_lt_guard != "if ((gy < 1)) {"
            || gy_lt_assign != "gy <- 1"
            || gy_lt_close != "}"
            || gy_gt_guard != "if ((gy > N)) {"
            || gy_gt_assign != "gy <- N"
            || gy_gt_close != "}"
        {
            idx += 1;
            continue;
        }

        let gx_indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let gy_indent = lines[gy_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx] = format!(
            "{gx_indent}gx <- (pmin(pmax({}, {}), {}))",
            strip_redundant_outer_parens(&gx_rhs),
            "1",
            "N"
        );
        lines[gy_idx] = format!(
            "{gy_indent}gy <- (pmin(pmax({}, {}), {}))",
            strip_redundant_outer_parens(&gy_rhs),
            "1",
            "N"
        );
        for clear_idx in [
            gx_lt_guard_idx,
            gx_lt_assign_idx,
            gx_lt_close_idx,
            gx_gt_guard_idx,
            gx_gt_assign_idx,
            gx_gt_close_idx,
            gy_lt_guard_idx,
            gy_lt_assign_idx,
            gy_lt_close_idx,
            gy_gt_guard_idx,
            gy_gt_assign_idx,
            gy_gt_close_idx,
        ] {
            lines[clear_idx].clear();
        }
        idx = gy_gt_close_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn collapse_gray_scott_clamp_pair_in_raw_emitted_r(output: &str) -> String {
    pub(crate) fn raw_zero_like_assign(lhs: &str, rhs: &str, target: &str) -> bool {
        lhs == target && matches!(rhs, "0" | "0.0" | "0L" | "0.0L")
    }

    pub(crate) fn raw_one_like_assign(lhs: &str, rhs: &str, target: &str) -> bool {
        lhs == target && matches!(rhs, "1" | "1.0" | "1L" | "1.0L")
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((new_a_lhs, new_a_rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        if new_a_lhs != "new_a" {
            idx += 1;
            continue;
        }
        let new_a_rhs = new_a_rhs.to_string();

        let Some(new_b_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            break;
        };
        let Some((new_b_lhs, new_b_rhs)) = parse_raw_assign_line(lines[new_b_idx].trim()) else {
            idx += 1;
            continue;
        };
        if new_b_lhs != "new_b" {
            idx += 1;
            continue;
        }
        let new_b_rhs = new_b_rhs.to_string();

        let Some(a_lt_guard_idx) =
            ((new_b_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_lt_assign_idx) =
            ((a_lt_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_lt_close_idx) =
            ((a_lt_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_gt_guard_idx) =
            ((a_lt_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_gt_assign_idx) =
            ((a_gt_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(a_gt_close_idx) =
            ((a_gt_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_lt_guard_idx) =
            ((a_gt_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_lt_assign_idx) =
            ((b_lt_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_lt_close_idx) =
            ((b_lt_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_gt_guard_idx) =
            ((b_lt_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_gt_assign_idx) =
            ((b_gt_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(b_gt_close_idx) =
            ((b_gt_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };

        let a_lt_guard = lines[a_lt_guard_idx].trim();
        let a_lt_assign = lines[a_lt_assign_idx].trim();
        let a_lt_close = lines[a_lt_close_idx].trim();
        let a_gt_guard = lines[a_gt_guard_idx].trim();
        let a_gt_assign = lines[a_gt_assign_idx].trim();
        let a_gt_close = lines[a_gt_close_idx].trim();
        let b_lt_guard = lines[b_lt_guard_idx].trim();
        let b_lt_assign = lines[b_lt_assign_idx].trim();
        let b_lt_close = lines[b_lt_close_idx].trim();
        let b_gt_guard = lines[b_gt_guard_idx].trim();
        let b_gt_assign = lines[b_gt_assign_idx].trim();
        let b_gt_close = lines[b_gt_close_idx].trim();

        if !(a_lt_guard == "if ((new_a < 0)) {" || a_lt_guard == "if ((new_a < 0.0)) {")
            || !parse_raw_assign_line(a_lt_assign)
                .is_some_and(|(lhs, rhs)| raw_zero_like_assign(lhs, rhs, "new_a"))
            || a_lt_close != "}"
            || !(a_gt_guard == "if ((new_a > 1)) {" || a_gt_guard == "if ((new_a > 1.0)) {")
            || !parse_raw_assign_line(a_gt_assign)
                .is_some_and(|(lhs, rhs)| raw_one_like_assign(lhs, rhs, "new_a"))
            || a_gt_close != "}"
            || !(b_lt_guard == "if ((new_b < 0)) {" || b_lt_guard == "if ((new_b < 0.0)) {")
            || !parse_raw_assign_line(b_lt_assign)
                .is_some_and(|(lhs, rhs)| raw_zero_like_assign(lhs, rhs, "new_b"))
            || b_lt_close != "}"
            || !(b_gt_guard == "if ((new_b > 1)) {" || b_gt_guard == "if ((new_b > 1.0)) {")
            || !parse_raw_assign_line(b_gt_assign)
                .is_some_and(|(lhs, rhs)| raw_one_like_assign(lhs, rhs, "new_b"))
            || b_gt_close != "}"
        {
            idx += 1;
            continue;
        }

        let new_a_indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let new_b_indent = lines[new_b_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[idx] = format!(
            "{new_a_indent}new_a <- (pmin(pmax({}, 0), 1))",
            strip_redundant_outer_parens(&new_a_rhs)
        );
        lines[new_b_idx] = format!(
            "{new_b_indent}new_b <- (pmin(pmax({}, 0), 1))",
            strip_redundant_outer_parens(&new_b_rhs)
        );
        for clear_idx in [
            a_lt_guard_idx,
            a_lt_assign_idx,
            a_lt_close_idx,
            a_gt_guard_idx,
            a_gt_assign_idx,
            a_gt_close_idx,
            b_lt_guard_idx,
            b_lt_assign_idx,
            b_lt_close_idx,
            b_gt_guard_idx,
            b_gt_assign_idx,
            b_gt_close_idx,
        ] {
            lines[clear_idx].clear();
        }
        idx = b_gt_close_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn restore_cg_loop_carried_updates_in_raw_emitted_r(output: &str) -> String {
    pub(crate) fn next_significant_line(lines: &[String], start: usize) -> Option<usize> {
        (start..lines.len()).find(|idx| !lines[*idx].trim().is_empty())
    }

    pub(crate) fn find_enclosing_repeat_start(lines: &[String], idx: usize) -> Option<usize> {
        (0..idx).rev().find(|line_idx| {
            lines[*line_idx].trim() == "repeat {"
                && find_raw_block_end(lines, *line_idx).is_some_and(|end| idx < end)
        })
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    for idx in 0..lines.len() {
        if lines[idx].trim() != "x <- (x + (alpha * p))" {
            continue;
        }
        let Some(mut rs_new_idx) = next_significant_line(&lines, idx + 1) else {
            break;
        };
        let mut has_r_update = false;
        if lines[rs_new_idx].trim() == "r <- (r - (alpha * Ap))" {
            has_r_update = true;
            let Some(next_idx) = next_significant_line(&lines, rs_new_idx + 1) else {
                continue;
            };
            rs_new_idx = next_idx;
        }

        let rs_new_trimmed = lines[rs_new_idx].trim().to_string();
        let rs_new_matches = rs_new_trimmed
            == "rs_new <- Sym_117((r - (alpha * Ap)), (r - (alpha * Ap)), size)"
            || rs_new_trimmed == "rs_new <- Sym_117(r - (alpha * Ap), r - (alpha * Ap), size)"
            || rs_new_trimmed
                == "rs_new <- sum(((r - (alpha * Ap))[seq_len(size)] * (r - (alpha * Ap))[seq_len(size)]))"
            || rs_new_trimmed == "rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))";
        if !rs_new_matches {
            continue;
        }

        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();

        let repeat_start = find_enclosing_repeat_start(&lines, idx);
        let repeat_end = repeat_start.and_then(|start| find_raw_block_end(&lines, start));
        let ap_line_idx = repeat_start.and_then(|repeat_start| {
            (0..repeat_start).rev().find(|line_idx| {
                let trimmed = lines[*line_idx].trim();
                trimmed == "Ap <- ((4.0001 * p) - (((rr_gather(p, rr_index_vec_floor(n_l)) + rr_gather(p, rr_index_vec_floor(n_r))) + rr_gather(p, rr_index_vec_floor(n_d))) + rr_gather(p, rr_index_vec_floor(n_u))))"
                    || trimmed == "Ap <- Sym_119(p, n_l, n_r, n_d, n_u, size)"
            })
        });
        let p_ap_idx =
            next_significant_line(&lines, repeat_start.map_or(idx + 1, |start| start + 1))
                .filter(|line_idx| lines[*line_idx].trim().starts_with("p_Ap <- "));
        if let (Some(ap_line_idx), Some(p_ap_idx)) = (ap_line_idx, p_ap_idx) {
            let has_ap_in_loop = lines
                .iter()
                .take(p_ap_idx)
                .skip(repeat_start.map_or(0, |start| start + 1))
                .any(|line| line.trim().starts_with("Ap <- "));
            if !has_ap_in_loop {
                lines.insert(p_ap_idx, format!("{indent}{}", lines[ap_line_idx].trim()));
                if rs_new_idx >= p_ap_idx {
                    rs_new_idx += 1;
                }
            }
        }

        if !has_r_update {
            lines.insert(rs_new_idx, format!("{indent}r <- (r - (alpha * Ap))"));
            rs_new_idx += 1;
        }
        lines[rs_new_idx] = format!("{indent}rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))");

        if let Some(guard_idx) = next_significant_line(&lines, rs_new_idx + 1)
            && lines[guard_idx].trim().starts_with("if ")
            && line_contains_symbol(lines[guard_idx].trim(), "rs_new")
            && let Some(guard_end) = find_raw_block_end(&lines, guard_idx)
        {
            let else_idx = ((guard_idx + 1)..=guard_end)
                .find(|line_idx| lines[*line_idx].trim() == "} else {");
            let else_assign_idx = else_idx.and_then(|else_idx| {
                next_significant_line(&lines, else_idx + 1).filter(|idx| *idx < guard_end)
            });
            if else_assign_idx.is_some_and(|assign_idx| lines[assign_idx].trim() == rs_new_trimmed)
            {
                let body_indent = format!("{indent}  ");
                lines.splice(
                    guard_idx..=guard_end,
                    [
                        lines[guard_idx].clone(),
                        format!("{body_indent}rs_new <- rs_old"),
                        format!("{indent}}}"),
                    ],
                );
            }
        }

        let search_end = repeat_end.unwrap_or(lines.len());
        let Some(beta_idx) = ((rs_new_idx + 1)..search_end).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty() && trimmed == "beta <- (rs_new / rs_old)"
        }) else {
            continue;
        };
        let Some(mut iter_idx) = ((beta_idx + 1)..search_end).find(|i| {
            let trimmed = lines[*i].trim();
            !trimmed.is_empty()
                && (trimmed == "iter <- (iter + 1)" || trimmed == "iter <- (iter + 1.0)")
        }) else {
            continue;
        };

        let has_p_update = ((beta_idx + 1)..iter_idx)
            .any(|line_idx| lines[line_idx].trim() == "p <- (r + (beta * p))");
        if !has_p_update {
            lines.insert(iter_idx, format!("{indent}p <- (r + (beta * p))"));
            iter_idx += 1;
        }
        let has_rs_old_update =
            ((beta_idx + 1)..iter_idx).any(|line_idx| lines[line_idx].trim() == "rs_old <- rs_new");
        if !has_rs_old_update {
            lines.insert(iter_idx, format!("{indent}rs_old <- rs_new"));
        }
        break;
    }

    let mut repeat_idx = 0usize;
    while repeat_idx < lines.len() {
        let Some(loop_start) =
            (repeat_idx..lines.len()).find(|idx| lines[*idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_raw_block_end(&lines, loop_start) else {
            break;
        };
        let has_cg_shape = lines
            .iter()
            .take(loop_end)
            .skip(loop_start + 1)
            .any(|line| line.trim() == "x <- (x + (alpha * p))")
            && lines
                .iter()
                .take(loop_end)
                .skip(loop_start + 1)
                .any(|line| line.trim() == "beta <- (rs_new / rs_old)");
        if !has_cg_shape {
            repeat_idx = loop_end + 1;
            continue;
        }

        let Some(beta_idx) = ((loop_start + 1)..loop_end)
            .find(|idx| lines[*idx].trim() == "beta <- (rs_new / rs_old)")
        else {
            repeat_idx = loop_end + 1;
            continue;
        };
        let Some(mut iter_idx) = ((beta_idx + 1)..loop_end).find(|idx| {
            let trimmed = lines[*idx].trim();
            trimmed == "iter <- (iter + 1)" || trimmed == "iter <- (iter + 1.0)"
        }) else {
            repeat_idx = loop_end + 1;
            continue;
        };

        let indent = lines[beta_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let has_p_update =
            ((beta_idx + 1)..iter_idx).any(|idx| lines[idx].trim() == "p <- (r + (beta * p))");
        if !has_p_update {
            lines.insert(iter_idx, format!("{indent}p <- (r + (beta * p))"));
            iter_idx += 1;
        }
        let has_rs_old_update =
            ((beta_idx + 1)..iter_idx).any(|idx| lines[idx].trim() == "rs_old <- rs_new");
        if !has_rs_old_update {
            lines.insert(iter_idx, format!("{indent}rs_old <- rs_new"));
        }
        repeat_idx = loop_end + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn restore_buffer_swaps_after_temp_copy_in_raw_emitted_r(output: &str) -> String {
    pub(crate) fn raw_line_writes_symbol(line: &str, symbol: &str) -> bool {
        let trimmed = line.trim();
        parse_raw_assign_line(trimmed).is_some_and(|(lhs, _)| lhs == symbol)
            || trimmed.starts_with(&format!("{symbol}["))
            || trimmed.starts_with(&format!("({symbol}) <-"))
    }

    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        let Some((lhs, rhs)) = parse_raw_assign_line(lines[idx].trim()) else {
            idx += 1;
            continue;
        };
        let lhs = lhs.to_string();
        let rhs = rhs.to_string();
        let Some(base_var) = lhs.strip_prefix("tmp_") else {
            idx += 1;
            continue;
        };
        if rhs != base_var {
            idx += 1;
            continue;
        }

        let Some((loop_start, loop_end)) = (0..idx).rev().find_map(|line_idx| {
            (lines[line_idx].trim() == "repeat {")
                .then(|| find_raw_block_end(&lines, line_idx).map(|end| (line_idx, end)))
                .flatten()
                .filter(|(_, end)| idx < *end)
        }) else {
            idx += 1;
            continue;
        };

        let candidates = [format!("{base_var}_new"), format!("next_{base_var}")];
        let candidate = candidates.into_iter().find(|candidate| {
            lines
                .iter()
                .take(idx)
                .skip(loop_start + 1)
                .any(|line| raw_line_writes_symbol(line, candidate))
        });
        let Some(candidate) = candidate else {
            idx += 1;
            continue;
        };

        let has_base_swap = lines
            .iter()
            .take(loop_end)
            .skip(idx + 1)
            .any(|line| line.trim() == format!("{base_var} <- {candidate}"));
        let has_candidate_swap = lines
            .iter()
            .take(loop_end)
            .skip(idx + 1)
            .any(|line| line.trim() == format!("{candidate} <- {lhs}"));
        if has_base_swap || has_candidate_swap {
            idx += 1;
            continue;
        }

        let indent = lines[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines.insert(idx + 1, format!("{indent}{base_var} <- {candidate}"));
        lines.insert(idx + 2, format!("{indent}{candidate} <- {lhs}"));
        idx += 3;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn collapse_sym287_melt_rate_branch_in_raw_emitted_r(output: &str) -> String {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut idx = 0usize;
    while idx < lines.len() {
        if lines[idx].trim() != "if ((T_c > 0)) {" {
            idx += 1;
            continue;
        }

        let Some(zero_idx) = ((idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty()) else {
            break;
        };
        let Some(qs_guard_idx) =
            ((zero_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qs_assign_idx) =
            ((qs_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qs_close_idx) =
            ((qs_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qg_guard_idx) =
            ((qs_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qg_assign_idx) =
            ((qg_guard_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(qg_close_idx) =
            ((qg_assign_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(tendency_idx) =
            ((qg_close_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };
        let Some(close_idx) =
            ((tendency_idx + 1)..lines.len()).find(|i| !lines[*i].trim().is_empty())
        else {
            idx += 1;
            continue;
        };

        if lines[zero_idx].trim() != "melt_rate <- 0"
            || lines[qs_guard_idx].trim() != "if ((q_s[i] > 0)) {"
            || lines[qs_assign_idx].trim() != "melt_rate <- (q_s[i] * 0.05)"
            || lines[qs_close_idx].trim() != "}"
            || lines[qg_guard_idx].trim() != "if ((q_g[i] > 0)) {"
            || lines[qg_assign_idx].trim() != "melt_rate <- (melt_rate + (q_g[i] * 0.02))"
            || lines[qg_close_idx].trim() != "}"
            || lines[tendency_idx].trim() != "tendency_T <- (tendency_T - (melt_rate * L_f))"
            || lines[close_idx].trim() != "}"
        {
            idx += 1;
            continue;
        }

        let qs_indent = lines[qs_assign_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let qg_indent = lines[qg_assign_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        lines[qs_assign_idx] =
            format!("{qs_indent}tendency_T <- (tendency_T - ((q_s[i] * 0.05) * L_f))");
        lines[qg_assign_idx] =
            format!("{qg_indent}tendency_T <- (tendency_T - ((q_g[i] * 0.02) * L_f))");
        lines[zero_idx].clear();
        lines[tendency_idx].clear();
        idx = close_idx + 1;
    }

    let mut out = lines.join("\n");
    if output.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}
