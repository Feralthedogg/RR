pub(in super::super) fn collapse_inlined_copy_vec_sequences_ir(lines: Vec<String>) -> Vec<String> {
    if !has_inlined_copy_vec_sequence_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let len = function.body.len();
        for idx in 0..len.saturating_sub(4) {
            let l0 = function.body[idx].text.trim().to_string();
            let l1 = function.body[idx + 1].text.trim().to_string();
            let l2 = function.body[idx + 2].text.trim().to_string();
            let l3 = function.body[idx + 3].text.trim().to_string();
            let l4 = function.body[idx + 4].text.trim().to_string();
            let Some(c0) = assign_re().and_then(|re| re.captures(&l0)) else {
                continue;
            };
            let Some(c1) = assign_re().and_then(|re| re.captures(&l1)) else {
                continue;
            };
            let Some(c2) = assign_re().and_then(|re| re.captures(&l2)) else {
                continue;
            };
            let Some(c3) = assign_re().and_then(|re| re.captures(&l3)) else {
                continue;
            };
            let Some(c4) = assign_re().and_then(|re| re.captures(&l4)) else {
                continue;
            };
            let n_var = c0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let n_rhs = c0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_var = c1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_rhs = c1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let i_var = c2.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let i_rhs = c2.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let out_replay_lhs = c3.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let src_rhs = c3.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let target_var = c4.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let target_rhs = c4.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let Some(src_var) = ({
                if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(src_rhs)) {
                    let dest = slice_caps
                        .name("dest")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim();
                    let start = slice_caps
                        .name("start")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim();
                    let end = slice_caps
                        .name("end")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim();
                    let rest = slice_caps
                        .name("rest")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim();
                    if dest == out_var
                        && start == i_var
                        && end == n_var
                        && plain_ident_re().is_some_and(|re| re.is_match(rest))
                    {
                        Some(rest.to_string())
                    } else {
                        None
                    }
                } else if plain_ident_re().is_some_and(|re| re.is_match(src_rhs)) {
                    Some(src_rhs.to_string())
                } else {
                    None
                }
            }) else {
                continue;
            };
            if !n_var.starts_with("inlined_")
                || !out_var.starts_with("inlined_")
                || !i_var.starts_with("inlined_")
                || out_replay_lhs != out_var
                || (target_rhs != out_var && target_rhs != src_var)
                || !literal_one_re().is_some_and(|re| re.is_match(i_rhs))
                || !n_rhs.starts_with("length(")
                || !out_rhs.starts_with("rep.int(0, ")
            {
                continue;
            }

            let mut final_assign_idx = None;
            for (search_idx, stmt) in function.body.iter().enumerate().skip(idx + 5) {
                let trimmed = stmt.text.trim();
                let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                    continue;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
                    continue;
                };
                let dest = slice_caps
                    .name("dest")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let start = slice_caps
                    .name("start")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let end = slice_caps
                    .name("end")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let rest = slice_caps
                    .name("rest")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if lhs == src_var
                    && dest == out_var
                    && start == i_var
                    && end == n_var
                    && rest == src_var
                {
                    final_assign_idx = Some(search_idx);
                    break;
                }
            }
            let Some(final_idx) = final_assign_idx else {
                continue;
            };
            let indent = function.body[idx + 4].indent();
            function.body[idx].clear();
            function.body[idx + 1].clear();
            function.body[idx + 2].clear();
            function.body[idx + 3].clear();
            function.body[idx + 4].replace_text(format!("{indent}{target_var} <- {src_var}"));
            let final_indent = function.body[final_idx].indent();
            function.body[final_idx]
                .replace_text(format!("{final_indent}{src_var} <- {target_var}"));
        }
    }
    program.into_lines()
}
