fn has_nested_index_vec_floor_candidates_ir(lines: &[String]) -> bool {
    let Some(re) = nested_index_vec_floor_re() else {
        return false;
    };
    lines.iter().any(|line| re.is_match(line))
}

fn apply_simplify_nested_index_vec_floor_calls_ir(program: &mut EmittedProgram) {
    let Some(re) = nested_index_vec_floor_re() else {
        return;
    };
    for item in &mut program.items {
        match item {
            EmittedItem::Raw(line) => {
                let mut rewritten = line.clone();
                loop {
                    let next = re
                        .replace_all(&rewritten, |caps: &Captures<'_>| {
                            format!(
                                "rr_index_vec_floor({})",
                                caps.name("inner").map(|m| m.as_str()).unwrap_or("")
                            )
                        })
                        .to_string();
                    if next == rewritten {
                        break;
                    }
                    rewritten = next;
                }
                *line = rewritten;
            }
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    let mut rewritten = stmt.text.clone();
                    loop {
                        let next = re
                            .replace_all(&rewritten, |caps: &Captures<'_>| {
                                format!(
                                    "rr_index_vec_floor({})",
                                    caps.name("inner").map(|m| m.as_str()).unwrap_or("")
                                )
                            })
                            .to_string();
                        if next == rewritten {
                            break;
                        }
                        rewritten = next;
                    }
                    if rewritten != stmt.text {
                        stmt.replace_text(rewritten);
                    }
                }
            }
        }
    }
}

pub(in super::super) fn run_post_passthrough_wrapper_cleanup_bundle_ir(
    lines: Vec<String>,
) -> Vec<String> {
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let needs_copy = has_inlined_copy_vec_sequence_candidates_ir(&lines);
    if !needs_floor && !needs_copy {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    if needs_copy {
        apply_collapse_inlined_copy_vec_sequences_ir(&mut program);
    }
    program.into_lines()
}

pub(in super::super) fn rewrite_readonly_param_aliases_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_readonly_param_aliases_ir(&mut program);
    program.into_lines()
}

pub(in super::super) fn run_arg_alias_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
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

pub(in super::super) fn strip_unused_arg_aliases_ir(lines: Vec<String>) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unused_arg_aliases_ir(&mut program);
    program.into_lines()
}

