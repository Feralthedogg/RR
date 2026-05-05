use super::*;
pub(crate) fn strip_unreachable_sym_helpers(output: &mut String) {
    let lines: Vec<String> = output.lines().map(str::to_string).collect();
    if lines.is_empty() {
        return;
    }

    let funcs = local_function_spans(&lines);
    let sym_funcs: FxHashMap<String, LocalFunctionSpan> = funcs
        .iter()
        .filter(|func| func.name.starts_with("Sym_"))
        .map(|func| (func.name.clone(), func.clone()))
        .collect();
    if sym_funcs.len() <= 1 {
        return;
    }

    let mut in_function = vec![false; lines.len()];
    for func in &funcs {
        for idx in func.start..=func.end {
            if idx < in_function.len() {
                in_function[idx] = true;
            }
        }
    }

    let sym_top_is_empty_entrypoint = |func: &LocalFunctionSpan| {
        let mut saw_return_null = false;
        for line in lines.iter().take(func.end + 1).skip(func.start + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            if trimmed == "return(NULL)" {
                saw_return_null = true;
                continue;
            }
            if !unquoted_sym_refs_local(trimmed).is_empty() {
                return false;
            }
            return false;
        }
        saw_return_null
    };

    let mut roots = FxHashSet::default();
    if sym_funcs.contains_key("Sym_top_0") {
        roots.insert("Sym_top_0".to_string());
    }
    for (idx, line) in lines.iter().enumerate() {
        if in_function[idx] {
            continue;
        }
        for name in unquoted_sym_refs_local(line) {
            if sym_funcs.contains_key(&name) {
                roots.insert(name);
            }
        }
    }
    if roots.is_empty() {
        return;
    }
    if roots.len() == 1
        && roots.contains("Sym_top_0")
        && sym_funcs
            .get("Sym_top_0")
            .is_some_and(sym_top_is_empty_entrypoint)
    {
        return;
    }

    let mut reachable = roots.clone();
    let mut work: Vec<String> = roots.into_iter().collect();
    while let Some(name) = work.pop() {
        let Some(func) = sym_funcs.get(&name) else {
            continue;
        };
        for line in lines.iter().take(func.end + 1).skip(func.start + 1) {
            for callee in unquoted_sym_refs_local(line) {
                if sym_funcs.contains_key(&callee) && reachable.insert(callee.clone()) {
                    work.push(callee);
                }
            }
        }
    }

    let mut kept = Vec::with_capacity(lines.len());
    let mut idx = 0usize;
    while idx < lines.len() {
        if let Some(func) = funcs.iter().find(|func| func.start == idx) {
            if !func.name.starts_with("Sym_") || reachable.contains(&func.name) {
                kept.extend(lines.iter().take(func.end + 1).skip(func.start).cloned());
            }
            idx = func.end + 1;
            continue;
        }
        kept.push(lines[idx].clone());
        idx += 1;
    }

    let mut rewritten = kept.join("\n");
    if output.ends_with('\n') || !rewritten.is_empty() {
        rewritten.push('\n');
    }
    *output = rewritten;
}
