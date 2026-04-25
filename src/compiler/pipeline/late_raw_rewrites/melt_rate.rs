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
