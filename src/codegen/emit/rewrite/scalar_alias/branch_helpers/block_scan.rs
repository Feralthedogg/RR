fn enclosing_branch_start_local(lines: &[String], idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for prev_idx in (0..idx).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "}" {
            depth += 1;
            continue;
        }
        if trimmed.ends_with('{') {
            if depth == 0 {
                return (trimmed.starts_with("if ") || trimmed.starts_with("if("))
                    .then_some(prev_idx);
            }
            depth = depth.saturating_sub(1);
        }
    }
    None
}

fn find_block_end_local(lines: &[String], start_idx: usize) -> Option<usize> {
    let mut depth = 0isize;
    let mut saw_open = false;
    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
        for ch in line.chars() {
            match ch {
                '{' => {
                    depth += 1;
                    saw_open = true;
                }
                '}' => depth -= 1,
                _ => {}
            }
        }
        if saw_open && depth <= 0 {
            return Some(idx);
        }
    }
    None
}

fn raw_is_loop_open_boundary_local(trimmed: &str) -> bool {
    trimmed == "repeat {"
        || trimmed.starts_with("while ")
        || trimmed.starts_with("while(")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("for(")
}

fn is_control_flow_boundary_local(trimmed: &str) -> bool {
    let is_single_line_guard =
        trimmed.starts_with("if ") && (trimmed.ends_with(" break") || trimmed.ends_with(" next"));
    trimmed == "{"
        || trimmed == "}"
        || trimmed == "repeat {"
        || (trimmed.starts_with("if ") && !is_single_line_guard)
        || trimmed.starts_with("if(")
        || trimmed.starts_with("else")
        || trimmed.starts_with("} else")
        || trimmed.starts_with("while")
        || trimmed.starts_with("for")
        || trimmed == "break"
        || trimmed == "next"
}

fn raw_line_is_within_loop_body_local(lines: &[String], idx: usize) -> bool {
    (0..idx).rev().any(|start_idx| {
        if !raw_is_loop_open_boundary_local(lines[start_idx].trim()) {
            return false;
        }
        find_block_end_local(lines, start_idx).is_some_and(|end_idx| idx < end_idx)
    })
}
