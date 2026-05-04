use super::*;

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

pub(crate) fn raw_zero_like_assign(lhs: &str, rhs: &str, target: &str) -> bool {
    lhs == target && matches!(rhs, "0" | "0.0" | "0L" | "0.0L")
}

pub(crate) fn raw_one_like_assign(lhs: &str, rhs: &str, target: &str) -> bool {
    lhs == target && matches!(rhs, "1" | "1.0" | "1L" | "1.0L")
}
