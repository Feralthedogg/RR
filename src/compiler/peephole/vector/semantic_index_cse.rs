use super::*;
pub(crate) fn indexed_sym_calls_in_line(line: &str) -> Vec<IndexedSymCall> {
    let mut out = Vec::new();
    let mut search = 0usize;
    while let Some(relative) = line[search..].find("Sym_") {
        let start = search + relative;
        let callee_end = line[start..]
            .char_indices()
            .find_map(|(offset, ch)| {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    None
                } else {
                    Some(start + offset)
                }
            })
            .unwrap_or(line.len());
        let callee = &line[start..callee_end];
        let Some((call_start, call_end)) = match_balanced_call_span(line, start, callee) else {
            search = callee_end;
            continue;
        };
        let mut bracket_start = call_end;
        while line[bracket_start..]
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_whitespace())
        {
            bracket_start += 1;
        }
        if !line[bracket_start..].starts_with('[') {
            search = call_end;
            continue;
        }
        let Some(bracket_end_relative) = line[(bracket_start + 1)..].find(']') else {
            search = call_end;
            continue;
        };
        let bracket_end = bracket_start + 1 + bracket_end_relative;
        let loop_var = line[(bracket_start + 1)..bracket_end].trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(loop_var)) {
            search = bracket_end + 1;
            continue;
        }
        let indexed_end = bracket_end + 1;
        out.push(IndexedSymCall {
            callee: callee.to_string(),
            call: line[call_start..call_end].trim().to_string(),
            indexed_expr: line[call_start..indexed_end].trim().to_string(),
            loop_var: loop_var.to_string(),
        });
        search = indexed_end;
    }
    out
}

pub(crate) fn enclosing_repeat_start(lines: &[String], target_idx: usize) -> Option<usize> {
    let mut stack = Vec::<Option<usize>>::new();
    for (idx, line) in lines.iter().enumerate().take(target_idx) {
        let trimmed = line.trim();
        let (opens, closes) = count_unquoted_braces_local(trimmed);
        for _ in 0..closes {
            stack.pop();
        }
        for open_idx in 0..opens {
            if open_idx == 0 && trimmed == "repeat {" {
                stack.push(Some(idx));
            } else {
                stack.push(None);
            }
        }
    }
    stack.into_iter().rev().flatten().next()
}

pub(crate) fn matching_block_end_local(lines: &[String], start_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
        let (opens, closes) = count_unquoted_braces_local(line);
        depth += opens;
        if closes > 0 {
            depth = depth.saturating_sub(closes);
            if depth == 0 && idx > start_idx {
                return Some(idx);
            }
        }
    }
    None
}

pub(crate) fn count_unquoted_braces_local(line: &str) -> (usize, usize) {
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for ch in line.chars() {
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
            '{' => opens += 1,
            '}' => closes += 1,
            _ => {}
        }
    }
    (opens, closes)
}

pub(crate) fn unique_semantic_temp_name(base: &str, used_names: &mut FxHashSet<String>) -> String {
    let base = sanitize_semantic_temp_name(base);
    if used_names.insert(base.clone()) {
        return base;
    }
    let mut idx = 2usize;
    loop {
        let candidate = format!("{base}_{idx}");
        if used_names.insert(candidate.clone()) {
            return candidate;
        }
        idx += 1;
    }
}

pub(crate) fn sanitize_semantic_temp_name(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() || out.starts_with(|ch: char| ch.is_ascii_digit()) {
        format!("rr_tmp_{out}")
    } else {
        out
    }
}

pub(crate) fn semantic_o3_cse_name_for_rhs(rhs: &str) -> Option<String> {
    let rhs = rhs.trim();
    if let Some(inner) = single_call_inner(rhs, "rr_index_vec_floor") {
        return Some(format!("idx_{}", semantic_index_suffix(inner)));
    }
    if let Some(inner) = single_call_inner(rhs, "rr_gather")
        && let Some(args) = split_top_level_args(inner)
        && args.len() == 2
    {
        let base = semantic_base_name(args[0].trim());
        if args[1].contains("rr_wrap_index_vec_i(") {
            return Some(format!("{base}_wrap"));
        }
        let raw_suffix = semantic_index_suffix(args[1].trim());
        let suffix = raw_suffix
            .strip_prefix("idx_")
            .unwrap_or(raw_suffix.as_str())
            .to_string();
        return Some(format!("{base}_{suffix}"));
    }
    None
}