fn apply_strip_unused_arg_aliases_ir(program: &mut EmittedProgram) {
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

fn apply_rewrite_readonly_param_aliases_ir(program: &mut EmittedProgram) {
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

pub(in super::super) fn rewrite_remaining_readonly_param_shadow_uses_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_remaining_readonly_param_shadow_uses_ir(&mut program);
    program.into_lines()
}

fn apply_rewrite_remaining_readonly_param_shadow_uses_ir(program: &mut EmittedProgram) {
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

pub(in super::super) fn rewrite_index_only_mutated_param_shadow_aliases_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_arg_alias_cleanup_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_index_only_mutated_param_shadow_aliases_ir(&mut program);
    program.into_lines()
}

fn apply_rewrite_index_only_mutated_param_shadow_aliases_ir(program: &mut EmittedProgram) {
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

fn rewrite_trimmed_helper_calls_in_text(
    text: &str,
    trims: &FxHashMap<String, HelperTrimIr>,
) -> String {
    let mut rewritten = text.to_string();
    loop {
        let mut changed = false;
        let mut next = String::with_capacity(rewritten.len());
        let mut idx = 0usize;
        while idx < rewritten.len() {
            let slice = &rewritten[idx..];
            let Some(caps) = ident_re().and_then(|re| re.captures(slice)) else {
                next.push_str(slice);
                break;
            };
            let Some(mat) = caps.get(0) else {
                next.push_str(slice);
                break;
            };
            let ident_start = idx + mat.start();
            let ident_end = idx + mat.end();
            next.push_str(&rewritten[idx..ident_start]);
            let ident = mat.as_str();
            let Some(trim) = trims.get(ident) else {
                next.push_str(ident);
                idx = ident_end;
                continue;
            };
            if !rewritten[ident_end..].starts_with('(') {
                next.push_str(ident);
                idx = ident_end;
                continue;
            }
            let mut depth = 0i32;
            let mut end = None;
            for (off, ch) in rewritten[ident_end..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(ident_end + off);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(call_end) = end else {
                next.push_str(ident);
                idx = ident_end;
                continue;
            };
            let args_inner = &rewritten[ident_end + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                next.push_str(&rewritten[ident_start..=call_end]);
                idx = call_end + 1;
                continue;
            };
            if args.len() != trim.original_len {
                next.push_str(&rewritten[ident_start..=call_end]);
                idx = call_end + 1;
                continue;
            }
            next.push_str(ident);
            next.push('(');
            next.push_str(
                &trim
                    .kept_indices
                    .iter()
                    .map(|idx| args[*idx].trim())
                    .collect::<Vec<_>>()
                    .join(", "),
            );
            next.push(')');
            idx = call_end + 1;
            changed = true;
        }
        if !changed || next == rewritten {
            break rewritten;
        }
        rewritten = next;
    }
}

fn has_unused_helper_param_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let Some(fn_name) = func.name.as_deref() else {
                return false;
            };
            if !fn_name.starts_with("Sym_") || func.params.is_empty() {
                return false;
            }
            if func.params.iter().any(|param| param.contains('=')) {
                return false;
            }
            let mut used_params = FxHashSet::default();
            for line in &lines[func.body_start..=func.end] {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    continue;
                }
                for ident in expr_idents(trimmed) {
                    used_params.insert(ident);
                }
            }
            func.params.iter().any(|param| !used_params.contains(param))
        })
}

fn apply_strip_unused_helper_params_ir(program: &mut EmittedProgram) {
    let mut trims = FxHashMap::<String, HelperTrimIr>::default();

    for (item_idx, item) in program.items.iter().enumerate() {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        if !fn_name.starts_with("Sym_")
            || params.is_empty()
            || params.iter().any(|param| param.contains('='))
        {
            continue;
        }

        let escaped = program
            .items
            .iter()
            .enumerate()
            .filter(|(other_idx, _)| *other_idx != item_idx)
            .any(|(_, other_item)| match other_item {
                EmittedItem::Raw(line) => {
                    let trimmed = line.trim();
                    crate::compiler::pipeline::line_contains_symbol(trimmed, &fn_name)
                        && !trimmed.contains(&format!("{fn_name}("))
                        && !trimmed.contains(&format!("{fn_name} <- function("))
                }
                EmittedItem::Function(other_function) => other_function.body.iter().any(|stmt| {
                    let trimmed = stmt.text.trim();
                    crate::compiler::pipeline::line_contains_symbol(trimmed, &fn_name)
                        && !trimmed.contains(&format!("{fn_name}("))
                        && !trimmed.contains(&format!("{fn_name} <- function("))
                }),
            });
        if escaped {
            continue;
        }

        let mut used_params = FxHashSet::default();
        for stmt in &function.body {
            let trimmed = stmt.text.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            for ident in expr_idents(trimmed) {
                used_params.insert(ident);
            }
        }
        let kept_indices: Vec<usize> = params
            .iter()
            .enumerate()
            .filter_map(|(idx, param)| used_params.contains(param).then_some(idx))
            .collect();
        if kept_indices.len() < params.len() {
            trims.insert(
                fn_name,
                HelperTrimIr {
                    original_len: params.len(),
                    kept_indices: kept_indices.clone(),
                    kept_params: kept_indices
                        .iter()
                        .map(|idx| params[*idx].clone())
                        .collect(),
                },
            );
        }
    }

    if trims.is_empty() {
        return;
    }

    for item in &mut program.items {
        match item {
            EmittedItem::Function(function) => {
                if let Some((fn_name, _)) = parse_function_header_ir(&function.header)
                    && let Some(trim) = trims.get(&fn_name)
                {
                    function.header =
                        format!("{} <- function({})", fn_name, trim.kept_params.join(", "));
                }
                for stmt in &mut function.body {
                    let rewritten = rewrite_trimmed_helper_calls_in_text(&stmt.text, &trims);
                    if rewritten != stmt.text {
                        stmt.replace_text(rewritten);
                    }
                }
            }
            EmittedItem::Raw(line) => {
                let rewritten = rewrite_trimmed_helper_calls_in_text(line, &trims);
                if rewritten != *line {
                    *line = rewritten;
                }
            }
        }
    }
}

