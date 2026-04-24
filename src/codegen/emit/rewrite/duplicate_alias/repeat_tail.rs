fn find_matching_open_brace_line_local(lines: &[String], close_idx: usize) -> Option<usize> {
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(close_idx + 1) {
        for ch in line.chars() {
            match ch {
                '{' => stack.push(idx),
                '}' => {
                    let open = stack.pop()?;
                    if idx == close_idx {
                        return Some(open);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

pub(super) fn strip_terminal_repeat_nexts(output: &mut String) {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 2 {
        return;
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if lines[idx].trim() == "next"
            && idx + 1 < lines.len()
            && lines[idx + 1].trim() == "}"
            && find_matching_open_brace_line_local(&lines, idx + 1)
                .is_some_and(|open_idx| lines[open_idx].trim() == "repeat {")
        {
            continue;
        }
        kept.push(lines[idx].clone());
    }

    let mut rewritten = kept.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
