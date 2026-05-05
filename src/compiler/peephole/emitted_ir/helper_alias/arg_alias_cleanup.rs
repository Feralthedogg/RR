use super::*;
pub(crate) fn rewrite_readonly_param_aliases_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_readonly_param_aliases_ir(&mut program);
    program.into_lines()
}

pub(crate) fn run_arg_alias_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unused_arg_aliases_ir(&mut program);
    apply_rewrite_readonly_param_aliases_ir(&mut program);
    apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
    apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    program.into_lines()
}

pub(crate) fn strip_unused_arg_aliases_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unused_arg_aliases_ir(&mut program);
    program.into_lines()
}

pub(crate) fn apply_strip_unused_arg_aliases_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let prologue_defs = collect_prologue_arg_alias_defs_ir(&function.body);
        if prologue_defs.is_empty() {
            continue;
        }
        let (_assigned, _stored_bases, mentioned) =
            collect_post_prologue_assignment_facts_ir(&function.body);
        for (idx, alias, _target) in prologue_defs {
            if !mentioned.contains(&alias)
                && let Some(stmt) = function.body.get_mut(idx)
            {
                stmt.clear();
            }
        }
    }
}

pub(crate) fn apply_rewrite_readonly_param_aliases_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((_, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let param_set: FxHashSet<String> = params.into_iter().collect();
        let prologue_defs = collect_prologue_arg_alias_defs_ir(&function.body);
        let mutated = collect_mutated_arg_aliases_ir(&function.body);
        let (assigned, stored_bases, _mentioned) =
            collect_post_prologue_assignment_facts_ir(&function.body);
        let mut safe_aliases = FxHashMap::default();
        for (_idx, alias, target) in &prologue_defs {
            if !param_set.contains(target) || mutated.contains(alias) {
                continue;
            }
            if assigned.contains(alias)
                || assigned.contains(target)
                || stored_bases.contains(alias)
                || stored_bases.contains(target)
            {
                continue;
            }
            safe_aliases.insert(alias.clone(), target.clone());
        }
        if safe_aliases.is_empty() {
            continue;
        }
        for stmt in &mut function.body {
            if !stmt.text.contains(".arg_") {
                continue;
            }
            if matches!(&stmt.kind, EmittedStmtKind::Assign { lhs, .. } if safe_aliases.contains_key(lhs))
            {
                stmt.clear();
                continue;
            }
            let rewritten = rewrite_known_aliases(&stmt.text, &safe_aliases);
            if rewritten != stmt.text {
                stmt.replace_text(rewritten);
            }
        }
    }
}

pub(crate) fn rewrite_remaining_readonly_param_shadow_uses_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
    program.into_lines()
}

pub(crate) fn apply_rewrite_remaining_readonly_param_shadow_uses_ir(program: &mut EmittedProgram) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((_, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let mutated = collect_mutated_arg_aliases_ir(&function.body);
        let (_assigned, _stored_bases, mentioned) =
            collect_post_prologue_assignment_facts_ir(&function.body);
        let mut safe_aliases = FxHashMap::default();
        for param in params {
            if !plain_ident_re().is_some_and(|re| re.is_match(&param)) {
                continue;
            }
            let alias = format!(".arg_{param}");
            if mutated.contains(&alias) {
                continue;
            }
            if mentioned.contains(&alias) {
                safe_aliases.insert(alias, param);
            }
        }
        if safe_aliases.is_empty() {
            continue;
        }
        for stmt in &mut function.body {
            if !stmt.text.contains(".arg_") {
                continue;
            }
            if let EmittedStmtKind::Assign { lhs, rhs } = &stmt.kind
                && safe_aliases.get(lhs).is_some_and(|param| param == rhs)
            {
                stmt.clear();
                continue;
            }
            let rewritten = rewrite_known_aliases(&stmt.text, &safe_aliases);
            if rewritten != stmt.text {
                stmt.replace_text(rewritten);
            }
        }
    }
}

pub(crate) fn rewrite_index_only_mutated_param_shadow_aliases_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    program.into_lines()
}

pub(crate) fn apply_rewrite_index_only_mutated_param_shadow_aliases_ir(
    program: &mut EmittedProgram,
) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((_, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let param_set: FxHashSet<String> = params.into_iter().collect();
        let prologue_defs = collect_prologue_arg_alias_defs_ir(&function.body);
        let (assigned, _stored_bases, _mentioned) =
            collect_post_prologue_assignment_facts_ir(&function.body);
        let mut safe_aliases = FxHashMap::default();
        for (_idx, alias, target) in &prologue_defs {
            if !param_set.contains(target) {
                continue;
            }
            if assigned.contains(alias) || assigned.contains(target) {
                continue;
            }
            safe_aliases.insert(alias.clone(), target.clone());
        }
        if safe_aliases.is_empty() {
            continue;
        }
        for stmt in &mut function.body {
            if !stmt.text.contains(".arg_") {
                continue;
            }
            if matches!(&stmt.kind, EmittedStmtKind::Assign { lhs, .. } if safe_aliases.contains_key(lhs))
            {
                stmt.clear();
                continue;
            }
            let rewritten = rewrite_known_aliases(&stmt.text, &safe_aliases);
            if rewritten != stmt.text {
                stmt.replace_text(rewritten);
            }
        }
    }
}
