pub(in super::super) fn strip_dead_simple_eval_lines_ir(lines: Vec<String>) -> Vec<String> {
    if !scan_basic_cleanup_candidates_ir(&lines).needs_dead_eval {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_dead_simple_eval_lines_ir(&mut program);
    program.into_lines()
}

fn apply_strip_dead_simple_eval_lines_ir(program: &mut EmittedProgram) {
    program.items = program
        .items
        .drain(..)
        .filter_map(|item| match item {
            EmittedItem::Raw(line) => {
                let trimmed = line.trim();
                (!is_dead_plain_ident_eval_line(trimmed)
                    && !is_dead_parenthesized_eval_line(trimmed))
                .then_some(EmittedItem::Raw(line))
            }
            EmittedItem::Function(mut function) => {
                function.body.retain(|stmt| {
                    let trimmed = stmt.text.trim();
                    !is_dead_plain_ident_eval_line(trimmed)
                        && !is_dead_parenthesized_eval_line(trimmed)
                });
                Some(EmittedItem::Function(function))
            }
        })
        .collect();
}

fn is_noop_self_assign_stmt(stmt: &EmittedStmt) -> bool {
    stmt.assign_parts()
        .is_some_and(|(lhs, rhs)| lhs.trim() == rhs.trim())
}

pub(in super::super) fn strip_noop_self_assignments_ir(lines: Vec<String>) -> Vec<String> {
    if !scan_basic_cleanup_candidates_ir(&lines).needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_noop_self_assignments_ir(&mut program);
    program.into_lines()
}

fn apply_strip_noop_self_assignments_ir(program: &mut EmittedProgram) {
    program.items = program
        .items
        .drain(..)
        .filter_map(|item| match item {
            EmittedItem::Raw(line) => {
                let stmt = EmittedStmt::parse(&line);
                (!is_noop_self_assign_stmt(&stmt)).then_some(EmittedItem::Raw(line))
            }
            EmittedItem::Function(mut function) => {
                function.body.retain(|stmt| !is_noop_self_assign_stmt(stmt));
                Some(EmittedItem::Function(function))
            }
        })
        .collect();
}

#[derive(Clone, Copy, Debug, Default)]
struct BasicCleanupCandidatesIr {
    needs_dead_eval: bool,
    needs_noop_assign: bool,
    needs_nested_temp: bool,
}

fn scan_basic_cleanup_candidates_ir(lines: &[String]) -> BasicCleanupCandidatesIr {
    let mut out = BasicCleanupCandidatesIr::default();
    for line in lines {
        let trimmed = line.trim();
        if !out.needs_dead_eval
            && (is_dead_plain_ident_eval_line(trimmed) || is_dead_parenthesized_eval_line(trimmed))
        {
            out.needs_dead_eval = true;
        }
        if !out.needs_noop_assign
            && assign_re()
                .and_then(|re| re.captures(trimmed))
                .is_some_and(|caps| {
                    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                    lhs == rhs
                })
        {
            out.needs_noop_assign = true;
        }
        if !out.needs_nested_temp
            && assign_re()
                .and_then(|re| re.captures(trimmed))
                .is_some_and(|caps| {
                    caps.name("lhs")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim()
                        .starts_with(".__rr_cse_")
                })
        {
            out.needs_nested_temp = true;
        }
        if out.needs_dead_eval && out.needs_noop_assign && out.needs_nested_temp {
            break;
        }
    }
    out
}

pub(in super::super) fn run_dead_eval_noop_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    if !needs_dead_eval && !needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    program.into_lines()
}
