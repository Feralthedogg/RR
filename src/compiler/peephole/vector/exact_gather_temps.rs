use super::*;
pub(crate) fn materialize_o3_exact_gather_index_temps(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut used_names: FxHashSet<String> = lines
        .iter()
        .flat_map(|line| expr_idents(line.trim()))
        .collect();
    let mut index_temp_by_expr = FxHashMap::<String, String>::default();
    let mut gather_temp_by_call = FxHashMap::<String, String>::default();

    for line in lines {
        if line.contains("<- function") {
            index_temp_by_expr.clear();
            gather_temp_by_call.clear();
            out.push(line);
            continue;
        }

        if let Some(base) = super::patterns::indexed_store_base_re()
            .and_then(|re| re.captures(line.trim()))
            .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
        {
            invalidate_o3_exact_gather_temp_maps_for_lhs(
                &base,
                &mut index_temp_by_expr,
                &mut gather_temp_by_call,
            );
        }

        let Some(caps) = assign_re().and_then(|re| re.captures(line.trim_end())) else {
            out.push(line);
            continue;
        };

        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        invalidate_o3_exact_gather_temp_maps_for_lhs(
            lhs,
            &mut index_temp_by_expr,
            &mut gather_temp_by_call,
        );

        if lhs.starts_with(".__rr_cse_") {
            record_existing_o3_exact_gather_temp(
                lhs,
                rhs,
                &mut index_temp_by_expr,
                &mut gather_temp_by_call,
            );
            out.push(line);
            continue;
        }

        let Some((base, index_arg)) = exact_o3_gather_args(rhs) else {
            record_existing_o3_exact_gather_temp(
                lhs,
                rhs,
                &mut index_temp_by_expr,
                &mut gather_temp_by_call,
            );
            out.push(line);
            continue;
        };
        let Some(index_inner) = single_call_inner(&index_arg, "rr_index_vec_floor") else {
            record_existing_o3_exact_gather_temp(
                lhs,
                rhs,
                &mut index_temp_by_expr,
                &mut gather_temp_by_call,
            );
            out.push(line);
            continue;
        };
        if !o3_exact_gather_expr_is_safe(&base) || !o3_exact_gather_expr_is_safe(index_inner) {
            record_existing_o3_exact_gather_temp(
                lhs,
                rhs,
                &mut index_temp_by_expr,
                &mut gather_temp_by_call,
            );
            out.push(line);
            continue;
        }

        let mut prefix_lines = Vec::new();
        let index_source = materialize_o3_index_expr_for_exact_gather(
            index_inner,
            indent,
            &mut used_names,
            &mut index_temp_by_expr,
            &mut gather_temp_by_call,
            &mut prefix_lines,
            0,
        );
        let index_temp = ensure_o3_index_temp_for_expr(
            &index_source,
            indent,
            &mut used_names,
            &mut index_temp_by_expr,
            &mut prefix_lines,
        );
        let rewritten_rhs = format!("rr_gather({}, {index_temp})", base.trim());
        out.extend(prefix_lines);
        out.push(format!("{indent}{lhs} <- {rewritten_rhs}"));
        gather_temp_by_call.insert(rewritten_rhs, lhs.to_string());
    }

    out
}

pub(crate) fn record_existing_o3_exact_gather_temp(
    lhs: &str,
    rhs: &str,
    index_temp_by_expr: &mut FxHashMap<String, String>,
    gather_temp_by_call: &mut FxHashMap<String, String>,
) {
    if single_call_inner(rhs, "rr_index_vec_floor").is_some() {
        index_temp_by_expr.insert(
            rhs_index_inner(rhs).unwrap_or(rhs).to_string(),
            lhs.to_string(),
        );
        return;
    }
    if exact_o3_gather_args(rhs).is_some() {
        gather_temp_by_call.insert(rhs.trim().to_string(), lhs.to_string());
    }
}

pub(crate) fn rhs_index_inner(rhs: &str) -> Option<&str> {
    single_call_inner(rhs.trim(), "rr_index_vec_floor")
}