fn helper_ident_is_start_ir(expr: &str, idx: usize) -> bool {
    let rest = &expr[idx..];
    let mut chars = rest.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first.is_ascii_alphabetic() || first == '_' {
        return true;
    }
    first == '.'
        && chars
            .next()
            .is_some_and(|next| next.is_ascii_alphabetic() || next == '_')
}

fn helper_ident_end_ir(expr: &str, start: usize) -> usize {
    let mut end = start;
    for (off, ch) in expr[start..].char_indices() {
        if !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.')) {
            break;
        }
        end = start + off + ch.len_utf8();
    }
    end
}

fn helper_ident_is_named_label_ir(expr: &str, end: usize) -> bool {
    let rest = &expr[end..];
    for (off, ch) in rest.char_indices() {
        if ch.is_ascii_whitespace() {
            continue;
        }
        if ch != '=' {
            return false;
        }
        let tail = &rest[off + ch.len_utf8()..];
        let next_non_ws = tail.chars().find(|ch| !ch.is_ascii_whitespace());
        return next_non_ws != Some('=');
    }
    false
}

fn substitute_helper_expr_ir(expr: &str, bindings: &FxHashMap<String, String>) -> String {
    let mut out = String::with_capacity(expr.len());
    let bytes = expr.as_bytes();
    let mut idx = 0usize;
    let mut in_single = false;
    let mut in_double = false;

    while idx < bytes.len() {
        match bytes[idx] {
            b'\'' if !in_double => {
                in_single = !in_single;
                out.push('\'');
                idx += 1;
                continue;
            }
            b'"' if !in_single => {
                in_double = !in_double;
                out.push('"');
                idx += 1;
                continue;
            }
            _ => {}
        }

        if !in_single && !in_double && helper_ident_is_start_ir(expr, idx) {
            let end = helper_ident_end_ir(expr, idx);
            let ident = &expr[idx..end];
            if !helper_ident_is_named_label_ir(expr, end)
                && let Some(replacement) = bindings.get(ident)
            {
                out.push_str(replacement);
            } else {
                out.push_str(ident);
            }
            idx = end;
            continue;
        }

        out.push(bytes[idx] as char);
        idx += 1;
    }

    out
}

fn collect_simple_expr_helpers_ir(
    lines: &[String],
    _pure_user_calls: &FxHashSet<String>,
) -> FxHashMap<String, SimpleExprHelperIr> {
    let mut out = FxHashMap::default();
    for func in build_function_text_index(lines, parse_function_header_ir) {
        let Some(fn_name) = func.name.as_ref() else {
            continue;
        };
        let params = &func.params;
        let Some(return_idx) = func.return_idx else {
            continue;
        };
        let return_line = lines[return_idx].trim();
        let Some(return_expr) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let mut bindings: FxHashMap<String, String> = FxHashMap::default();
        let mut locals: FxHashSet<String> = FxHashSet::default();
        let mut simple = true;
        for line in lines.iter().take(return_idx).skip(func.body_start) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                simple = false;
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                simple = false;
                break;
            }
            let expanded = substitute_helper_expr_ir(rhs, &bindings);
            bindings.insert(lhs.to_string(), expanded);
            locals.insert(lhs.to_string());
        }
        if !simple {
            continue;
        }

        let expanded_return = substitute_helper_expr_ir(return_expr, &bindings);
        if expanded_return.contains(&format!("{fn_name}(")) {
            continue;
        }
        if expr_idents(&expanded_return)
            .iter()
            .any(|ident| locals.contains(ident) && !params.iter().any(|param| param == ident))
        {
            continue;
        }
        out.insert(
            fn_name.clone(),
            SimpleExprHelperIr {
                params: params.clone(),
                expr: expanded_return,
            },
        );
    }
    out
}

