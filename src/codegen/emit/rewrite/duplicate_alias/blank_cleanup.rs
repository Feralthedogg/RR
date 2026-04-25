pub(super) fn strip_single_blank_spacers(output: &mut String) {
    let lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.len() < 3 {
        return;
    }

    let mut kept = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if idx > 0 && idx + 1 < lines.len() && lines[idx].trim().is_empty() {
            let Some(prev_idx) = (0..idx)
                .rev()
                .find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };
            let Some(next_idx) =
                ((idx + 1)..lines.len()).find(|line_idx| !lines[*line_idx].trim().is_empty())
            else {
                continue;
            };

            let prev = lines[prev_idx].trim();
            let next = lines[next_idx].trim();
            let prev_is_assign = parse_local_assign_line(prev).is_some();
            let next_is_assign = parse_local_assign_line(next).is_some();
            let next_is_control = next == "repeat {" || next == "}";
            let next_is_branch = next.starts_with("if (") || next.starts_with("if(");
            let next_is_return = next.starts_with("return(") || next.starts_with("return (");
            let prev_opens_block = prev.ends_with('{');
            let prev_is_return = prev.starts_with("return(") || prev.starts_with("return (");
            let prev_is_break = prev.starts_with("if (") && prev.ends_with("break");

            if (prev_is_assign && (next_is_assign || next_is_control || next_is_branch))
                || (prev_opens_block && (next_is_assign || next_is_return || next_is_branch))
                || (prev == "{"
                    && (next_is_assign || next_is_return || next_is_control || next_is_branch))
                || (prev == "}" && (next_is_assign || next == "}"))
                || (prev_is_break && (next_is_assign || next_is_branch || next_is_return))
                || (prev_is_return && next == "}")
            {
                continue;
            }
        }
        kept.push(lines[idx].clone());
    }

    let mut rewritten = kept.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}

pub(super) fn compact_blank_lines(output: &mut String) {
    let mut out = String::new();
    let mut blank_run = 0usize;
    for line in output.lines() {
        if line.trim().is_empty() {
            blank_run += 1;
            if blank_run > 1 {
                continue;
            }
        } else {
            blank_run = 0;
        }
        out.push_str(line);
        out.push('\n');
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

pub(super) fn strip_orphan_rr_cse_pruned_markers(output: &mut String) {
    let mut lines: Vec<String> = output.lines().map(|line| line.to_string()).collect();
    if lines.is_empty() {
        return;
    }
    for line in &mut lines {
        if line.trim() == "# rr-cse-pruned" {
            line.clear();
        }
    }
    let mut rewritten = lines.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