pub(crate) fn exact_o3_gather_args(rhs: &str) -> Option<(String, String)> {
    let inner = single_call_inner(rhs.trim(), "rr_gather")?;
    let args = split_top_level_args(inner)?;
    if args.len() != 2 {
        return None;
    }
    Some((args[0].trim().to_string(), args[1].trim().to_string()))
}

pub(crate) fn materialize_o3_index_expr_for_exact_gather(
    expr: &str,
    indent: &str,
    used_names: &mut FxHashSet<String>,
    index_temp_by_expr: &mut FxHashMap<String, String>,
    gather_temp_by_call: &mut FxHashMap<String, String>,
    prefix_lines: &mut Vec<String>,
    depth: usize,
) -> String {
    let expr = expr.trim();
    if depth >= 4 {
        return expr.to_string();
    }
    let Some((base, index_arg)) = exact_o3_gather_args(expr) else {
        return expr.to_string();
    };
    if !o3_exact_gather_expr_is_safe(&base) || !o3_exact_gather_expr_is_safe(&index_arg) {
        return expr.to_string();
    }
    let normalized_index = normalize_o3_gather_index_arg_for_exact_temp(
        &index_arg,
        indent,
        used_names,
        index_temp_by_expr,
        gather_temp_by_call,
        prefix_lines,
        depth + 1,
    );
    let call = format!("rr_gather({}, {normalized_index})", base.trim());
    if let Some(existing) = gather_temp_by_call.get(&call) {
        return existing.clone();
    }
    let Some(base_name) = semantic_o3_cse_name_for_rhs(&call) else {
        return expr.to_string();
    };
    let temp = unique_semantic_temp_name(&base_name, used_names);
    prefix_lines.push(format!("{indent}{temp} <- {call}"));
    gather_temp_by_call.insert(call, temp.clone());
    temp
}

pub(crate) fn normalize_o3_gather_index_arg_for_exact_temp(
    index_arg: &str,
    indent: &str,
    used_names: &mut FxHashSet<String>,
    index_temp_by_expr: &mut FxHashMap<String, String>,
    gather_temp_by_call: &mut FxHashMap<String, String>,
    prefix_lines: &mut Vec<String>,
    depth: usize,
) -> String {
    let index_arg = index_arg.trim();
    let Some(inner) = single_call_inner(index_arg, "rr_index_vec_floor") else {
        return index_arg.to_string();
    };
    let index_source = materialize_o3_index_expr_for_exact_gather(
        inner,
        indent,
        used_names,
        index_temp_by_expr,
        gather_temp_by_call,
        prefix_lines,
        depth + 1,
    );
    ensure_o3_index_temp_for_expr(
        &index_source,
        indent,
        used_names,
        index_temp_by_expr,
        prefix_lines,
    )
}

pub(crate) fn ensure_o3_index_temp_for_expr(
    expr: &str,
    indent: &str,
    used_names: &mut FxHashSet<String>,
    index_temp_by_expr: &mut FxHashMap<String, String>,
    prefix_lines: &mut Vec<String>,
) -> String {
    let expr = expr.trim();
    if let Some(existing) = index_temp_by_expr.get(expr) {
        return existing.clone();
    }
    let base_name = format!("idx_{}", semantic_index_suffix(expr));
    let temp = unique_semantic_temp_name(&base_name, used_names);
    prefix_lines.push(format!("{indent}{temp} <- rr_index_vec_floor({expr})"));
    index_temp_by_expr.insert(expr.to_string(), temp.clone());
    temp
}

pub(crate) fn o3_exact_gather_expr_is_safe(expr: &str) -> bool {
    let expr = expr.trim();
    if expr.is_empty()
        || expr.contains("<-")
        || expr.contains('"')
        || expr.contains('\'')
        || expr.contains("function(")
    {
        return false;
    }
    !expr_has_call_like_syntax(expr) || expr_has_only_o3_pure_call_like_syntax(expr)
}

pub(crate) fn invalidate_o3_exact_gather_temp_maps_for_lhs(
    lhs: &str,
    index_temp_by_expr: &mut FxHashMap<String, String>,
    gather_temp_by_call: &mut FxHashMap<String, String>,
) {
    if lhs.is_empty() {
        return;
    }
    index_temp_by_expr.retain(|expr, temp| temp != lhs && !text_might_mention_ident(expr, lhs));
    gather_temp_by_call.retain(|call, temp| temp != lhs && !text_might_mention_ident(call, lhs));
}

