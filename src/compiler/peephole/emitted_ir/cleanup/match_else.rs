use super::*;
pub(crate) fn strip_empty_else_blocks_ir(lines: Vec<String>) -> Vec<String> {
    if !has_empty_else_block_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_empty_else_blocks_ir(&mut program);
    program.into_lines()
}

pub(crate) fn apply_strip_empty_else_blocks_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        let mut idx = 0usize;
        while idx < function.body.len() {
            if matches!(function.body[idx].kind, EmittedStmtKind::ElseOpen) {
                let mut close_idx = idx + 1;
                while close_idx < function.body.len()
                    && matches!(function.body[close_idx].kind, EmittedStmtKind::Blank)
                {
                    close_idx += 1;
                }
                if close_idx < function.body.len()
                    && matches!(function.body[close_idx].kind, EmittedStmtKind::BlockClose)
                {
                    out.push(EmittedStmt::block_close_with_indent(
                        &function.body[idx].indent(),
                    ));
                    idx = close_idx + 1;
                    continue;
                }
            }
            out.push(function.body[idx].clone());
            idx += 1;
        }
        function.body = out;
    }
}

pub(crate) fn parse_singleton_list_match_cond_ir(line: &str) -> Option<String> {
    let pattern = format!(
        r#"^if \(\(\(length\((?P<base>{})\) == 1L\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some(caps.name("base")?.as_str().to_string())
}

pub(crate) fn parse_single_field_record_match_cond_ir(line: &str) -> Option<(String, String)> {
    let pattern = format!(
        r#"^if \(\(\(TRUE & rr_field_exists\((?P<base>{}), "(?P<field>[^"]+)"\)\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some((
        caps.name("base")?.as_str().to_string(),
        caps.name("field")?.as_str().to_string(),
    ))
}

pub(crate) fn has_restore_empty_match_single_bind_candidates_ir(lines: &[String]) -> bool {
    for idx in 0..lines.len().saturating_sub(3) {
        if lines[idx + 1].trim() != "} else {" || lines[idx + 3].trim() != "}" {
            continue;
        }
        let Some(phi_caps) = assign_re().and_then(|re| re.captures(lines[idx + 2].trim())) else {
            continue;
        };
        let phi_lhs = phi_caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if !phi_lhs.starts_with(".phi_") {
            continue;
        }
        if parse_singleton_list_match_cond_ir(&lines[idx]).is_some()
            || parse_single_field_record_match_cond_ir(&lines[idx]).is_some()
        {
            return true;
        }
    }
    false
}

pub(crate) fn has_empty_else_block_candidates_ir(lines: &[String]) -> bool {
    for idx in 0..lines.len().saturating_sub(1) {
        if lines[idx].trim() != "} else {" {
            continue;
        }
        let mut close_idx = idx + 1;
        while close_idx < lines.len() && lines[close_idx].trim().is_empty() {
            close_idx += 1;
        }
        if close_idx < lines.len() && lines[close_idx].trim() == "}" {
            return true;
        }
    }
    false
}

pub(crate) fn apply_restore_empty_match_single_bind_arms_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut idx = 0usize;
        while idx + 3 < function.body.len() {
            if function.body[idx + 1].text.trim() != "} else {"
                || function.body[idx + 3].text.trim() != "}"
            {
                idx += 1;
                continue;
            }

            let Some(phi_caps) =
                assign_re().and_then(|re| re.captures(function.body[idx + 2].text.trim()))
            else {
                idx += 1;
                continue;
            };
            let phi_lhs = phi_caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim()
                .to_string();
            if !phi_lhs.starts_with(".phi_") {
                idx += 1;
                continue;
            }

            let indent = function.body[idx + 2].indent();

            if let Some(base) = parse_singleton_list_match_cond_ir(&function.body[idx].text) {
                function.body.insert(
                    idx + 1,
                    EmittedStmt::parse(&format!("{indent}{phi_lhs} <- {base}[1L]")),
                );
                idx += 5;
                continue;
            }

            if let Some((base, field)) =
                parse_single_field_record_match_cond_ir(&function.body[idx].text)
            {
                function.body.insert(
                    idx + 1,
                    EmittedStmt::parse(&format!(r#"{indent}{phi_lhs} <- {base}[["{field}"]]"#)),
                );
                idx += 5;
                continue;
            }

            idx += 1;
        }
    }
}

pub(crate) fn run_empty_else_match_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let needs_empty_else = has_empty_else_block_candidates_ir(&lines);
    let needs_match_phi = has_restore_empty_match_single_bind_candidates_ir(&lines);
    if !needs_empty_else && !needs_match_phi {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_match_phi {
        apply_restore_empty_match_single_bind_arms_ir(&mut program);
    }
    if needs_empty_else {
        apply_strip_empty_else_blocks_ir(&mut program);
    }
    program.into_lines()
}
