pub(in super::super) fn strip_terminal_repeat_nexts_ir(lines: Vec<String>) -> Vec<String> {
    if !has_terminal_repeat_next_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_terminal_repeat_nexts_ir(&mut program);
    program.into_lines()
}

fn apply_strip_terminal_repeat_nexts_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        let mut stack: Vec<BlockKind> = Vec::new();
        for stmt in function.body.drain(..) {
            match stmt.kind {
                EmittedStmtKind::RepeatOpen => {
                    stack.push(BlockKind::Repeat);
                    out.push(stmt);
                }
                EmittedStmtKind::IfOpen
                | EmittedStmtKind::ForSeqLen { .. }
                | EmittedStmtKind::ForOpen
                | EmittedStmtKind::WhileOpen
                | EmittedStmtKind::OtherOpen => {
                    stack.push(BlockKind::Other);
                    out.push(stmt);
                }
                EmittedStmtKind::ElseOpen => {
                    let _ = stack.pop();
                    stack.push(BlockKind::Other);
                    out.push(stmt);
                }
                EmittedStmtKind::BlockClose => {
                    let closed = stack.pop();
                    if closed == Some(BlockKind::Repeat)
                        && out
                            .iter()
                            .rfind(|prev| !matches!(prev.kind, EmittedStmtKind::Blank))
                            .is_some_and(|prev| matches!(prev.kind, EmittedStmtKind::Next))
                    {
                        if let Some(remove_idx) = out
                            .iter()
                            .rposition(|prev| !matches!(prev.kind, EmittedStmtKind::Blank))
                        {
                            out.remove(remove_idx);
                        }
                    }
                    out.push(stmt);
                }
                _ => out.push(stmt),
            }
        }
        function.body = out;
    }
}