pub(crate) fn parse_return_expr_line(line: &str) -> Option<(&str, String)> {
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("return(")
        .and_then(|rest| rest.strip_suffix(')'))?;
    if inner.is_empty() || !return_outer_parens_are_balanced(trimmed) {
        return None;
    }
    Some((indent, inner.trim().to_string()))
}

pub(crate) fn return_outer_parens_are_balanced(expr: &str) -> bool {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in expr.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if in_string && ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' => depth += 1,
            ')' => {
                let Some(next_depth) = depth.checked_sub(1) else {
                    return false;
                };
                depth = next_depth;
                if depth == 0 && idx + ch.len_utf8() != expr.len() {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

#[derive(Clone)]
pub(crate) struct IndexedSymCall {
    pub(crate) callee: String,
    pub(crate) call: String,
    pub(crate) indexed_expr: String,
    pub(crate) loop_var: String,
}

pub(crate) struct LocalSymHelper {
    pub(crate) name: String,
    pub(crate) body: String,
}

pub(crate) fn line_indent(line: &str) -> String {
    line.chars()
        .take_while(|ch| ch.is_ascii_whitespace())
        .collect()
}

pub(crate) fn materialize_loop_indexed_vector_helper_calls(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    if !lines
        .iter()
        .any(|line| line.contains("repeat {") || line.contains("Sym_"))
    {
        return lines;
    }

    let pure_sym_helpers = collect_locally_pure_sym_helpers(&lines, pure_user_calls);
    let mut out = lines;
    let mut inserted_before = FxHashMap::<usize, Vec<String>>::default();
    let mut loop_call_temps = FxHashMap::<(usize, String), String>::default();
    let mut used_names: FxHashSet<String> = out
        .iter()
        .flat_map(|line| expr_idents(line.trim()))
        .collect();

    for idx in 0..out.len() {
        if !out[idx].contains("Sym_") || !out[idx].contains('[') {
            continue;
        }
        let Some(loop_start) = enclosing_repeat_start(&out, idx) else {
            continue;
        };
        let Some(mutated_base) = indexed_store_base_re()
            .and_then(|re| re.captures(out[idx].trim()))
            .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
        else {
            continue;
        };

        let occurrences = indexed_sym_calls_in_line(&out[idx]);
        if occurrences.is_empty() {
            continue;
        }

        let loop_indent = line_indent(&out[loop_start]);
        let mut rewritten = out[idx].clone();
        for occurrence in occurrences {
            if !(pure_user_calls.contains(&occurrence.callee)
                || pure_sym_helpers.contains(&occurrence.callee))
            {
                continue;
            }
            let deps = expr_idents(&occurrence.call);
            if deps.iter().any(|dep| dep == &occurrence.loop_var)
                || !deps.iter().any(|dep| dep == &mutated_base)
            {
                continue;
            }

            let key = (loop_start, occurrence.call.clone());
            let temp = if let Some(existing) = loop_call_temps.get(&key) {
                existing.clone()
            } else if let Some(existing) =
                exact_call_temp_available_before_loop(&out, loop_start, &occurrence.call)
            {
                loop_call_temps.insert(key, existing.clone());
                existing
            } else {
                let base_name = semantic_loop_helper_temp_name(&rewritten, &occurrence.call);
                let temp = unique_semantic_temp_name(&base_name, &mut used_names);
                inserted_before
                    .entry(loop_start)
                    .or_default()
                    .push(format!("{loop_indent}{temp} <- {}", occurrence.call));
                loop_call_temps.insert(key, temp.clone());
                temp
            };
            let replacement = format!("{temp}[{}]", occurrence.loop_var);
            rewritten = rewritten.replace(&occurrence.indexed_expr, &replacement);
        }
        out[idx] = rewritten;
    }

    if inserted_before.is_empty() {
        return out;
    }

    let mut final_lines = Vec::with_capacity(
        out.len()
            + inserted_before
                .values()
                .map(std::vec::Vec::len)
                .sum::<usize>(),
    );
    for (idx, line) in out.into_iter().enumerate() {
        if let Some(prefix) = inserted_before.remove(&idx) {
            final_lines.extend(prefix);
        }
        final_lines.push(line);
    }
    final_lines
}

#[path = "exact_gather_temps/indexed_gather_hoist.rs"]
mod indexed_gather_hoist;
pub(crate) use self::indexed_gather_hoist::*;
pub(crate) fn hoist_loop_invariant_indexed_gathers(lines: Vec<String>) -> Vec<String> {
    if !lines
        .iter()
        .any(|line| line.contains("repeat {") && !line.contains("<- function"))
        || !lines.iter().any(|line| line.contains("rr_index1_read("))
    {
        return lines;
    }

    let mut out = lines;
    let mut used_names: FxHashSet<String> = out
        .iter()
        .flat_map(|line| expr_idents(line.trim()))
        .collect();
    let mut loop_start = 0usize;
    while loop_start < out.len() {
        if out[loop_start].trim() != "repeat {" {
            loop_start += 1;
            continue;
        }
        let Some(loop_end) = matching_block_end_local(&out, loop_start) else {
            loop_start += 1;
            continue;
        };
        let Some((loop_var, loop_upper)) = parse_repeat_loop_guard(&out, loop_start, loop_end)
        else {
            loop_start = loop_end + 1;
            continue;
        };
        if !loop_has_proven_nonzero_trip(&out, loop_start, &loop_var, &loop_upper) {
            loop_start = loop_end + 1;
            continue;
        }
        if loop_has_observable_effects_before_error(&out[(loop_start + 1)..loop_end]) {
            loop_start = loop_end + 1;
            continue;
        }

        loop_start = build_loop_invariant_indexed_gather_plan(
            &out,
            loop_start,
            loop_end,
            &loop_var,
            &mut used_names,
        )
        .map_or(loop_end + 1, |plan| {
            apply_indexed_gather_hoist_plan(&mut out, loop_start, loop_end, plan)
        });
    }

    out
}

pub(crate) fn exact_call_temp_available_before_loop(
    lines: &[String],
    loop_start: usize,
    call: &str,
) -> Option<String> {
    let deps = expr_idents(call);
    for scan_idx in (0..loop_start).rev() {
        let trimmed = lines[scan_idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        if lines[scan_idx].contains("<- function") || trimmed == "}" {
            break;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if deps.iter().any(|dep| dep == lhs) {
            break;
        }
        if rhs == call
            && plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && !lhs.starts_with(".arg_")
            && !lhs.starts_with(".__rr_cse_")
        {
            return Some(lhs.to_string());
        }
    }
    None
}

pub(crate) fn parse_repeat_loop_guard(
    lines: &[String],
    loop_start: usize,
    loop_end: usize,
) -> Option<(String, String)> {
    for line in lines.iter().take(loop_end).skip(loop_start + 1).take(5) {
        let trimmed = line.trim();
        let Some(inner) = trimmed
            .strip_prefix("if (!(")
            .and_then(|rest| rest.strip_suffix(")) break"))
        else {
            continue;
        };
        let Some((lhs, rhs)) = inner.split_once(" <= ") else {
            continue;
        };
        let lhs = lhs.trim();
        let rhs = rhs.trim();
        if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
            return Some((lhs.to_string(), rhs.to_string()));
        }
    }
    None
}

#[derive(Clone, Debug)]
pub(crate) struct LoopIndexAlias {
    pub(crate) alias: String,
    pub(crate) index_vec: String,
    pub(crate) line_idx: usize,
}

pub(crate) fn collect_loop_index_aliases(
    lines: &[String],
    base_idx: usize,
    loop_var: &str,
) -> Vec<LoopIndexAlias> {
    let mut out = Vec::new();
    for (offset, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(inner) = single_call_inner(rhs, "rr_index1_read_idx") else {
            continue;
        };
        let Some(args) = split_top_level_args(inner) else {
            continue;
        };
        if args.len() < 2 || args[1].trim() != loop_var {
            continue;
        }
        let index_vec = args[0].trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs) && re.is_match(index_vec)) {
            continue;
        }
        out.push(LoopIndexAlias {
            alias: lhs.to_string(),
            index_vec: index_vec.to_string(),
            line_idx: base_idx + offset,
        });
    }
    out
}

pub(crate) fn loop_has_proven_nonzero_trip(
    lines: &[String],
    loop_start: usize,
    loop_var: &str,
    upper_expr: &str,
) -> bool {
    let Some(start) = latest_numeric_assignment_before(lines, loop_start, loop_var) else {
        return false;
    };
    let upper = parse_numeric_literal_expr(upper_expr).or_else(|| {
        plain_ident_re()
            .is_some_and(|re| re.is_match(upper_expr))
            .then(|| latest_numeric_assignment_before(lines, loop_start, upper_expr))
            .flatten()
    });
    upper.is_some_and(|upper| start <= upper)
}

pub(crate) fn latest_numeric_assignment_before(
    lines: &[String],
    before: usize,
    name: &str,
) -> Option<f64> {
    for line in lines.iter().take(before).rev() {
        let trimmed = line.trim();
        if trimmed.contains("<- function") {
            break;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs != name {
            continue;
        }
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        return parse_numeric_literal_expr(rhs);
    }
    None
}

pub(crate) fn parse_numeric_literal_expr(expr: &str) -> Option<f64> {
    let mut text = expr.trim();
    while let Some(inner) = text.strip_prefix('(').and_then(|s| s.strip_suffix(')')) {
        text = inner.trim();
    }
    text.parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value > 0.0)
}

pub(crate) fn collect_loop_assigned_bases(lines: &[String]) -> FxHashSet<String> {
    let mut assigned = FxHashSet::<String>::default();
    for line in lines {
        let trimmed = line.trim();
        if let Some(caps) = assign_re().and_then(|re| re.captures(trimmed))
            && let Some(lhs) = caps.name("lhs")
        {
            assigned.insert(lhs.as_str().trim().to_string());
        }
        if let Some(base) = indexed_store_base_re()
            .and_then(|re| re.captures(trimmed))
            .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
        {
            assigned.insert(base);
        }
    }
    assigned
}

pub(crate) fn line_reassigns_name(line: &str, name: &str) -> bool {
    let trimmed = line.trim();
    assign_re()
        .and_then(|re| re.captures(trimmed))
        .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim() == name))
        .unwrap_or(false)
}

