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

fn helper_arg_is_trivial_to_duplicate_ir(arg: &str) -> bool {
    let trimmed = arg.trim();
    plain_ident_re().is_some_and(|re| re.is_match(trimmed))
        || scalar_lit_re().is_some_and(|re| re.is_match(trimmed))
        || ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
}

fn helper_param_use_counts_ir(
    expr: &str,
    params: &[String],
) -> FxHashMap<String, usize> {
    let params = params.iter().map(String::as_str).collect::<FxHashSet<_>>();
    let mut counts = FxHashMap::default();
    for ident in expr_idents(expr) {
        if params.contains(ident.as_str()) {
            *counts.entry(ident).or_insert(0) += 1;
        }
    }
    counts
}

fn helper_arg_is_bloat_sensitive_ir(arg: &str) -> bool {
    let trimmed = arg.trim();
    !helper_arg_is_trivial_to_duplicate_ir(trimmed)
        && (trimmed.len() > 48
            || trimmed.contains("Sym_")
            || trimmed.contains("list(")
            || trimmed.contains("[[")
            || trimmed.contains("rr_field_set("))
}

fn helper_expr_is_aggregate_like_ir(expr: &str) -> bool {
    expr.contains("list(")
        || expr.contains("rr_field_set(")
        || expr.contains("rr_named_list(")
        || expr.contains("[[")
}

fn simple_expr_inline_would_bloat_ir(
    helper: &SimpleExprHelperIr,
    args: &[String],
    original_call: &str,
    expanded: &str,
    allowed_helpers: Option<&FxHashSet<String>>,
) -> bool {
    if allowed_helpers.is_some() {
        return false;
    }

    let param_uses = helper_param_use_counts_ir(&helper.expr, &helper.params);
    if helper
        .params
        .iter()
        .zip(args.iter())
        .any(|(param, arg)| {
            param_uses.get(param).copied().unwrap_or(0) > 1
                && helper_arg_is_bloat_sensitive_ir(arg)
        })
    {
        return true;
    }

    let aggregate_heavy = expanded.contains("rr_field_set(")
        || expanded.matches("list(").count() >= 2
        || expanded.matches("[[").count() >= 4;
    aggregate_heavy
        && expanded.len()
            > original_call
                .len()
                .saturating_mul(4)
                .max(160)
}

fn next_simple_expr_inline_temp_ir(counter: &mut usize) -> String {
    let name = format!(".__rr_inline_expr_{}", *counter);
    *counter += 1;
    name
}

fn next_simple_expr_inline_temp_index_ir(body: &[EmittedStmt]) -> usize {
    body.iter()
        .flat_map(|stmt| expr_idents(&stmt.text))
        .filter_map(|ident| {
            ident
                .strip_prefix(".__rr_inline_expr_")
                .and_then(|idx| idx.parse::<usize>().ok())
        })
        .max()
        .map_or(0, |idx| idx + 1)
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
            let original_call = &rewritten[ident_start..=call_end];
            if simple_expr_inline_would_bloat_ir(
                helper,
                &args,
                original_call,
                &expanded,
                allowed_helpers,
            ) {
                next.push_str(original_call);
                idx = call_end + 1;
                continue;
            }
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

fn rewrite_simple_expr_helper_calls_expr_with_let_lift_ir(
    expr: &str,
    helpers: &FxHashMap<String, SimpleExprHelperIr>,
    allowed_helpers: Option<&FxHashSet<String>>,
    temp_counter: &mut usize,
    allow_lift_result: bool,
    indent: &str,
) -> (Vec<String>, String) {
    let mut rewritten = expr.to_string();
    let mut hoisted = Vec::new();
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
            let original_call = &rewritten[ident_start..=call_end];
            let args_inner = &rewritten[ident_end + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                next.push_str(original_call);
                idx = call_end + 1;
                continue;
            };
            if args.len() != helper.params.len() {
                next.push_str(original_call);
                idx = call_end + 1;
                continue;
            }

            let mut processed_args = Vec::with_capacity(args.len());
            for arg in args {
                let (mut arg_hoists, rewritten_arg) =
                    rewrite_simple_expr_helper_calls_expr_with_let_lift_ir(
                        &arg,
                        helpers,
                        allowed_helpers,
                        temp_counter,
                        true,
                        indent,
                    );
                hoisted.append(&mut arg_hoists);
                processed_args.push(rewritten_arg);
            }

            let subst = helper
                .params
                .iter()
                .zip(processed_args.iter())
                .map(|(param, arg)| (param.clone(), arg.trim().to_string()))
                .collect::<FxHashMap<_, _>>();
            let expanded = substitute_helper_expr_ir(&helper.expr, &subst);
            let (mut expanded_hoists, expanded) =
                rewrite_simple_expr_helper_calls_expr_with_let_lift_ir(
                    &expanded,
                    helpers,
                    allowed_helpers,
                    temp_counter,
                    allow_lift_result,
                    indent,
                );
            hoisted.append(&mut expanded_hoists);

            let should_lift = allowed_helpers.is_none()
                && allow_lift_result
                && (helper_expr_is_aggregate_like_ir(&expanded)
                    || simple_expr_inline_would_bloat_ir(
                        helper,
                        &processed_args,
                        original_call,
                        &expanded,
                        allowed_helpers,
                    ));
            if should_lift {
                let temp = next_simple_expr_inline_temp_ir(temp_counter);
                hoisted.push(format!("{indent}{temp} <- {expanded}"));
                next.push_str(&temp);
            } else {
                next.push('(');
                next.push_str(&expanded);
                next.push(')');
            }
            idx = call_end + 1;
            changed = true;
        }
        if !changed || next == rewritten {
            break;
        }
        rewritten = next;
    }
    (hoisted, rewritten)
}

