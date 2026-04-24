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

pub(in super::super) fn strip_empty_else_blocks_ir(lines: Vec<String>) -> Vec<String> {
    if !has_empty_else_block_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_empty_else_blocks_ir(&mut program);
    program.into_lines()
}

fn apply_strip_empty_else_blocks_ir(program: &mut EmittedProgram) {
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

pub(in super::super) fn strip_dead_simple_eval_lines_ir(lines: Vec<String>) -> Vec<String> {
    if !scan_basic_cleanup_candidates_ir(&lines).needs_dead_eval {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_dead_simple_eval_lines_ir(&mut program);
    program.into_lines()
}

fn apply_strip_dead_simple_eval_lines_ir(program: &mut EmittedProgram) {
    program.items = program
        .items
        .drain(..)
        .filter_map(|item| match item {
            EmittedItem::Raw(line) => {
                let trimmed = line.trim();
                (!is_dead_plain_ident_eval_line(trimmed)
                    && !is_dead_parenthesized_eval_line(trimmed))
                .then_some(EmittedItem::Raw(line))
            }
            EmittedItem::Function(mut function) => {
                function.body.retain(|stmt| {
                    let trimmed = stmt.text.trim();
                    !is_dead_plain_ident_eval_line(trimmed)
                        && !is_dead_parenthesized_eval_line(trimmed)
                });
                Some(EmittedItem::Function(function))
            }
        })
        .collect();
}

fn is_noop_self_assign_stmt(stmt: &EmittedStmt) -> bool {
    stmt.assign_parts()
        .is_some_and(|(lhs, rhs)| lhs.trim() == rhs.trim())
}

pub(in super::super) fn strip_noop_self_assignments_ir(lines: Vec<String>) -> Vec<String> {
    if !scan_basic_cleanup_candidates_ir(&lines).needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_noop_self_assignments_ir(&mut program);
    program.into_lines()
}

fn apply_strip_noop_self_assignments_ir(program: &mut EmittedProgram) {
    program.items = program
        .items
        .drain(..)
        .filter_map(|item| match item {
            EmittedItem::Raw(line) => {
                let stmt = EmittedStmt::parse(&line);
                (!is_noop_self_assign_stmt(&stmt)).then_some(EmittedItem::Raw(line))
            }
            EmittedItem::Function(mut function) => {
                function.body.retain(|stmt| !is_noop_self_assign_stmt(stmt));
                Some(EmittedItem::Function(function))
            }
        })
        .collect();
}

#[derive(Clone, Copy, Debug, Default)]
struct BasicCleanupCandidatesIr {
    needs_dead_eval: bool,
    needs_noop_assign: bool,
    needs_nested_temp: bool,
}

fn scan_basic_cleanup_candidates_ir(lines: &[String]) -> BasicCleanupCandidatesIr {
    let mut out = BasicCleanupCandidatesIr::default();
    for line in lines {
        let trimmed = line.trim();
        if !out.needs_dead_eval
            && (is_dead_plain_ident_eval_line(trimmed) || is_dead_parenthesized_eval_line(trimmed))
        {
            out.needs_dead_eval = true;
        }
        if !out.needs_noop_assign
            && assign_re()
                .and_then(|re| re.captures(trimmed))
                .is_some_and(|caps| {
                    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                    lhs == rhs
                })
        {
            out.needs_noop_assign = true;
        }
        if !out.needs_nested_temp
            && assign_re()
                .and_then(|re| re.captures(trimmed))
                .is_some_and(|caps| {
                    caps.name("lhs")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .trim()
                        .starts_with(".__rr_cse_")
                })
        {
            out.needs_nested_temp = true;
        }
        if out.needs_dead_eval && out.needs_noop_assign && out.needs_nested_temp {
            break;
        }
    }
    out
}

pub(in super::super) fn run_dead_eval_noop_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    if !needs_dead_eval && !needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    program.into_lines()
}

pub(in super::super) fn run_secondary_finalize_cleanup_bundle_ir(
    lines: Vec<String>,
    preserve_all_defs: bool,
) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_tail_assign = has_tail_assign_slice_return_candidates_ir(&lines);
    let needs_unreachable = !preserve_all_defs && has_unreachable_sym_helper_candidates_ir(&lines);
    if !needs_dead_eval && !needs_noop_assign && !needs_tail_assign && !needs_unreachable {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_secondary_finalize_cleanup_bundle_ir(
        &mut program,
        needs_dead_eval,
        needs_noop_assign,
        needs_unreachable,
        needs_tail_assign,
    );
    program.into_lines()
}

fn apply_secondary_finalize_cleanup_bundle_ir(
    program: &mut EmittedProgram,
    needs_dead_eval: bool,
    needs_noop_assign: bool,
    needs_unreachable: bool,
    needs_tail_assign: bool,
) {
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(program);
    }
    if needs_unreachable {
        apply_strip_unreachable_sym_helpers_ir(program);
    }
    if needs_tail_assign {
        apply_strip_redundant_tail_assign_slice_return_ir(program);
    }
}

