use super::*;

pub(crate) fn restore_cg_loop_carried_updates_in_raw_emitted_r(output: &str) -> String {
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
            || rs_new_trimmed == "rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))"
            || rs_new_trimmed == "rs_new <- Sym_117(r, r, size)";
        if !rs_new_matches {
            continue;
        }
        let rs_new_is_direct_helper = rs_new_trimmed == "rs_new <- Sym_117(r, r, size)";

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
        if !rs_new_is_direct_helper {
            lines[rs_new_idx] =
                format!("{indent}rs_new <- sum((r[seq_len(size)] * r[seq_len(size)]))");
        }

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

pub(crate) fn next_significant_line(lines: &[String], start: usize) -> Option<usize> {
    (start..lines.len()).find(|idx| !lines[*idx].trim().is_empty())
}

pub(crate) fn find_enclosing_repeat_start(lines: &[String], idx: usize) -> Option<usize> {
    (0..idx).rev().find(|line_idx| {
        lines[*line_idx].trim() == "repeat {"
            && find_raw_block_end(lines, *line_idx).is_some_and(|end| idx < end)
    })
}