fn collect_simple_expr_helpers_from_program_ir(
    program: &EmittedProgram,
    _pure_user_calls: &FxHashSet<String>,
) -> FxHashMap<String, SimpleExprHelperIr> {
    let mut out = FxHashMap::default();
    for item in &program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        let Some(return_idx) = function
            .body
            .iter()
            .rposition(|stmt| matches!(stmt.kind, EmittedStmtKind::Return))
        else {
            continue;
        };
        let return_line = function.body[return_idx].text.trim();
        let Some(return_expr) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            continue;
        };

        let mut bindings: FxHashMap<String, String> = FxHashMap::default();
        let mut locals: FxHashSet<String> = FxHashSet::default();
        let mut simple = true;
        for stmt in function.body.iter().take(return_idx) {
            let trimmed = stmt.text.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                simple = false;
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                simple = false;
                break;
            }
            let expanded = substitute_helper_expr_ir(rhs, &bindings);
            bindings.insert(lhs.to_string(), expanded);
            locals.insert(lhs.to_string());
        }
        if !simple {
            continue;
        }

        let expanded_return = substitute_helper_expr_ir(return_expr, &bindings);
        if expanded_return.contains(&format!("{fn_name}(")) {
            continue;
        }
        if expr_idents(&expanded_return)
            .iter()
            .any(|ident| locals.contains(ident) && !params.iter().any(|param| param == ident))
        {
            continue;
        }
        out.insert(
            fn_name,
            SimpleExprHelperIr {
                params,
                expr: expanded_return,
            },
        );
    }
    out
}

fn collect_metric_helpers_ir(lines: &[String]) -> FxHashMap<String, MetricHelperIr> {
    let mut out = FxHashMap::default();
    for func in build_function_text_index(lines, parse_function_header_ir) {
        let Some(fn_name) = func.name.as_ref() else {
            continue;
        };
        let params = &func.params;
        if params.len() != 2 {
            continue;
        }
        let body_lines: Vec<String> = lines
            .iter()
            .take(func.end)
            .skip(func.body_start)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "{" && s != "}")
            .collect();
        if body_lines.len() < 3 || body_lines.len() > 5 {
            continue;
        }
        let name_param = params[0].clone();
        let value_param = params[1].clone();
        let Some(return_line) = body_lines.last() else {
            continue;
        };
        if return_line != &format!("return({value_param})") {
            continue;
        }
        let print_name_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({name_param})"));
        let print_value_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({value_param})"));
        let (Some(print_name_idx), Some(print_value_idx)) = (print_name_idx, print_value_idx)
        else {
            continue;
        };
        if print_name_idx >= print_value_idx || print_value_idx + 1 != body_lines.len() - 1 {
            continue;
        }
        out.insert(
            fn_name.clone(),
            MetricHelperIr {
                name_param,
                value_param,
                pre_name_lines: body_lines[..print_name_idx].to_vec(),
                pre_value_lines: body_lines[print_name_idx + 1..print_value_idx].to_vec(),
            },
        );
    }
    out
}

