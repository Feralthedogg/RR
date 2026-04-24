fn scalar_rhs_from_singleton_rest_ir(rest: &str) -> Option<String> {
    let trimmed = rest.trim();
    if let Some(inner) = trimmed
        .strip_prefix("rep.int(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let args = split_top_level_args(inner)?;
        if args.len() == 2 && literal_one_re().is_some_and(|re| re.is_match(args[1].trim())) {
            return Some(args[0].trim().to_string());
        }
    }
    (scalar_lit_re().is_some_and(|re| re.is_match(trimmed))
        || plain_ident_re().is_some_and(|re| re.is_match(trimmed)))
    .then_some(trimmed.to_string())
}

fn collapse_singleton_assign_slice_scalar_stmt_text_ir(text: &str) -> Option<String> {
    let trimmed = text.trim();
    let caps = assign_re().and_then(|re| re.captures(trimmed))?;
    let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
    let slice_caps = assign_slice_re().and_then(|re| re.captures(rhs))?;
    let dest = slice_caps
        .name("dest")
        .map(|m| m.as_str())
        .unwrap_or("")
        .trim();
    let start = slice_caps
        .name("start")
        .map(|m| m.as_str())
        .unwrap_or("")
        .trim();
    let end = slice_caps
        .name("end")
        .map(|m| m.as_str())
        .unwrap_or("")
        .trim();
    let rest = slice_caps
        .name("rest")
        .map(|m| m.as_str())
        .unwrap_or("")
        .trim();
    if lhs != dest || start != end {
        return None;
    }
    let scalar_rhs = scalar_rhs_from_singleton_rest_ir(rest)?;
    Some(format!(
        "{indent}{lhs} <- replace({dest}, {start}, {scalar_rhs})"
    ))
}

fn has_singleton_assign_slice_scalar_edit_candidates_ir(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        line.contains("rr_assign_slice(")
            && collapse_singleton_assign_slice_scalar_stmt_text_ir(line).is_some()
    })
}

pub(in super::super) fn collapse_singleton_assign_slice_scalar_edits_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_singleton_assign_slice_scalar_edit_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_singleton_assign_slice_scalar_edits_ir(&mut program);
    program.into_lines()
}

fn apply_collapse_singleton_assign_slice_scalar_edits_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        match item {
            EmittedItem::Raw(line) => {
                if let Some(rewritten) = collapse_singleton_assign_slice_scalar_stmt_text_ir(line) {
                    *line = rewritten;
                }
            }
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    if let Some(rewritten) =
                        collapse_singleton_assign_slice_scalar_stmt_text_ir(&stmt.text)
                    {
                        stmt.replace_text(rewritten);
                    }
                }
            }
        }
    }
}
