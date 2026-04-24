fn is_identical_pure_rebind_candidate_ir(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    let is_pure_call = rhs.contains('(') && expr_has_only_pure_calls(rhs, pure_user_calls);
    let is_literal_field_read = literal_field_read_expr_re().is_some_and(|re| re.is_match(rhs));
    plain_ident_re().is_some_and(|re| re.is_match(lhs))
        && !lhs.starts_with(".arg_")
        && !lhs.starts_with(".__rr_cse_")
        && (is_pure_call || is_literal_field_read)
}

pub(in super::super) fn strip_redundant_identical_pure_rebinds_ir(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    if !lines.iter().any(|line| line.contains("<-")) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_redundant_identical_pure_rebinds_ir(&mut program, pure_user_calls);
    program.into_lines()
}

fn apply_strip_redundant_identical_pure_rebinds_ir(
    program: &mut EmittedProgram,
    pure_user_calls: &FxHashSet<String>,
) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut removable = vec![false; function.body.len()];
        for idx in 0..function.body.len() {
            let EmittedStmtKind::Assign { lhs, rhs } = &function.body[idx].kind else {
                continue;
            };
            if !is_identical_pure_rebind_candidate_ir(lhs, rhs, pure_user_calls) {
                continue;
            }
            let rhs_canonical = strip_redundant_outer_parens(rhs);
            let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
            let cur_indent = function.body[idx].indent().len();
            let mut depth = 0usize;
            let mut crossed_enclosing_if_boundary = false;
            let mut found = false;
            for prev_idx in (0..idx).rev() {
                let prev_stmt = &function.body[prev_idx];
                match &prev_stmt.kind {
                    EmittedStmtKind::Blank => continue,
                    EmittedStmtKind::BlockClose => {
                        depth += 1;
                        continue;
                    }
                    EmittedStmtKind::IfOpen | EmittedStmtKind::ElseOpen => {
                        if depth > 0 {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                            continue;
                        }
                        if matches!(prev_stmt.kind, EmittedStmtKind::IfOpen)
                            && !crossed_enclosing_if_boundary
                        {
                            crossed_enclosing_if_boundary = true;
                            continue;
                        }
                        break;
                    }
                    EmittedStmtKind::RepeatOpen
                    | EmittedStmtKind::ForSeqLen { .. }
                    | EmittedStmtKind::ForOpen
                    | EmittedStmtKind::WhileOpen
                    | EmittedStmtKind::OtherOpen => {
                        if depth > 0 {
                            depth -= 1;
                        }
                        continue;
                    }
                    EmittedStmtKind::Assign {
                        lhs: prev_lhs,
                        rhs: prev_rhs,
                    } => {
                        if prev_lhs == lhs {
                            if depth == 0 {
                                let prev_indent = prev_stmt.indent().len();
                                let same_scope_rebind = prev_indent == cur_indent;
                                let enclosing_if_rebind = crossed_enclosing_if_boundary;
                                if strip_redundant_outer_parens(prev_rhs) == rhs_canonical
                                    && (same_scope_rebind || enclosing_if_rebind)
                                {
                                    found = true;
                                }
                            }
                            break;
                        }
                        if deps.contains(prev_lhs) {
                            break;
                        }
                    }
                    _ => {
                        if let Some(base) = indexed_store_base_re()
                            .and_then(|re| re.captures(prev_stmt.text.trim()))
                            .and_then(|caps| {
                                caps.name("base").map(|m| m.as_str().trim().to_string())
                            })
                        {
                            if base == *lhs || deps.contains(&base) {
                                break;
                            }
                        }
                    }
                }
            }
            removable[idx] = found;
        }
        function.body = function
            .body
            .drain(..)
            .enumerate()
            .filter_map(|(idx, stmt)| (!removable[idx]).then_some(stmt))
            .collect();
    }
}
