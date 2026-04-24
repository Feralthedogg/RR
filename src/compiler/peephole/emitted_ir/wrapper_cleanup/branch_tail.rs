fn apply_collapse_identical_if_else_tail_assignments_late_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut i = 0usize;
        while i < function.body.len() {
            if !matches!(function.body[i].kind, EmittedStmtKind::IfOpen) {
                i += 1;
                continue;
            }
            let Some((else_idx, end_idx)) = find_if_else_bounds_ir(&function.body, i) else {
                i += 1;
                continue;
            };

            let then_lines: Vec<usize> = ((i + 1)..else_idx)
                .filter(|idx| !function.body[*idx].text.trim().is_empty())
                .collect();
            let else_lines: Vec<usize> = ((else_idx + 1)..end_idx)
                .filter(|idx| !function.body[*idx].text.trim().is_empty())
                .collect();

            let mut t = then_lines.len();
            let mut e = else_lines.len();
            let mut shared = Vec::<(usize, usize, String)>::new();
            while t > 0 && e > 0 {
                let then_idx = then_lines[t - 1];
                let else_line_idx = else_lines[e - 1];
                let then_trimmed = function.body[then_idx].text.trim();
                let else_trimmed = function.body[else_line_idx].text.trim();
                if then_trimmed != else_trimmed {
                    break;
                }
                if assign_re()
                    .and_then(|re| re.captures(then_trimmed))
                    .is_none()
                {
                    break;
                }
                shared.push((then_idx, else_line_idx, then_trimmed.to_string()));
                t -= 1;
                e -= 1;
            }

            if shared.is_empty() {
                i = end_idx + 1;
                continue;
            }

            shared.reverse();
            let indent = function.body[i].indent();
            for (then_idx, else_idx_line, _) in &shared {
                function.body[*then_idx].clear();
                function.body[*else_idx_line].clear();
            }
            let mut insert_at = end_idx + 1;
            for (_, _, assign) in &shared {
                function
                    .body
                    .insert(insert_at, EmittedStmt::parse(&format!("{indent}{assign}")));
                insert_at += 1;
            }
            i = insert_at;
        }
    }
}
