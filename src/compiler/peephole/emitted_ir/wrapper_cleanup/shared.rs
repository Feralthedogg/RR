use super::*;
pub(crate) fn previous_non_empty_stmt(body: &[EmittedStmt], idx: usize) -> Option<usize> {
    (0..idx).rev().find(|i| !body[*i].text.trim().is_empty())
}

pub(crate) fn find_if_else_bounds_ir(
    body: &[EmittedStmt],
    if_idx: usize,
) -> Option<(usize, usize)> {
    let mut depth = 1usize;
    let mut else_idx = None;
    for (idx, stmt) in body.iter().enumerate().skip(if_idx + 1) {
        match stmt.kind {
            EmittedStmtKind::ElseOpen if depth == 1 => {
                else_idx = Some(idx);
            }
            EmittedStmtKind::IfOpen
            | EmittedStmtKind::RepeatOpen
            | EmittedStmtKind::ForSeqLen { .. }
            | EmittedStmtKind::ForOpen
            | EmittedStmtKind::WhileOpen
            | EmittedStmtKind::OtherOpen => {
                depth += 1;
            }
            EmittedStmtKind::BlockClose => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return else_idx.map(|else_idx| (else_idx, idx));
                }
            }
            _ => {}
        }
    }
    None
}