fn collect_metric_helpers_from_program_ir(
    program: &EmittedProgram,
) -> FxHashMap<String, MetricHelperIr> {
    let mut out = FxHashMap::default();
    for item in &program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let Some((fn_name, params)) = parse_function_header_ir(&function.header) else {
            continue;
        };
        if params.len() != 2 {
            continue;
        }
        let body_lines: Vec<String> = function
            .body
            .iter()
            .map(|stmt| stmt.text.trim().to_string())
            .filter(|s| !s.is_empty() && s != "{" && s != "}")
            .collect();
        if body_lines.len() < 3 || body_lines.len() > 5 {
            continue;
        }
        let name_param = params[0].clone();
        let value_param = params[1].clone();
        let Some(return_line) = body_lines.last() else {
            continue;
        };
        if return_line != &format!("return({value_param})") {
            continue;
        }
        let print_name_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({name_param})"));
        let print_value_idx = body_lines
            .iter()
            .position(|line| line == &format!("print({value_param})"));
        let (Some(print_name_idx), Some(print_value_idx)) = (print_name_idx, print_value_idx)
        else {
            continue;
        };
        if print_name_idx >= print_value_idx || print_value_idx + 1 != body_lines.len() - 1 {
            continue;
        }
        out.insert(
            fn_name,
            MetricHelperIr {
                name_param,
                value_param,
                pre_name_lines: body_lines[..print_name_idx].to_vec(),
                pre_value_lines: body_lines[print_name_idx + 1..print_value_idx].to_vec(),
            },
        );
    }
    out
}