pub(in super::super) fn run_secondary_empty_else_finalize_bundle_ir(
    lines: Vec<String>,
    preserve_all_defs: bool,
) -> Vec<String> {
    let needs_empty_else = has_empty_else_block_candidates_ir(&lines);
    let needs_match_phi = has_restore_empty_match_single_bind_candidates_ir(&lines);
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_tail_assign = has_tail_assign_slice_return_candidates_ir(&lines);
    let needs_unreachable = !preserve_all_defs && has_unreachable_sym_helper_candidates_ir(&lines);
    if !needs_empty_else
        && !needs_match_phi
        && !needs_dead_eval
        && !needs_noop_assign
        && !needs_tail_assign
        && !needs_unreachable
    {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_match_phi {
        apply_restore_empty_match_single_bind_arms_ir(&mut program);
    }
    if needs_empty_else {
        apply_strip_empty_else_blocks_ir(&mut program);
    }
    apply_secondary_finalize_cleanup_bundle_ir(
        &mut program,
        needs_dead_eval,
        needs_noop_assign,
        needs_unreachable,
        needs_tail_assign,
    );
    program.into_lines()
}

pub(in super::super) fn run_empty_else_match_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
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

fn parse_singleton_list_match_cond_ir(line: &str) -> Option<String> {
    let pattern = format!(
        r#"^if \(\(\(length\((?P<base>{})\) == 1L\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some(caps.name("base")?.as_str().to_string())
}

fn parse_single_field_record_match_cond_ir(line: &str) -> Option<(String, String)> {
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

fn has_restore_empty_match_single_bind_candidates_ir(lines: &[String]) -> bool {
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

fn has_empty_else_block_candidates_ir(lines: &[String]) -> bool {
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

fn apply_restore_empty_match_single_bind_arms_ir(program: &mut EmittedProgram) {
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

fn apply_strip_unreachable_sym_helpers_ir(program: &mut EmittedProgram) {
    let mut item_index_by_name = FxHashMap::<String, usize>::default();
    for (item_idx, item) in program.items.iter().enumerate() {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, _)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        if fn_name.starts_with("Sym_") {
            item_index_by_name.insert(fn_name, item_idx);
        }
    }
    if item_index_by_name.is_empty() {
        return;
    }

    let sym_top_is_empty_entrypoint = |function: &EmittedFunction| {
        let mut saw_return_null = false;
        for stmt in &function.body {
            let trimmed = stmt.text.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            if trimmed == "return(NULL)" {
                saw_return_null = true;
                continue;
            }
            if !unquoted_sym_refs(trimmed).is_empty() {
                return false;
            }
            return false;
        }
        saw_return_null
    };

    let mut roots = FxHashSet::default();
    if item_index_by_name.contains_key("Sym_top_0") {
        roots.insert("Sym_top_0".to_string());
    }
    for item in &program.items {
        if let EmittedItem::Raw(line) = item {
            for name in unquoted_sym_refs(line) {
                if item_index_by_name.contains_key(&name) {
                    roots.insert(name);
                }
            }
        }
    }
    if roots.is_empty() {
        return;
    }
    if roots.len() == 1
        && roots.contains("Sym_top_0")
        && item_index_by_name
            .get("Sym_top_0")
            .and_then(|idx| match &program.items[*idx] {
                EmittedItem::Function(function) => Some(function),
                _ => None,
            })
            .is_some_and(sym_top_is_empty_entrypoint)
    {
        return;
    }

    let mut reachable = roots.clone();
    let mut work: Vec<String> = roots.into_iter().collect();
    while let Some(name) = work.pop() {
        let Some(item_idx) = item_index_by_name.get(&name) else {
            continue;
        };
        let EmittedItem::Function(function) = &program.items[*item_idx] else {
            continue;
        };
        for stmt in &function.body {
            for callee in unquoted_sym_refs(&stmt.text) {
                if item_index_by_name.contains_key(&callee) && reachable.insert(callee.clone()) {
                    work.push(callee);
                }
            }
        }
    }

    program.items.retain(|item| match item {
        EmittedItem::Raw(_) => true,
        EmittedItem::Function(function) => {
            parse_function_header_ir(&function.header).is_none_or(|(fn_name, _)| {
                !fn_name.starts_with("Sym_") || reachable.contains(&fn_name)
            })
        }
    });
}
