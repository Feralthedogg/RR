pub(super) fn strip_noop_self_assignments(output: &mut String) {
    let mut out = String::new();
    for line in output.lines() {
        let keep = if let Some((lhs, rhs)) = parse_local_assign_line(line.trim()) {
            lhs != strip_outer_parens_local(rhs)
        } else {
            true
        };
        if keep {
            out.push_str(line);
            out.push('\n');
        }
    }
    if output.is_empty() {
        *output = String::new();
        return;
    }
    if !output.ends_with('\n') {
        out.pop();
    }
    *output = out;
}

pub(super) fn strip_empty_else_blocks(output: &mut String) {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }

    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim() == "} else {" {
            let mut close_idx = i + 1;
            while close_idx < lines.len() && lines[close_idx].trim().is_empty() {
                close_idx += 1;
            }
            if close_idx < lines.len() && lines[close_idx].trim() == "}" {
                let indent_len = line.len() - line.trim_start().len();
                let indent = &line[..indent_len];
                out.push(format!("{indent}}}"));
                i = close_idx + 1;
                continue;
            }
        }
        out.push(line.clone());
        i += 1;
    }

    let mut rendered = out.join("\n");
    if output.ends_with('\n') || !rendered.is_empty() {
        rendered.push('\n');
    }
    *output = rendered;
}

pub(super) fn collapse_nested_else_if_blocks(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return;
    }

    let mut changed = true;
    while changed {
        changed = false;
        let mut idx = 0usize;
        while idx < lines.len() {
            if lines[idx].trim() != "} else {" {
                idx += 1;
                continue;
            }
            let Some(nested_if_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                break;
            };
            let nested_if = lines[nested_if_idx].trim().to_string();
            if !nested_if.starts_with("if (") || !nested_if.ends_with('{') {
                idx += 1;
                continue;
            }
            let Some(nested_if_end) = find_block_end_local(&lines, nested_if_idx) else {
                idx += 1;
                continue;
            };
            let Some(else_close_idx) = ((nested_if_end + 1)..lines.len())
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                idx += 1;
                continue;
            };
            if lines[else_close_idx].trim() != "}" {
                idx += 1;
                continue;
            }

            let indent = lines[idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            lines[idx] = format!("{indent}}} else {nested_if}");
            lines[nested_if_idx].clear();
            lines[else_close_idx].clear();
            changed = true;
            idx = else_close_idx + 1;
        }
    }

    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