fn rewrite_simple_expr_helper_calls_in_text_ir(
    text: &str,
    helpers: &FxHashMap<String, SimpleExprHelperIr>,
    allowed_helpers: Option<&FxHashSet<String>>,
) -> String {
    let mut rewritten = text.to_string();
    loop {
        let mut changed = false;
        let mut next = String::with_capacity(rewritten.len());
        let mut idx = 0usize;
        while idx < rewritten.len() {
            let slice = &rewritten[idx..];
            let Some(caps) = ident_re().and_then(|re| re.captures(slice)) else {
                next.push_str(slice);
                break;
            };
            let Some(mat) = caps.get(0) else {
                next.push_str(slice);
                break;
            };
            let ident_start = idx + mat.start();
            let ident_end = idx + mat.end();
            next.push_str(&rewritten[idx..ident_start]);
            let ident = mat.as_str();
            let Some(helper) = helpers.get(ident) else {
                next.push_str(ident);
                idx = ident_end;
                continue;
            };
            if allowed_helpers.is_some_and(|allowed| !allowed.contains(ident)) {
                next.push_str(ident);
                idx = ident_end;
                continue;
            }
            if !rewritten[ident_end..].starts_with('(') {
                next.push_str(ident);
                idx = ident_end;
                continue;
            }
            let mut depth = 0i32;
            let mut end = None;
            for (off, ch) in rewritten[ident_end..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(ident_end + off);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(call_end) = end else {
                next.push_str(ident);
                idx = ident_end;
                continue;
            };
            let args_inner = &rewritten[ident_end + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                next.push_str(&rewritten[ident_start..=call_end]);
                idx = call_end + 1;
                continue;
            };
            if args.len() != helper.params.len() {
                next.push_str(&rewritten[ident_start..=call_end]);
                idx = call_end + 1;
                continue;
            }
            let subst = helper
                .params
                .iter()
                .zip(args.iter())
                .map(|(param, arg)| (param.clone(), arg.trim().to_string()))
                .collect::<FxHashMap<_, _>>();
            let expanded = substitute_helper_expr_ir(&helper.expr, &subst);
            next.push('(');
            next.push_str(&expanded);
            next.push(')');
            idx = call_end + 1;
            changed = true;
        }
        if !changed || next == rewritten {
            break rewritten;
        }
        rewritten = next;
    }
}

pub(in super::super) fn rewrite_simple_expr_helper_calls_ir(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    allowed_helpers: Option<&FxHashSet<String>>,
) -> Vec<String> {
    if !has_simple_expr_helper_candidates_ir(&lines) {
        return lines;
    }
    let helpers = collect_simple_expr_helpers_ir(&lines, pure_user_calls);
    if helpers.is_empty() {
        return lines;
    }
    let helper_names: Vec<&str> = helpers.keys().map(String::as_str).collect();
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_simple_expr_helper_calls_ir(
        &mut program,
        &helpers,
        &helper_names,
        allowed_helpers,
    );
    program.into_lines()
}

fn apply_rewrite_simple_expr_helper_calls_ir(
    program: &mut EmittedProgram,
    helpers: &FxHashMap<String, SimpleExprHelperIr>,
    helper_names: &[&str],
    allowed_helpers: Option<&FxHashSet<String>>,
) {
    for item in &mut program.items {
        match item {
            EmittedItem::Function(function) => {
                for stmt in &mut function.body {
                    if !stmt.text.contains('(')
                        || !stmt.text.contains("Sym_")
                        || !helper_names.iter().any(|name| stmt.text.contains(name))
                    {
                        continue;
                    }
                    let rewritten = rewrite_simple_expr_helper_calls_in_text_ir(
                        &stmt.text,
                        &helpers,
                        allowed_helpers,
                    );
                    if rewritten != stmt.text {
                        stmt.replace_text(rewritten);
                    }
                }
            }
            EmittedItem::Raw(line) => {
                if !line.contains('(')
                    || !line.contains("Sym_")
                    || !helper_names.iter().any(|name| line.contains(name))
                {
                    continue;
                }
                let rewritten =
                    rewrite_simple_expr_helper_calls_in_text_ir(line, &helpers, allowed_helpers);
                if rewritten != *line {
                    *line = rewritten;
                }
            }
        }
    }
}

pub(in super::super) fn rewrite_metric_helper_return_calls_ir(lines: Vec<String>) -> Vec<String> {
    if !has_metric_helper_candidates_ir(&lines) {
        return lines;
    }
    let helpers = collect_metric_helpers_ir(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    let mut temp_counter = 0usize;
    apply_rewrite_metric_helper_return_calls_ir(&mut program, &helpers, &mut temp_counter);
    program.into_lines()
}

pub(in super::super) fn rewrite_metric_helper_statement_calls_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_metric_helper_candidates_ir(&lines) {
        return lines;
    }
    let helpers = collect_metric_helpers_ir(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    let mut temp_counter = 0usize;
    apply_rewrite_metric_helper_statement_calls_ir(&mut program, &helpers, &mut temp_counter);
    program.into_lines()
}

fn apply_rewrite_metric_helper_return_calls_ir(
    program: &mut EmittedProgram,
    helpers: &FxHashMap<String, MetricHelperIr>,
    temp_counter: &mut usize,
) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        for stmt in function.body.drain(..) {
            let trimmed = stmt.text.trim().to_string();
            let Some(inner) = trimmed
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
            else {
                out.push(stmt);
                continue;
            };
            let Some((callee, args_str)) = inner.split_once('(') else {
                out.push(stmt);
                continue;
            };
            let Some(args_inner) = args_str.strip_suffix(')') else {
                out.push(stmt);
                continue;
            };
            let Some(helper) = helpers.get(callee.trim()) else {
                out.push(stmt);
                continue;
            };
            let Some(args) = split_top_level_args(args_inner) else {
                out.push(stmt);
                continue;
            };
            if args.len() != 2 {
                out.push(stmt);
                continue;
            }
            let indent = stmt.indent();
            let metric_name = args[0].trim();
            let metric_value = args[1].trim();
            for pre in &helper.pre_name_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({metric_name})")));
            let temp_name = format!(".__rr_inline_metric_{}", *temp_counter);
            *temp_counter += 1;
            out.push(EmittedStmt::parse(&format!(
                "{indent}{temp_name} <- {metric_value}"
            )));
            for pre in &helper.pre_value_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({temp_name})")));
            out.push(EmittedStmt::parse(&format!("{indent}return({temp_name})")));
        }
        function.body = out;
    }
}