fn rewrite_simple_expr_helper_calls_stmt_with_let_lift_ir(
    stmt: &EmittedStmt,
    helpers: &FxHashMap<String, SimpleExprHelperIr>,
    allowed_helpers: Option<&FxHashSet<String>>,
    temp_counter: &mut usize,
) -> Vec<EmittedStmt> {
    match &stmt.kind {
        EmittedStmtKind::Assign { lhs, rhs } => {
            let indent = stmt.indent();
            let (hoists, rewritten_rhs) = rewrite_simple_expr_helper_calls_expr_with_let_lift_ir(
                rhs,
                helpers,
                allowed_helpers,
                temp_counter,
                false,
                &indent,
            );
            let mut out = hoists
                .into_iter()
                .map(|line| EmittedStmt::parse(&line))
                .collect::<Vec<_>>();
            out.push(EmittedStmt::parse(&format!(
                "{indent}{lhs} <- {rewritten_rhs}"
            )));
            out
        }
        EmittedStmtKind::Return => {
            let trimmed = stmt.text.trim();
            let Some(inner) = trimmed
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
            else {
                return vec![stmt.clone()];
            };
            let indent = stmt.indent();
            let (hoists, rewritten_inner) = rewrite_simple_expr_helper_calls_expr_with_let_lift_ir(
                inner,
                helpers,
                allowed_helpers,
                temp_counter,
                false,
                &indent,
            );
            let mut out = hoists
                .into_iter()
                .map(|line| EmittedStmt::parse(&line))
                .collect::<Vec<_>>();
            out.push(EmittedStmt::parse(&format!(
                "{indent}return({rewritten_inner})"
            )));
            out
        }
        _ => {
            let rewritten = rewrite_simple_expr_helper_calls_in_text_ir(
                &stmt.text,
                helpers,
                allowed_helpers,
            );
            vec![EmittedStmt::parse(&rewritten)]
        }
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
                if allowed_helpers.is_none() {
                    let mut temp_counter =
                        next_simple_expr_inline_temp_index_ir(&function.body);
                    let mut rewritten_body = Vec::with_capacity(function.body.len());
                    for stmt in &function.body {
                        if !stmt.text.contains('(')
                            || !stmt.text.contains("Sym_")
                            || !helper_names.iter().any(|name| stmt.text.contains(name))
                        {
                            rewritten_body.push(stmt.clone());
                            continue;
                        }
                        rewritten_body.extend(rewrite_simple_expr_helper_calls_stmt_with_let_lift_ir(
                            stmt,
                            helpers,
                            allowed_helpers,
                            &mut temp_counter,
                        ));
                    }
                    function.body = rewritten_body;
                    continue;
                }

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