pub(crate) fn single_call_inner<'a>(expr: &'a str, callee: &str) -> Option<&'a str> {
    let (start, end) = match_balanced_call_span(expr, 0, callee)?;
    if start != 0 || end != expr.len() {
        return None;
    }
    let open = callee.len();
    expr.get((open + 1)..(end - 1)).map(str::trim)
}

pub(crate) fn semantic_base_name(expr: &str) -> String {
    let trimmed = expr.trim();
    if plain_ident_re().is_some_and(|re| re.is_match(trimmed)) {
        return trimmed.to_string();
    }
    sanitize_semantic_temp_name(trimmed)
}

pub(crate) fn semantic_index_suffix(expr: &str) -> String {
    let trimmed = expr.trim();
    if let Some(inner) = single_call_inner(trimmed, "rr_index_vec_floor") {
        return semantic_index_suffix(inner);
    }
    if let Some(stripped) = trimmed.strip_prefix("idx_") {
        return stripped.to_string();
    }
    if let Some(stripped) = trimmed.strip_prefix(".__rr_cse_") {
        return format!("cse_{stripped}");
    }
    if let Some(stripped) = trimmed.strip_prefix("adj_") {
        return stripped.to_string();
    }
    if let Some(stripped) = trimmed.strip_prefix("n_") {
        return stripped.to_string();
    }
    if let Some(inner) = single_call_inner(trimmed, "rr_gather")
        && let Some(args) = split_top_level_args(inner)
        && args.len() == 2
    {
        let base = semantic_base_name(args[0].trim());
        let suffix = semantic_index_suffix(args[1].trim());
        return format!("{base}_{suffix}");
    }
    sanitize_semantic_temp_name(trimmed)
}

pub(crate) fn straight_line_assignment_indent(line: &str) -> Option<usize> {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || line.contains("<- function")
        || trimmed == "{"
        || trimmed == "}"
        || trimmed == "} else {"
        || trimmed.starts_with("if ")
        || trimmed.starts_with("for ")
        || trimmed.starts_with("while ")
        || trimmed == "repeat {"
        || trimmed == "next"
        || trimmed == "break"
        || trimmed.starts_with("return(")
    {
        return None;
    }
    assign_re().and_then(|re| re.captures(trimmed))?;
    Some(line.len() - line.trim_start().len())
}

pub(crate) fn straight_line_o3_index_cse_region_indent(line: &str) -> Option<usize> {
    straight_line_assignment_indent(line).or_else(|| {
        let trimmed = line.trim();
        let is_mark =
            trimmed.starts_with("rr_mark(") && (trimmed.ends_with(");") || trimmed.ends_with(')'));
        let is_return = parse_return_expr_line(line).is_some();
        (is_mark || is_return).then_some(line.len() - line.trim_start().len())
    })
}

pub(crate) fn hoist_index_vec_floor_calls_in_region(
    region: &[String],
    next_cse_idx: usize,
) -> (Vec<String>, usize) {
    let (region, _line_map, next_cse_idx) =
        hoist_index_vec_floor_calls_in_region_with_map(region, next_cse_idx);
    (region, next_cse_idx)
}

pub(crate) fn hoist_index_vec_floor_calls_in_region_with_map(
    region: &[String],
    mut next_cse_idx: usize,
) -> (Vec<String>, Vec<u32>, usize) {
    if region.len() < 2 {
        let line_map = (1..=region.len() as u32).collect::<Vec<_>>();
        return (region.to_vec(), line_map, next_cse_idx);
    }

    let mut rewritten = region.to_vec();
    let mut prefix_lines = vec![Vec::<String>::new(); region.len()];

    for _ in 0..16 {
        let Some((candidate, first_idx, last_idx, indent)) =
            best_region_index_vec_floor_cse(&rewritten)
        else {
            break;
        };
        let temp = format!(".__rr_cse_{}", next_cse_idx);
        next_cse_idx += 1;
        prefix_lines[first_idx].push(format!("{indent}{temp} <- {candidate}"));
        for line in rewritten.iter_mut().take(last_idx + 1).skip(first_idx) {
            *line = line.replace(&candidate, &temp);
        }
    }

    let mut out =
        Vec::with_capacity(region.len() + prefix_lines.iter().map(Vec::len).sum::<usize>());
    let mut line_map = vec![0u32; region.len()];
    for (idx, line) in rewritten.into_iter().enumerate() {
        out.append(&mut prefix_lines[idx]);
        line_map[idx] = (out.len() + 1) as u32;
        out.push(line);
    }
    (out, line_map, next_cse_idx)
}

