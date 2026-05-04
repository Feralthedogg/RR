use super::*;
pub(crate) fn apply_collapse_inlined_copy_vec_sequences_ir(program: &mut EmittedProgram) {
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
                    (dest == out_var
                        && start == i_var
                        && end == n_var
                        && rest.starts_with("rr_index1_read_vec("))
                    .then_some(rest.to_string())
                } else {
                    None
                }
            }) else {
                continue;
            };
            if out_var != out_replay_lhs
                || target_var != out_var
                || n_rhs != format!("length({out_rhs})")
                || i_rhs != "1"
                || !target_rhs.starts_with("rr_assign_slice(")
                || !src_var.contains(out_rhs)
            {
                continue;
            }
            function.body[idx].clear();
            function.body[idx + 2].clear();
            function.body[idx + 3].clear();
        }
    }
}

pub(crate) fn has_inlined_copy_vec_sequence_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let body = &lines[func.body_start..=func.end];
            body.iter().any(|line| line.contains("inlined_"))
                && body.iter().any(|line| line.contains("rr_assign_slice("))
                && body.iter().any(|line| line.contains("length("))
                && body.iter().any(|line| line.contains("rep.int("))
        })
}
