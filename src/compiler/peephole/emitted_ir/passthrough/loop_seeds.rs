use super::*;
pub(crate) fn rewrite_dead_zero_loop_seeds_before_for_ir(lines: Vec<String>) -> Vec<String> {
    if !has_dead_zero_loop_seed_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_dead_zero_loop_seeds_before_for_ir(&mut program);
    program.into_lines()
}

pub(crate) fn apply_rewrite_dead_zero_loop_seeds_before_for_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut removable = vec![false; function.body.len()];
        for (idx, removable_slot) in removable.iter_mut().enumerate().take(function.body.len()) {
            let EmittedStmtKind::Assign { lhs, rhs } = &function.body[idx].kind else {
                continue;
            };
            let seed = rhs.trim();
            if seed != "0" && seed != "1" {
                continue;
            }
            let Some(for_idx) = ((idx + 1)..function.body.len()).take(12).find(|line_idx| {
                matches!(
                    &function.body[*line_idx].kind,
                    EmittedStmtKind::ForSeqLen { iter_var, .. } if iter_var == lhs
                )
            }) else {
                continue;
            };
            let used_before_for = function.body[(idx + 1)..for_idx]
                .iter()
                .any(|stmt| stmt.mentions_ident(lhs));
            if !used_before_for {
                *removable_slot = true;
            }
        }
        function.body = function
            .body
            .drain(..)
            .enumerate()
            .filter_map(|(idx, stmt)| (!removable[idx]).then_some(stmt))
            .collect();
    }
}

pub(crate) fn has_dead_zero_loop_seed_candidates_ir(lines: &[String]) -> bool {
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if rhs != "0" && rhs != "1" {
            continue;
        }
        let Some(for_idx) = ((idx + 1)..lines.len()).take(12).find(|line_idx| {
            parse_for_seq_len_header(lines[*line_idx].trim())
                .is_some_and(|(iter_var, _)| iter_var == lhs)
        }) else {
            continue;
        };
        let used_before_for = lines[(idx + 1)..for_idx]
            .iter()
            .any(|line| expr_idents(line.trim()).iter().any(|ident| ident == lhs));
        if !used_before_for {
            return true;
        }
    }
    false
}