pub(crate) fn loop_has_observable_effects_before_error(lines: &[String]) -> bool {
    const EFFECTS: &[&str] = &[
        "print(",
        "cat(",
        "message(",
        "warning(",
        "stop(",
        "quit(",
        "tryCatch(",
        "dyn.load(",
        "system(",
        "source(",
        "assign(",
        "<<-",
    ];
    lines
        .iter()
        .any(|line| EFFECTS.iter().any(|effect| line.contains(effect)))
}

pub(crate) fn collect_index1_read_alias_calls(line: &str, alias: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut search = 0usize;
    while let Some(relative) = line[search..].find("rr_index1_read(") {
        let start = search + relative;
        let Some((call_start, call_end)) = match_balanced_call_span(line, start, "rr_index1_read")
        else {
            break;
        };
        let call = line[call_start..call_end].trim();
        if let Some((_base, idx_alias)) = parse_index1_read_base_alias(call)
            && idx_alias == alias
        {
            out.push(call.to_string());
        }
        search = call_end;
    }
    out
}

pub(crate) fn parse_index1_read_base_alias(call: &str) -> Option<(String, String)> {
    let inner = single_call_inner(call, "rr_index1_read")?;
    let args = split_top_level_args(inner)?;
    if args.len() < 2 {
        return None;
    }
    let base = args[0].trim();
    let alias = args[1].trim();
    if plain_ident_re().is_some_and(|re| re.is_match(base) && re.is_match(alias)) {
        Some((base.to_string(), alias.to_string()))
    } else {
        None
    }
}

