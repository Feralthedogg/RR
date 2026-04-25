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

pub(in super::super) fn strip_unused_helper_params_ir(lines: Vec<String>) -> Vec<String> {
    if !has_unused_helper_param_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unused_helper_params_ir(&mut program);
    program.into_lines()
}
