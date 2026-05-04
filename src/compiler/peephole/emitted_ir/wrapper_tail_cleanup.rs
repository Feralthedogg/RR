use super::*;
pub(crate) fn has_assignment_to_one_before_ir(lines: &[String], idx: usize, var: &str) -> bool {
    (0..idx).rev().any(|i| {
        assign_re()
            .and_then(|re| re.captures(lines[i].trim()))
            .is_some_and(|caps| {
                caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == var
                    && literal_one_re().is_some_and(|re| {
                        re.is_match(caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim())
                    })
            })
    })
}

pub(crate) fn function_has_matching_exprmap_whole_assign_ir(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    if !temp_var.starts_with(".tachyon_exprmap") {
        return false;
    }
    let Some(temp_idx) = lines.iter().position(|line| {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| {
                caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == temp_var
            })
    }) else {
        return false;
    };
    let Some(temp_rhs) = assign_re()
        .and_then(|re| re.captures(lines[temp_idx].trim()))
        .and_then(|caps| caps.name("rhs").map(|m| m.as_str().trim().to_string()))
    else {
        return false;
    };

    for line in lines.iter().skip(temp_idx + 1) {
        let trimmed = line.trim();
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
        if lhs == dest_var
            && dest == dest_var
            && end == end_expr
            && (rest == temp_rhs || rest == temp_var)
        {
            return true;
        }
    }
    false
}

pub(crate) fn function_has_non_empty_repeat_whole_assign_ir(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    let Some(temp_idx) = lines.iter().position(|line| {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| {
                caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == temp_var
            })
    }) else {
        return false;
    };
    let Some(temp_rhs) = assign_re()
        .and_then(|re| re.captures(lines[temp_idx].trim()))
        .and_then(|caps| caps.name("rhs").map(|m| m.as_str().trim().to_string()))
    else {
        return false;
    };

    let mut assign_idx = None;
    for idx in temp_idx + 1..lines.len() {
        let trimmed = lines[idx].trim();
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
        if lhs == dest_var
            && dest == dest_var
            && end == end_expr
            && (rest == temp_rhs || rest == temp_var)
            && plain_ident_re().is_some_and(|re| re.is_match(start))
            && has_assignment_to_one_before_ir(lines, idx, start)
        {
            assign_idx = Some(idx);
            break;
        }
    }
    let Some(assign_idx) = assign_idx else {
        return false;
    };

    let Some(repeat_idx) = (0..assign_idx)
        .rev()
        .find(|idx| lines[*idx].trim() == "repeat {")
    else {
        return false;
    };
    let Some(guard_idx) = (repeat_idx + 1..assign_idx).find(|idx| {
        lines[*idx].trim().starts_with("if !(") || lines[*idx].trim().starts_with("if (!(")
    }) else {
        return false;
    };
    let guard = lines[guard_idx].trim();
    let Some(inner) = guard
        .strip_prefix("if (!(")
        .and_then(|s| s.strip_suffix(")) break"))
    else {
        return false;
    };
    let Some((iter_var, bound)) = inner.split_once("<=") else {
        return false;
    };
    literal_positive_re().is_some_and(|re| re.is_match(bound.trim()))
        && has_assignment_to_one_before_ir(lines, guard_idx, iter_var.trim())
}

pub(crate) fn strip_redundant_tail_assign_slice_return_ir(lines: Vec<String>) -> Vec<String> {
    if !has_tail_assign_slice_return_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_redundant_tail_assign_slice_return_ir(&mut program);
    program.into_lines()
}

pub(crate) fn apply_strip_redundant_tail_assign_slice_return_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let return_trimmed = function.body[return_idx].text.trim().to_string();
        let Some(ret_var) = return_trimmed
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let Some(assign_idx) = previous_non_empty_stmt(&function.body, return_idx) else {
            continue;
        };
        let Some((lhs, rhs)) = function.body[assign_idx].assign_parts() else {
            continue;
        };
        if lhs.trim() != ret_var {
            continue;
        }

        let Some(assign_caps) = assign_slice_re().and_then(|re| re.captures(rhs.trim())) else {
            continue;
        };
        let dest = assign_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let start = assign_caps
            .name("start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = assign_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let temp = assign_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if dest != ret_var
            || !literal_one_re().is_some_and(|re| re.is_match(start))
            || !plain_ident_re().is_some_and(|re| re.is_match(temp))
        {
            continue;
        }

        let mut fn_lines = Vec::with_capacity(function.body.len() + 1);
        fn_lines.push(function.header.clone());
        for stmt in &function.body {
            if stmt.text.trim() != "}" {
                fn_lines.push(stmt.text.clone());
            }
        }
        if function_has_non_empty_repeat_whole_assign_ir(&fn_lines, ret_var, end, temp)
            || function_has_matching_exprmap_whole_assign_ir(&fn_lines, ret_var, end, temp)
        {
            function.body[assign_idx].clear();
        }
    }
}

pub(crate) fn strip_redundant_nested_temp_reassigns_ir(lines: Vec<String>) -> Vec<String> {
    if !scan_basic_cleanup_candidates_ir(&lines).needs_nested_temp {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_redundant_nested_temp_reassigns_ir(&mut program);
    program.into_lines()
}

pub(crate) fn apply_strip_redundant_nested_temp_reassigns_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut remove = vec![false; function.body.len()];
        for (idx, remove_slot) in remove.iter_mut().enumerate().take(function.body.len()) {
            let Some((lhs, rhs)) = function.body[idx].assign_parts() else {
                continue;
            };
            let lhs = lhs.trim();
            let rhs = rhs.trim();
            if !lhs.starts_with(".__rr_cse_") {
                continue;
            }
            let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
            let cur_indent = function.body[idx].indent().len();
            let mut j = idx;
            while j > 0 {
                j -= 1;
                let prev = function.body[j].text.trim();
                if prev.is_empty() {
                    continue;
                }
                if matches!(
                    function.body[j].kind,
                    EmittedStmtKind::RepeatOpen
                        | EmittedStmtKind::ForSeqLen { .. }
                        | EmittedStmtKind::ForOpen
                        | EmittedStmtKind::WhileOpen
                ) {
                    break;
                }
                if let Some((prev_lhs, prev_rhs)) = function.body[j].assign_parts() {
                    let prev_lhs = prev_lhs.trim();
                    let prev_rhs = prev_rhs.trim();
                    if prev_lhs == lhs {
                        if prev_rhs == lhs {
                            continue;
                        }
                        let prev_indent = function.body[j].indent().len();
                        if prev_rhs == rhs && prev_indent < cur_indent {
                            *remove_slot = true;
                        }
                        break;
                    }
                    if deps.contains(prev_lhs) {
                        break;
                    }
                }
            }
        }
        function.body = function
            .body
            .drain(..)
            .enumerate()
            .filter_map(|(idx, stmt)| (!remove[idx]).then_some(stmt))
            .collect();
    }
}

pub(crate) fn run_exact_pre_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_nested_temp = scan.needs_nested_temp;
    if !needs_dead_eval && !needs_noop_assign && !needs_nested_temp {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    if needs_nested_temp {
        apply_strip_redundant_nested_temp_reassigns_ir(&mut program);
    }
    program.into_lines()
}