pub(crate) fn collect_vector_calls_by_name(expr: &str, callee: &str) -> Vec<String> {
    let mut calls = Vec::new();
    for idx in expr.char_indices().map(|(idx, _)| idx) {
        if let Some((start, end)) = match_balanced_call_span(expr, idx, callee) {
            calls.push(expr[start..end].trim().to_string());
        }
    }
    calls
}

pub(crate) fn collect_locally_pure_sym_helpers(
    lines: &[String],
    pure_user_calls: &FxHashSet<String>,
) -> FxHashSet<String> {
    let mut helpers = Vec::<LocalSymHelper>::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            idx += 1;
            continue;
        };
        let name = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !name.starts_with("Sym_") || !rhs.starts_with("function") {
            idx += 1;
            continue;
        }
        let Some(end) = matching_block_end_local(lines, idx) else {
            idx += 1;
            continue;
        };
        let body = if idx < end {
            lines[(idx + 1)..end].join("\n")
        } else {
            String::new()
        };
        helpers.push(LocalSymHelper {
            name: name.to_string(),
            body,
        });
        idx = end + 1;
    }

    let mut pure = FxHashSet::<String>::default();
    loop {
        let before = pure.len();
        for helper in &helpers {
            if pure.contains(&helper.name) || sym_helper_body_has_impure_effects(&helper.body) {
                continue;
            }
            let calls = call_like_idents(&helper.body);
            let all_calls_pure = calls.into_iter().all(|callee| {
                known_pure_call_ident(&callee)
                    || pure_user_calls.contains(&callee)
                    || pure.contains(&callee)
            });
            if all_calls_pure {
                pure.insert(helper.name.clone());
            }
        }
        if pure.len() == before {
            break;
        }
    }
    pure
}