pub(crate) fn best_region_index_vec_floor_cse(
    lines: &[String],
) -> Option<(String, usize, usize, String)> {
    let mut locations = FxHashMap::<String, Vec<usize>>::default();
    for (idx, line) in lines.iter().enumerate() {
        let Some(rhs) = index_cse_expr_for_line(line) else {
            continue;
        };
        for call in collect_index_vec_floor_calls(&rhs) {
            locations.entry(call).or_default().push(idx);
        }
    }

    locations
        .into_iter()
        .filter(|(call, locs)| locs.len() >= 2 && call.len() > 18)
        .filter_map(|(call, locs)| {
            let first_idx = *locs.first()?;
            let last_idx = *locs.last()?;
            if !region_index_cse_is_safe(lines, &call, first_idx, last_idx) {
                return None;
            }
            let indent = lines[first_idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            let temp = ".__rr_cse_0";
            let savings = estimated_cse_savings(&indent, &call, locs.len(), temp);
            (savings >= -16).then_some((call, first_idx, last_idx, indent, savings, locs.len()))
        })
        .max_by(
            |(lhs_call, lhs_first, _, _, lhs_savings, lhs_count),
             (rhs_call, rhs_first, _, _, rhs_savings, rhs_count)| {
                lhs_savings
                    .cmp(rhs_savings)
                    .then_with(|| lhs_call.len().cmp(&rhs_call.len()))
                    .then_with(|| lhs_count.cmp(rhs_count))
                    .then_with(|| rhs_first.cmp(lhs_first))
            },
        )
        .map(|(call, first_idx, last_idx, indent, _, _)| (call, first_idx, last_idx, indent))
}

pub(crate) fn index_cse_expr_for_line(line: &str) -> Option<String> {
    if let Some(caps) = assign_re().and_then(|re| re.captures(line.trim())) {
        return caps.name("rhs").map(|m| m.as_str().trim().to_string());
    }
    parse_return_expr_line(line).map(|(_, rhs)| rhs)
}

pub(crate) fn region_index_cse_is_safe(
    lines: &[String],
    call: &str,
    first_idx: usize,
    last_idx: usize,
) -> bool {
    if call.contains("<-") || call.contains('"') || call.contains("function(") {
        return false;
    }
    let deps: FxHashSet<String> = expr_idents(call)
        .into_iter()
        .filter(|ident| !ident.starts_with("rr_"))
        .collect();
    if deps.is_empty() {
        return false;
    }

    for line in lines.iter().take(last_idx + 1).skip(first_idx) {
        let trimmed = line.trim();
        if let Some(base) = super::patterns::indexed_store_base_re()
            .and_then(|re| re.captures(trimmed))
            .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
            && deps.contains(&base)
        {
            return false;
        }
        if let Some(lhs) = assign_re()
            .and_then(|re| re.captures(trimmed))
            .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()))
            && deps.contains(&lhs)
        {
            return false;
        }
    }
    true
}

pub(crate) fn collect_index_vec_floor_calls(expr: &str) -> Vec<String> {
    let mut calls = Vec::new();
    for idx in expr.char_indices().map(|(idx, _)| idx) {
        if let Some((start, end)) = match_balanced_call_span(expr, idx, "rr_index_vec_floor") {
            calls.push(expr[start..end].trim().to_string());
        }
    }
    calls
}

pub(crate) fn best_profitable_arithmetic_subexpr_cse(
    expr: &str,
    indent: &str,
    temp: &str,
) -> Option<String> {
    collect_repeated_arithmetic_subexprs(expr)
        .into_iter()
        .filter(|(candidate, _)| candidate.trim() != expr.trim())
        .filter_map(|(candidate, count)| {
            let savings = estimated_cse_savings(indent, &candidate, count, temp);
            (savings >= 24).then_some((candidate, savings, count))
        })
        .max_by(
            |(lhs_expr, lhs_savings, lhs_count), (rhs_expr, rhs_savings, rhs_count)| {
                lhs_savings
                    .cmp(rhs_savings)
                    .then_with(|| lhs_expr.len().cmp(&rhs_expr.len()))
                    .then_with(|| lhs_count.cmp(rhs_count))
            },
        )
        .map(|(candidate, _, _)| candidate)
}

