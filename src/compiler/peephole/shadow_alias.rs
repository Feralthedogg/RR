use super::*;

pub(in super::super) fn rewrite_readonly_param_aliases(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut fn_start = 0usize;
    while fn_start < out.len() {
        while fn_start < out.len() && !out[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= out.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&out, fn_start) else {
            break;
        };
        let body_start = fn_start + 1;
        let fn_text = out[fn_start..=fn_end].join("\n");
        let mutated_arg_aliases = collect_mutated_arg_aliases(&fn_text);
        let mut aliases = FxHashMap::default();
        for line in out.iter().take(fn_end).skip(body_start) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs.starts_with(".arg_")
                && !mutated_arg_aliases.contains(lhs)
                && plain_ident_re().is_some_and(|re| re.is_match(rhs))
            {
                aliases.insert(lhs.to_string(), rhs.to_string());
                continue;
            }
            break;
        }
        if aliases.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }

        let mut assigned_idents = FxHashSet::default();
        let mut stored_bases = FxHashSet::default();
        let mut alias_defs = FxHashSet::default();
        for (alias, param) in &aliases {
            alias_defs.insert((alias.clone(), param.clone()));
        }
        for line in out.iter().take(fn_end).skip(body_start) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" {
                continue;
            }
            if let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) {
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                if !alias_defs.contains(&(lhs.to_string(), rhs.to_string())) {
                    assigned_idents.insert(lhs.to_string());
                }
            }
            if let Some(caps) = indexed_store_base_re().and_then(|re| re.captures(trimmed)) {
                let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                stored_bases.insert(base.to_string());
            }
        }

        let mut safe_aliases = FxHashMap::default();
        for (alias, param) in &aliases {
            if assigned_idents.contains(alias)
                || assigned_idents.contains(param)
                || stored_bases.contains(alias)
                || stored_bases.contains(param)
            {
                continue;
            }
            safe_aliases.insert(alias.clone(), param.clone());
        }

        if safe_aliases.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }

        for line in out.iter_mut().take(fn_end).skip(body_start) {
            if line.trim_start().starts_with(".arg_")
                && let Some(caps) = assign_re().and_then(|re| re.captures(line.trim()))
                && safe_aliases
                    .contains_key(caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim())
            {
                line.clear();
                continue;
            }
            *line = rewrite_known_aliases(line, &safe_aliases);
        }

        fn_start = fn_end + 1;
    }
    out
}

pub(in super::super) fn rewrite_remaining_readonly_param_shadow_uses(
    lines: Vec<String>,
) -> Vec<String> {
    let mut out = lines;
    let mut fn_start = 0usize;
    while fn_start < out.len() {
        while fn_start < out.len() && !out[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= out.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&out, fn_start) else {
            break;
        };
        let header = out[fn_start].trim();
        let Some(args_inner) = header
            .split_once("function(")
            .and_then(|(_, rest)| rest.strip_suffix(") "))
            .or_else(|| {
                header
                    .split_once("function(")
                    .and_then(|(_, rest)| rest.strip_suffix(')'))
            })
        else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some(params) = split_top_level_args(args_inner) else {
            fn_start = fn_end + 1;
            continue;
        };
        let fn_text = out[fn_start..=fn_end].join("\n");
        let mutated_arg_aliases = collect_mutated_arg_aliases(&fn_text);

        let mut safe_aliases = FxHashMap::default();
        for param in params {
            let param = param.trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(param)) {
                continue;
            }
            let alias = format!(".arg_{param}");
            if mutated_arg_aliases.contains(&alias) {
                continue;
            }
            if out
                .iter()
                .take(fn_end)
                .skip(fn_start + 1)
                .any(|line| line.contains(&alias))
            {
                safe_aliases.insert(alias, param.to_string());
            }
        }
        if safe_aliases.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }

        for line in out.iter_mut().take(fn_end).skip(fn_start + 1) {
            let trimmed = line.trim();
            if let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) {
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                if safe_aliases.get(lhs).is_some_and(|param| param == rhs) {
                    line.clear();
                    continue;
                }
            }
            *line = rewrite_known_aliases(line, &safe_aliases);
        }

        fn_start = fn_end + 1;
    }
    out
}

pub(in super::super) fn rewrite_index_only_mutated_param_shadow_aliases(
    lines: Vec<String>,
) -> Vec<String> {
    let mut out = lines;
    let mut fn_start = 0usize;
    while fn_start < out.len() {
        while fn_start < out.len() && !out[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= out.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&out, fn_start) else {
            break;
        };
        let Some((_, params)) = parse_function_header(&out[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let param_set: FxHashSet<String> = params.into_iter().collect();
        let body_start = fn_start + 1;
        let mut candidates = FxHashMap::<String, String>::default();
        for line in out.iter().take(fn_end).skip(body_start) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs.starts_with(".arg_")
                && plain_ident_re().is_some_and(|re| re.is_match(rhs))
                && param_set.contains(rhs)
            {
                candidates.insert(lhs.to_string(), rhs.to_string());
                continue;
            }
            break;
        }
        if candidates.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }

        let mut safe_aliases = FxHashMap::default();
        'candidate: for (alias, param) in &candidates {
            for line in out.iter().take(fn_end).skip(body_start) {
                let trimmed = line.trim();
                if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                    continue;
                }
                if let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) {
                    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                    if lhs == alias && rhs == param {
                        continue;
                    }
                    if lhs == alias || lhs == param {
                        continue 'candidate;
                    }
                }
            }
            safe_aliases.insert(alias.clone(), param.clone());
        }

        if safe_aliases.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }

        for line in out.iter_mut().take(fn_end).skip(body_start) {
            if line.trim_start().starts_with(".arg_")
                && let Some(caps) = assign_re().and_then(|re| re.captures(line.trim()))
                && safe_aliases
                    .contains_key(caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim())
            {
                line.clear();
                continue;
            }
            *line = rewrite_known_aliases(line, &safe_aliases);
        }

        fn_start = fn_end + 1;
    }
    out
}
