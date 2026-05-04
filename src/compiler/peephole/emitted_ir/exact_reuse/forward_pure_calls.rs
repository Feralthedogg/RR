use super::*;
pub(crate) fn rewrite_forward_exact_pure_call_reuse_ir(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    if !lines
        .iter()
        .any(|line| line.contains("<-") && line.contains('('))
    {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_forward_exact_pure_call_reuse_ir(&mut program, pure_user_calls);
    program.into_lines()
}

pub(crate) fn apply_rewrite_forward_exact_pure_call_reuse_ir(
    program: &mut EmittedProgram,
    pure_user_calls: &FxHashSet<String>,
) {
    let debug = std::env::var_os("RR_DEBUG_IR_PURE_CALL").is_some();
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let straight_region_ends = compute_straight_line_region_ends(&function.body);
        let assign_line_indices = collect_assign_line_indices(&function.body);
        for (idx, straight_region_end) in straight_region_ends
            .iter()
            .copied()
            .enumerate()
            .take(function.body.len())
        {
            let Some((lhs, rhs)) = function.body[idx].assign_parts() else {
                continue;
            };
            let lhs = lhs.to_string();
            let rhs = rhs.to_string();
            if !rhs.contains('(') {
                continue;
            }
            if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
                || lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || !expr_has_only_pure_calls(&rhs, pure_user_calls)
            {
                continue;
            }
            let deps: FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
            if deps.contains(&lhs) {
                continue;
            }
            let next_lhs_def =
                next_assign_line_before(&assign_line_indices, &lhs, idx, straight_region_end);
            let lhs_reassigned_later = next_lhs_def < straight_region_end;
            let scan_end = next_lhs_def;
            if scan_end <= idx + 1 {
                continue;
            }
            if !function.body[(idx + 1)..scan_end]
                .iter()
                .any(|stmt| stmt.text.contains(&rhs))
            {
                continue;
            }
            let mut line_no = idx + 1;
            while line_no < scan_end {
                let mut should_break = false;
                let mut should_continue = false;
                let current_text = function.body[line_no].text.clone();
                let line_trimmed = current_text.trim().to_string();
                if let Some(base) = indexed_store_base_re()
                    .and_then(|re| re.captures(&line_trimmed))
                    .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
                    && (base == lhs || deps.contains(&base))
                {
                    should_break = true;
                }
                if should_break {
                    break;
                }
                let assign_parts = function.body[line_no]
                    .assign_parts()
                    .map(|(lhs, rhs)| (lhs.to_string(), rhs.to_string()));
                if let Some((next_lhs, next_rhs)) = assign_parts {
                    if next_lhs == lhs {
                        should_break = true;
                    } else {
                        if next_rhs.contains(&rhs) {
                            if lhs_reassigned_later {
                                should_continue = true;
                            } else if let Some(new_text) =
                                replace_exact_rhs_occurrence(&function.body[line_no], &rhs, &lhs)
                            {
                                if debug {
                                    eprintln!(
                                        "RR_DEBUG_IR_PURE_CALL rewrite cand_line={} lhs=`{}` rhs=`{}` target_line={} before=`{}` after=`{}`",
                                        idx + 1,
                                        lhs,
                                        rhs,
                                        line_no + 1,
                                        function.body[line_no].text.trim(),
                                        new_text.trim()
                                    );
                                }
                                function.body[line_no].replace_text(new_text);
                            }
                        }
                        if deps.contains(&next_lhs) {
                            should_break = true;
                        }
                    }
                } else {
                    if line_trimmed.contains(&rhs)
                        && let Some(new_text) =
                            replace_exact_rhs_occurrence(&function.body[line_no], &rhs, &lhs)
                    {
                        if debug {
                            eprintln!(
                                "RR_DEBUG_IR_PURE_CALL rewrite cand_line={} lhs=`{}` rhs=`{}` target_line={} before=`{}` after=`{}`",
                                idx + 1,
                                lhs,
                                rhs,
                                line_no + 1,
                                function.body[line_no].text.trim(),
                                new_text.trim()
                            );
                        }
                        function.body[line_no].replace_text(new_text);
                    }
                    if line_trimmed == "return(NULL)"
                        || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
                    {
                        should_break = true;
                    }
                }
                if should_break {
                    break;
                }
                if should_continue {
                    line_no += 1;
                    continue;
                }
                line_no += 1;
            }
        }
    }
}