pub(crate) fn collect_repeated_arithmetic_subexprs(expr: &str) -> Vec<(String, usize)> {
    let mut stack = Vec::<usize>::new();
    let mut counts = FxHashMap::<String, usize>::default();
    for (idx, ch) in expr.char_indices() {
        match ch {
            '(' => stack.push(idx),
            ')' => {
                let Some(start) = stack.pop() else {
                    continue;
                };
                let end = idx + ch.len_utf8();
                let candidate = expr[start..end].trim();
                if reusable_arithmetic_subexpr(candidate) {
                    *counts.entry(candidate.to_string()).or_default() += 1;
                }
            }
            _ => {}
        }
    }
    let mut repeated: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .collect();
    repeated.sort_by(|(lhs, lhs_count), (rhs, rhs_count)| {
        rhs.len()
            .cmp(&lhs.len())
            .then_with(|| rhs_count.cmp(lhs_count))
            .then_with(|| lhs.cmp(rhs))
    });
    repeated
}

pub(crate) fn reusable_arithmetic_subexpr(expr: &str) -> bool {
    if expr.len() < 40
        || expr.contains("<-")
        || expr.contains("ifelse(")
        || expr.contains("Sym_")
        || expr.contains("return(")
    {
        return false;
    }
    if expr_has_call_like_syntax(expr) && !expr_has_only_o3_pure_call_like_syntax(expr) {
        return false;
    }
    if !expr
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "._ +-*/^()<>!=&|,[]\"".contains(ch))
    {
        return false;
    }
    let inner = expr
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(expr)
        .trim();
    inner.contains(" + ")
        || inner.contains(" - ")
        || inner.contains(" * ")
        || inner.contains(" / ")
        || inner.contains(" > ")
        || inner.contains(" < ")
        || inner.contains("==")
        || inner.contains("!=")
}

pub(crate) fn expr_has_only_o3_pure_call_like_syntax(expr: &str) -> bool {
    let mut saw_call = false;
    for (idx, ch) in expr.char_indices() {
        if ch != '(' {
            continue;
        }
        let Some(callee) = call_like_ident_before_open(expr, idx) else {
            continue;
        };
        saw_call = true;
        if !matches!(
            callee,
            "rr_index1_read"
                | "rr_index1_read_idx"
                | "rr_gather"
                | "rr_index_vec_floor"
                | "seq_len"
        ) {
            return false;
        }
    }
    saw_call
}

pub(crate) fn expr_has_call_like_syntax(expr: &str) -> bool {
    for (idx, ch) in expr.char_indices() {
        if ch != '(' {
            continue;
        }
        if call_like_ident_before_open(expr, idx).is_some() {
            return true;
        }
    }
    false
}

pub(crate) fn call_like_ident_before_open(expr: &str, open_idx: usize) -> Option<&str> {
    let before = &expr[..open_idx];
    let before = before.trim_end();
    let ident_end = before.len();
    let ident_start = before
        .char_indices()
        .rev()
        .take_while(|(_, ch)| helper_ident_char(*ch))
        .last()
        .map(|(idx, _)| idx)
        .unwrap_or(ident_end);
    (ident_start < ident_end).then_some(&before[ident_start..ident_end])
}

pub(crate) fn hoist_repeated_vector_helper_calls_with_options_and_map(
    lines: Vec<String>,
    allow_general_lhs: bool,
    max_temps_per_line: usize,
    min_count: usize,
    min_savings: isize,
) -> (Vec<String>, Vec<u32>) {
    let mut out = Vec::with_capacity(lines.len());
    let mut line_map = vec![0u32; lines.len()];
    let mut next_cse_idx = next_generated_cse_index(&lines);

    for (idx, line) in lines.into_iter().enumerate() {
        let Some(caps) = assign_re().and_then(|re| re.captures(line.trim_end())) else {
            line_map[idx] = (out.len() + 1) as u32;
            out.push(line);
            continue;
        };
        if line.contains("<- function") {
            line_map[idx] = (out.len() + 1) as u32;
            out.push(line);
            continue;
        }

        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let mut rhs = caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if lhs.starts_with(".__rr_cse_")
            || rhs.is_empty()
            || (!allow_general_lhs && !lhs.starts_with(".tachyon_exprmap"))
        {
            line_map[idx] = (out.len() + 1) as u32;
            out.push(line);
            continue;
        }
        if !helper_call_prefixes()
            .iter()
            .any(|prefix| rhs.contains(prefix))
            && !reusable_vector_helper_names()
                .iter()
                .any(|callee| rhs.contains(&format!("{callee}(")))
        {
            line_map[idx] = (out.len() + 1) as u32;
            out.push(line);
            continue;
        }

        let mut prefix_lines = Vec::new();
        while prefix_lines.len() < max_temps_per_line {
            let temp = format!(".__rr_cse_{}", next_cse_idx);
            let candidate = if allow_general_lhs {
                best_profitable_vector_helper_cse(&rhs, indent, &temp, min_count, min_savings)
            } else {
                collect_repeated_vector_helper_calls(&rhs)
                    .into_iter()
                    .next()
            };
            let Some(candidate) = candidate else {
                break;
            };
            next_cse_idx += 1;
            prefix_lines.push(format!("{indent}{temp} <- {candidate}"));
            rhs = rhs.replace(&candidate, &temp);
        }

        out.extend(prefix_lines);
        if rhs == caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim() {
            line_map[idx] = (out.len() + 1) as u32;
            out.push(line);
        } else {
            line_map[idx] = (out.len() + 1) as u32;
            out.push(format!("{indent}{lhs} <- {rhs}"));
        }
    }

    (out, line_map)
}