pub(crate) fn sym_helper_body_has_impure_effects(body: &str) -> bool {
    const IMPURE_SNIPPETS: &[&str] = &[
        "<<-",
        "rr_mark(",
        "print(",
        "cat(",
        "message(",
        "warning(",
        "stop(",
        "quit(",
        "tryCatch(",
        "dyn.load(",
        "system(",
        "source(",
        "assign(",
        ".Call(",
        "library(",
        "require(",
    ];
    IMPURE_SNIPPETS.iter().any(|snippet| body.contains(snippet))
}
pub(crate) fn known_pure_call_ident(callee: &str) -> bool {
    const PURE_CALLS: &[&str] = &[
        "return",
        "ifelse",
        "abs",
        "sqrt",
        "log",
        "log10",
        "log2",
        "exp",
        "sign",
        "floor",
        "ceiling",
        "trunc",
        "sin",
        "cos",
        "tan",
        "asin",
        "acos",
        "atan",
        "atan2",
        "sinh",
        "cosh",
        "tanh",
        "length",
        "seq_len",
        "seq_along",
        "mean",
        "sum",
        "min",
        "max",
        "pmin",
        "pmax",
        "is.na",
        "is.finite",
        "rep.int",
        "numeric",
        "vector",
        "matrix",
        "c",
        "rr_index1_read",
        "rr_index1_read_vec",
        "rr_index1_read_vec_floor",
        "rr_index_vec_floor",
        "rr_gather",
        "rr_wrap_index_vec_i",
        "rr_idx_cube_vec_i",
        "rr_parallel_typed_vec_call",
        "rr_field_get",
        "rr_field_exists",
        "rr_list_pattern_matchable",
        "rr_named_list",
        "replace",
    ];
    PURE_CALLS.contains(&callee)
}

pub(crate) fn call_like_idents(expr: &str) -> FxHashSet<String> {
    let mut out = FxHashSet::<String>::default();
    for (idx, ch) in expr.char_indices() {
        if ch != '(' {
            continue;
        }
        if let Some(callee) = call_like_ident_before_open(expr, idx) {
            out.insert(callee.to_string());
        }
    }
    out
}

pub(crate) fn semantic_loop_helper_temp_name(line: &str, call: &str) -> String {
    if line.contains("du1[") {
        return "adv_u".to_string();
    }
    if line.contains("du2[") {
        return "adv_u2".to_string();
    }
    if line.contains("du3[") {
        return "adv_u3".to_string();
    }
    let Some(open) = call.find('(') else {
        return "vec_helper".to_string();
    };
    let Some(inner) = call
        .strip_suffix(')')
        .and_then(|_| call.get(open + 1..call.len() - 1))
    else {
        return "vec_helper".to_string();
    };
    if let Some(args) = split_top_level_args(inner)
        && let Some(first) = args.first()
    {
        let base = semantic_base_name(first.trim());
        return format!("{base}_vec");
    }
    "vec_helper".to_string()
}