fn apply_rewrite_metric_helper_statement_calls_ir(
    program: &mut EmittedProgram,
    helpers: &FxHashMap<String, MetricHelperIr>,
    temp_counter: &mut usize,
) {
    for item in &mut program.items {
        let EmittedItem::Function(function) = item else {
            continue;
        };
        let mut out = Vec::with_capacity(function.body.len());
        for stmt in function.body.drain(..) {
            let trimmed = stmt.text.trim().to_string();
            let Some((callee, args_str)) = trimmed.split_once('(') else {
                out.push(stmt);
                continue;
            };
            if trimmed.contains("<-") || trimmed.starts_with("return(") {
                out.push(stmt);
                continue;
            }
            let Some(args_inner) = args_str.strip_suffix(')') else {
                out.push(stmt);
                continue;
            };
            let Some(helper) = helpers.get(callee.trim()) else {
                out.push(stmt);
                continue;
            };
            let Some(args) = split_top_level_args(args_inner) else {
                out.push(stmt);
                continue;
            };
            if args.len() != 2 {
                out.push(stmt);
                continue;
            }
            let indent = stmt.indent();
            let metric_name = args[0].trim();
            let metric_value = args[1].trim();
            for pre in &helper.pre_name_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({metric_name})")));
            let temp_name = format!(".__rr_inline_metric_{}", *temp_counter);
            *temp_counter += 1;
            out.push(EmittedStmt::parse(&format!(
                "{indent}{temp_name} <- {metric_value}"
            )));
            for pre in &helper.pre_value_lines {
                out.push(EmittedStmt::parse(&format!("{indent}{pre}")));
            }
            out.push(EmittedStmt::parse(&format!("{indent}print({temp_name})")));
        }
        function.body = out;
    }
}

#[derive(Default)]
pub(in super::super) struct SecondaryMetricBundleProfile {
    pub(in super::super) post_wrapper_elapsed_ns: u128,
    pub(in super::super) metric_elapsed_ns: u128,
}

pub(in super::super) fn run_post_passthrough_metric_bundle_ir(
    lines: Vec<String>,
) -> (Vec<String>, SecondaryMetricBundleProfile) {
    let needs_arg_return_wrapper = has_arg_return_wrapper_candidates_ir(&lines);
    let needs_passthrough_return_wrapper = has_passthrough_return_wrapper_candidates_ir(&lines);
    let needs_dot_product_wrapper = has_trivial_dot_product_wrapper_candidates_ir(&lines);
    let needs_wrapper =
        needs_arg_return_wrapper || needs_passthrough_return_wrapper || needs_dot_product_wrapper;
    let maybe_passthrough_helpers = has_passthrough_helper_candidates_ir(&lines);
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let maybe_metric_helpers = has_metric_helper_candidates_ir(&lines);
    let helpers = if maybe_metric_helpers {
        collect_metric_helpers_ir(&lines)
    } else {
        FxHashMap::default()
    };
    let needs_metric = !helpers.is_empty();
    if !needs_wrapper && !maybe_passthrough_helpers && !needs_floor && !needs_metric {
        return (lines, SecondaryMetricBundleProfile::default());
    }

    let mut profile = SecondaryMetricBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);
    let started = std::time::Instant::now();
    if needs_arg_return_wrapper {
        apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    }
    if needs_passthrough_return_wrapper {
        apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    }
    if needs_dot_product_wrapper {
        apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    }
    if maybe_passthrough_helpers {
        let passthrough = collect_passthrough_helpers_from_program_ir(&program);
        if !passthrough.is_empty() {
            apply_rewrite_passthrough_helper_calls_ir(&mut program, &passthrough);
        }
    }
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    profile.post_wrapper_elapsed_ns = started.elapsed().as_nanos();
    if needs_metric {
        let started = std::time::Instant::now();
        let mut stmt_temp_counter = 0usize;
        apply_rewrite_metric_helper_statement_calls_ir(
            &mut program,
            &helpers,
            &mut stmt_temp_counter,
        );
        let mut return_temp_counter = 0usize;
        apply_rewrite_metric_helper_return_calls_ir(
            &mut program,
            &helpers,
            &mut return_temp_counter,
        );
        profile.metric_elapsed_ns = started.elapsed().as_nanos();
    }
    (program.into_lines(), profile)
}