pub(crate) fn rewrite_forward_exact_vector_helper_reuse(lines: Vec<String>) -> Vec<String> {
    rewrite_forward_exact_vector_helper_reuse_impl(lines, false)
}

pub(crate) fn rewrite_forward_exact_vector_helper_reuse_aggressive(
    lines: Vec<String>,
) -> Vec<String> {
    rewrite_forward_exact_vector_helper_reuse_impl(lines, true)
}

#[derive(Debug)]
pub(crate) struct ParsedAssignment {
    pub(crate) lhs: String,
    pub(crate) rhs: String,
}

pub(crate) fn parse_assignment_owned(line: &str) -> Option<ParsedAssignment> {
    let caps = assign_re().and_then(|re| re.captures(line.trim()))?;
    Some(ParsedAssignment {
        lhs: caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string(),
        rhs: caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string(),
    })
}

pub(crate) fn line_indent_width(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

pub(crate) fn is_forward_reuse_boundary(
    line: &str,
    trimmed: &str,
    indent: usize,
    candidate_indent: usize,
) -> bool {
    (!trimmed.is_empty() && indent < candidate_indent)
        || line.contains("<- function")
        || trimmed == "repeat {"
        || trimmed.starts_with("while")
        || trimmed.starts_with("for")
}

pub(crate) fn lhs_reassigned_later_within_forward_region(
    lines: &[String],
    idx: usize,
    candidate_indent: usize,
    lhs: &str,
) -> bool {
    (idx + 1..lines.len()).any(|scan_idx| {
        let scan_trimmed = lines[scan_idx].trim();
        let scan_indent = line_indent_width(&lines[scan_idx]);
        if is_forward_reuse_boundary(
            &lines[scan_idx],
            scan_trimmed,
            scan_indent,
            candidate_indent,
        ) {
            return false;
        }
        parse_assignment_owned(scan_trimmed).is_some_and(|assign| assign.lhs == lhs)
    })
}

pub(crate) fn has_tachyon_consumer_after(lines: &[String], idx: usize) -> bool {
    lines[(idx + 1)..]
        .iter()
        .take_while(|line| {
            let trimmed = line.trim();
            !(line.contains("<- function")
                || trimmed == "repeat {"
                || trimmed.starts_with("while")
                || trimmed.starts_with("for"))
        })
        .any(|line| line.contains(".tachyon_exprmap") || line.contains("rr_assign_slice("))
}

pub(crate) fn forward_exact_helper_uses(
    out: &mut [String],
    idx: usize,
    candidate_indent: usize,
    lhs: &str,
    rhs: &str,
    deps: &FxHashSet<String>,
    lhs_reassigned_later: bool,
) {
    let mut line_no = idx + 1;
    while line_no < out.len() {
        let line_trimmed = out[line_no].trim().to_string();
        let next_indent = line_indent_width(&out[line_no]);
        if is_forward_reuse_boundary(&out[line_no], &line_trimmed, next_indent, candidate_indent) {
            break;
        }
        if let Some(base) = super::patterns::indexed_store_base_re()
            .and_then(|re| re.captures(&line_trimmed))
            .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
            && (base == lhs || deps.contains(&base))
        {
            break;
        }

        if let Some(next_assign) = parse_assignment_owned(&line_trimmed) {
            if next_assign.lhs == lhs {
                break;
            }
            if next_assign.rhs.contains(rhs) {
                if lhs_reassigned_later {
                    line_no += 1;
                    continue;
                }
                out[line_no] = out[line_no].replacen(rhs, lhs, usize::MAX);
            }
            if deps.contains(&next_assign.lhs) {
                break;
            }
            line_no += 1;
            continue;
        }

        if line_trimmed.contains(rhs) {
            out[line_no] = out[line_no].replacen(rhs, lhs, usize::MAX);
        }
        if line_trimmed == "return(NULL)"
            || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
        {
            break;
        }
        line_no += 1;
    }
}

pub(crate) fn rewrite_forward_exact_vector_helper_reuse_impl(
    lines: Vec<String>,
    allow_plain_lhs: bool,
) -> Vec<String> {
    let mut out = lines;
    let len = out.len();
    for idx in 0..len {
        let line_owned = out[idx].clone();
        let trimmed = line_owned.trim();
        let candidate_indent = line_indent_width(&line_owned);
        let Some(assign) = parse_assignment_owned(trimmed) else {
            continue;
        };
        if !(plain_ident_re().is_some_and(|re| re.is_match(&assign.lhs))
            || assign.lhs.starts_with(".__rr_cse_"))
            || !expr_is_exact_reusable_vector_helper(&assign.rhs)
        {
            continue;
        }
        if !assign.lhs.starts_with(".__rr_cse_") && !allow_plain_lhs {
            continue;
        }
        if allow_plain_lhs && assign.lhs.starts_with(".arg_") {
            continue;
        }

        let lhs_reassigned_later =
            lhs_reassigned_later_within_forward_region(&out, idx, candidate_indent, &assign.lhs);
        let deps: FxHashSet<String> = expr_idents(&assign.rhs).into_iter().collect();
        if !allow_plain_lhs && !has_tachyon_consumer_after(&out, idx) {
            continue;
        }
        forward_exact_helper_uses(
            &mut out,
            idx,
            candidate_indent,
            &assign.lhs,
            &assign.rhs,
            &deps,
            lhs_reassigned_later,
        );
    }
    out
}

pub(crate) fn mark_unused_tachyon_exprmap_temps(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    for idx in (0..out.len()).rev() {
        let line = out[idx].clone();
        let Some(caps) = assign_re().and_then(|re| re.captures(line.trim())) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".tachyon_exprmap") {
            continue;
        }
        let is_live_later = out
            .iter()
            .skip(idx + 1)
            .any(|later| line_contains_symbol(later.trim(), lhs));
        if !is_live_later {
            let indent_len = line.len() - line.trim_start().len();
            out[idx] = format!("{}# rr-cse-pruned", &line[..indent_len]);
        }
    }
    out.into_iter()
        .filter(|line| line.trim() != "# rr-cse-pruned")
        .collect()
}

