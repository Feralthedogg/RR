fn build_block_end_map_ir(lines: &[String]) -> Vec<Option<usize>> {
    let mut out = vec![None; lines.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let (opens, closes) = count_unquoted_braces(line.trim());
        for _ in 0..closes {
            let Some(open_idx) = stack.pop() else {
                break;
            };
            out[open_idx] = Some(idx);
        }
        for _ in 0..opens {
            stack.push(idx);
        }
    }
    out
}

fn has_terminal_repeat_next_candidates_ir(lines: &[String]) -> bool {
    let block_end_map = build_block_end_map_ir(lines);
    for (idx, line) in lines.iter().enumerate() {
        if line.trim() != "repeat {" {
            continue;
        }
        let Some(end_idx) = block_end_map.get(idx).and_then(|entry| *entry) else {
            continue;
        };
        let prev_non_empty = ((idx + 1)..end_idx).rev().find_map(|line_idx| {
            let text = lines[line_idx].trim();
            (!text.is_empty()).then_some(text)
        });
        if prev_non_empty == Some("next") {
            return true;
        }
    }
    false
}

fn has_identical_if_else_tail_assign_candidates_ir(lines: &[String]) -> bool {
    let block_end_map = build_block_end_map_ir(lines);
    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            continue;
        }
        let Some(end_idx) = block_end_map.get(idx).and_then(|entry| *entry) else {
            continue;
        };
        let Some(else_idx) =
            ((idx + 1)..end_idx).find(|line_idx| lines[*line_idx].trim() == "} else {")
        else {
            continue;
        };
        let then_tail = ((idx + 1)..else_idx).rev().find_map(|line_idx| {
            let text = lines[line_idx].trim();
            (!text.is_empty()).then_some(text)
        });
        let else_tail = ((else_idx + 1)..end_idx).rev().find_map(|line_idx| {
            let text = lines[line_idx].trim();
            (!text.is_empty()).then_some(text)
        });
        let (Some(then_tail), Some(else_tail)) = (then_tail, else_tail) else {
            continue;
        };
        if then_tail == else_tail && assign_re().and_then(|re| re.captures(then_tail)).is_some() {
            return true;
        }
    }
    false
}

fn has_tail_assign_slice_return_candidates_ir(lines: &[String]) -> bool {
    build_function_text_index(lines, parse_function_header_ir)
        .into_iter()
        .any(|func| {
            let Some(return_idx) = func.return_idx else {
                return false;
            };
            let ret_var = lines[return_idx]
                .trim()
                .strip_prefix("return(")
                .and_then(|s| s.strip_suffix(')'))
                .map(str::trim);
            let Some(ret_var) = ret_var else {
                return false;
            };
            ((func.body_start)..return_idx)
                .rev()
                .find_map(|line_idx| {
                    let text = lines[line_idx].trim();
                    if text.is_empty() || text == "{" || text == "}" {
                        return None;
                    }
                    Some(text)
                })
                .is_some_and(|prev| {
                    assign_re()
                        .and_then(|re| re.captures(prev))
                        .is_some_and(|caps| {
                            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                            lhs == ret_var && rhs.contains("rr_assign_slice(")
                        })
                })
        })
}

fn has_unreachable_sym_helper_candidates_ir(lines: &[String]) -> bool {
    let functions = build_function_text_index(lines, parse_function_header_ir);
    let sym_funcs: FxHashMap<String, IndexedFunction> = functions
        .iter()
        .filter_map(|func| {
            let name = func.name.clone()?;
            name.starts_with("Sym_").then_some((name, func.clone()))
        })
        .collect();
    if sym_funcs.len() <= 1 {
        return false;
    }

    let mut in_function = vec![false; lines.len()];
    for func in &functions {
        for idx in func.start..=func.end {
            if idx < in_function.len() {
                in_function[idx] = true;
            }
        }
    }

    let mut roots = FxHashSet::default();
    if sym_funcs.contains_key("Sym_top_0") {
        roots.insert("Sym_top_0".to_string());
    }
    for (idx, line) in lines.iter().enumerate() {
        if in_function[idx] {
            continue;
        }
        for name in unquoted_sym_refs(line) {
            if sym_funcs.contains_key(&name) {
                roots.insert(name);
            }
        }
    }
    if roots.is_empty() {
        return false;
    }

    let sym_top_is_empty_entrypoint = |func: &IndexedFunction| {
        let Some(return_idx) = func.return_idx else {
            return false;
        };
        let mut saw_return_null = false;
        for line in &lines[func.body_start..=return_idx] {
            let trimmed = line.trim();
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
    if roots.len() == 1
        && roots.contains("Sym_top_0")
        && sym_funcs
            .get("Sym_top_0")
            .is_some_and(sym_top_is_empty_entrypoint)
    {
        return false;
    }

    let mut reachable = roots.clone();
    let mut work: Vec<String> = roots.into_iter().collect();
    while let Some(name) = work.pop() {
        let Some(func) = sym_funcs.get(&name) else {
            continue;
        };
        for line in &lines[func.body_start..=func.end] {
            for callee in unquoted_sym_refs(line) {
                if sym_funcs.contains_key(&callee) && reachable.insert(callee.clone()) {
                    work.push(callee);
                }
            }
        }
    }

    reachable.len() < sym_funcs.len()
}
