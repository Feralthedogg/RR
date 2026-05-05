use super::*;
pub(crate) fn rewrite_forward_exact_expr_reuse_ir(lines: Vec<String>) -> Vec<String> {
    if !lines.iter().any(|line| line.contains("<-")) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    program.into_lines()
}

pub(crate) fn apply_rewrite_forward_exact_expr_reuse_ir(program: &mut EmittedProgram) {
    let debug = std::env::var_os("RR_DEBUG_IR_EXACT_EXPR").is_some();
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let straight_region_ends = compute_straight_line_region_ends(&function.body);
        let assign_line_indices = collect_assign_line_indices(&function.body);
        let mut function_lines = None;
        let candidate_snapshots = function
            .body
            .iter()
            .enumerate()
            .filter_map(|(idx, stmt)| {
                let (lhs, rhs) = stmt.assign_parts()?;
                Some((idx, lhs.to_string(), rhs.to_string()))
            })
            .collect::<Vec<_>>();
        for (idx, lhs, rhs) in candidate_snapshots {
            if idx >= function.body.len() {
                continue;
            }
            let ident_count = expr_idents(&rhs).len();
            let replacement_symbol = prefer_smaller_cse_symbol(&lhs, &rhs);
            if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
                || !expr_is_exact_reusable_scalar(&rhs)
                || (lhs.starts_with(".__rr_cse_") && ident_count > 2 && replacement_symbol == lhs)
            {
                continue;
            }
            let straight_region_end = straight_region_ends[idx];
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
            let deps: FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
            let mut prologue_arg_aliases = None;
            for line_no in idx + 1..scan_end {
                let mut should_break = false;
                let mut should_continue = false;
                let current_text = function.body[line_no].text.clone();
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
                            } else if let Some(new_text) = replace_exact_rhs_occurrence(
                                &function.body[line_no],
                                &rhs,
                                &replacement_symbol,
                            ) {
                                if debug {
                                    eprintln!(
                                        "RR_DEBUG_IR_EXACT_EXPR rewrite cand_line={} lhs=`{}` rhs=`{}` target_line={} before=`{}` after=`{}`",
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
                            let mut same_rhs_as_previous = false;
                            if let Some(prev_idx) =
                                prev_assign_line_before(&assign_line_indices, &next_lhs, line_no)
                            {
                                let Some((_, prev_rhs)) = function.body[prev_idx].assign_parts()
                                else {
                                    should_break = true;
                                    if should_break {
                                        break;
                                    }
                                    continue;
                                };
                                let aliases = prologue_arg_aliases.get_or_insert_with(|| {
                                    let lines = function_lines.get_or_insert_with(|| {
                                        function
                                            .body
                                            .iter()
                                            .map(|stmt| stmt.text.clone())
                                            .collect::<Vec<_>>()
                                    });
                                    collect_prologue_arg_aliases(lines, idx)
                                });
                                let prev_norm = normalize_expr_with_aliases(prev_rhs, aliases);
                                let next_norm = normalize_expr_with_aliases(&next_rhs, aliases);
                                if prev_norm == next_norm {
                                    same_rhs_as_previous = true;
                                }
                            }
                            if same_rhs_as_previous {
                                should_continue = true;
                            } else {
                                should_break = true;
                            }
                        }
                    }
                } else {
                    let line_trimmed = current_text.trim().to_string();
                    if line_trimmed.contains(&rhs)
                        && let Some(new_text) = replace_exact_rhs_occurrence(
                            &function.body[line_no],
                            &rhs,
                            &replacement_symbol,
                        )
                    {
                        if debug {
                            eprintln!(
                                "RR_DEBUG_IR_EXACT_EXPR rewrite cand_line={} lhs=`{}` rhs=`{}` target_line={} before=`{}` after=`{}`",
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
                    continue;
                }
            }
        }
    }
}
