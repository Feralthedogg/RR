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