pub(crate) fn rewrite_forward_temp_aliases(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    for idx in 0..out.len() {
        let line_owned = out[idx].clone();
        let trimmed = line_owned.trim();
        let candidate_indent = line_owned.len() - line_owned.trim_start().len();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let rhs = caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if !lhs.starts_with(".__rr_cse_")
            || !plain_ident_re().is_some_and(|re| re.is_match(&rhs))
            || lhs == rhs
        {
            continue;
        }

        let mut line_no = idx + 1;
        while line_no < out.len() {
            let line_trimmed = out[line_no].trim().to_string();
            let next_indent = out[line_no].len() - out[line_no].trim_start().len();
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                break;
            }
            if out[line_no].contains("<- function")
                || line_trimmed == "repeat {"
                || line_trimmed.starts_with("while")
                || line_trimmed.starts_with("for")
            {
                break;
            }

            if let Some(next_caps) = assign_re().and_then(|re| re.captures(&line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if next_lhs == lhs || next_lhs == rhs {
                    break;
                }
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if text_might_mention_ident(&next_rhs, &lhs)
                    && expr_idents(&next_rhs).iter().any(|ident| ident == &lhs)
                {
                    out[line_no] = out[line_no].replacen(&lhs, &rhs, usize::MAX);
                }
                line_no += 1;
                continue;
            }

            if text_might_mention_ident(&line_trimmed, &lhs)
                && expr_idents(&line_trimmed).iter().any(|ident| ident == &lhs)
            {
                out[line_no] = out[line_no].replacen(&lhs, &rhs, usize::MAX);
            }
            if line_trimmed == "return(NULL)"
                || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
            {
                break;
            }
            line_no += 1;
        }
    }
    out
}