pub(in super::super) fn run_passthrough_secondary_bundle_ir(
    lines: Vec<String>,
) -> (Vec<String>, SecondaryMetricBundleProfile) {
    let needs_arg_return_wrapper = has_arg_return_wrapper_candidates_ir(&lines);
    let needs_passthrough_return_wrapper = has_passthrough_return_wrapper_candidates_ir(&lines);
    let needs_dot_product_wrapper = has_trivial_dot_product_wrapper_candidates_ir(&lines);
    let needs_wrapper =
        needs_arg_return_wrapper || needs_passthrough_return_wrapper || needs_dot_product_wrapper;
    let needs_passthrough_helpers = has_passthrough_helper_candidates_ir(&lines);
    let needs_floor = has_nested_index_vec_floor_candidates_ir(&lines);
    let needs_copy = has_inlined_copy_vec_sequence_candidates_ir(&lines);
    let maybe_metric_helpers = has_metric_helper_candidates_ir(&lines);
    let helpers = if maybe_metric_helpers {
        collect_metric_helpers_ir(&lines)
    } else {
        FxHashMap::default()
    };
    let needs_metric = !helpers.is_empty();
    if !needs_wrapper && !needs_passthrough_helpers && !needs_floor && !needs_copy && !needs_metric
    {
        return (lines, SecondaryMetricBundleProfile::default());
    }

    let mut profile = SecondaryMetricBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);
    let started = std::time::Instant::now();
    if needs_arg_return_wrapper {
        apply_strip_arg_aliases_in_trivial_return_wrappers_ir(&mut program);
    }
    if needs_passthrough_return_wrapper {
        apply_collapse_trivial_passthrough_return_wrappers_ir(&mut program);
    }
    if needs_dot_product_wrapper {
        apply_collapse_trivial_dot_product_wrappers_ir(&mut program);
    }
    if needs_passthrough_helpers {
        let passthrough = collect_passthrough_helpers_from_program_ir(&program);
        if !passthrough.is_empty() {
            apply_rewrite_passthrough_helper_calls_ir(&mut program, &passthrough);
        }
    }
    if needs_floor {
        apply_simplify_nested_index_vec_floor_calls_ir(&mut program);
    }
    if needs_copy {
        apply_collapse_inlined_copy_vec_sequences_ir(&mut program);
    }
    profile.post_wrapper_elapsed_ns = started.elapsed().as_nanos();
    if needs_metric {
        let started = std::time::Instant::now();
        let mut stmt_temp_counter = 0usize;
        apply_rewrite_metric_helper_statement_calls_ir(
            &mut program,
            &helpers,
            &mut stmt_temp_counter,
        );
        let mut return_temp_counter = 0usize;
        apply_rewrite_metric_helper_return_calls_ir(
            &mut program,
            &helpers,
            &mut return_temp_counter,
        );
        profile.metric_elapsed_ns = started.elapsed().as_nanos();
    }
    (program.into_lines(), profile)
}

pub(in super::super) fn strip_unused_helper_params_ir(lines: Vec<String>) -> Vec<String> {
    if !has_unused_helper_param_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unused_helper_params_ir(&mut program);
    program.into_lines()
}

pub(in super::super) fn collapse_trivial_scalar_clamp_wrappers_ir(
    lines: Vec<String>,
) -> Vec<String> {
    if !has_trivial_scalar_clamp_wrapper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_trivial_scalar_clamp_wrappers_ir(&mut program);
    program.into_lines()
}
