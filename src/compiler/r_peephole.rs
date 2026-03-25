use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::OnceLock;

const IDENT_PATTERN: &str = r"(?:[A-Za-z_][A-Za-z0-9._]*|\.[A-Za-z_][A-Za-z0-9._]*)";

fn compile_regex(pattern: String) -> Option<Regex> {
    Regex::new(&pattern).ok()
}

fn assign_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^(?P<indent>\s*)(?P<lhs>{}) <- (?P<rhs>.+)$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn indexed_store_base_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^(?P<indent>\s*)(?P<base>{})\s*\[[^\]]+\]\s*<-\s*.+$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn range_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^(?P<start>{}|1L?|1(?:\.0+)?)\:(?P<end>{}|\d+(?:\.\d+)?)$",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn floor_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^rr_index_vec_floor\((?P<src>[^\)]*)\)$".to_string()))
        .as_ref()
}

fn nested_index_vec_floor_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"rr_index_vec_floor\(rr_index_vec_floor\((?P<inner>{})\)\)",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn seq_len_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^seq_len\((?P<len>{}|\d+(?:\.\d+)?)\)$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn rep_int_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^rep\.int\([^,]+,\s*(?P<len>{}|\d+(?:\.\d+)?)\)$",
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn length_call_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(format!(r"length\((?P<var>{})\)", IDENT_PATTERN)))
        .as_ref()
}

fn scalar_lit_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^\d+(?:\.\d+)?L?$".to_string()))
        .as_ref()
}

fn plain_ident_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(format!(r"^{}$", IDENT_PATTERN)))
        .as_ref()
}

fn ident_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(IDENT_PATTERN.to_string()))
        .as_ref()
}

fn call_map_slice_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r"^rr_call_map_slice_auto\((?P<dest>[^,]+),\s*(?P<start>[^,]+),\s*(?P<end>[^,]+),\s*(?P<rest>.+)\)$"
                .to_string(),
        )
    })
    .as_ref()
}

fn assign_slice_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r"^rr_assign_slice\((?P<dest>[^,]+),\s*(?P<start>[^,]+),\s*(?P<end>[^,]+),\s*(?P<rest>.+)\)$"
                .to_string(),
        )
    })
    .as_ref()
}

fn call_map_whole_builtin_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r#"^rr_call_map_whole_auto\((?P<dest>[^,]+),\s*"(?P<callee>abs|sqrt|log|pmax|pmin)",\s*[^,]+,\s*c\((?P<slots>[^\)]*)\),\s*(?P<args>.+)\)$"#.to_string(),
        )
    })
    .as_ref()
}

fn split_top_level_args(expr: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (idx, ch) in expr.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                args.push(expr[start..idx].trim().to_string());
                start = idx + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    args.push(expr[start..].trim().to_string());
    Some(args)
}

fn rewrite_direct_vec_helper_expr(expr: &str, enabled: bool) -> String {
    if !enabled {
        return expr.trim().to_string();
    }
    let expr = expr.trim();
    for prefix in [
        "rr_parallel_vec_add_f64(",
        "rr_parallel_vec_sub_f64(",
        "rr_parallel_vec_mul_f64(",
        "rr_parallel_vec_div_f64(",
        "rr_parallel_vec_pmax_f64(",
        "rr_parallel_vec_pmin_f64(",
        "rr_intrinsic_vec_add_f64(",
        "rr_intrinsic_vec_sub_f64(",
        "rr_intrinsic_vec_mul_f64(",
        "rr_intrinsic_vec_div_f64(",
        "rr_intrinsic_vec_pmax_f64(",
        "rr_intrinsic_vec_pmin_f64(",
    ] {
        if let Some(inner) = expr.strip_prefix(prefix).and_then(|s| s.strip_suffix(')'))
            && let Some(args) = split_top_level_args(inner)
            && args.len() == 2
        {
            let lhs = rewrite_direct_vec_helper_expr(&args[0], enabled);
            let rhs = rewrite_direct_vec_helper_expr(&args[1], enabled);
            return if prefix.contains("_pmax_") {
                format!("pmax({lhs}, {rhs})")
            } else if prefix.contains("_pmin_") {
                format!("pmin({lhs}, {rhs})")
            } else {
                let op = if prefix.contains("_add_") {
                    "+"
                } else if prefix.contains("_sub_") {
                    "-"
                } else if prefix.contains("_mul_") {
                    "*"
                } else {
                    "/"
                };
                format!("({lhs} {op} {rhs})")
            };
        }
    }

    for prefix in [
        "rr_parallel_vec_abs_f64(",
        "rr_parallel_vec_log_f64(",
        "rr_parallel_vec_sqrt_f64(",
        "rr_intrinsic_vec_abs_f64(",
        "rr_intrinsic_vec_log_f64(",
        "rr_intrinsic_vec_sqrt_f64(",
    ] {
        if let Some(inner) = expr.strip_prefix(prefix).and_then(|s| s.strip_suffix(')')) {
            let arg = rewrite_direct_vec_helper_expr(inner, enabled);
            let fun = if prefix.contains("_abs_") {
                "abs"
            } else if prefix.contains("_log_") {
                "log"
            } else {
                "sqrt"
            };
            return format!("{fun}({arg})");
        }
    }

    rewrite_helper_subcalls(expr, enabled)
}

fn helper_call_prefixes() -> &'static [&'static str] {
    &[
        "rr_parallel_vec_add_f64(",
        "rr_parallel_vec_sub_f64(",
        "rr_parallel_vec_mul_f64(",
        "rr_parallel_vec_div_f64(",
        "rr_parallel_vec_abs_f64(",
        "rr_parallel_vec_log_f64(",
        "rr_parallel_vec_sqrt_f64(",
        "rr_parallel_vec_pmax_f64(",
        "rr_parallel_vec_pmin_f64(",
        "rr_intrinsic_vec_add_f64(",
        "rr_intrinsic_vec_sub_f64(",
        "rr_intrinsic_vec_mul_f64(",
        "rr_intrinsic_vec_div_f64(",
        "rr_intrinsic_vec_abs_f64(",
        "rr_intrinsic_vec_log_f64(",
        "rr_intrinsic_vec_sqrt_f64(",
        "rr_intrinsic_vec_pmax_f64(",
        "rr_intrinsic_vec_pmin_f64(",
    ]
}

fn rewrite_helper_subcalls(expr: &str, enabled: bool) -> String {
    if !enabled {
        return expr.trim().to_string();
    }
    let mut out = expr.trim().to_string();
    loop {
        let mut changed = false;
        let bytes = out.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            let slice = &out[i..];
            let Some(prefix) = helper_call_prefixes()
                .iter()
                .find(|prefix| slice.starts_with(**prefix))
            else {
                i += 1;
                continue;
            };
            let call_start = i;
            let mut depth = 0i32;
            let mut end = None;
            for (off, ch) in out[call_start..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(call_start + off + 1);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(call_end) = end else {
                break;
            };
            let call = &out[call_start..call_end];
            let rewritten = rewrite_direct_vec_helper_expr(call, enabled);
            if rewritten != call {
                out = format!("{}{}{}", &out[..call_start], rewritten, &out[call_end..]);
                changed = true;
                break;
            }
            i = call_start + prefix.len();
        }
        if !changed {
            break;
        }
    }
    out
}

fn resolve_alias(name: &str, aliases: &FxHashMap<String, String>) -> String {
    let mut current = name;
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    while let Some(next) = aliases.get(current) {
        if !seen.insert(current) {
            break;
        }
        current = next;
    }
    current.to_string()
}

fn is_peephole_temp(name: &str) -> bool {
    name.starts_with(".__rr_")
        || name.starts_with(".tachyon_")
        || name.starts_with("i_")
        || name.starts_with(".tmp")
}

fn alias_chain_contains(name: &str, needle: &str, aliases: &FxHashMap<String, String>) -> bool {
    let mut current = name;
    let mut seen: FxHashSet<&str> = FxHashSet::default();
    while let Some(next) = aliases.get(current) {
        if next == needle {
            return true;
        }
        if !seen.insert(current) {
            break;
        }
        current = next;
    }
    false
}

fn invalidate_aliases_for_write(lhs: &str, aliases: &mut FxHashMap<String, String>) {
    let doomed: Vec<String> = aliases
        .keys()
        .filter(|name| name.as_str() == lhs || alias_chain_contains(name, lhs, aliases))
        .cloned()
        .collect();
    for name in doomed {
        aliases.remove(&name);
    }
}

fn rewrite_known_aliases(expr: &str, aliases: &FxHashMap<String, String>) -> String {
    let Some(re) = ident_re() else {
        return expr.to_string();
    };
    let rewritten = re.replace_all(expr, |caps: &Captures<'_>| {
        let ident = caps.get(0).map(|m| m.as_str()).unwrap_or("");
        aliases
            .get(ident)
            .map(|_| resolve_alias(ident, aliases))
            .unwrap_or_else(|| ident.to_string())
    });
    rewritten.to_string()
}

fn normalize_expr_with_aliases(expr: &str, aliases: &FxHashMap<String, String>) -> String {
    let mut out = rewrite_known_aliases(expr, aliases);
    let mut ordered_aliases: Vec<(&str, &str)> = aliases
        .iter()
        .map(|(alias, target)| (alias.as_str(), target.as_str()))
        .collect();
    ordered_aliases.sort_by(|(lhs_a, _), (lhs_b, _)| {
        lhs_b.len().cmp(&lhs_a.len()).then_with(|| lhs_a.cmp(lhs_b))
    });
    for (alias, target) in ordered_aliases {
        out = out.replace(alias, target);
    }
    out = out.replace(".arg_", "");
    out
}

fn collect_prologue_arg_aliases(lines: &[String], idx: usize) -> FxHashMap<String, String> {
    let mut fn_start = 0usize;
    for prev in (0..=idx).rev() {
        if lines[prev].contains("<- function") {
            fn_start = prev + 1;
            break;
        }
    }
    let mut aliases = FxHashMap::default();
    for line in lines.iter().take(idx).skip(fn_start) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if is_control_flow_boundary(trimmed) {
            break;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs.starts_with(".arg_") && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
            aliases.insert(lhs.to_string(), rhs.to_string());
        }
    }
    aliases
}

fn shifted_minus_one_temp_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^\(\((?P<a>{}) \+ (?P<b>{})\) - 1\)$",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn shifted_minus_one_temp_name(rhs: &str) -> Option<String> {
    let caps = shifted_minus_one_temp_re().and_then(|re| re.captures(rhs.trim()))?;
    let a = caps.name("a").map(|m| m.as_str()).unwrap_or("").trim();
    let b = caps.name("b").map(|m| m.as_str()).unwrap_or("").trim();
    (a == b).then(|| a.to_string())
}

fn doubled_temp_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^\((?P<a>{}) \+ (?P<b>{})\)$",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn doubled_temp_name(rhs: &str) -> Option<String> {
    let caps = doubled_temp_re().and_then(|re| re.captures(rhs.trim()))?;
    let a = caps.name("a").map(|m| m.as_str()).unwrap_or("").trim();
    let b = caps.name("b").map(|m| m.as_str()).unwrap_or("").trim();
    (a == b).then(|| a.to_string())
}

fn rewrite_shifted_square_scalar_reuse(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut base_to_named = FxHashMap::<String, String>::default();
    let mut doubled_to_named = FxHashMap::<String, String>::default();
    let mut idx = 0usize;
    while idx < out.len() {
        let trimmed = out[idx].trim().to_string();
        if trimmed.contains("<- function") {
            base_to_named.clear();
            doubled_to_named.clear();
            idx += 1;
            continue;
        }
        if is_control_flow_boundary(&trimmed)
            && trimmed != "}"
            && !trimmed.starts_with("if ")
            && !trimmed.starts_with("else")
            && !trimmed.starts_with("} else")
        {
            base_to_named.clear();
            doubled_to_named.clear();
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            idx += 1;
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
            if let Some(temp) = shifted_minus_one_temp_name(rhs) {
                base_to_named.insert(temp.to_string(), lhs.to_string());
            } else if let Some(temp) = doubled_temp_name(rhs)
                && let Some(named) = base_to_named.get(&temp).cloned()
            {
                doubled_to_named.insert(lhs.to_string(), named);
            }
        }
        if !rhs.contains("* (") || !rhs.contains("- 1") {
            idx += 1;
            continue;
        }
        let mut rewritten_rhs = rhs.to_string();
        for (temp, named) in &doubled_to_named {
            let needle = format!("(({temp} - 1) * ({temp} - 1))");
            let replacement = format!("({named} * {named})");
            rewritten_rhs = rewritten_rhs.replace(&needle, &replacement);
        }
        if rewritten_rhs != rhs {
            let indent_len = out[idx].len() - out[idx].trim_start().len();
            let indent = &out[idx][..indent_len];
            out[idx] = format!("{indent}{lhs} <- {rewritten_rhs}");
        }
        idx += 1;
    }
    out
}

fn read_vec_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>{}|rr_index_vec_floor\([^\)]*\)|[^,\)]*:[^\)]*)\)",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn normalize_expr(expr: &str, scalar_consts: &FxHashMap<String, String>) -> String {
    let trimmed = expr.trim();
    scalar_consts
        .get(trimmed)
        .cloned()
        .unwrap_or_else(|| trimmed.to_string())
}

fn expr_idents(expr: &str) -> Vec<String> {
    let Some(re) = ident_re() else {
        return Vec::new();
    };
    re.find_iter(expr).map(|m| m.as_str().to_string()).collect()
}

fn cse_temp_index_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"\.__rr_cse_(?P<idx>\d+)\b".to_string()))
        .as_ref()
}

fn next_generated_cse_index(lines: &[String]) -> usize {
    lines
        .iter()
        .flat_map(|line| {
            cse_temp_index_re()
                .into_iter()
                .flat_map(|re| re.captures_iter(line))
                .filter_map(|caps| {
                    caps.name("idx")
                        .and_then(|m| m.as_str().parse::<usize>().ok())
                })
        })
        .max()
        .map_or(0, |idx| idx + 1)
}

fn reusable_vector_helper_names() -> &'static [&'static str] {
    &[
        "rr_index1_read_vec_floor",
        "rr_index1_read_vec",
        "rr_gather",
        "rr_index_vec_floor",
    ]
}

fn helper_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '.'
}

fn match_balanced_call_span(expr: &str, start: usize, callee: &str) -> Option<(usize, usize)> {
    if !expr.get(start..)?.starts_with(callee) {
        return None;
    }
    let prev_ok = expr[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !helper_ident_char(ch));
    if !prev_ok {
        return None;
    }
    let open = start + callee.len();
    if expr.as_bytes().get(open).copied() != Some(b'(') {
        return None;
    }
    let mut depth = 0i32;
    for (off, ch) in expr[open..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((start, open + off + 1));
                }
            }
            _ => {}
        }
    }
    None
}

fn collect_repeated_vector_helper_calls(expr: &str) -> Vec<String> {
    let mut counts = FxHashMap::<String, usize>::default();
    for idx in expr.char_indices().map(|(idx, _)| idx) {
        for &callee in reusable_vector_helper_names() {
            let Some((start, end)) = match_balanced_call_span(expr, idx, callee) else {
                continue;
            };
            if start != idx {
                continue;
            }
            let call = expr[start..end].trim().to_string();
            *counts.entry(call).or_default() += 1;
        }
    }
    let mut repeated: Vec<(String, usize)> = counts
        .into_iter()
        .filter(|(call, count)| *count > 2 && call.len() > 12)
        .collect();
    repeated.sort_by(|(lhs_call, lhs_count), (rhs_call, rhs_count)| {
        rhs_call
            .len()
            .cmp(&lhs_call.len())
            .then_with(|| rhs_count.cmp(lhs_count))
            .then_with(|| lhs_call.cmp(rhs_call))
    });
    repeated.into_iter().map(|(call, _)| call).collect()
}

fn expr_is_exact_reusable_vector_helper(rhs: &str) -> bool {
    let rhs = rhs.trim();
    if rhs.is_empty() || rhs.contains("<-") || rhs.contains("function(") || rhs.contains('"') {
        return false;
    }
    reusable_vector_helper_names().iter().any(|callee| {
        match_balanced_call_span(rhs, 0, callee)
            .is_some_and(|(start, end)| start == 0 && end == rhs.len())
    })
}

fn hoist_repeated_vector_helper_calls_within_lines(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut next_cse_idx = next_generated_cse_index(&lines);

    for line in lines {
        let Some(caps) = assign_re().and_then(|re| re.captures(line.trim_end())) else {
            out.push(line);
            continue;
        };
        if line.contains("<- function") {
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
        if lhs.starts_with(".__rr_cse_") || rhs.is_empty() || !lhs.starts_with(".tachyon_exprmap") {
            out.push(line);
            continue;
        }

        let mut prefix_lines = Vec::new();
        loop {
            let Some(candidate) = collect_repeated_vector_helper_calls(&rhs)
                .into_iter()
                .next()
            else {
                break;
            };
            let temp = format!(".__rr_cse_{}", next_cse_idx);
            next_cse_idx += 1;
            prefix_lines.push(format!("{indent}{temp} <- {candidate}"));
            rhs = rhs.replace(&candidate, &temp);
        }

        out.extend(prefix_lines);
        if rhs == caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim() {
            out.push(line);
        } else {
            out.push(format!("{indent}{lhs} <- {rhs}"));
        }
    }

    out
}

fn rewrite_forward_exact_vector_helper_reuse(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let len = out.len();
    for idx in 0..len {
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
        if !(plain_ident_re().is_some_and(|re| re.is_match(&lhs)) || lhs.starts_with(".__rr_cse_"))
            || !expr_is_exact_reusable_vector_helper(&rhs)
        {
            continue;
        }
        if !lhs.starts_with(".__rr_cse_") {
            continue;
        }

        let lhs_reassigned_later = (idx + 1..out.len()).any(|scan_idx| {
            let scan_trimmed = out[scan_idx].trim();
            let scan_indent = out[scan_idx].len() - out[scan_idx].trim_start().len();
            if !scan_trimmed.is_empty() && scan_indent < candidate_indent {
                return false;
            }
            if out[scan_idx].contains("<- function")
                || scan_trimmed == "repeat {"
                || scan_trimmed.starts_with("while")
                || scan_trimmed.starts_with("for")
            {
                return false;
            }
            assign_re()
                .and_then(|re| re.captures(scan_trimmed))
                .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()))
                .is_some_and(|scan_lhs| scan_lhs == lhs)
        });

        let deps: FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
        let has_tachyon_consumer = out[(idx + 1)..]
            .iter()
            .take_while(|line| {
                let trimmed = line.trim();
                !(line.contains("<- function")
                    || trimmed == "repeat {"
                    || trimmed.starts_with("while")
                    || trimmed.starts_with("for"))
            })
            .any(|line| line.contains(".tachyon_exprmap") || line.contains("rr_assign_slice("));
        if !has_tachyon_consumer {
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
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if next_lhs == lhs {
                    break;
                }
                if next_rhs.contains(&rhs) {
                    if lhs_reassigned_later {
                        line_no += 1;
                        continue;
                    }
                    out[line_no] = out[line_no].replacen(&rhs, &lhs, usize::MAX);
                }
                if deps.contains(&next_lhs) {
                    break;
                }
                line_no += 1;
                continue;
            }

            if line_trimmed.contains(&rhs) {
                out[line_no] = out[line_no].replacen(&rhs, &lhs, usize::MAX);
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

fn rewrite_forward_temp_aliases(lines: Vec<String>) -> Vec<String> {
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
                if expr_idents(&next_rhs).iter().any(|ident| ident == &lhs) {
                    out[line_no] = out[line_no].replacen(&lhs, &rhs, usize::MAX);
                }
                line_no += 1;
                continue;
            }

            if expr_idents(&line_trimmed).iter().any(|ident| ident == &lhs) {
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

fn is_expr_builtin_name(name: &str) -> bool {
    matches!(
        name,
        "abs"
            | "sqrt"
            | "log"
            | "pmax"
            | "pmin"
            | "ifelse"
            | "seq_len"
            | "rr_parallel_vec_add_f64"
            | "rr_parallel_vec_sub_f64"
            | "rr_parallel_vec_mul_f64"
            | "rr_parallel_vec_div_f64"
            | "rr_parallel_vec_abs_f64"
            | "rr_parallel_vec_log_f64"
            | "rr_parallel_vec_sqrt_f64"
            | "rr_intrinsic_vec_add_f64"
            | "rr_intrinsic_vec_sub_f64"
            | "rr_intrinsic_vec_mul_f64"
            | "rr_intrinsic_vec_div_f64"
            | "rr_intrinsic_vec_abs_f64"
            | "rr_intrinsic_vec_log_f64"
            | "rr_intrinsic_vec_sqrt_f64"
    )
}

fn expr_proven_no_na(
    expr: &str,
    no_na_vars: &FxHashSet<String>,
    scalar_consts: &FxHashMap<String, String>,
) -> bool {
    let expr = expr.trim();
    if scalar_lit_re().is_some_and(|re| re.is_match(expr)) || expr == "TRUE" || expr == "FALSE" {
        return true;
    }
    if no_na_vars.contains(expr) || scalar_consts.contains_key(expr) {
        return true;
    }
    if let Some(inner) = expr
        .strip_prefix("abs(")
        .and_then(|s| s.strip_suffix(')'))
        .or_else(|| expr.strip_prefix("sqrt(").and_then(|s| s.strip_suffix(')')))
        .or_else(|| expr.strip_prefix("log(").and_then(|s| s.strip_suffix(')')))
        .or_else(|| {
            expr.strip_prefix("floor(")
                .and_then(|s| s.strip_suffix(')'))
        })
        .or_else(|| {
            expr.strip_prefix("ceiling(")
                .and_then(|s| s.strip_suffix(')'))
        })
        .or_else(|| {
            expr.strip_prefix("trunc(")
                .and_then(|s| s.strip_suffix(')'))
        })
    {
        return expr_proven_no_na(inner, no_na_vars, scalar_consts);
    }
    if let Some(inner) = expr
        .strip_prefix("seq_len(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return expr_proven_no_na(inner, no_na_vars, scalar_consts);
    }
    if let Some(args) = expr
        .strip_prefix("pmax(")
        .and_then(|s| s.strip_suffix(')'))
        .or_else(|| expr.strip_prefix("pmin(").and_then(|s| s.strip_suffix(')')))
        .or_else(|| {
            expr.strip_prefix("ifelse(")
                .and_then(|s| s.strip_suffix(')'))
        })
    {
        return expr_idents(args)
            .into_iter()
            .filter(|ident| !is_expr_builtin_name(ident))
            .all(|ident| no_na_vars.contains(&ident) || scalar_consts.contains_key(&ident));
    }
    if expr.contains("rr_index1_read")
        || expr.contains("rr_call_map")
        || expr.contains("rr_assign_slice")
        || expr.contains("rr_ifelse_strict")
    {
        return false;
    }
    expr_idents(expr)
        .into_iter()
        .filter(|ident| !is_expr_builtin_name(ident))
        .all(|ident| no_na_vars.contains(&ident) || scalar_consts.contains_key(&ident))
}

fn expr_is_logical_comparison(
    expr: &str,
    no_na_vars: &FxHashSet<String>,
    scalar_consts: &FxHashMap<String, String>,
) -> bool {
    let expr = expr.trim();
    let has_logical_shape = ["<=", ">=", "==", "!=", "<", ">", "&&", "||"]
        .iter()
        .any(|op| expr.contains(op));
    has_logical_shape && expr_proven_no_na(expr, no_na_vars, scalar_consts)
}

fn rewrite_strict_ifelse_expr(
    expr: &str,
    no_na_vars: &FxHashSet<String>,
    scalar_consts: &FxHashMap<String, String>,
) -> String {
    if let Some(inner) = expr
        .trim()
        .strip_prefix("rr_ifelse_strict(")
        .and_then(|s| s.strip_suffix(')'))
    {
        let args = split_top_level_args(inner);
        if args.as_ref().is_some_and(|parts| {
            parts.len() == 3 && expr_is_logical_comparison(&parts[0], no_na_vars, scalar_consts)
        }) {
            return format!("ifelse({inner})");
        }
    }
    expr.to_string()
}

fn helper_heavy_runtime_auto_args(args: &str) -> bool {
    [
        "rr_gather(",
        "rr_array3_gather_values(",
        "rr_index1_read_vec(",
        "rr_index1_read_vec_floor(",
        "rr_ifelse_strict(",
        "rr_assign_slice(",
        "rr_dim1_read_values(",
        "rr_dim2_read_values(",
        "rr_dim3_read_values(",
    ]
    .iter()
    .any(|needle| args.contains(needle))
}

fn helper_heavy_runtime_auto_args_with_temps(
    args: &str,
    helper_heavy_vars: &FxHashSet<String>,
) -> bool {
    helper_heavy_runtime_auto_args(args)
        || expr_idents(args)
            .into_iter()
            .any(|ident| helper_heavy_vars.contains(&ident))
}

fn is_one(expr: &str, scalar_consts: &FxHashMap<String, String>) -> bool {
    matches!(
        normalize_expr(expr, scalar_consts).as_str(),
        "1" | "1L" | "1.0"
    )
}

fn infer_len_from_expr(
    expr: &str,
    vector_lens: &FxHashMap<String, String>,
    scalar_consts: &FxHashMap<String, String>,
) -> Option<String> {
    if let Some(caps) = seq_len_re().and_then(|re| re.captures(expr)) {
        return Some(normalize_expr(&caps["len"], scalar_consts));
    }
    if let Some(caps) = rep_int_re().and_then(|re| re.captures(expr)) {
        return Some(normalize_expr(&caps["len"], scalar_consts));
    }

    let mut seen = FxHashSet::default();
    let mut out: Option<String> = None;
    let bytes = expr.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if c.is_ascii_alphabetic() || c == '_' || c == '.' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
                    i += 1;
                } else {
                    break;
                }
            }
            let ident = &expr[start..i];
            if !seen.insert(ident.to_string()) {
                continue;
            }
            let Some(len) = vector_lens.get(ident).cloned() else {
                continue;
            };
            match &out {
                None => out = Some(len),
                Some(prev) if prev == &len => {}
                Some(_) => return None,
            }
        } else {
            i += 1;
        }
    }
    out
}

fn rewrite_known_length_calls(expr: &str, vector_lens: &FxHashMap<String, String>) -> String {
    let Some(re) = length_call_re() else {
        return expr.to_string();
    };
    re.replace_all(expr, |caps: &Captures<'_>| {
        let var = caps.name("var").map(|m| m.as_str()).unwrap_or("");
        vector_lens
            .get(var)
            .cloned()
            .unwrap_or_else(|| caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string())
    })
    .to_string()
}

fn identity_index_end_expr(
    idx: &str,
    identity_indices: &FxHashMap<String, String>,
    scalar_consts: &FxHashMap<String, String>,
) -> Option<String> {
    let idx = idx.trim();
    if let Some(end) = identity_indices.get(idx) {
        return Some(end.clone());
    }
    if let Some(caps) = range_re().and_then(|re| re.captures(idx)) {
        let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
        if is_one(start, scalar_consts) {
            return Some(normalize_expr(
                caps.name("end").map(|m| m.as_str()).unwrap_or(""),
                scalar_consts,
            ));
        }
    }
    if let Some(caps) = floor_re().and_then(|re| re.captures(idx)) {
        let src = caps.name("src").map(|m| m.as_str()).unwrap_or("").trim();
        if let Some(end) = identity_index_end_expr(src, identity_indices, scalar_consts) {
            return Some(end);
        }
    }
    None
}

fn clear_linear_facts(
    scalar_consts: &mut FxHashMap<String, String>,
    vector_lens: &mut FxHashMap<String, String>,
    identity_indices: &mut FxHashMap<String, String>,
    aliases: &mut FxHashMap<String, String>,
    no_na_vars: &mut FxHashSet<String>,
    helper_heavy_vars: &mut FxHashSet<String>,
) {
    scalar_consts.clear();
    vector_lens.clear();
    identity_indices.clear();
    aliases.clear();
    no_na_vars.clear();
    helper_heavy_vars.clear();
}

fn clear_loop_boundary_facts(
    identity_indices: &mut FxHashMap<String, String>,
    aliases: &mut FxHashMap<String, String>,
    no_na_vars: &mut FxHashSet<String>,
    helper_heavy_vars: &mut FxHashSet<String>,
) {
    identity_indices.clear();
    aliases.clear();
    no_na_vars.clear();
    helper_heavy_vars.clear();
}

fn is_control_flow_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    let is_single_line_guard =
        trimmed.starts_with("if ") && (trimmed.ends_with(" break") || trimmed.ends_with(" next"));
    trimmed == "{"
        || trimmed == "}"
        || trimmed == "repeat {"
        || (trimmed.starts_with("if ") && !is_single_line_guard)
        || trimmed.starts_with("if(")
        || trimmed.starts_with("else")
        || trimmed.starts_with("} else")
        || trimmed.starts_with("while")
        || trimmed.starts_with("for")
        || trimmed == "break"
        || trimmed == "next"
}

fn is_dead_parenthesized_eval_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.contains("<-")
        || trimmed.starts_with("return(")
        || trimmed == "return(NULL)"
        || trimmed.starts_with("rr_mark(")
        || trimmed.starts_with("print(")
        || is_control_flow_boundary(line)
    {
        return false;
    }
    trimmed.starts_with('(') && trimmed.ends_with(')')
}

fn is_dead_plain_ident_eval_line(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty()
        || trimmed.contains("<-")
        || trimmed.starts_with("return(")
        || trimmed == "return(NULL)"
        || trimmed.starts_with("rr_mark(")
        || trimmed.starts_with("print(")
        || is_control_flow_boundary(line)
    {
        return false;
    }
    plain_ident_re().is_some_and(|re| re.is_match(trimmed))
}

pub(crate) fn optimize_emitted_r(code: &str, direct_builtin_call_map: bool) -> String {
    optimize_emitted_r_with_context(code, direct_builtin_call_map, &FxHashSet::default()).0
}

pub(crate) fn optimize_emitted_r_with_line_map(
    code: &str,
    direct_builtin_call_map: bool,
) -> (String, Vec<u32>) {
    optimize_emitted_r_with_context(code, direct_builtin_call_map, &FxHashSet::default())
}

#[derive(Debug, Clone)]
struct PureCallBinding {
    expr: String,
    var: String,
    deps: FxHashSet<String>,
}

#[derive(Debug, Clone)]
struct SimpleExprHelper {
    params: Vec<String>,
    expr: String,
}

#[derive(Debug, Clone)]
struct MetricHelper {
    name_param: String,
    value_param: String,
    pre_name_lines: Vec<String>,
    pre_value_lines: Vec<String>,
}

fn collect_mutated_arg_aliases(code: &str) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let mut seen_initial_aliases = FxHashSet::default();

    for line in code.lines() {
        let trimmed = line.trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            if let Some(caps) = indexed_store_base_re().and_then(|re| re.captures(trimmed)) {
                let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                if base.starts_with(".arg_") {
                    out.insert(base.to_string());
                }
            }
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".arg_") {
            continue;
        }
        let is_initial_alias = plain_ident_re().is_some_and(|re| re.is_match(rhs))
            && !seen_initial_aliases.contains(lhs);
        if is_initial_alias {
            seen_initial_aliases.insert(lhs.to_string());
            continue;
        }
        out.insert(lhs.to_string());
    }

    out
}

pub(crate) fn optimize_emitted_r_with_context(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
) -> (String, Vec<u32>) {
    optimize_emitted_r_with_context_and_fresh_with_options(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        &FxHashSet::default(),
        false,
    )
}

pub(crate) fn optimize_emitted_r_with_context_and_fresh(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
) -> (String, Vec<u32>) {
    optimize_emitted_r_with_context_and_fresh_with_options(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        fresh_user_calls,
        false,
    )
}

#[allow(dead_code)]
struct EmittedROptimizationConfig<'a> {
    direct_builtin_call_map: bool,
    pure_user_calls: &'a FxHashSet<String>,
    fresh_user_calls: &'a FxHashSet<String>,
    reusable_pure_user_calls: &'a FxHashSet<String>,
    preserve_all_defs: bool,
}

#[allow(dead_code)]
fn run_initial_emitted_r_rewrite_pass(
    code: &str,
    cfg: &EmittedROptimizationConfig<'_>,
) -> Vec<String> {
    let direct_builtin_call_map = cfg.direct_builtin_call_map;
    let fresh_user_calls = cfg.fresh_user_calls;
    let reusable_pure_user_calls = cfg.reusable_pure_user_calls;
    let preserve_all_defs = cfg.preserve_all_defs;

    let mut scalar_consts: FxHashMap<String, String> = FxHashMap::default();
    let mut vector_lens: FxHashMap<String, String> = FxHashMap::default();
    let mut identity_indices: FxHashMap<String, String> = FxHashMap::default();
    let mut aliases: FxHashMap<String, String> = FxHashMap::default();
    let mut no_na_vars: FxHashSet<String> = FxHashSet::default();
    let mut helper_heavy_vars: FxHashSet<String> = FxHashSet::default();
    let mut fresh_expr_for_var: FxHashMap<String, String> = FxHashMap::default();
    let mut pure_call_bindings: Vec<PureCallBinding> = Vec::new();
    let mut last_rhs_for_var: FxHashMap<String, String> = FxHashMap::default();
    let mut out_lines = Vec::new();
    let mut conditional_depth = 0usize;
    let mutated_arg_aliases = collect_mutated_arg_aliases(code);

    for line in code.lines() {
        let trimmed_line = line.trim();
        let closes_conditional = trimmed_line == "}";
        let else_boundary = trimmed_line.starts_with("} else {");
        let opens_conditional = trimmed_line.starts_with("if ") && trimmed_line.ends_with('{');
        if closes_conditional && !else_boundary && conditional_depth > 0 {
            conditional_depth -= 1;
        }
        if trimmed_line.starts_with("if ")
            && trimmed_line.ends_with(" break")
            && trimmed_line.contains("rr_truthy1(")
        {
            let rewritten = rewrite_guard_truthy_line(line, &no_na_vars, &scalar_consts);
            out_lines.push(rewritten);
            continue;
        }
        if trimmed_line.starts_with("if ")
            && trimmed_line.ends_with('{')
            && trimmed_line.contains("rr_truthy1(")
        {
            let rewritten = rewrite_if_truthy_line(line, &no_na_vars, &scalar_consts);
            out_lines.push(rewritten);
            if opens_conditional {
                conditional_depth += 1;
            }
            continue;
        }

        if line.contains("<- function") {
            clear_linear_facts(
                &mut scalar_consts,
                &mut vector_lens,
                &mut identity_indices,
                &mut aliases,
                &mut no_na_vars,
                &mut helper_heavy_vars,
            );
            fresh_expr_for_var.clear();
            last_rhs_for_var.clear();
            out_lines.push(line.to_string());
            if opens_conditional {
                conditional_depth += 1;
            }
            continue;
        }

        if is_control_flow_boundary(line) {
            let mut rewritten_line = line.to_string();
            if rewritten_line.trim().starts_with("if ")
                && rewritten_line.trim().ends_with('{')
                && rewritten_line.contains("rr_truthy1(")
            {
                rewritten_line =
                    rewrite_if_truthy_line(&rewritten_line, &no_na_vars, &scalar_consts);
            }
            if trimmed_line == "repeat {" {
                clear_loop_boundary_facts(
                    &mut identity_indices,
                    &mut aliases,
                    &mut no_na_vars,
                    &mut helper_heavy_vars,
                );
            } else {
                clear_linear_facts(
                    &mut scalar_consts,
                    &mut vector_lens,
                    &mut identity_indices,
                    &mut aliases,
                    &mut no_na_vars,
                    &mut helper_heavy_vars,
                );
            }
            fresh_expr_for_var.clear();
            last_rhs_for_var.clear();
            out_lines.push(rewritten_line);
            continue;
        }

        if let Some(base) = indexed_store_base_re()
            .and_then(|re| re.captures(line))
            .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
        {
            fresh_expr_for_var.remove(&base);
            scalar_consts.remove(&base);
            vector_lens.remove(&base);
            identity_indices.remove(&base);
            no_na_vars.remove(&base);
            helper_heavy_vars.remove(&base);
            last_rhs_for_var.remove(&base);
            invalidate_aliases_for_write(&base, &mut aliases);
            pure_call_bindings
                .retain(|binding| binding.var != base && !binding.deps.contains(&base));
            out_lines.push(line.to_string());
            continue;
        }

        let Some(caps) = assign_re().and_then(|re| re.captures(line)) else {
            let rewritten_line = rewrite_return_expr_line(line, &last_rhs_for_var);
            out_lines.push(rewritten_line);
            continue;
        };

        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();

        let rewritten_rhs = if let Some(re) = read_vec_re() {
            re.replace_all(rhs, |caps: &Captures<'_>| {
                let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                let idx = caps.name("idx").map(|m| m.as_str()).unwrap_or("");
                match (
                    identity_index_end_expr(idx, &identity_indices, &scalar_consts),
                    vector_lens.get(base),
                ) {
                    (Some(end), Some(base_len)) if &end == base_len => base.to_string(),
                    _ => caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
                }
            })
            .to_string()
        } else {
            rhs.to_string()
        };
        let rewritten_rhs = rewrite_known_length_calls(&rewritten_rhs, &vector_lens);
        let rewritten_rhs = rewrite_known_aliases(&rewritten_rhs, &aliases);
        let rewritten_rhs = rewrite_direct_vec_helper_expr(&rewritten_rhs, direct_builtin_call_map);
        let rewritten_rhs = rewrite_pure_call_reuse(&rewritten_rhs, &pure_call_bindings);

        let rewritten_rhs =
            if let Some(caps) = call_map_slice_re().and_then(|re| re.captures(&rewritten_rhs)) {
                let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
                let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
                let end = caps.name("end").map(|m| m.as_str()).unwrap_or("").trim();
                let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("").trim();
                let end = normalize_expr(end, &scalar_consts);
                if is_one(start, &scalar_consts)
                    && vector_lens
                        .get(dest)
                        .is_some_and(|dest_len| dest_len == &end)
                {
                    format!("rr_call_map_whole_auto({dest}, {rest})")
                } else {
                    rewritten_rhs
                }
            } else {
                rewritten_rhs
            };

        let rewritten_rhs = if direct_builtin_call_map {
            if let Some(caps) =
                call_map_whole_builtin_re().and_then(|re| re.captures(&rewritten_rhs))
            {
                let callee = caps.name("callee").map(|m| m.as_str()).unwrap_or("");
                let slots = caps.name("slots").map(|m| m.as_str()).unwrap_or("").trim();
                let args = caps.name("args").map(|m| m.as_str()).unwrap_or("").trim();
                if (slots == "1L" || slots == "1")
                    && !helper_heavy_runtime_auto_args_with_temps(args, &helper_heavy_vars)
                {
                    match callee {
                        "abs" | "sqrt" | "log" => format!("{callee}({args})"),
                        "pmax" | "pmin" => format!("{callee}({args})"),
                        _ => rewritten_rhs,
                    }
                } else {
                    rewritten_rhs
                }
            } else {
                rewritten_rhs
            }
        } else {
            rewritten_rhs
        };

        let rewritten_rhs = if preserve_all_defs {
            rewritten_rhs
        } else {
            rewrite_strict_ifelse_expr(&rewritten_rhs, &no_na_vars, &scalar_consts)
        };

        let rewritten_rhs =
            if let Some(caps) = assign_slice_re().and_then(|re| re.captures(&rewritten_rhs)) {
                let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
                let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
                let end = normalize_expr(
                    caps.name("end").map(|m| m.as_str()).unwrap_or("").trim(),
                    &scalar_consts,
                );
                let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("").trim();
                if is_one(start, &scalar_consts)
                    && vector_lens
                        .get(dest)
                        .is_some_and(|dest_len| dest_len == &end)
                    && infer_len_from_expr(rest, &vector_lens, &scalar_consts)
                        .is_some_and(|len| len == end)
                {
                    rest.to_string()
                } else {
                    rewritten_rhs
                }
            } else {
                rewritten_rhs
            };

        let rewritten_rhs = rewrite_known_length_calls(&rewritten_rhs, &vector_lens);
        let rewritten_rhs = rewrite_known_aliases(&rewritten_rhs, &aliases);
        let rewritten_rhs = rewrite_direct_vec_helper_expr(&rewritten_rhs, direct_builtin_call_map);
        let rewritten_rhs = rewrite_pure_call_reuse(&rewritten_rhs, &pure_call_bindings);
        let rewritten_rhs = if preserve_all_defs {
            rewritten_rhs
        } else {
            rewrite_strict_ifelse_expr(&rewritten_rhs, &no_na_vars, &scalar_consts)
        };
        let rewritten_rhs = if !last_rhs_for_var.contains_key(lhs) {
            maybe_expand_fresh_alias_rhs(&rewritten_rhs, &fresh_expr_for_var)
                .unwrap_or(rewritten_rhs)
        } else {
            rewritten_rhs
        };

        if scalar_lit_re().is_some_and(|re| re.is_match(rewritten_rhs.trim())) {
            scalar_consts.insert(lhs.to_string(), rewritten_rhs.trim().to_string());
        } else {
            scalar_consts.remove(lhs);
        }

        if let Some(base) = written_base_var(lhs) {
            fresh_expr_for_var.remove(base);
            scalar_consts.remove(base);
            vector_lens.remove(base);
            identity_indices.remove(base);
            no_na_vars.remove(base);
            helper_heavy_vars.remove(base);
            last_rhs_for_var.retain(|var, rhs| {
                var != base && !expr_idents(rhs).iter().any(|ident| ident == base)
            });
            invalidate_aliases_for_write(base, &mut aliases);
            pure_call_bindings
                .retain(|binding| binding.var != base && !binding.deps.contains(base));
        }
        invalidate_aliases_for_write(lhs, &mut aliases);
        pure_call_bindings.retain(|binding| binding.var != lhs && !binding.deps.contains(lhs));
        last_rhs_for_var
            .retain(|var, rhs| var != lhs && !expr_idents(rhs).iter().any(|ident| ident == lhs));
        let rhs_ident = rewritten_rhs.trim();
        let allow_simple_alias = !preserve_all_defs
            && conditional_depth == 0
            && plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && plain_ident_re().is_some_and(|re| re.is_match(rhs_ident))
            && !mutated_arg_aliases.contains(lhs)
            && !mutated_arg_aliases.contains(rhs_ident)
            && !fresh_expr_for_var.contains_key(rhs_ident);
        if (is_peephole_temp(lhs) || allow_simple_alias) && rhs_ident != lhs {
            aliases.insert(lhs.to_string(), resolve_alias(rhs_ident, &aliases));
        }

        if let Some(caps) = range_re().and_then(|re| re.captures(rewritten_rhs.trim())) {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            if is_one(start, &scalar_consts) {
                identity_indices.insert(
                    lhs.to_string(),
                    normalize_expr(
                        caps.name("end").map(|m| m.as_str()).unwrap_or(""),
                        &scalar_consts,
                    ),
                );
            } else {
                identity_indices.remove(lhs);
            }
        } else if let Some(caps) = floor_re().and_then(|re| re.captures(rewritten_rhs.trim())) {
            let src = caps.name("src").map(|m| m.as_str()).unwrap_or("");
            if let Some(end) = identity_indices.get(src).cloned() {
                identity_indices.insert(lhs.to_string(), end);
            } else {
                identity_indices.remove(lhs);
            }
        } else {
            identity_indices.remove(lhs);
        }

        if let Some(len) = infer_len_from_expr(&rewritten_rhs, &vector_lens, &scalar_consts) {
            vector_lens.insert(lhs.to_string(), len);
        } else {
            vector_lens.remove(lhs);
        }

        if expr_proven_no_na(&rewritten_rhs, &no_na_vars, &scalar_consts) {
            no_na_vars.insert(lhs.to_string());
        } else {
            no_na_vars.remove(lhs);
        }

        if helper_heavy_runtime_auto_args(&rewritten_rhs) {
            helper_heavy_vars.insert(lhs.to_string());
        } else {
            helper_heavy_vars.remove(lhs);
        }

        if expr_is_fresh_allocation_like(&rewritten_rhs, fresh_user_calls) {
            fresh_expr_for_var.insert(lhs.to_string(), rewritten_rhs.clone());
        }

        if conditional_depth == 0
            && !is_peephole_temp(lhs)
            && let Some(binding) =
                extract_pure_call_binding(lhs, &rewritten_rhs, reusable_pure_user_calls)
        {
            pure_call_bindings.push(binding);
        }
        last_rhs_for_var.insert(lhs.to_string(), rewritten_rhs.clone());

        out_lines.push(format!("{indent}{lhs} <- {rewritten_rhs}"));
        if opens_conditional {
            conditional_depth += 1;
        }
    }

    out_lines
}

#[allow(dead_code)]
fn run_post_linear_peephole_passes(
    out_lines: Vec<String>,
    _cfg: &EmittedROptimizationConfig<'_>,
) -> (Vec<String>, Vec<u32>) {
    (out_lines, Vec::new())
}

/// Run the emitted-R peephole pipeline with the explicit call purity/freshness
/// context collected by earlier compiler stages.
pub(crate) fn optimize_emitted_r_with_context_and_fresh_with_options(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
) -> (String, Vec<u32>) {
    optimize_emitted_r_pipeline_impl(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        fresh_user_calls,
        preserve_all_defs,
    )
}

fn optimize_emitted_r_pipeline_impl(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
) -> (String, Vec<u32>) {
    let reusable_pure_user_calls: FxHashSet<String> = pure_user_calls
        .iter()
        .filter(|name| !fresh_user_calls.contains(*name))
        .cloned()
        .collect();
    let mut scalar_consts: FxHashMap<String, String> = FxHashMap::default();
    let mut vector_lens: FxHashMap<String, String> = FxHashMap::default();
    let mut identity_indices: FxHashMap<String, String> = FxHashMap::default();
    let mut aliases: FxHashMap<String, String> = FxHashMap::default();
    let mut no_na_vars: FxHashSet<String> = FxHashSet::default();
    let mut helper_heavy_vars: FxHashSet<String> = FxHashSet::default();
    let mut fresh_expr_for_var: FxHashMap<String, String> = FxHashMap::default();
    let mut pure_call_bindings: Vec<PureCallBinding> = Vec::new();
    let mut last_rhs_for_var: FxHashMap<String, String> = FxHashMap::default();
    let mut out_lines = Vec::new();
    let mut conditional_depth = 0usize;
    let mutated_arg_aliases = collect_mutated_arg_aliases(code);

    for line in code.lines() {
        let trimmed_line = line.trim();
        let closes_conditional = trimmed_line == "}";
        let else_boundary = trimmed_line.starts_with("} else {");
        let opens_conditional = trimmed_line.starts_with("if ") && trimmed_line.ends_with('{');
        if closes_conditional && !else_boundary && conditional_depth > 0 {
            conditional_depth -= 1;
        }
        if trimmed_line.starts_with("if ")
            && trimmed_line.ends_with(" break")
            && trimmed_line.contains("rr_truthy1(")
        {
            let rewritten = rewrite_guard_truthy_line(line, &no_na_vars, &scalar_consts);
            out_lines.push(rewritten);
            continue;
        }
        if trimmed_line.starts_with("if ")
            && trimmed_line.ends_with('{')
            && trimmed_line.contains("rr_truthy1(")
        {
            let rewritten = rewrite_if_truthy_line(line, &no_na_vars, &scalar_consts);
            out_lines.push(rewritten);
            if opens_conditional {
                conditional_depth += 1;
            }
            continue;
        }

        if line.contains("<- function") {
            clear_linear_facts(
                &mut scalar_consts,
                &mut vector_lens,
                &mut identity_indices,
                &mut aliases,
                &mut no_na_vars,
                &mut helper_heavy_vars,
            );
            fresh_expr_for_var.clear();
            last_rhs_for_var.clear();
            out_lines.push(line.to_string());
            if opens_conditional {
                conditional_depth += 1;
            }
            continue;
        }

        if is_control_flow_boundary(line) {
            let mut rewritten_line = line.to_string();
            if rewritten_line.trim().starts_with("if ")
                && rewritten_line.trim().ends_with('{')
                && rewritten_line.contains("rr_truthy1(")
            {
                rewritten_line =
                    rewrite_if_truthy_line(&rewritten_line, &no_na_vars, &scalar_consts);
            }
            if trimmed_line == "repeat {" {
                clear_loop_boundary_facts(
                    &mut identity_indices,
                    &mut aliases,
                    &mut no_na_vars,
                    &mut helper_heavy_vars,
                );
            } else {
                clear_linear_facts(
                    &mut scalar_consts,
                    &mut vector_lens,
                    &mut identity_indices,
                    &mut aliases,
                    &mut no_na_vars,
                    &mut helper_heavy_vars,
                );
            }
            fresh_expr_for_var.clear();
            last_rhs_for_var.clear();
            out_lines.push(rewritten_line);
            continue;
        }

        if let Some(base) = indexed_store_base_re()
            .and_then(|re| re.captures(line))
            .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
        {
            fresh_expr_for_var.remove(&base);
            scalar_consts.remove(&base);
            vector_lens.remove(&base);
            identity_indices.remove(&base);
            no_na_vars.remove(&base);
            helper_heavy_vars.remove(&base);
            last_rhs_for_var.remove(&base);
            invalidate_aliases_for_write(&base, &mut aliases);
            pure_call_bindings
                .retain(|binding| binding.var != base && !binding.deps.contains(&base));
            out_lines.push(line.to_string());
            continue;
        }

        let Some(caps) = assign_re().and_then(|re| re.captures(line)) else {
            let rewritten_line = rewrite_return_expr_line(line, &last_rhs_for_var);
            out_lines.push(rewritten_line);
            continue;
        };

        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();

        let rewritten_rhs = if let Some(re) = read_vec_re() {
            re.replace_all(rhs, |caps: &Captures<'_>| {
                let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                let idx = caps.name("idx").map(|m| m.as_str()).unwrap_or("");
                match (
                    identity_index_end_expr(idx, &identity_indices, &scalar_consts),
                    vector_lens.get(base),
                ) {
                    (Some(end), Some(base_len)) if &end == base_len => base.to_string(),
                    _ => caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
                }
            })
            .to_string()
        } else {
            rhs.to_string()
        };
        let rewritten_rhs = rewrite_known_length_calls(&rewritten_rhs, &vector_lens);
        let rewritten_rhs = rewrite_known_aliases(&rewritten_rhs, &aliases);
        let rewritten_rhs = rewrite_direct_vec_helper_expr(&rewritten_rhs, direct_builtin_call_map);
        let rewritten_rhs = rewrite_pure_call_reuse(&rewritten_rhs, &pure_call_bindings);

        let rewritten_rhs =
            if let Some(caps) = call_map_slice_re().and_then(|re| re.captures(&rewritten_rhs)) {
                let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
                let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
                let end = caps.name("end").map(|m| m.as_str()).unwrap_or("").trim();
                let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("").trim();
                let end = normalize_expr(end, &scalar_consts);
                if is_one(start, &scalar_consts)
                    && vector_lens
                        .get(dest)
                        .is_some_and(|dest_len| dest_len == &end)
                {
                    format!("rr_call_map_whole_auto({dest}, {rest})")
                } else {
                    rewritten_rhs
                }
            } else {
                rewritten_rhs
            };

        let rewritten_rhs = if direct_builtin_call_map {
            if let Some(caps) =
                call_map_whole_builtin_re().and_then(|re| re.captures(&rewritten_rhs))
            {
                let callee = caps.name("callee").map(|m| m.as_str()).unwrap_or("");
                let slots = caps.name("slots").map(|m| m.as_str()).unwrap_or("").trim();
                let args = caps.name("args").map(|m| m.as_str()).unwrap_or("").trim();
                if (slots == "1L" || slots == "1")
                    && !helper_heavy_runtime_auto_args_with_temps(args, &helper_heavy_vars)
                {
                    match callee {
                        "abs" | "sqrt" | "log" => format!("{callee}({args})"),
                        "pmax" | "pmin" => format!("{callee}({args})"),
                        _ => rewritten_rhs,
                    }
                } else {
                    rewritten_rhs
                }
            } else {
                rewritten_rhs
            }
        } else {
            rewritten_rhs
        };

        let rewritten_rhs = if preserve_all_defs {
            rewritten_rhs
        } else {
            rewrite_strict_ifelse_expr(&rewritten_rhs, &no_na_vars, &scalar_consts)
        };

        let rewritten_rhs =
            if let Some(caps) = assign_slice_re().and_then(|re| re.captures(&rewritten_rhs)) {
                let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
                let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
                let end = normalize_expr(
                    caps.name("end").map(|m| m.as_str()).unwrap_or("").trim(),
                    &scalar_consts,
                );
                let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("").trim();
                if is_one(start, &scalar_consts)
                    && vector_lens
                        .get(dest)
                        .is_some_and(|dest_len| dest_len == &end)
                    && infer_len_from_expr(rest, &vector_lens, &scalar_consts)
                        .is_some_and(|len| len == end)
                {
                    rest.to_string()
                } else {
                    rewritten_rhs
                }
            } else {
                rewritten_rhs
            };

        let rewritten_rhs = rewrite_known_length_calls(&rewritten_rhs, &vector_lens);
        let rewritten_rhs = rewrite_known_aliases(&rewritten_rhs, &aliases);
        let rewritten_rhs = rewrite_direct_vec_helper_expr(&rewritten_rhs, direct_builtin_call_map);
        let rewritten_rhs = rewrite_pure_call_reuse(&rewritten_rhs, &pure_call_bindings);
        let rewritten_rhs = if preserve_all_defs {
            rewritten_rhs
        } else {
            rewrite_strict_ifelse_expr(&rewritten_rhs, &no_na_vars, &scalar_consts)
        };
        let rewritten_rhs = if !last_rhs_for_var.contains_key(lhs) {
            maybe_expand_fresh_alias_rhs(&rewritten_rhs, &fresh_expr_for_var)
                .unwrap_or(rewritten_rhs)
        } else {
            rewritten_rhs
        };

        if scalar_lit_re().is_some_and(|re| re.is_match(rewritten_rhs.trim())) {
            scalar_consts.insert(lhs.to_string(), rewritten_rhs.trim().to_string());
        } else {
            scalar_consts.remove(lhs);
        }

        if let Some(base) = written_base_var(lhs) {
            fresh_expr_for_var.remove(base);
            scalar_consts.remove(base);
            vector_lens.remove(base);
            identity_indices.remove(base);
            no_na_vars.remove(base);
            helper_heavy_vars.remove(base);
            last_rhs_for_var.retain(|var, rhs| {
                var != base && !expr_idents(rhs).iter().any(|ident| ident == base)
            });
            invalidate_aliases_for_write(base, &mut aliases);
            pure_call_bindings
                .retain(|binding| binding.var != base && !binding.deps.contains(base));
        }
        invalidate_aliases_for_write(lhs, &mut aliases);
        pure_call_bindings.retain(|binding| binding.var != lhs && !binding.deps.contains(lhs));
        last_rhs_for_var
            .retain(|var, rhs| var != lhs && !expr_idents(rhs).iter().any(|ident| ident == lhs));
        let rhs_ident = rewritten_rhs.trim();
        let allow_simple_alias = !preserve_all_defs
            && conditional_depth == 0
            && plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && plain_ident_re().is_some_and(|re| re.is_match(rhs_ident))
            && !mutated_arg_aliases.contains(lhs)
            && !mutated_arg_aliases.contains(rhs_ident)
            && !fresh_expr_for_var.contains_key(rhs_ident);
        if (is_peephole_temp(lhs) || allow_simple_alias) && rhs_ident != lhs {
            aliases.insert(lhs.to_string(), resolve_alias(rhs_ident, &aliases));
        }

        if let Some(caps) = range_re().and_then(|re| re.captures(rewritten_rhs.trim())) {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            if is_one(start, &scalar_consts) {
                identity_indices.insert(
                    lhs.to_string(),
                    normalize_expr(
                        caps.name("end").map(|m| m.as_str()).unwrap_or(""),
                        &scalar_consts,
                    ),
                );
            } else {
                identity_indices.remove(lhs);
            }
        } else if let Some(caps) = floor_re().and_then(|re| re.captures(rewritten_rhs.trim())) {
            let src = caps.name("src").map(|m| m.as_str()).unwrap_or("");
            if let Some(end) = identity_indices.get(src).cloned() {
                identity_indices.insert(lhs.to_string(), end);
            } else {
                identity_indices.remove(lhs);
            }
        } else {
            identity_indices.remove(lhs);
        }

        if let Some(len) = infer_len_from_expr(&rewritten_rhs, &vector_lens, &scalar_consts) {
            vector_lens.insert(lhs.to_string(), len);
        } else {
            vector_lens.remove(lhs);
        }

        if expr_proven_no_na(&rewritten_rhs, &no_na_vars, &scalar_consts) {
            no_na_vars.insert(lhs.to_string());
        } else {
            no_na_vars.remove(lhs);
        }

        if helper_heavy_runtime_auto_args(&rewritten_rhs) {
            helper_heavy_vars.insert(lhs.to_string());
        } else {
            helper_heavy_vars.remove(lhs);
        }

        if expr_is_fresh_allocation_like(&rewritten_rhs, fresh_user_calls) {
            fresh_expr_for_var.insert(lhs.to_string(), rewritten_rhs.clone());
        }

        if conditional_depth == 0
            && !is_peephole_temp(lhs)
            && let Some(binding) =
                extract_pure_call_binding(lhs, &rewritten_rhs, &reusable_pure_user_calls)
        {
            pure_call_bindings.push(binding);
        }
        last_rhs_for_var.insert(lhs.to_string(), rewritten_rhs.clone());

        out_lines.push(format!("{indent}{lhs} <- {rewritten_rhs}"));
        if opens_conditional {
            conditional_depth += 1;
        }
    }

    let out_lines = collapse_common_if_else_tail_assignments(out_lines);
    let out_lines = rewrite_full_range_conditional_scalar_loops(out_lines);
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_inline_full_range_slice_ops(out_lines, direct_builtin_call_map)
    };
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_one_based_full_range_index_alias_reads(out_lines)
    };
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_forward_simple_alias_guards(out_lines)
    };
    let out_lines = rewrite_loop_index_alias_ii(out_lines);
    let out_lines = rewrite_safe_loop_index_write_calls(out_lines);
    let out_lines = rewrite_safe_loop_neighbor_read_calls(out_lines);
    let out_lines = rewrite_temp_uses_after_named_copy(out_lines);
    let out_lines =
        hoist_branch_local_named_scalar_assigns_used_after_branch(out_lines, pure_user_calls);
    let out_lines = inline_immediate_single_use_scalar_temps(out_lines);
    let out_lines = inline_single_use_named_scalar_index_reads_within_straight_line_region(
        out_lines,
        pure_user_calls,
    );
    let out_lines = inline_two_use_named_scalar_index_reads_within_straight_line_region(
        out_lines,
        pure_user_calls,
    );
    let out_lines = inline_immediate_single_use_named_scalar_exprs(out_lines, pure_user_calls);
    let out_lines = inline_single_use_scalar_temps_within_straight_line_region(out_lines);
    let out_lines = inline_two_use_scalar_temps_within_straight_line_region(out_lines);
    let out_lines = inline_immediate_single_use_index_temps(out_lines);
    let out_lines = hoist_repeated_vector_helper_calls_within_lines(out_lines);
    let out_lines = rewrite_forward_exact_vector_helper_reuse(out_lines);
    let out_lines = rewrite_forward_temp_aliases(out_lines);
    let out_lines = rewrite_forward_exact_pure_call_reuse(out_lines, pure_user_calls);
    let out_lines = rewrite_adjacent_duplicate_pure_call_assignments(out_lines, pure_user_calls);
    let out_lines = rewrite_adjacent_duplicate_symbol_assignments(out_lines);
    let out_lines = collapse_trivial_dot_product_wrappers(out_lines);
    let out_lines = rewrite_simple_expr_helper_calls(out_lines, pure_user_calls);
    let out_lines = rewrite_dead_zero_loop_seeds_before_for(out_lines);
    let out_lines = restore_missing_scalar_loop_increments(out_lines);
    let out_lines = restore_constant_one_guard_repeat_loop_counters(out_lines);
    let out_lines = restore_missing_scalar_loop_next_increments(out_lines);
    let out_lines =
        hoist_loop_invariant_pure_assignments_from_counted_repeat_loops(out_lines, pure_user_calls);
    let out_lines = rewrite_canonical_counted_repeat_loops_to_for(out_lines);
    let out_lines = strip_terminal_repeat_nexts(out_lines);
    let out_lines = simplify_same_var_is_na_or_not_finite_guards(out_lines);
    let out_lines = simplify_not_finite_or_zero_guard_parens(out_lines);
    let out_lines = simplify_wrapped_not_finite_parens(out_lines);
    let out_lines = rewrite_forward_exact_expr_reuse(out_lines);
    let out_lines = strip_redundant_identical_pure_rebinds(out_lines, pure_user_calls);
    let out_lines = restore_empty_match_single_bind_arms(out_lines);
    let out_lines = strip_empty_else_blocks(out_lines);
    let out_lines = strip_arg_aliases_in_trivial_return_wrappers(out_lines);
    let out_lines = collapse_trivial_passthrough_return_wrappers(out_lines);
    let out_lines = rewrite_passthrough_helper_calls(out_lines);
    let out_lines = collapse_trivial_dot_product_wrappers(out_lines);
    let out_lines = rewrite_simple_expr_helper_calls(out_lines, pure_user_calls);
    let out_lines = simplify_nested_index_vec_floor_calls(out_lines);
    let out_lines = rewrite_metric_helper_statement_calls(out_lines);
    let out_lines = rewrite_metric_helper_return_calls(out_lines);
    let out_lines = collapse_inlined_copy_vec_sequences(out_lines);
    let out_lines = rewrite_readonly_param_aliases(out_lines);
    let out_lines = strip_unused_arg_aliases(out_lines);
    let out_lines = rewrite_remaining_readonly_param_shadow_uses(out_lines);
    let out_lines = rewrite_index_only_mutated_param_shadow_aliases(out_lines);
    let out_lines = rewrite_literal_field_get_calls(out_lines);
    let out_lines = rewrite_literal_named_list_calls(out_lines);
    let out_lines = rewrite_safe_named_index_read_calls(out_lines);
    let out_lines = rewrite_safe_flat_loop_index_read_calls(out_lines);
    let out_lines = rewrite_same_len_scalar_tail_reads(out_lines);
    let out_lines = rewrite_wrap_index_scalar_access_helpers(out_lines);
    let out_lines = collapse_singleton_assign_slice_scalar_edits(out_lines);
    let out_lines = strip_unused_helper_params(out_lines);
    let out_lines = collapse_trivial_scalar_clamp_wrappers(out_lines);
    let out_lines = rewrite_simple_expr_helper_calls(out_lines, pure_user_calls);
    let out_lines = rewrite_safe_named_index_read_calls(out_lines);
    let out_lines = rewrite_safe_flat_loop_index_read_calls(out_lines);
    let out_lines = rewrite_same_len_scalar_tail_reads(out_lines);
    let out_lines = collapse_identical_if_else_tail_assignments_late(out_lines);
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_one_based_full_range_index_alias_reads(out_lines)
    };
    let out_lines = strip_dead_simple_eval_lines(out_lines);
    let out_lines = strip_noop_self_assignments(out_lines);
    let out_lines = strip_redundant_nested_temp_reassigns(out_lines);
    let out_lines = rewrite_forward_exact_pure_call_reuse(out_lines, pure_user_calls);
    let out_lines = rewrite_forward_exact_expr_reuse(out_lines);
    let out_lines = hoist_repeated_vector_helper_calls_within_lines(out_lines);
    let out_lines = rewrite_forward_exact_vector_helper_reuse(out_lines);
    let out_lines = rewrite_forward_temp_aliases(out_lines);
    let out_lines = strip_redundant_identical_pure_rebinds(out_lines, pure_user_calls);
    let out_lines = strip_noop_self_assignments(out_lines);
    let out_lines = strip_redundant_nested_temp_reassigns(out_lines);
    let out_lines = rewrite_forward_exact_pure_call_reuse(out_lines, pure_user_calls);
    let out_lines = rewrite_forward_exact_expr_reuse(out_lines);
    let out_lines = strip_redundant_identical_pure_rebinds(out_lines, pure_user_calls);
    let out_lines = rewrite_shifted_square_scalar_reuse(out_lines);
    let out_lines = strip_dead_simple_eval_lines(out_lines);
    let out_lines = strip_noop_self_assignments(out_lines);
    let out_lines = strip_redundant_nested_temp_reassigns(out_lines);
    let out_lines = strip_redundant_tail_assign_slice_return(out_lines);
    let (out_lines, line_map) = strip_dead_temps(out_lines, pure_user_calls);
    let out_lines =
        hoist_branch_local_named_scalar_assigns_used_after_branch(out_lines, pure_user_calls);
    let out_lines = inline_immediate_single_use_scalar_temps(out_lines);
    let out_lines = inline_single_use_named_scalar_index_reads_within_straight_line_region(
        out_lines,
        pure_user_calls,
    );
    let out_lines = inline_two_use_named_scalar_index_reads_within_straight_line_region(
        out_lines,
        pure_user_calls,
    );
    let out_lines = inline_immediate_single_use_named_scalar_exprs(out_lines, pure_user_calls);
    let out_lines = inline_single_use_scalar_temps_within_straight_line_region(out_lines);
    let out_lines = inline_two_use_scalar_temps_within_straight_line_region(out_lines);
    let out_lines = inline_immediate_single_use_index_temps(out_lines);
    let out_lines = rewrite_adjacent_duplicate_pure_call_assignments(out_lines, pure_user_calls);
    let out_lines = rewrite_adjacent_duplicate_symbol_assignments(out_lines);
    let out_lines = rewrite_dead_zero_loop_seeds_before_for(out_lines);
    let out_lines = strip_terminal_repeat_nexts(out_lines);
    let out_lines = simplify_same_var_is_na_or_not_finite_guards(out_lines);
    let out_lines = simplify_not_finite_or_zero_guard_parens(out_lines);
    let out_lines = simplify_wrapped_not_finite_parens(out_lines);
    let out_lines = run_exact_expr_cleanup_rounds(out_lines, 4);
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_one_based_full_range_index_alias_reads(out_lines)
    };
    let out_lines = rewrite_shifted_square_scalar_reuse(out_lines);
    let out_lines = strip_arg_aliases_in_trivial_return_wrappers(out_lines);
    let out_lines = collapse_trivial_passthrough_return_wrappers(out_lines);
    let out_lines = rewrite_passthrough_helper_calls(out_lines);
    let out_lines = collapse_trivial_dot_product_wrappers(out_lines);
    let out_lines = rewrite_simple_expr_helper_calls(out_lines, pure_user_calls);
    let out_lines = simplify_nested_index_vec_floor_calls(out_lines);
    let out_lines = rewrite_metric_helper_statement_calls(out_lines);
    let out_lines = rewrite_metric_helper_return_calls(out_lines);
    let out_lines = collapse_inlined_copy_vec_sequences(out_lines);
    let out_lines = rewrite_readonly_param_aliases(out_lines);
    let out_lines = strip_unused_arg_aliases(out_lines);
    let out_lines = rewrite_remaining_readonly_param_shadow_uses(out_lines);
    let out_lines = collapse_singleton_assign_slice_scalar_edits(out_lines);
    let out_lines = collapse_trivial_scalar_clamp_wrappers(out_lines);
    let out_lines = rewrite_simple_expr_helper_calls(out_lines, pure_user_calls);
    let out_lines = collapse_identical_if_else_tail_assignments_late(out_lines);
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_inline_full_range_slice_ops(out_lines, direct_builtin_call_map)
    };
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        collapse_contextual_full_range_gather_replays(out_lines)
    };
    let out_lines = rewrite_temp_uses_after_named_copy(out_lines);
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_one_based_full_range_index_alias_reads(out_lines)
    };
    let out_lines = restore_empty_match_single_bind_arms(out_lines);
    let out_lines = strip_empty_else_blocks(out_lines);
    let out_lines = strip_noop_self_assignments(out_lines);
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        strip_unreachable_sym_helpers(out_lines)
    };
    let out_lines = strip_redundant_tail_assign_slice_return(out_lines);
    let (out_lines, final_compact_map) = strip_dead_temps(out_lines, pure_user_calls);
    let out_lines = strip_unused_helper_params(out_lines);
    let line_map = compose_line_maps(&line_map, &final_compact_map);
    let mut out = out_lines.join("\n");
    if code.ends_with('\n') {
        out.push('\n');
    }
    (out, line_map)
}

fn rewrite_loop_index_reads_to_whole_expr(expr: &str, idx_var: &str) -> String {
    let pattern = format!(r"(?P<base>{})\[(?P<idx>[^\]]+)\]", IDENT_PATTERN);
    let Some(index_re) = compile_regex(pattern) else {
        return expr.to_string();
    };
    index_re
        .replace_all(expr, |caps: &Captures<'_>| {
            let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
            let idx = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
            if idx == idx_var {
                base.to_string()
            } else {
                caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
            }
        })
        .to_string()
}

fn parse_break_guard(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("if (!(")
        .and_then(|s| s.strip_suffix(")) break"))?;
    let (lhs, rhs) = inner.split_once("<=")?;
    Some((lhs.trim().to_string(), rhs.trim().to_string()))
}

fn parse_indexed_store_assign(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let (lhs, rhs) = trimmed.split_once("<-")?;
    let lhs = lhs.trim();
    let rhs = rhs.trim().to_string();
    let (base, idx) = lhs.split_once('[')?;
    let idx = idx.trim_end_matches(']').trim();
    Some((base.trim().to_string(), idx.to_string(), rhs))
}

fn rewrite_full_range_conditional_scalar_loops(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = lines[i].trim();
        let Some(init_caps) = assign_re().and_then(|re| re.captures(line)) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        let idx_var = init_caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let init_rhs = init_caps
            .name("rhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if !idx_var.starts_with("i_") || !literal_one_re().is_some_and(|re| re.is_match(init_rhs)) {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }

        let Some(repeat_line) = lines.get(i + 1).map(|s| s.trim()) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        if repeat_line != "repeat {" {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }
        let mut cursor = i + 2;
        while matches!(lines.get(cursor).map(|s| s.trim()), Some(line) if line.is_empty() || line.starts_with("rr_mark("))
        {
            cursor += 1;
        }
        let Some((guard_idx, end_expr)) = lines.get(cursor).and_then(|s| parse_break_guard(s))
        else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        if guard_idx != idx_var {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }

        cursor += 1;
        while matches!(lines.get(cursor).map(|s| s.trim()), Some(line) if line.is_empty() || line.starts_with("rr_mark("))
        {
            cursor += 1;
        }
        let Some(if_line) = lines.get(cursor).map(|s| s.trim()) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        let Some(cond_inner) = if_line
            .strip_prefix("if ((")
            .and_then(|s| s.strip_suffix(")) {"))
            .or_else(|| {
                if_line
                    .strip_prefix("if (")
                    .and_then(|s| s.strip_suffix(") {"))
            })
        else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        let mut then_cursor = cursor + 1;
        while matches!(lines.get(then_cursor).map(|s| s.trim()), Some(line) if line.is_empty() || line.starts_with("rr_mark("))
        {
            then_cursor += 1;
        }
        let then_line = lines.get(then_cursor).map(|s| s.trim()).unwrap_or("");
        let else_header = lines.get(then_cursor + 1).map(|s| s.trim()).unwrap_or("");
        let mut else_cursor = then_cursor + 2;
        while matches!(lines.get(else_cursor).map(|s| s.trim()), Some(line) if line.is_empty() || line.starts_with("rr_mark("))
        {
            else_cursor += 1;
        }
        let else_line = lines.get(else_cursor).map(|s| s.trim()).unwrap_or("");
        let close_line = lines.get(else_cursor + 1).map(|s| s.trim()).unwrap_or("");
        let incr_line = lines.get(else_cursor + 2).map(|s| s.trim()).unwrap_or("");
        let next_line = lines.get(else_cursor + 3).map(|s| s.trim()).unwrap_or("");
        let end_repeat = lines.get(else_cursor + 4).map(|s| s.trim()).unwrap_or("");
        if else_header != "} else {"
            || close_line != "}"
            || next_line != "next"
            || end_repeat != "}"
        {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }
        let Some((dest_base, then_idx, then_rhs)) = parse_indexed_store_assign(then_line) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        let Some((else_base, else_idx, else_rhs)) = parse_indexed_store_assign(else_line) else {
            out.push(lines[i].clone());
            i += 1;
            continue;
        };
        if dest_base != else_base || then_idx != idx_var || else_idx != idx_var {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }
        let expected_incr = format!("{idx_var} <- ({idx_var} + 1L)");
        if incr_line != expected_incr {
            out.push(lines[i].clone());
            i += 1;
            continue;
        }

        let cond_whole = rewrite_loop_index_reads_to_whole_expr(cond_inner.trim(), &idx_var);
        let then_whole = rewrite_loop_index_reads_to_whole_expr(&then_rhs, &idx_var);
        let else_whole = rewrite_loop_index_reads_to_whole_expr(&else_rhs, &idx_var);
        let indent = &lines[i][..lines[i].len() - lines[i].trim_start().len()];
        out.push(format!(
            "{indent}{dest_base} <- rr_assign_slice({dest_base}, 1L, {end_expr}, rr_ifelse_strict(({cond_whole}), {then_whole}, {else_whole}))"
        ));
        i = else_cursor + 5;
    }
    out
}

fn rewrite_inline_full_range_reads(expr: &str, start: &str, end: &str) -> String {
    let mut out = expr.to_string();
    let start_pat = regex::escape(start.trim());
    let end_pat = regex::escape(end.trim());
    let direct_pat = format!(
        r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*:\s*{}\)",
        IDENT_PATTERN, start_pat, end_pat
    );
    if let Some(re) = compile_regex(direct_pat) {
        out = re
            .replace_all(&out, |caps: &Captures<'_>| {
                caps.name("base")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .to_string();
    }
    let floor_pat = format!(
        r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*rr_index_vec_floor\(\s*{}\s*:\s*{}\s*\)\)",
        IDENT_PATTERN, start_pat, end_pat
    );
    if let Some(re) = compile_regex(floor_pat) {
        out = re
            .replace_all(&out, |caps: &Captures<'_>| {
                caps.name("base")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .to_string();
    }
    out
}

fn compact_expr(expr: &str) -> String {
    expr.chars().filter(|c| !c.is_whitespace()).collect()
}

fn expr_is_full_range_index_alias(expr: &str, start: &str, end: &str) -> bool {
    let expr = compact_expr(expr);
    let mut starts = vec![start.trim().to_string()];
    for one in ["1L", "1", "1.0", "1.0L"] {
        if !starts.iter().any(|s| s == one) {
            starts.push(one.to_string());
        }
    }
    starts.into_iter().any(|start_expr| {
        let direct = compact_expr(&format!("{}:{}", start_expr, end.trim()));
        let floor = compact_expr(&format!(
            "rr_index_vec_floor({}:{})",
            start_expr,
            end.trim()
        ));
        expr == direct || expr == floor
    })
}

fn expr_is_one_based_full_range_alias(expr: &str) -> bool {
    let expr = compact_expr(expr);
    let starts = ["1L", "1", "1.0", "1.0L"];
    starts.iter().any(|start_expr| {
        expr.starts_with(&format!("{}:", start_expr))
            || expr.starts_with(&format!("rr_index_vec_floor({}:", start_expr))
    })
}

fn rewrite_inline_full_range_reads_with_aliases(
    expr: &str,
    start: &str,
    end: &str,
    whole_range_index_aliases: &FxHashMap<String, String>,
) -> String {
    let mut out = rewrite_inline_full_range_reads(expr, start, end);
    for (alias, alias_rhs) in whole_range_index_aliases {
        if !expr_is_full_range_index_alias(alias_rhs, start, end) {
            continue;
        }
        let pattern = format!(
            r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*\)",
            IDENT_PATTERN,
            regex::escape(alias),
        );
        let Some(re) = compile_regex(pattern) else {
            continue;
        };
        out = re
            .replace_all(&out, |caps: &Captures<'_>| {
                caps.name("base")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .to_string()
            })
            .to_string();
    }
    out
}

fn start_expr_is_one_in_context(lines: &[String], idx: usize, start: &str) -> bool {
    if literal_one_re().is_some_and(|re| re.is_match(start.trim())) {
        return true;
    }
    for prev_idx in (0..idx).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
            continue;
        }
        if lines[prev_idx].contains("<- function") || is_control_flow_boundary(trimmed) {
            break;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs == start.trim() {
            return literal_one_re().is_some_and(|re| re.is_match(rhs));
        }
    }
    false
}

fn end_expr_is_singleton(end: &str) -> bool {
    literal_one_re().is_some_and(|re| re.is_match(end.trim()))
}

fn strip_redundant_outer_parens(expr: &str) -> &str {
    let mut expr = expr.trim();
    loop {
        if !(expr.starts_with('(') && expr.ends_with(')')) {
            break;
        }
        let mut depth = 0i32;
        let mut wraps = true;
        for (idx, ch) in expr.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && idx + ch.len_utf8() < expr.len() {
                        wraps = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !wraps {
            break;
        }
        expr = expr[1..expr.len() - 1].trim();
    }
    expr
}

fn expr_has_top_level_arith(expr: &str) -> bool {
    let expr = strip_redundant_outer_parens(expr);
    let bytes = expr.as_bytes();
    let mut depth = 0i32;
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] as char {
            '(' => depth += 1,
            ')' => depth -= 1,
            '+' | '-' | '*' | '/' if depth == 0 => return true,
            '%' if depth == 0 && i + 1 < bytes.len() && bytes[i + 1] as char == '%' => {
                return true;
            }
            '"' | '\'' => {
                let quote = bytes[i];
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == quote {
                        break;
                    }
                    i += 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

fn end_expr_can_cover_full_range(lines: &[String], idx: usize, end: &str) -> bool {
    let end = end.trim();
    if end_expr_is_singleton(end) {
        return false;
    }
    if !expr_has_top_level_arith(end) {
        return true;
    }
    if end.starts_with("length(") {
        return !expr_has_top_level_arith(
            end.strip_prefix("length(")
                .and_then(|s| s.strip_suffix(')'))
                .unwrap_or(end),
        );
    }
    for prev_idx in (0..idx).rev() {
        let trimmed = lines[prev_idx].trim();
        if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
            continue;
        }
        if lines[prev_idx].contains("<- function") || is_control_flow_boundary(trimmed) {
            break;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs == end {
            return !expr_has_top_level_arith(rhs);
        }
    }
    false
}

fn rewrite_inline_full_range_slice_ops(
    lines: Vec<String>,
    direct_builtin_call_map: bool,
) -> Vec<String> {
    let mut out = lines;
    let mut whole_range_index_aliases: FxHashMap<String, String> = FxHashMap::default();
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();

        if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) {
            let dest = slice_caps
                .name("dest")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let start = slice_caps
                .name("start")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let end = slice_caps
                .name("end")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let rest = slice_caps
                .name("rest")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if lhs == dest
                && start_expr_is_one_in_context(&out, idx, start)
                && end_expr_can_cover_full_range(&out, idx, end)
            {
                let rewritten_rest = rewrite_inline_full_range_reads_with_aliases(
                    rest,
                    start,
                    end,
                    &whole_range_index_aliases,
                );
                let rewritten_rest = rewritten_rest.replace("rr_ifelse_strict(", "ifelse(");
                out[idx] = format!("{lhs} <- {rewritten_rest}");
            } else {
                whole_range_index_aliases.remove(lhs);
                continue;
            }
        }

        if let Some(call_caps) = call_map_slice_re().and_then(|re| re.captures(rhs)) {
            let dest = call_caps
                .name("dest")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let start = call_caps
                .name("start")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let end = call_caps
                .name("end")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let rest = call_caps
                .name("rest")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if lhs == dest
                && start_expr_is_one_in_context(&out, idx, start)
                && end_expr_can_cover_full_range(&out, idx, end)
            {
                let Some(parts) = split_top_level_args(rest) else {
                    continue;
                };
                if parts.len() < 4 {
                    continue;
                }
                let callee = parts[0].trim().trim_matches('"');
                let slots = parts[2].trim();
                let args: Vec<String> = parts[3..]
                    .iter()
                    .map(|arg| {
                        rewrite_inline_full_range_reads_with_aliases(
                            arg,
                            start,
                            end,
                            &whole_range_index_aliases,
                        )
                    })
                    .collect();
                if direct_builtin_call_map && (slots == "c(1L)" || slots == "c(1)") {
                    match (callee, args.as_slice()) {
                        ("pmax", [a, b]) | ("pmin", [a, b]) => {
                            out[idx] = format!("{lhs} <- {callee}({a}, {b})");
                        }
                        ("abs", [a]) | ("sqrt", [a]) | ("log", [a]) => {
                            out[idx] = format!("{lhs} <- {callee}({a})");
                        }
                        _ => {}
                    }
                }
                if out[idx].trim() == trimmed {
                    let joined = parts
                        .iter()
                        .take(3)
                        .cloned()
                        .chain(args.into_iter())
                        .collect::<Vec<_>>()
                        .join(", ");
                    out[idx] = format!("{lhs} <- rr_call_map_whole_auto({dest}, {joined})");
                }
            } else {
                whole_range_index_aliases.remove(lhs);
                continue;
            }
        }

        let current_trimmed = out[idx].trim().to_string();
        if let Some(current_caps) = assign_re().and_then(|re| re.captures(&current_trimmed)) {
            let current_lhs = current_caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let current_rhs = current_caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if current_lhs.starts_with('.')
                && (current_rhs.contains(':') || current_rhs.starts_with("rr_index_vec_floor("))
            {
                whole_range_index_aliases.insert(current_lhs.to_string(), current_rhs.to_string());
            } else {
                whole_range_index_aliases.remove(current_lhs);
            }
        }
    }
    out
}

fn collapse_contextual_full_range_gather_replays(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(re) = compile_regex(format!(
        r"^(?P<indent>\s*)(?P<lhs>{id}) <- rr_assign_slice\((?P<dest>{id}),\s*(?P<start>[^,]+),\s*(?P<end>[^,]+),\s*rr_gather\((?P<base>{id}),\s*rr_index_vec_floor\(rr_index1_read_vec(?:_floor)?\((?P<inner_base>{id}),\s*rr_index_vec_floor\((?P<inner_start>[^:]+):(?P<inner_end>[^\)]+)\)\)\)\)\)$",
        id = IDENT_PATTERN
    )) else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = re.captures(&trimmed) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
        let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
        let end = caps.name("end").map(|m| m.as_str()).unwrap_or("").trim();
        let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
        let inner_base = caps
            .name("inner_base")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let inner_start = caps
            .name("inner_start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let inner_end = caps
            .name("inner_end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if lhs != dest
            || base != inner_base
            || compact_expr(start) != compact_expr(inner_start)
            || compact_expr(end) != compact_expr(inner_end)
            || !start_expr_is_one_in_context(&out, idx, start)
            || !end_expr_can_cover_full_range(&out, idx, end)
        {
            continue;
        }
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        out[idx] = format!("{indent}{lhs} <- rr_gather({base}, rr_index_vec_floor({base}))");
    }
    out
}

fn rewrite_one_based_full_range_index_alias_reads(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut whole_range_index_aliases: FxHashMap<String, String> = FxHashMap::default();
    for line in lines {
        let trimmed = line.trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            if is_control_flow_boundary(&trimmed) || line.contains("<- function") {
                whole_range_index_aliases.clear();
            }
            out.push(line);
            continue;
        };
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let mut rewritten_rhs = rhs.to_string();
        if let Some(re) = compile_regex(format!(
            r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>{}|rr_index_vec_floor\([^\)]*\)|[^,\)]*:[^\)]*)\)",
            IDENT_PATTERN, IDENT_PATTERN
        )) {
            rewritten_rhs = re
                .replace_all(&rewritten_rhs, |caps: &Captures<'_>| {
                    let idx_expr = caps.name("idx").map(|m| m.as_str()).unwrap_or("").trim();
                    if expr_is_one_based_full_range_alias(idx_expr) {
                        caps.name("base")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .to_string()
                    } else {
                        caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string()
                    }
                })
                .to_string();
        }
        for (alias, alias_rhs) in &whole_range_index_aliases {
            if !expr_is_one_based_full_range_alias(alias_rhs) {
                continue;
            }
            let pattern = format!(
                r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*\)",
                IDENT_PATTERN,
                regex::escape(alias),
            );
            let Some(re) = compile_regex(pattern) else {
                continue;
            };
            rewritten_rhs = re
                .replace_all(&rewritten_rhs, |caps: &Captures<'_>| {
                    caps.name("base")
                        .map(|m| m.as_str())
                        .unwrap_or("")
                        .to_string()
                })
                .to_string();
        }
        if rewritten_rhs != rhs {
            rewritten_rhs = rewritten_rhs.replace("rr_ifelse_strict(", "ifelse(");
        }
        if lhs.starts_with('.') && expr_is_one_based_full_range_alias(&rewritten_rhs) {
            whole_range_index_aliases.insert(lhs.to_string(), rewritten_rhs.clone());
        } else {
            whole_range_index_aliases.remove(lhs);
        }
        out.push(format!("{indent}{lhs} <- {rewritten_rhs}"));
    }
    out
}

fn compose_line_maps(first: &[u32], second: &[u32]) -> Vec<u32> {
    first
        .iter()
        .map(|line| {
            if *line == 0 {
                return 0;
            }
            let idx = (*line as usize).saturating_sub(1);
            second.get(idx).copied().unwrap_or(*line)
        })
        .collect()
}

fn extract_pure_call_binding(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> Option<PureCallBinding> {
    let rhs = rhs.trim();
    let (callee, rest) = rhs.split_once('(')?;
    if !pure_user_calls.contains(callee.trim()) || !rhs.ends_with(')') {
        return None;
    }
    let args = rest.strip_suffix(')')?.trim();
    let deps = expr_idents(args)
        .into_iter()
        .filter(|ident| ident != callee.trim())
        .collect();
    Some(PureCallBinding {
        expr: rhs.to_string(),
        var: lhs.to_string(),
        deps,
    })
}

fn rewrite_pure_call_reuse(expr: &str, bindings: &[PureCallBinding]) -> String {
    let mut out = expr.to_string();
    for binding in bindings {
        if out.contains(&binding.expr) {
            out = out.replace(&binding.expr, &binding.var);
        }
    }
    out
}

fn rewrite_return_expr_line(line: &str, last_rhs_for_var: &FxHashMap<String, String>) -> String {
    let trimmed = line.trim();
    let Some(inner) = trimmed
        .strip_prefix("return(")
        .and_then(|s| s.strip_suffix(')'))
    else {
        return line.to_string();
    };
    let inner = inner.trim();
    let Some((var, _rhs)) = last_rhs_for_var
        .iter()
        .find(|(_, rhs)| rhs.as_str() == inner)
    else {
        return line.to_string();
    };
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    format!("{indent}return({var})")
}

fn written_base_var(lhs: &str) -> Option<&str> {
    let lhs = lhs.trim();
    if let Some((base, _)) = lhs.split_once('[') {
        let base = base.trim();
        if plain_ident_re().is_some_and(|re| re.is_match(base)) {
            return Some(base);
        }
        return None;
    }
    if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
        Some(lhs)
    } else {
        None
    }
}

fn maybe_expand_fresh_alias_rhs(
    rhs: &str,
    fresh_expr_for_var: &FxHashMap<String, String>,
) -> Option<String> {
    let ident = rhs.trim();
    if !plain_ident_re().is_some_and(|re| re.is_match(ident)) {
        return None;
    }
    fresh_expr_for_var.get(ident).cloned()
}

fn strip_empty_else_blocks(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut i = 0usize;
    while i < lines.len() {
        let line = &lines[i];
        if line.trim() == "} else {" {
            let mut close_idx = i + 1;
            while close_idx < lines.len() && lines[close_idx].trim().is_empty() {
                close_idx += 1;
            }
            if close_idx < lines.len() && lines[close_idx].trim() == "}" {
                let indent_len = line.len() - line.trim_start().len();
                let indent = &line[..indent_len];
                out.push(format!("{indent}}}"));
                i = close_idx + 1;
                continue;
            }
        }
        out.push(line.clone());
        i += 1;
    }
    out
}

fn strip_dead_simple_eval_lines(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| {
            !is_dead_plain_ident_eval_line(line) && !is_dead_parenthesized_eval_line(line)
        })
        .collect()
}

fn strip_noop_self_assignments(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| {
            let trimmed = line.trim();
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                return true;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            lhs != rhs
        })
        .collect()
}

fn strip_unused_arg_aliases(lines: Vec<String>) -> Vec<String> {
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
        let mut idx = body_start;
        while idx < fn_end {
            let trimmed = out[idx].trim().to_string();
            if trimmed.is_empty() || trimmed == "{" {
                idx += 1;
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !lhs.starts_with(".arg_") || !plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                break;
            }

            let mut used = false;
            for later_line in out.iter().take(fn_end).skip(idx + 1) {
                let later_trimmed = later_line.trim();
                if later_trimmed.is_empty() {
                    continue;
                }
                if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                    used = true;
                    break;
                }
            }
            if !used {
                out[idx].clear();
            }
            idx += 1;
        }
        fn_start = fn_end + 1;
    }
    out
}

fn strip_arg_aliases_in_trivial_return_wrappers(lines: Vec<String>) -> Vec<String> {
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
        let Some(return_idx) = previous_non_empty_line(&out, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let return_line = out[return_idx].trim().to_string();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            fn_start = fn_end + 1;
            continue;
        };

        let mut aliases = FxHashMap::default();
        let mut trivial = true;
        for line in out.iter().take(return_idx).skip(body_start) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                trivial = false;
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs.starts_with(".arg_") && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                aliases.insert(lhs.to_string(), rhs.to_string());
            } else {
                trivial = false;
                break;
            }
        }
        if !trivial || aliases.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }

        let rewritten = normalize_expr_with_aliases(inner, &aliases);
        if rewritten != inner {
            let indent_len = out[return_idx].len() - out[return_idx].trim_start().len();
            let indent = &out[return_idx][..indent_len];
            out[return_idx] = format!("{indent}return({rewritten})");
            for line in out.iter_mut().take(return_idx).skip(body_start) {
                if line.trim_start().starts_with(".arg_") {
                    line.clear();
                }
            }
        }

        fn_start = fn_end + 1;
    }
    out
}

fn collapse_trivial_passthrough_return_wrappers(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let no_fresh_user_calls = FxHashSet::default();
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
        let Some(return_idx) = previous_non_empty_line(&out, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let return_line = out[return_idx].trim().to_string();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            fn_start = fn_end + 1;
            continue;
        };

        let mut last_assign_to_return: Option<(usize, String)> = None;
        let mut trivial = true;
        for (idx, line) in out.iter().enumerate().take(return_idx).skip(body_start) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed == "{" || trimmed == "}" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                trivial = false;
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs == inner && plain_ident_re().is_some_and(|re| re.is_match(rhs)) {
                last_assign_to_return = Some((idx, rhs.to_string()));
            } else if !expr_is_trivial_passthrough_setup_rhs(rhs, &no_fresh_user_calls) {
                trivial = false;
                break;
            }
        }
        let Some((assign_idx, passthrough_ident)) = last_assign_to_return else {
            fn_start = fn_end + 1;
            continue;
        };
        if !trivial {
            fn_start = fn_end + 1;
            continue;
        }

        let indent_len = out[return_idx].len() - out[return_idx].trim_start().len();
        let indent = &out[return_idx][..indent_len];
        out[return_idx] = format!("{indent}return({passthrough_ident})");
        for line in out.iter_mut().take(return_idx).skip(body_start) {
            let trimmed = line.trim();
            if trimmed == "{" || trimmed == "}" {
                continue;
            }
            line.clear();
        }
        out[assign_idx].clear();
        fn_start = fn_end + 1;
    }
    out
}

fn collapse_trivial_scalar_clamp_wrappers(lines: Vec<String>) -> Vec<String> {
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
        let Some((_, _params)) = parse_function_header(&out[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body: Vec<String> = out
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "{" && line != "}")
            .collect();
        if body.len() != 6 {
            fn_start = fn_end + 1;
            continue;
        }
        let Some((tmp, init_expr)) = body[0]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some((assign_lo_lhs, lo_expr)) = body[2]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some((assign_hi_lhs, hi_expr)) = body[4]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        if assign_lo_lhs != tmp || assign_hi_lhs != tmp || body[5] != format!("return({tmp})") {
            fn_start = fn_end + 1;
            continue;
        }
        let first_guard_ok = body[1] == format!("if (({init_expr} < {lo_expr})) {{")
            || body[1] == format!("if (({tmp} < {lo_expr})) {{");
        let second_guard_ok = body[3] == format!("if (({tmp} > {hi_expr})) {{");
        if !first_guard_ok || !second_guard_ok {
            fn_start = fn_end + 1;
            continue;
        }

        let return_idx = previous_non_empty_line(&out, fn_end).unwrap_or(fn_end);
        let indent_len = out[return_idx].len() - out[return_idx].trim_start().len();
        let indent = out[return_idx][..indent_len].to_string();
        let open_idx = fn_start + 1;
        if open_idx < out.len() {
            out[open_idx] = "{".to_string();
        }
        if open_idx + 1 < out.len() {
            out[open_idx + 1] =
                format!("{indent}return(pmin(pmax({init_expr}, {lo_expr}), {hi_expr}))");
        }
        for line in out.iter_mut().take(fn_end).skip(open_idx + 2) {
            line.clear();
        }
        fn_start = fn_end + 1;
    }
    out
}

fn collapse_singleton_assign_slice_scalar_edits(lines: Vec<String>) -> Vec<String> {
    fn scalar_rhs_from_singleton_rest(rest: &str) -> Option<String> {
        let trimmed = rest.trim();
        if let Some(inner) = trimmed
            .strip_prefix("rep.int(")
            .and_then(|s| s.strip_suffix(')'))
        {
            let args = split_top_level_args(inner)?;
            if args.len() == 2 && literal_one_re().is_some_and(|re| re.is_match(args[1].trim())) {
                return Some(args[0].trim().to_string());
            }
        }
        (scalar_lit_re().is_some_and(|re| re.is_match(trimmed))
            || plain_ident_re().is_some_and(|re| re.is_match(trimmed)))
        .then_some(trimmed.to_string())
    }

    let mut out = lines;
    for line in &mut out {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
            continue;
        };
        let dest = slice_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let start = slice_caps
            .name("start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = slice_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let rest = slice_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if lhs != dest || start != end {
            continue;
        }
        let Some(scalar_rhs) = scalar_rhs_from_singleton_rest(rest) else {
            continue;
        };
        *line = format!("{indent}{lhs} <- replace({dest}, {start}, {scalar_rhs})");
    }
    out
}

fn collapse_trivial_dot_product_wrappers(lines: Vec<String>) -> Vec<String> {
    fn is_zero_literal(expr: &str) -> bool {
        matches!(expr.trim(), "0" | "0L" | "0.0")
    }

    fn is_one_literal(expr: &str) -> bool {
        matches!(expr.trim(), "1" | "1L" | "1.0")
    }

    fn parse_accumulate_product_line(line: &str, acc: &str) -> Option<(String, String, String)> {
        let pattern = format!(
            r"^(?P<lhs>{}) <- \({} \+ \((?P<a>{})\[(?P<idx_a>{})\] \* (?P<b>{})\[(?P<idx_b>{})\]\)\)$",
            IDENT_PATTERN,
            regex::escape(acc),
            IDENT_PATTERN,
            IDENT_PATTERN,
            IDENT_PATTERN,
            IDENT_PATTERN
        );
        let caps = compile_regex(pattern)?.captures(line.trim())?;
        let lhs = caps.name("lhs")?.as_str().trim();
        let lhs_vec = caps.name("a")?.as_str().trim();
        let rhs_vec = caps.name("b")?.as_str().trim();
        let idx_a = caps.name("idx_a")?.as_str().trim();
        let idx_b = caps.name("idx_b")?.as_str().trim();
        if lhs != acc || idx_a != idx_b {
            return None;
        }
        Some((lhs_vec.to_string(), rhs_vec.to_string(), idx_a.to_string()))
    }

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
        if params.len() != 3 {
            fn_start = fn_end + 1;
            continue;
        }

        let body: Vec<String> = out
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && line != "{" && line != "}")
            .collect();
        if body.len() < 7 {
            fn_start = fn_end + 1;
            continue;
        }

        let mut aliases: FxHashMap<String, String> = params
            .iter()
            .cloned()
            .map(|param| (param.clone(), param))
            .collect();
        let mut idx = 0usize;
        while idx < body.len() {
            let Some((lhs, rhs)) = body[idx]
                .split_once(" <- ")
                .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
            else {
                break;
            };
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                break;
            }
            if params.iter().any(|param| param == rhs) {
                aliases.insert(lhs.to_string(), rhs.to_string());
                idx += 1;
                continue;
            }
            break;
        }

        if idx + 6 >= body.len() {
            fn_start = fn_end + 1;
            continue;
        }
        let Some((acc, init_expr)) = body[idx]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some((iter_var, iter_init)) = body[idx + 1]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
        else {
            fn_start = fn_end + 1;
            continue;
        };
        if !plain_ident_re().is_some_and(|re| re.is_match(acc))
            || !plain_ident_re().is_some_and(|re| re.is_match(iter_var))
            || !is_zero_literal(init_expr)
            || !is_one_literal(iter_init)
            || body[idx + 2] != "repeat {"
        {
            fn_start = fn_end + 1;
            continue;
        }

        let guard_line = format!("if (!({iter_var} <= {})) break", params[2]);
        let guard_line_with_alias = aliases.iter().find_map(|(alias, base)| {
            (base == &params[2] && alias != &params[2])
                .then(|| format!("if (!({iter_var} <= {alias})) break"))
        });
        if body[idx + 3] != guard_line
            && guard_line_with_alias.as_deref() != Some(body[idx + 3].as_str())
        {
            fn_start = fn_end + 1;
            continue;
        }

        let mut product_idx = idx + 4;
        let mut index_ref = iter_var.to_string();
        if let Some((alias_lhs, alias_rhs)) = body[product_idx]
            .split_once(" <- ")
            .map(|(lhs, rhs)| (lhs.trim(), rhs.trim()))
            && alias_rhs == iter_var
            && plain_ident_re().is_some_and(|re| re.is_match(alias_lhs))
        {
            index_ref = alias_lhs.to_string();
            product_idx += 1;
        }
        if product_idx + 2 >= body.len() {
            fn_start = fn_end + 1;
            continue;
        }
        let Some((lhs_vec, rhs_vec, vec_index_ref)) =
            parse_accumulate_product_line(&body[product_idx], acc)
        else {
            fn_start = fn_end + 1;
            continue;
        };
        let resolved_lhs = aliases
            .get(&lhs_vec)
            .map(String::as_str)
            .unwrap_or(lhs_vec.as_str());
        let resolved_rhs = aliases
            .get(&rhs_vec)
            .map(String::as_str)
            .unwrap_or(rhs_vec.as_str());
        if vec_index_ref != index_ref
            || resolved_lhs != params[0]
            || resolved_rhs != params[1]
            || !matches!(
                body[product_idx + 1].as_str(),
                line if line == format!("{iter_var} <- ({iter_var} + 1)")
                    || line == format!("{iter_var} <- ({iter_var} + 1L)")
                    || line == format!("{iter_var} <- ({iter_var} + 1.0)")
            )
            || body[product_idx + 2] != "next"
            || body.last().map(String::as_str) != Some(&format!("return({acc})"))
            || body.len() != product_idx + 4
        {
            fn_start = fn_end + 1;
            continue;
        }

        let return_idx = previous_non_empty_line(&out, fn_end).unwrap_or(fn_end);
        let indent_len = out[return_idx].len() - out[return_idx].trim_start().len();
        let indent = out[return_idx][..indent_len].to_string();
        let open_idx = fn_start + 1;
        if open_idx < out.len() {
            out[open_idx] = "{".to_string();
        }
        if open_idx + 1 < out.len() {
            out[open_idx + 1] = format!(
                "{indent}return(sum(({}[seq_len({})] * {}[seq_len({})])))",
                params[0], params[2], params[1], params[2]
            );
        }
        for line in out.iter_mut().take(fn_end).skip(open_idx + 2) {
            line.clear();
        }
        fn_start = fn_end + 1;
    }
    out
}

fn expr_is_trivial_passthrough_setup_rhs(rhs: &str, fresh_user_calls: &FxHashSet<String>) -> bool {
    let rhs = rhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || expr_is_fresh_allocation_like(rhs, fresh_user_calls)
        || rhs
            .strip_prefix("length(")
            .and_then(|s| s.strip_suffix(')'))
            .is_some_and(|inner| plain_ident_re().is_some_and(|re| re.is_match(inner.trim())))
}

fn collect_passthrough_helpers(lines: &[String]) -> FxHashMap<String, String> {
    let mut out = FxHashMap::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(lines, fn_start) else {
            break;
        };
        let header = lines[fn_start].trim();
        let Some((fn_name, _)) = header.split_once("<- function") else {
            fn_start = fn_end + 1;
            continue;
        };
        let fn_name = fn_name.trim();
        let Some(return_idx) = previous_non_empty_line(lines, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let body_lines: Vec<&str> = lines
            .iter()
            .take(return_idx)
            .skip(fn_start + 1)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty() && *s != "{" && *s != "}")
            .collect();
        if !body_lines.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }
        let return_line = lines[return_idx].trim();
        let Some(inner) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            fn_start = fn_end + 1;
            continue;
        };
        if plain_ident_re().is_some_and(|re| re.is_match(inner)) {
            out.insert(fn_name.to_string(), inner.to_string());
        }
        fn_start = fn_end + 1;
    }
    out
}

fn rewrite_passthrough_helper_calls(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let passthrough = collect_passthrough_helpers(&out);
    if passthrough.is_empty() {
        return out;
    }
    for line in &mut out {
        let trimmed = line.trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some((callee, args_str)) = rhs.split_once('(') else {
            continue;
        };
        let Some(args_inner) = args_str.strip_suffix(')') else {
            continue;
        };
        let Some(param_name) = passthrough.get(callee.trim()) else {
            continue;
        };
        let Some(args) = split_top_level_args(args_inner) else {
            continue;
        };
        if args.len() != 1 {
            continue;
        }
        if param_name.is_empty() {
            continue;
        }
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        *line = format!("{indent}{lhs} <- {}", args[0].trim());
    }
    out
}

fn parse_function_header(line: &str) -> Option<(String, Vec<String>)> {
    let trimmed = line.trim();
    let (name, rest) = trimmed.split_once("<- function(")?;
    let args_inner = rest.split_once(')')?.0.trim();
    let params = if args_inner.is_empty() {
        Vec::new()
    } else {
        split_top_level_args(args_inner)?
            .into_iter()
            .map(|arg| arg.trim().to_string())
            .collect()
    };
    Some((name.trim().to_string(), params))
}

fn substitute_helper_expr(expr: &str, bindings: &FxHashMap<String, String>) -> String {
    let mut out = expr.to_string();
    let mut keys: Vec<&String> = bindings.keys().collect();
    keys.sort_by_key(|key| std::cmp::Reverse(key.len()));
    for key in keys {
        let Some(re) = compile_regex(format!(
            r"(?P<prefix>^|[^A-Za-z0-9._]){}(?P<suffix>$|[^A-Za-z0-9._])",
            regex::escape(key)
        )) else {
            continue;
        };
        let replacement = bindings.get(key).map(String::as_str).unwrap_or("");
        out = re
            .replace_all(&out, |caps: &Captures<'_>| {
                let prefix = caps.name("prefix").map(|m| m.as_str()).unwrap_or("");
                let suffix = caps.name("suffix").map(|m| m.as_str()).unwrap_or("");
                format!("{prefix}{replacement}{suffix}")
            })
            .to_string();
    }
    out
}

fn collect_simple_expr_helpers(
    lines: &[String],
    _pure_user_calls: &FxHashSet<String>,
) -> FxHashMap<String, SimpleExprHelper> {
    let mut out = FxHashMap::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(lines, fn_start) else {
            break;
        };
        let Some((fn_name, params)) = parse_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        let Some(return_idx) = previous_non_empty_line(lines, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let return_line = lines[return_idx].trim();
        let Some(return_expr) = return_line
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            fn_start = fn_end + 1;
            continue;
        };

        let mut bindings: FxHashMap<String, String> = FxHashMap::default();
        let mut locals: FxHashSet<String> = FxHashSet::default();
        let mut simple = true;
        for line in lines.iter().take(return_idx).skip(fn_start + 1) {
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
            let expanded = substitute_helper_expr(rhs, &bindings);
            bindings.insert(lhs.to_string(), expanded);
            locals.insert(lhs.to_string());
        }
        if !simple {
            fn_start = fn_end + 1;
            continue;
        }

        let expanded_return = substitute_helper_expr(return_expr, &bindings);
        if expanded_return.contains(&format!("{fn_name}(")) {
            fn_start = fn_end + 1;
            continue;
        }
        if expr_idents(&expanded_return)
            .iter()
            .any(|ident| locals.contains(ident) && !params.iter().any(|param| param == ident))
        {
            fn_start = fn_end + 1;
            continue;
        }
        out.insert(
            fn_name,
            SimpleExprHelper {
                params,
                expr: expanded_return,
            },
        );
        fn_start = fn_end + 1;
    }
    out
}

fn rewrite_simple_expr_helper_calls_filtered(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    allowed_helpers: Option<&FxHashSet<String>>,
) -> Vec<String> {
    let helpers = collect_simple_expr_helpers(&lines, pure_user_calls);
    if helpers.is_empty() {
        return lines;
    }
    let mut out = lines;
    for line in &mut out {
        if line.contains("<- function") {
            continue;
        }
        let mut rewritten = line.clone();
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
                let expanded = substitute_helper_expr(&helper.expr, &subst);
                next.push('(');
                next.push_str(&expanded);
                next.push(')');
                idx = call_end + 1;
                changed = true;
            }
            if !changed || next == rewritten {
                break;
            }
            rewritten = next;
        }
        *line = rewritten;
    }
    out
}

fn rewrite_simple_expr_helper_calls(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    rewrite_simple_expr_helper_calls_filtered(lines, pure_user_calls, None)
}

fn simplify_nested_index_vec_floor_calls(lines: Vec<String>) -> Vec<String> {
    let Some(re) = nested_index_vec_floor_re() else {
        return lines;
    };
    lines
        .into_iter()
        .map(|line| {
            let mut rewritten = line;
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
                    break rewritten;
                }
                rewritten = next;
            }
        })
        .collect()
}

pub(crate) fn rewrite_selected_simple_expr_helper_calls_in_text(
    code: &str,
    helper_names: &[&str],
) -> String {
    let allowed_helpers: FxHashSet<String> = helper_names
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    let out_lines = rewrite_simple_expr_helper_calls_filtered(
        code.lines().map(str::to_string).collect(),
        &FxHashSet::default(),
        Some(&allowed_helpers),
    );
    let mut out = out_lines.join("\n");
    if code.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

pub(crate) fn simplify_nested_index_vec_floor_calls_in_text(code: &str) -> String {
    let out_lines =
        simplify_nested_index_vec_floor_calls(code.lines().map(str::to_string).collect());
    let mut out = out_lines.join("\n");
    if code.ends_with('\n') || !out.is_empty() {
        out.push('\n');
    }
    out
}

fn strip_unused_helper_params(lines: Vec<String>) -> Vec<String> {
    #[derive(Clone)]
    struct HelperTrim {
        original_len: usize,
        kept_indices: Vec<usize>,
        kept_params: Vec<String>,
    }

    let mut trims = FxHashMap::<String, HelperTrim>::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(&lines, fn_start) else {
            break;
        };
        let Some((fn_name, params)) = parse_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if !fn_name.starts_with("Sym_") || params.is_empty() {
            fn_start = fn_end + 1;
            continue;
        }
        let escaped = lines
            .iter()
            .enumerate()
            .filter(|(idx, _)| *idx < fn_start || *idx > fn_end)
            .any(|(_, line)| {
                unquoted_sym_refs(line).iter().any(|name| name == &fn_name)
                    && !line.contains(&format!("{fn_name}("))
            });
        if escaped {
            fn_start = fn_end + 1;
            continue;
        }
        let mut used_params = FxHashSet::default();
        for line in lines.iter().take(fn_end).skip(fn_start + 1) {
            let trimmed = line.trim();
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
                HelperTrim {
                    original_len: params.len(),
                    kept_indices: kept_indices.clone(),
                    kept_params: kept_indices
                        .iter()
                        .map(|idx| params[*idx].clone())
                        .collect(),
                },
            );
        }
        fn_start = fn_end + 1;
    }

    if trims.is_empty() {
        return lines;
    }

    let mut out = lines;
    for line in &mut out {
        if line.contains("<- function") {
            if let Some((fn_name, _)) = parse_function_header(line)
                && let Some(trim) = trims.get(&fn_name)
            {
                *line = format!("{} <- function({})", fn_name, trim.kept_params.join(", "));
            }
            continue;
        }
        let mut rewritten = line.clone();
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
                break;
            }
            rewritten = next;
        }
        *line = rewritten;
    }
    out
}

fn collect_metric_helpers(lines: &[String]) -> FxHashMap<String, MetricHelper> {
    let mut out = FxHashMap::default();
    let mut fn_start = 0usize;
    while fn_start < lines.len() {
        while fn_start < lines.len() && !lines[fn_start].contains("<- function") {
            fn_start += 1;
        }
        if fn_start >= lines.len() {
            break;
        }
        let Some(fn_end) = find_matching_block_end(lines, fn_start) else {
            break;
        };
        let Some((fn_name, params)) = parse_function_header(&lines[fn_start]) else {
            fn_start = fn_end + 1;
            continue;
        };
        if params.len() != 2 {
            fn_start = fn_end + 1;
            continue;
        }
        let body_lines: Vec<String> = lines
            .iter()
            .take(fn_end)
            .skip(fn_start + 1)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "{" && s != "}")
            .collect();
        if body_lines.len() < 3 || body_lines.len() > 5 {
            fn_start = fn_end + 1;
            continue;
        }
        let name_param = params[0].clone();
        let value_param = params[1].clone();
        let Some(return_line) = body_lines.last() else {
            fn_start = fn_end + 1;
            continue;
        };
        if return_line != &format!("return({value_param})") {
            fn_start = fn_end + 1;
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
            fn_start = fn_end + 1;
            continue;
        };
        if print_name_idx >= print_value_idx || print_value_idx + 1 != body_lines.len() - 1 {
            fn_start = fn_end + 1;
            continue;
        }
        let pre_name_lines = body_lines[..print_name_idx].to_vec();
        let pre_value_lines = body_lines[print_name_idx + 1..print_value_idx].to_vec();
        let helper = MetricHelper {
            name_param,
            value_param,
            pre_name_lines,
            pre_value_lines,
        };
        out.insert(fn_name, helper);
        fn_start = fn_end + 1;
    }
    out
}

fn rewrite_metric_helper_return_calls(lines: Vec<String>) -> Vec<String> {
    let helpers = collect_metric_helpers(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut temp_counter = 0usize;
    for line in lines {
        let trimmed = line.trim();
        let Some(inner) = trimmed
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
        else {
            out.push(line);
            continue;
        };
        let Some((callee, args_str)) = inner.split_once('(') else {
            out.push(line);
            continue;
        };
        let Some(args_inner) = args_str.strip_suffix(')') else {
            out.push(line);
            continue;
        };
        let Some(helper) = helpers.get(callee.trim()) else {
            out.push(line);
            continue;
        };
        let Some(args) = split_top_level_args(args_inner) else {
            out.push(line);
            continue;
        };
        if args.len() != 2 {
            out.push(line);
            continue;
        }
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let metric_name = args[0].trim();
        let metric_value = args[1].trim();
        for pre in &helper.pre_name_lines {
            out.push(format!("{indent}{pre}"));
        }
        out.push(format!("{indent}print({metric_name})"));
        let temp_name = format!(".__rr_inline_metric_{temp_counter}");
        temp_counter += 1;
        out.push(format!("{indent}{temp_name} <- {metric_value}"));
        for pre in &helper.pre_value_lines {
            out.push(format!("{indent}{pre}"));
        }
        out.push(format!("{indent}print({temp_name})"));
        out.push(format!("{indent}return({temp_name})"));
    }
    out
}

fn rewrite_metric_helper_statement_calls(lines: Vec<String>) -> Vec<String> {
    let helpers = collect_metric_helpers(&lines);
    if helpers.is_empty() {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    let mut temp_counter = 0usize;
    for line in lines {
        let trimmed = line.trim();
        let Some((callee, args_str)) = trimmed.split_once('(') else {
            out.push(line);
            continue;
        };
        if trimmed.contains("<-") || trimmed.starts_with("return(") {
            out.push(line);
            continue;
        }
        let Some(args_inner) = args_str.strip_suffix(')') else {
            out.push(line);
            continue;
        };
        let Some(helper) = helpers.get(callee.trim()) else {
            out.push(line);
            continue;
        };
        let Some(args) = split_top_level_args(args_inner) else {
            out.push(line);
            continue;
        };
        if args.len() != 2 {
            out.push(line);
            continue;
        }
        let indent_len = line.len() - line.trim_start().len();
        let indent = &line[..indent_len];
        let metric_name = args[0].trim();
        let metric_value = args[1].trim();
        for pre in &helper.pre_name_lines {
            out.push(format!("{indent}{pre}"));
        }
        out.push(format!("{indent}print({metric_name})"));
        let temp_name = format!(".__rr_inline_metric_{temp_counter}");
        temp_counter += 1;
        out.push(format!("{indent}{temp_name} <- {metric_value}"));
        for pre in &helper.pre_value_lines {
            out.push(format!("{indent}{pre}"));
        }
        out.push(format!("{indent}print({temp_name})"));
    }
    out
}

fn collapse_inlined_copy_vec_sequences(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let len = out.len();
    for idx in 0..len.saturating_sub(4) {
        let l0 = out[idx].trim().to_string();
        let l1 = out[idx + 1].trim().to_string();
        let l2 = out[idx + 2].trim().to_string();
        let l3 = out[idx + 3].trim().to_string();
        let l4 = out[idx + 4].trim().to_string();
        let Some(c0) = assign_re().and_then(|re| re.captures(&l0)) else {
            continue;
        };
        let Some(c1) = assign_re().and_then(|re| re.captures(&l1)) else {
            continue;
        };
        let Some(c2) = assign_re().and_then(|re| re.captures(&l2)) else {
            continue;
        };
        let Some(c3) = assign_re().and_then(|re| re.captures(&l3)) else {
            continue;
        };
        let Some(c4) = assign_re().and_then(|re| re.captures(&l4)) else {
            continue;
        };
        let n_var = c0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let n_rhs = c0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let out_var = c1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let out_rhs = c1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let i_var = c2.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let i_rhs = c2.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let out_replay_lhs = c3.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let src_rhs = c3.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let target_var = c4.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let target_rhs = c4.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(src_var) = ({
            if let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(src_rhs)) {
                let dest = slice_caps
                    .name("dest")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let start = slice_caps
                    .name("start")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let end = slice_caps
                    .name("end")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let rest = slice_caps
                    .name("rest")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if dest == out_var
                    && start == i_var
                    && end == n_var
                    && plain_ident_re().is_some_and(|re| re.is_match(rest))
                {
                    Some(rest.to_string())
                } else {
                    None
                }
            } else if plain_ident_re().is_some_and(|re| re.is_match(src_rhs)) {
                Some(src_rhs.to_string())
            } else {
                None
            }
        }) else {
            continue;
        };
        if !n_var.starts_with("inlined_")
            || !out_var.starts_with("inlined_")
            || !i_var.starts_with("inlined_")
            || out_replay_lhs != out_var
            || (target_rhs != out_var && target_rhs != src_var)
            || !literal_one_re().is_some_and(|re| re.is_match(i_rhs))
            || !n_rhs.starts_with("length(")
            || !out_rhs.starts_with("rep.int(0, ")
        {
            continue;
        }

        let mut final_assign_idx = None;
        for (search_idx, line) in out.iter().enumerate().skip(idx + 5) {
            let trimmed = line.trim();
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                continue;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
                continue;
            };
            let dest = slice_caps
                .name("dest")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let start = slice_caps
                .name("start")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let end = slice_caps
                .name("end")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let rest = slice_caps
                .name("rest")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if lhs == src_var
                && dest == out_var
                && start == i_var
                && end == n_var
                && rest == src_var
            {
                final_assign_idx = Some(search_idx);
                break;
            }
        }
        let Some(final_idx) = final_assign_idx else {
            continue;
        };
        let indent_len = out[idx + 4].len() - out[idx + 4].trim_start().len();
        let indent = out[idx + 4][..indent_len].to_string();
        out[idx].clear();
        out[idx + 1].clear();
        out[idx + 2].clear();
        out[idx + 3].clear();
        out[idx + 4] = format!("{indent}{target_var} <- {src_var}");
        let final_indent_len = out[final_idx].len() - out[final_idx].trim_start().len();
        let final_indent = out[final_idx][..final_indent_len].to_string();
        out[final_idx] = format!("{final_indent}{src_var} <- {target_var}");
    }
    out
}

fn rewrite_readonly_param_aliases(lines: Vec<String>) -> Vec<String> {
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

fn rewrite_remaining_readonly_param_shadow_uses(lines: Vec<String>) -> Vec<String> {
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

fn rewrite_index_only_mutated_param_shadow_aliases(lines: Vec<String>) -> Vec<String> {
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

fn literal_field_get_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r#"rr_field_get\((?P<base>{}),\s*"(?P<name>[A-Za-z_][A-Za-z0-9_]*)"\)"#,
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn rewrite_literal_field_get_calls(lines: Vec<String>) -> Vec<String> {
    let Some(re) = literal_field_get_re() else {
        return lines;
    };
    lines
        .into_iter()
        .map(|line| {
            if line.contains("<- function") {
                return line;
            }
            re.replace_all(&line, |caps: &Captures<'_>| {
                let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                let name = caps.name("name").map(|m| m.as_str()).unwrap_or("").trim();
                format!(r#"{base}[["{name}"]]"#)
            })
            .to_string()
        })
        .collect()
}

fn literal_record_field_name(arg: &str) -> Option<String> {
    let trimmed = arg.trim();
    let inner = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
        })?;
    plain_ident_re()
        .is_some_and(|re| re.is_match(inner))
        .then(|| inner.to_string())
}

fn rewrite_literal_named_list_calls(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| {
            if line.contains("rr_named_list <- function") {
                return line;
            }
            let mut rewritten = line;
            loop {
                let Some(start) = rewritten.find("rr_named_list(") else {
                    break;
                };
                let call_start = start + "rr_named_list".len();
                let mut depth = 0i32;
                let mut end = None;
                for (off, ch) in rewritten[call_start..].char_indices() {
                    match ch {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = Some(call_start + off);
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let Some(call_end) = end else {
                    break;
                };
                let args_inner = &rewritten[call_start + 1..call_end];
                let Some(args) = split_top_level_args(args_inner) else {
                    break;
                };
                if args.len() % 2 != 0 {
                    break;
                }
                let mut fields = Vec::new();
                let mut ok = true;
                for pair in args.chunks(2) {
                    let Some(name) = literal_record_field_name(pair[0].trim()) else {
                        ok = false;
                        break;
                    };
                    fields.push(format!("{name} = {}", pair[1].trim()));
                }
                if !ok {
                    break;
                }
                let replacement = if fields.is_empty() {
                    "list()".to_string()
                } else {
                    format!("list({})", fields.join(", "))
                };
                rewritten.replace_range(start..=call_end, &replacement);
            }
            rewritten
        })
        .collect()
}

fn expr_is_safe_scalar_index_source(expr: &str) -> bool {
    let expr = expr.trim();
    expr.starts_with("rr_idx_cube_vec_i(")
        || expr.starts_with("rr_wrap_index_vec_i(")
        || expr_is_floor_clamped_scalar_index_source(expr)
}

fn expr_is_floor_clamped_scalar_index_source(expr: &str) -> bool {
    let mut compact = compact_expr(expr);
    if compact.starts_with('(') && compact.ends_with(')') {
        compact = compact[1..compact.len() - 1].to_string();
    }
    compact.starts_with("pmin(pmax((1+floor(") || compact.starts_with("pmin(pmax(1+floor(")
}

fn rewrite_safe_named_index_read_calls(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut safe_index_vars = FxHashSet::<String>::default();
    for line in lines {
        if line.contains("<- function") {
            safe_index_vars.clear();
            out.push(line);
            continue;
        }
        let trimmed = line.trim().to_string();
        if is_control_flow_boundary(&trimmed) {
            if trimmed == "repeat {"
                || trimmed == "}"
                || trimmed.starts_with("} else")
                || trimmed.starts_with("else")
            {
                safe_index_vars.clear();
            }
            out.push(line);
            continue;
        }

        let mut rewritten = line;
        if let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                safe_index_vars.remove(lhs);
                if expr_is_safe_scalar_index_source(rhs) {
                    safe_index_vars.insert(lhs.to_string());
                }
            }
        }

        loop {
            let Some(start) = rewritten.find("rr_index1_read(") else {
                break;
            };
            let call_start = start + "rr_index1_read".len();
            let mut depth = 0i32;
            let mut end = None;
            for (off, ch) in rewritten[call_start..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(call_start + off);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(call_end) = end else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                break;
            };
            if args.len() < 2 {
                break;
            }
            let base = args[0].trim();
            let idx = args[1].trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(base))
                || !(safe_index_vars.contains(idx) || expr_is_safe_scalar_index_source(idx))
                || !(args.len() == 2 || is_literal_index_ctx(&args[2]))
            {
                break;
            }
            rewritten.replace_range(start..=call_end, &format!("{base}[{idx}]"));
        }

        out.push(rewritten);
    }
    out
}

fn parse_flat_positive_loop_index_expr(expr: &str) -> Option<(String, String, String)> {
    let mut compact = compact_expr(expr);
    if compact.starts_with('(') && compact.ends_with(')') {
        compact = compact[1..compact.len() - 1].to_string();
    }
    let re = compile_regex(format!(
        r"^\(\((?P<outer>{})-1(?:L|\.0+)?\)\*(?P<bound>{})\)\+(?P<inner>{})$",
        IDENT_PATTERN, IDENT_PATTERN, IDENT_PATTERN
    ))?;
    let caps = re.captures(&compact)?;
    Some((
        caps.name("outer")?.as_str().to_string(),
        caps.name("bound")?.as_str().to_string(),
        caps.name("inner")?.as_str().to_string(),
    ))
}

fn rewrite_safe_flat_loop_index_read_calls(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    for idx in 0..out.len() {
        if out[idx].contains("<- function") {
            continue;
        }
        let mut rewritten = out[idx].clone();
        loop {
            let Some(start) = rewritten.find("rr_index1_read(") else {
                break;
            };
            let call_start = start + "rr_index1_read".len();
            let mut depth = 0i32;
            let mut end = None;
            for (off, ch) in rewritten[call_start..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(call_start + off);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(call_end) = end else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                break;
            };
            if args.len() < 2 {
                break;
            }
            let base = args[0].trim();
            let index_expr = args[1].trim();
            let Some((outer, bound, inner)) = parse_flat_positive_loop_index_expr(index_expr)
            else {
                break;
            };
            if !plain_ident_re().is_some_and(|re| re.is_match(base))
                || !var_has_known_positive_progression_before(&out, idx, &outer)
                || !var_has_known_positive_progression_before(&out, idx, &inner)
                || !positive_guard_for_var_before(&out, idx, &inner, &bound)
                || !(args.len() == 2 || is_literal_index_ctx(&args[2]))
            {
                break;
            }
            rewritten.replace_range(start..=call_end, &format!("{base}[{index_expr}]"));
        }
        out[idx] = rewritten;
    }
    out
}

fn is_length_preserving_call(name: &str) -> bool {
    matches!(
        name,
        "abs"
            | "sqrt"
            | "log"
            | "log10"
            | "log2"
            | "exp"
            | "sign"
            | "floor"
            | "ceiling"
            | "trunc"
            | "pmin"
            | "pmax"
            | "ifelse"
            | "rr_ifelse_strict"
    )
}

fn expr_is_length_preserving_shape(expr: &str) -> bool {
    let expr = expr.trim();
    if expr.is_empty() || expr.contains("<-") || expr.contains("function(") {
        return false;
    }
    let Some(re) = compile_regex(format!(r"(?P<callee>{})\s*\(", IDENT_PATTERN)) else {
        return false;
    };
    re.captures_iter(expr).all(|caps| {
        let callee = caps.name("callee").map(|m| m.as_str()).unwrap_or("").trim();
        is_length_preserving_call(callee)
    })
}

fn infer_same_len_expr(expr: &str, vector_lens: &FxHashMap<String, String>) -> Option<String> {
    let mut expr = expr.trim();
    loop {
        if !(expr.starts_with('(') && expr.ends_with(')')) {
            break;
        }
        let mut depth = 0i32;
        let mut wraps = true;
        for (idx, ch) in expr.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 && idx + ch.len_utf8() < expr.len() {
                        wraps = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !wraps {
            break;
        }
        expr = expr[1..expr.len() - 1].trim();
    }

    if let Some(inner) = expr
        .strip_prefix("seq_len(")
        .and_then(|s| s.strip_suffix(')'))
    {
        return Some(inner.trim().to_string());
    }
    if let Some(inner) = expr
        .strip_prefix("rep.int(")
        .and_then(|s| s.strip_suffix(')'))
        && let Some(args) = split_top_level_args(inner)
        && args.len() == 2
    {
        return Some(args[1].trim().to_string());
    }

    let vector_idents: Vec<String> = expr_idents(expr)
        .into_iter()
        .filter_map(|ident| vector_lens.get(&ident).cloned())
        .collect();
    if vector_idents.is_empty() || !expr_is_length_preserving_shape(expr) {
        return None;
    }
    let first = vector_idents[0].clone();
    vector_idents
        .iter()
        .all(|len| len == &first)
        .then_some(first)
}

fn rewrite_same_len_scalar_tail_reads(lines: Vec<String>) -> Vec<String> {
    let mut out = Vec::with_capacity(lines.len());
    let mut scalar_positive = FxHashSet::<String>::default();
    let mut vector_lens = FxHashMap::<String, String>::default();

    for line in lines {
        if line.contains("<- function") {
            scalar_positive.clear();
            vector_lens.clear();
            out.push(line);
            continue;
        }

        let trimmed = line.trim().to_string();
        let mut rewritten = line;

        loop {
            let Some(start) = rewritten.find("rr_index1_read(") else {
                break;
            };
            let call_start = start + "rr_index1_read".len();
            let mut depth = 0i32;
            let mut end = None;
            for (off, ch) in rewritten[call_start..].char_indices() {
                match ch {
                    '(' => depth += 1,
                    ')' => {
                        depth -= 1;
                        if depth == 0 {
                            end = Some(call_start + off);
                            break;
                        }
                    }
                    _ => {}
                }
            }
            let Some(call_end) = end else {
                break;
            };
            let args_inner = &rewritten[call_start + 1..call_end];
            let Some(args) = split_top_level_args(args_inner) else {
                break;
            };
            if args.len() < 2 {
                break;
            }
            let base = args[0].trim();
            let idx = args[1].trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(base))
                || !plain_ident_re().is_some_and(|re| re.is_match(idx))
                || !scalar_positive.contains(idx)
                || vector_lens.get(base).map(String::as_str) != Some(idx)
                || !(args.len() == 2 || is_literal_index_ctx(&args[2]))
            {
                break;
            }
            rewritten.replace_range(start..=call_end, &format!("{base}[{idx}]"));
        }

        if is_control_flow_boundary(&trimmed) {
            if trimmed.starts_with("} else") || trimmed.starts_with("else") {
                scalar_positive.clear();
                vector_lens.clear();
            }
            out.push(rewritten);
            continue;
        }

        if let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) {
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                let inferred_len = infer_same_len_expr(rhs, &vector_lens);
                scalar_positive.remove(lhs);
                vector_lens.remove(lhs);
                if literal_integer_value(rhs).is_some_and(|value| value >= 1) {
                    scalar_positive.insert(lhs.to_string());
                }
                if let Some(len) = inferred_len {
                    vector_lens.insert(lhs.to_string(), len);
                }
            }
        }

        out.push(rewritten);
    }
    out
}

fn is_literal_index_ctx(arg: &str) -> bool {
    matches!(arg.trim(), "\"index\"" | "'index'")
}

fn rewrite_wrap_index_scalar_access_helpers(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| {
            if line.contains("<- function") {
                return line;
            }
            let mut rewritten = line;
            loop {
                let mut changed = false;
                for callee in ["rr_index1_read", "rr_index1_write"] {
                    let Some(start) = rewritten.find(&format!("{callee}(")) else {
                        continue;
                    };
                    let call_start = start + callee.len();
                    let mut depth = 0i32;
                    let mut end = None;
                    for (off, ch) in rewritten[call_start..].char_indices() {
                        match ch {
                            '(' => depth += 1,
                            ')' => {
                                depth -= 1;
                                if depth == 0 {
                                    end = Some(call_start + off);
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    let Some(call_end) = end else {
                        continue;
                    };
                    let args_inner = &rewritten[call_start + 1..call_end];
                    let Some(args) = split_top_level_args(args_inner) else {
                        continue;
                    };
                    let replacement = match callee {
                        "rr_index1_read" if args.len() >= 2 => {
                            let base = args[0].trim();
                            let idx = args[1].trim();
                            if plain_ident_re().is_some_and(|re| re.is_match(base))
                                && idx.starts_with("rr_wrap_index_vec_i(")
                                && (args.len() == 2 || is_literal_index_ctx(&args[2]))
                            {
                                Some(format!("{base}[{idx}]"))
                            } else {
                                None
                            }
                        }
                        "rr_index1_write" if !args.is_empty() => {
                            let idx = args[0].trim();
                            if idx.starts_with("rr_wrap_index_vec_i(")
                                && (args.len() == 1 || is_literal_index_ctx(&args[1]))
                            {
                                Some(idx.to_string())
                            } else {
                                None
                            }
                        }
                        _ => None,
                    };
                    let Some(replacement) = replacement else {
                        continue;
                    };
                    rewritten.replace_range(start..=call_end, &replacement);
                    changed = true;
                    break;
                }
                if !changed {
                    break;
                }
            }
            rewritten
        })
        .collect()
}

fn run_exact_expr_cleanup_rounds(mut lines: Vec<String>, max_rounds: usize) -> Vec<String> {
    for _ in 0..max_rounds {
        let before = lines.clone();
        lines = rewrite_forward_exact_expr_reuse(lines);
        lines = rewrite_temp_minus_one_scaled_to_named_scalar(lines);
        lines = strip_noop_self_assignments(lines);
        if lines == before {
            break;
        }
    }
    lines
}

fn rewrite_temp_minus_one_scaled_to_named_scalar(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(assign_re) = assign_re() else {
        return out;
    };
    let minus_one_re = compile_regex(r"^\((?P<inner>.+)\s-\s1L?\)$".to_string());

    let mut named_minus_one = FxHashMap::<String, String>::default();
    let mut temp_inner = FxHashMap::<String, String>::default();

    for line in &out {
        let trimmed = line.trim();
        let Some(caps) = assign_re.captures(trimmed) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if let Some(inner) = minus_one_re
            .as_ref()
            .and_then(|re| re.captures(rhs))
            .and_then(|caps| caps.name("inner").map(|m| m.as_str()))
            && plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && !lhs.starts_with('.')
        {
            named_minus_one.insert(inner.to_string(), lhs.to_string());
        } else if lhs.starts_with(".__rr_cse_") {
            temp_inner.insert(lhs.to_string(), rhs.to_string());
        }
    }

    for line in &mut out {
        let mut rewritten = line.clone();
        for (temp, inner) in &temp_inner {
            let Some(name) = named_minus_one.get(inner) else {
                continue;
            };
            let pattern = format!(
                r"\(\(\s*{}\s*-\s*1\s*\)\s*\*\s*([^\)]+)\)",
                regex::escape(temp)
            );
            if let Some(re) = compile_regex(pattern) {
                let replacement = format!("({name} * $1)");
                rewritten = re.replace_all(&rewritten, replacement.as_str()).to_string();
            }
        }
        *line = rewritten;
    }

    out
}

fn strip_redundant_nested_temp_reassigns(lines: Vec<String>) -> Vec<String> {
    let out = lines;
    let mut remove = vec![false; out.len()];
    for idx in 0..out.len() {
        let trimmed = out[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".__rr_cse_") {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let cur_indent = out[idx].len() - out[idx].trim_start().len();
        let mut j = idx;
        while j > 0 {
            j -= 1;
            let prev = out[j].trim();
            if prev.is_empty() {
                continue;
            }
            if out[j].contains("<- function")
                || prev == "repeat {"
                || prev.starts_with("while")
                || prev.starts_with("for")
            {
                break;
            }
            if let Some(prev_caps) = assign_re().and_then(|re| re.captures(prev)) {
                let prev_lhs = prev_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let prev_rhs = prev_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if prev_lhs == lhs {
                    if prev_rhs == lhs {
                        continue;
                    }
                    let prev_indent = out[j].len() - out[j].trim_start().len();
                    if prev_rhs == rhs && prev_indent < cur_indent {
                        remove[idx] = true;
                    }
                    break;
                }
                if deps.contains(prev_lhs) {
                    break;
                }
            }
        }
    }
    out.into_iter()
        .enumerate()
        .filter_map(|(idx, line)| (!remove[idx]).then_some(line))
        .collect()
}

fn strip_redundant_tail_assign_slice_return(lines: Vec<String>) -> Vec<String> {
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

        let Some(return_idx) = previous_non_empty_line(&out, fn_end) else {
            fn_start = fn_end + 1;
            continue;
        };
        let return_trimmed = out[return_idx].trim();
        let Some(ret_var) = return_trimmed
            .strip_prefix("return(")
            .and_then(|s| s.strip_suffix(')'))
            .map(str::trim)
        else {
            fn_start = fn_end + 1;
            continue;
        };

        let Some(assign_idx) = previous_non_empty_line(&out, return_idx) else {
            fn_start = fn_end + 1;
            continue;
        };
        let assign_trimmed = out[assign_idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(assign_trimmed)) else {
            fn_start = fn_end + 1;
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs != ret_var {
            fn_start = fn_end + 1;
            continue;
        }

        let Some(assign_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
            fn_start = fn_end + 1;
            continue;
        };
        let dest = assign_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let start = assign_caps
            .name("start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = assign_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let temp = assign_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if dest != ret_var
            || !literal_one_re().is_some_and(|re| re.is_match(start))
            || !plain_ident_re().is_some_and(|re| re.is_match(temp))
        {
            fn_start = fn_end + 1;
            continue;
        }

        if function_has_non_empty_repeat_whole_assign(&out[fn_start..fn_end], ret_var, end, temp)
            || function_has_matching_exprmap_whole_assign(
                &out[fn_start..fn_end],
                ret_var,
                end,
                temp,
            )
        {
            out[assign_idx].clear();
        }

        fn_start = fn_end + 1;
    }
    out
}

fn function_has_matching_exprmap_whole_assign(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    if !temp_var.starts_with(".tachyon_exprmap") {
        return false;
    }
    let Some(temp_idx) = lines.iter().position(|line| {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| {
                caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == temp_var
            })
    }) else {
        return false;
    };
    let Some(temp_rhs) = assign_re()
        .and_then(|re| re.captures(lines[temp_idx].trim()))
        .and_then(|caps| caps.name("rhs").map(|m| m.as_str().trim().to_string()))
    else {
        return false;
    };

    for line in lines.iter().skip(temp_idx + 1) {
        let trimmed = line.trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
            continue;
        };
        let dest = slice_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = slice_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let rest = slice_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if lhs == dest_var
            && dest == dest_var
            && end == end_expr
            && (rest == temp_rhs || rest == temp_var)
        {
            return true;
        }
    }
    false
}

fn function_has_non_empty_repeat_whole_assign(
    lines: &[String],
    dest_var: &str,
    end_expr: &str,
    temp_var: &str,
) -> bool {
    let debug_tail = std::env::var_os("RR_DEBUG_TAIL").is_some()
        && dest_var == "x"
        && temp_var == ".tachyon_exprmap0_1";
    let Some(temp_idx) = lines.iter().position(|line| {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| {
                caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == temp_var
            })
    }) else {
        if debug_tail {
            eprintln!("tail-debug: no temp_idx");
        }
        return false;
    };
    let Some(temp_rhs) = assign_re()
        .and_then(|re| re.captures(lines[temp_idx].trim()))
        .and_then(|caps| caps.name("rhs").map(|m| m.as_str().trim().to_string()))
    else {
        if debug_tail {
            eprintln!("tail-debug: no temp_rhs");
        }
        return false;
    };

    let mut assign_idx = None;
    for idx in temp_idx + 1..lines.len() {
        let trimmed = lines[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let Some(slice_caps) = assign_slice_re().and_then(|re| re.captures(rhs)) else {
            continue;
        };
        let dest = slice_caps
            .name("dest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let start = slice_caps
            .name("start")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let end = slice_caps
            .name("end")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        let rest = slice_caps
            .name("rest")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if lhs == dest_var
            && dest == dest_var
            && end == end_expr
            && (rest == temp_rhs || rest == temp_var)
            && plain_ident_re().is_some_and(|re| re.is_match(start))
            && has_assignment_to_one_before(lines, idx, start)
        {
            assign_idx = Some(idx);
            break;
        }
    }
    let Some(assign_idx) = assign_idx else {
        if debug_tail {
            eprintln!("tail-debug: no inner assign match");
        }
        return false;
    };

    let Some(repeat_idx) = (0..assign_idx)
        .rev()
        .find(|idx| lines[*idx].trim() == "repeat {")
    else {
        if debug_tail {
            eprintln!("tail-debug: no repeat_idx");
        }
        return false;
    };
    let Some(guard_idx) = (repeat_idx + 1..assign_idx).find(|idx| {
        lines[*idx].trim().starts_with("if !(") || lines[*idx].trim().starts_with("if (!(")
    }) else {
        if debug_tail {
            eprintln!("tail-debug: no guard_idx");
        }
        return false;
    };
    let guard = lines[guard_idx].trim();
    let Some(inner) = guard
        .strip_prefix("if (!(")
        .and_then(|s| s.strip_suffix(")) break"))
    else {
        if debug_tail {
            eprintln!("tail-debug: guard parse failed: {}", guard);
        }
        return false;
    };
    let Some((iter_var, bound)) = inner.split_once("<=") else {
        if debug_tail {
            eprintln!("tail-debug: split <= failed: {}", inner);
        }
        return false;
    };
    let positive = literal_positive_re().is_some_and(|re| re.is_match(bound.trim()));
    let has_one = has_assignment_to_one_before(lines, guard_idx, iter_var.trim());
    if debug_tail {
        eprintln!(
            "tail-debug: temp_idx={} assign_idx={} repeat_idx={} guard_idx={} inner={} positive={} has_one={}",
            temp_idx, assign_idx, repeat_idx, guard_idx, inner, positive, has_one
        );
    }
    positive && has_one
}

fn previous_non_empty_line(lines: &[String], idx: usize) -> Option<usize> {
    (0..idx).rev().find(|i| !lines[*i].trim().is_empty())
}

fn has_assignment_to_one_before(lines: &[String], idx: usize, var: &str) -> bool {
    (0..idx).rev().any(|i| {
        assign_re()
            .and_then(|re| re.captures(lines[i].trim()))
            .is_some_and(|caps| {
                caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == var
                    && literal_one_re().is_some_and(|re| {
                        re.is_match(caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim())
                    })
            })
    })
}

fn literal_one_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^(?:1L?|1(?:\.0+)?)$".to_string()))
        .as_ref()
}

fn literal_positive_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"^(?:[1-9]\d*)(?:\.0+)?L?$".to_string()))
        .as_ref()
}

fn literal_integer_value(expr: &str) -> Option<i64> {
    let trimmed = expr.trim().trim_end_matches('L').trim_end_matches('l');
    if let Ok(value) = trimmed.parse::<i64>() {
        return Some(value);
    }
    let value = trimmed.parse::<f64>().ok()?;
    (value.fract() == 0.0).then_some(value as i64)
}

fn latest_literal_assignment_before(lines: &[String], idx: usize, var: &str) -> Option<i64> {
    for i in (0..idx).rev() {
        let Some(caps) = assign_re().and_then(|re| re.captures(lines[i].trim())) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs != var {
            continue;
        }
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        return literal_integer_value(rhs);
    }
    None
}

fn parse_repeat_guard_cmp_line(line: &str) -> Option<(String, String, String)> {
    let trimmed = line.trim();
    let inner = trimmed
        .strip_prefix("if !(")
        .or_else(|| trimmed.strip_prefix("if (!("))?;
    let inner = inner
        .strip_suffix(")) break")
        .or_else(|| inner.strip_suffix(") break"))?;
    if let Some((lhs, rhs)) = inner.split_once("<=") {
        return Some((
            lhs.trim().to_string(),
            "<=".to_string(),
            rhs.trim().to_string(),
        ));
    }
    if let Some((lhs, rhs)) = inner.split_once('<') {
        return Some((
            lhs.trim().to_string(),
            "<".to_string(),
            rhs.trim().to_string(),
        ));
    }
    None
}

fn var_has_known_positive_progression_before(lines: &[String], idx: usize, var: &str) -> bool {
    let mut seen_assign = false;
    for line in lines.iter().take(idx) {
        let Some(caps) = assign_re().and_then(|re| re.captures(line.trim())) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs != var {
            continue;
        }
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let is_positive_reset = literal_integer_value(rhs).is_some_and(|value| value >= 1);
        let is_canonical_incr = rhs == format!("({var} + 1)")
            || rhs == format!("({var} + 1L)")
            || rhs == format!("({var} + 1.0)");
        if !is_positive_reset && !is_canonical_incr {
            return false;
        }
        seen_assign = true;
    }
    seen_assign
}

fn positive_guard_for_var_before(
    lines: &[String],
    idx: usize,
    iter_var: &str,
    bound_var: &str,
) -> bool {
    (0..idx).rev().any(|i| {
        parse_repeat_guard_cmp_line(lines[i].trim()).is_some_and(|(iter, _op, bound)| {
            iter == iter_var
                && bound == bound_var
                && var_has_known_positive_progression_before(lines, i, iter_var)
        })
    })
}

fn count_unquoted_braces(line: &str) -> (usize, usize) {
    let mut opens = 0usize;
    let mut closes = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;
    for ch in line.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' && (in_single || in_double) {
            escaped = true;
            continue;
        }
        if ch == '\'' && !in_double {
            in_single = !in_single;
            continue;
        }
        if ch == '"' && !in_single {
            in_double = !in_double;
            continue;
        }
        if in_single || in_double {
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

fn collapse_common_if_else_tail_assignments(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut i = 0usize;
    while i < out.len() {
        let trimmed = out[i].trim();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            i += 1;
            continue;
        }
        let Some((else_idx, end_idx)) = find_if_else_bounds(&out, i) else {
            i += 1;
            continue;
        };
        let mut then_idx = else_idx;
        let mut else_tail_idx = end_idx;
        let mut shared = Vec::<(usize, usize, String)>::new();
        loop {
            let Some((cur_then_idx, then_assign)) = last_non_empty_assign_before(&out, then_idx)
            else {
                break;
            };
            let Some((cur_else_idx, else_assign)) =
                last_non_empty_assign_before(&out, else_tail_idx)
            else {
                break;
            };
            if cur_then_idx <= i || cur_else_idx <= else_idx {
                break;
            }
            if then_assign.trim() != else_assign.trim() {
                break;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(then_assign.trim())) else {
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
                break;
            }
            shared.push((cur_then_idx, cur_else_idx, then_assign.trim().to_string()));
            then_idx = cur_then_idx;
            else_tail_idx = cur_else_idx;
        }
        if shared.is_empty() {
            i = end_idx + 1;
            continue;
        }
        shared.reverse();
        let indent_len = shared[0].2.len() - shared[0].2.trim_start().len();
        let indent = " ".repeat(indent_len);
        for (then_assign_idx, else_assign_idx, _) in &shared {
            out[*then_assign_idx].clear();
            out[*else_assign_idx].clear();
        }
        let mut insert_at = end_idx + 1;
        for (_, _, assign) in &shared {
            out.insert(insert_at, format!("{indent}{assign}"));
            insert_at += 1;
        }
        i = insert_at;
    }
    out
}

fn collapse_identical_if_else_tail_assignments_late(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut i = 0usize;
    while i < out.len() {
        let trimmed = out[i].trim();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            i += 1;
            continue;
        }
        let Some((else_idx, end_idx)) = find_if_else_bounds(&out, i) else {
            i += 1;
            continue;
        };

        let then_lines: Vec<usize> = ((i + 1)..else_idx)
            .filter(|idx| {
                let t = out[*idx].trim();
                !t.is_empty()
            })
            .collect();
        let else_lines: Vec<usize> = ((else_idx + 1)..end_idx)
            .filter(|idx| {
                let t = out[*idx].trim();
                !t.is_empty()
            })
            .collect();

        let mut t = then_lines.len();
        let mut e = else_lines.len();
        let mut shared = Vec::<(usize, usize, String)>::new();
        while t > 0 && e > 0 {
            let then_idx = then_lines[t - 1];
            let else_line_idx = else_lines[e - 1];
            let then_trimmed = out[then_idx].trim();
            let else_trimmed = out[else_line_idx].trim();
            if then_trimmed != else_trimmed {
                break;
            }
            if assign_re()
                .and_then(|re| re.captures(then_trimmed))
                .is_none()
            {
                break;
            }
            shared.push((then_idx, else_line_idx, then_trimmed.to_string()));
            t -= 1;
            e -= 1;
        }

        if shared.is_empty() {
            i = end_idx + 1;
            continue;
        }

        shared.reverse();
        let indent_len = out[i].len() - out[i].trim_start().len();
        let indent = " ".repeat(indent_len);
        for (then_idx, else_idx_line, _) in &shared {
            out[*then_idx].clear();
            out[*else_idx_line].clear();
        }
        let mut insert_at = end_idx + 1;
        for (_, _, assign) in &shared {
            out.insert(insert_at, format!("{indent}{assign}"));
            insert_at += 1;
        }
        i = insert_at;
    }
    out
}

fn rewrite_forward_simple_alias_guards(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    let len = out.len();
    for idx in 0..len {
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
        if is_peephole_temp(&lhs)
            || !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
            || !plain_ident_re().is_some_and(|re| re.is_match(&rhs))
            || lhs == rhs
        {
            continue;
        }

        let alias_map = FxHashMap::from_iter([(lhs.clone(), rhs.clone())]);
        let mut replaced_any = false;
        let mut unsafe_use_seen = false;

        let mut relative_depth = 0i32;
        for line in out.iter_mut().skip(idx + 1) {
            let line_trimmed = line.trim();
            let next_indent = line.len() - line.trim_start().len();
            if line.contains("<- function") {
                break;
            }
            if line_trimmed == "}" {
                if relative_depth == 0 {
                    break;
                }
                relative_depth -= 1;
                continue;
            }
            if line_trimmed.starts_with("} else") {
                if relative_depth == 0 {
                    break;
                }
                continue;
            }
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    unsafe_use_seen = true;
                    break;
                }
                if next_lhs == rhs {
                    break;
                }
                if expr_idents(next_rhs).iter().any(|ident| ident == &lhs) {
                    unsafe_use_seen = true;
                    break;
                }
                continue;
            }

            if line_trimmed.starts_with("if ") && line_trimmed.ends_with('{') {
                let rewritten = ident_re
                    .replace_all(line, |caps: &Captures<'_>| {
                        let ident = caps.get(0).map(|m| m.as_str()).unwrap_or("");
                        alias_map
                            .get(ident)
                            .cloned()
                            .unwrap_or_else(|| ident.to_string())
                    })
                    .to_string();
                if rewritten != *line {
                    *line = rewritten;
                    replaced_any = true;
                }
                relative_depth += 1;
                continue;
            }
            if line_trimmed.ends_with('{') {
                relative_depth += 1;
            }

            if expr_idents(line_trimmed).iter().any(|ident| ident == &lhs) {
                unsafe_use_seen = true;
                break;
            }

            if line_trimmed == "return(NULL)"
                || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
            {
                break;
            }
        }

        if replaced_any && !unsafe_use_seen {
            out[idx].clear();
        }
    }
    out
}

fn rewrite_loop_index_alias_ii(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if lhs != "ii" || rhs != "i" {
            continue;
        }

        let mut replaced_any = false;
        for line in out.iter_mut().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == "ii" || next_lhs == "i" {
                    break;
                }
            }
            if !expr_idents(line_trimmed).iter().any(|ident| ident == "ii") {
                continue;
            }
            let rewritten = ident_re
                .replace_all(line, |m: &Captures<'_>| {
                    let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                    if ident == "ii" {
                        "i".to_string()
                    } else {
                        ident.to_string()
                    }
                })
                .to_string();
            if rewritten != *line {
                *line = rewritten;
                replaced_any = true;
            }
        }
        if replaced_any {
            out[idx].clear();
        }
    }
    out
}

fn rewrite_safe_loop_index_write_calls(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut repeat_idx = 0usize;
    while repeat_idx < out.len() {
        let Some(next_repeat) = (repeat_idx..out.len()).find(|idx| out[*idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_matching_block_end(&out, next_repeat) else {
            break;
        };
        let Some(guard_idx) = (next_repeat + 1..loop_end).find(|idx| {
            let trimmed = out[*idx].trim();
            trimmed.starts_with("if (!(") && trimmed.ends_with(")) break")
        }) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some((iter_var, _op, _bound)) = parse_repeat_guard_cmp_line(out[guard_idx].trim())
        else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let start_value = latest_literal_assignment_before(&out, guard_idx, &iter_var);
        if !plain_ident_re().is_some_and(|re| re.is_match(&iter_var))
            || start_value.is_none_or(|value| value < 1)
        {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let mut safe = true;
        for line in out.iter().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                continue;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs != iter_var {
                continue;
            }
            let canonical_incr = rhs == format!("({iter_var} + 1)")
                || rhs == format!("({iter_var} + 1L)")
                || rhs == format!("({iter_var} + 1.0)");
            if !canonical_incr {
                safe = false;
                break;
            }
        }
        if !safe {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let needle = format!("rr_index1_write({iter_var}, \"index\")");
        let needle_single = format!("rr_index1_write({iter_var}, 'index')");
        for line in out.iter_mut().take(loop_end).skip(guard_idx + 1) {
            if line.contains(&needle) {
                *line = line.replace(&needle, &iter_var);
            }
            if line.contains(&needle_single) {
                *line = line.replace(&needle_single, &iter_var);
            }
        }
        repeat_idx = next_repeat + 1;
    }
    out
}

fn rewrite_safe_loop_neighbor_read_calls(lines: Vec<String>) -> Vec<String> {
    fn rewrite_loop_read_expr(
        expr: &str,
        iter_var: &str,
        allow_prev: bool,
        allow_next: bool,
    ) -> String {
        let mut out = expr.to_string();
        let iter_esc = regex::escape(iter_var);
        let ctx = r#"(?:"index"|'index')"#;

        let direct_pat = format!(
            r#"rr_index1_read\((?P<base>{}),\s*{}\s*,\s*{}\)"#,
            IDENT_PATTERN, iter_esc, ctx
        );
        if let Some(re) = compile_regex(direct_pat) {
            out = re
                .replace_all(&out, |caps: &Captures<'_>| {
                    let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                    format!("{base}[{iter_var}]")
                })
                .to_string();
        }

        if allow_prev {
            let prev_pat = format!(
                r#"rr_index1_read\((?P<base>{}),\s*\(\s*{}\s*-\s*1(?:L|\.0+)?\s*\)\s*,\s*{}\)"#,
                IDENT_PATTERN, iter_esc, ctx
            );
            if let Some(re) = compile_regex(prev_pat) {
                out = re
                    .replace_all(&out, |caps: &Captures<'_>| {
                        let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                        format!("{base}[({iter_var} - 1)]")
                    })
                    .to_string();
            }
        }

        if allow_next {
            let next_pat = format!(
                r#"rr_index1_read\((?P<base>{}),\s*\(\s*{}\s*\+\s*1(?:L|\.0+)?\s*\)\s*,\s*{}\)"#,
                IDENT_PATTERN, iter_esc, ctx
            );
            if let Some(re) = compile_regex(next_pat) {
                out = re
                    .replace_all(&out, |caps: &Captures<'_>| {
                        let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                        format!("{base}[({iter_var} + 1)]")
                    })
                    .to_string();
            }
        }
        out
    }

    let mut out = lines;
    let mut repeat_idx = 0usize;
    while repeat_idx < out.len() {
        let Some(next_repeat) = (repeat_idx..out.len()).find(|idx| out[*idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_matching_block_end(&out, next_repeat) else {
            break;
        };
        let Some(guard_idx) = (next_repeat + 1..loop_end).find(|idx| {
            let trimmed = out[*idx].trim();
            trimmed.starts_with("if !(")
                || (trimmed.starts_with("if (!(") && trimmed.ends_with("break"))
        }) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some((iter_var, op, _bound)) = parse_repeat_guard_cmp_line(out[guard_idx].trim())
        else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let start_value = latest_literal_assignment_before(&out, guard_idx, &iter_var);
        if !plain_ident_re().is_some_and(|re| re.is_match(&iter_var))
            || start_value.is_none_or(|value| value < 1)
        {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let mut safe = true;
        for line in out.iter().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                continue;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if lhs != iter_var {
                continue;
            }
            let canonical_incr = rhs == format!("({iter_var} + 1)")
                || rhs == format!("({iter_var} + 1L)")
                || rhs == format!("({iter_var} + 1.0)");
            if !canonical_incr {
                safe = false;
                break;
            }
        }
        if !safe {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let allow_prev = start_value.is_some_and(|value| value >= 2);
        let allow_next = op == "<";
        for line in out.iter_mut().take(loop_end).skip(guard_idx + 1) {
            *line = rewrite_loop_read_expr(line, &iter_var, allow_prev, allow_next);
        }
        repeat_idx = next_repeat + 1;
    }
    out
}

fn rewrite_temp_uses_after_named_copy(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim();
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
        if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
            || !(rhs.starts_with(".__pc_src_tmp") || rhs.starts_with(".__rr_cse_"))
        {
            continue;
        }

        let temp = rhs;
        for line in out.iter_mut().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == temp {
                    break;
                }
                if next_lhs == lhs {
                    if next_rhs == temp {
                        continue;
                    }
                    break;
                }
            }
            if !line.contains(&temp)
                || !expr_idents(line_trimmed).iter().any(|ident| ident == &temp)
            {
                continue;
            }
            let rewritten = ident_re
                .replace_all(line, |m: &Captures<'_>| {
                    let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                    if ident == temp {
                        lhs.to_string()
                    } else {
                        ident.to_string()
                    }
                })
                .to_string();
            if rewritten != *line {
                *line = rewritten;
            }
        }
    }
    out
}

fn inline_immediate_single_use_scalar_temps(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".__rr_cse_") || !expr_is_exact_reusable_scalar(rhs) {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..out.len()).find(|i| !out[*i].trim().is_empty()) else {
            continue;
        };
        let next_trimmed = out[next_idx].trim().to_string();
        if out[next_idx].contains("<- function")
            || is_control_flow_boundary(&next_trimmed)
            || !expr_idents(&next_trimmed).iter().any(|ident| ident == lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in out.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let rewritten = ident_re
            .replace_all(&out[next_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[next_idx] {
            out[next_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}

fn expr_is_simple_scalar_index_read(rhs: &str) -> bool {
    compile_regex(format!(r"^{}\[[^\],]+\]$", IDENT_PATTERN))
        .is_some_and(|re| re.is_match(rhs.trim()))
}

fn expr_is_inlineable_named_scalar_rhs(rhs: &str, pure_user_calls: &FxHashSet<String>) -> bool {
    let rhs = rhs.trim();
    expr_is_simple_scalar_index_read(rhs)
        || (rhs.starts_with("rr_")
            && rhs.contains('(')
            && !rhs.starts_with("rr_parallel_typed_vec_call(")
            && expr_has_only_pure_calls(rhs, pure_user_calls))
}

fn inline_single_use_named_scalar_index_reads_within_straight_line_region(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || !expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
            || line_is_within_loop_body(&out, idx)
        {
            continue;
        }
        let mut later_reassigned = false;
        for later_line in out.iter().skip(idx + 1) {
            let later_trimmed = later_line.trim();
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    later_reassigned = true;
                    break;
                }
            }
        }
        if later_reassigned {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let mut use_lines = Vec::new();
        let mut dep_write_lines = Vec::new();
        for (line_no, line) in out.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") || is_control_flow_boundary(line_trimmed) {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    if expr_idents(next_rhs).iter().any(|ident| ident == lhs) {
                        dep_write_lines.push(line_no);
                    }
                    break;
                }
                if deps.contains(next_lhs) {
                    dep_write_lines.push(line_no);
                }
            }
            if expr_idents(line_trimmed).iter().any(|ident| ident == lhs) {
                use_lines.push(line_no);
                if use_lines.len() > 1 {
                    break;
                }
            }
        }
        if use_lines.len() != 1 {
            continue;
        }
        let use_idx = use_lines[0];
        if dep_write_lines.iter().any(|dep_idx| *dep_idx < use_idx) {
            continue;
        }
        let mut used_after_region = false;
        for later_line in out.iter().skip(use_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after_region = true;
                break;
            }
        }
        if used_after_region {
            continue;
        }
        let rewritten = ident_re
            .replace_all(&out[use_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[use_idx] {
            out[use_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}

fn expr_is_inlineable_named_scalar_expr(rhs: &str, pure_user_calls: &FxHashSet<String>) -> bool {
    let rhs = rhs.trim();
    if rhs.is_empty()
        || plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || rhs.contains('"')
        || rhs.contains(',')
        || rhs.contains("Sym_")
        || rhs.starts_with("rr_parallel_typed_vec_call(")
    {
        return false;
    }
    !rhs.contains("rr_") || expr_has_only_pure_calls(rhs, pure_user_calls)
}

fn inline_immediate_single_use_named_scalar_exprs(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || !expr_is_inlineable_named_scalar_expr(rhs, pure_user_calls)
            || line_is_within_loop_body(&out, idx)
        {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..out.len()).find(|i| !out[*i].trim().is_empty()) else {
            continue;
        };
        let next_trimmed = out[next_idx].trim().to_string();
        if out[next_idx].contains("<- function")
            || is_control_flow_boundary(&next_trimmed)
            || !expr_idents(&next_trimmed).iter().any(|ident| ident == lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in out.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let rewritten = ident_re
            .replace_all(&out[next_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[next_idx] {
            out[next_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}

fn hoist_branch_local_named_scalar_assigns_used_after_branch(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    let mut idx = 0usize;
    while idx < out.len() {
        let trimmed = out[idx].trim();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            idx += 1;
            continue;
        }
        let guard_idents = expr_idents(trimmed);
        let Some(end_idx) = find_matching_block_end(&out, idx) else {
            break;
        };
        let mut trailing = Vec::new();
        let mut scan = end_idx;
        while scan > idx + 1 {
            scan -= 1;
            let trimmed_line = out[scan].trim();
            if trimmed_line.is_empty() {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed_line)) else {
                break;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
                || lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || !expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
            {
                break;
            }
            trailing.push((scan, lhs.to_string(), rhs.to_string()));
        }
        if trailing.is_empty() {
            idx = end_idx + 1;
            continue;
        }
        trailing.reverse();

        let mut hoisted = Vec::new();
        for (assign_idx, lhs, rhs) in trailing {
            if guard_idents.iter().any(|ident| ident == &lhs) {
                continue;
            }
            let deps: FxHashSet<String> = expr_idents(rhs.as_str()).into_iter().collect();
            let dep_written_in_branch = out
                .iter()
                .take(assign_idx)
                .skip(idx + 1)
                .filter_map(|line| {
                    assign_re()
                        .and_then(|re| re.captures(line.trim()))
                        .map(|caps| {
                            caps.name("lhs")
                                .map(|m| m.as_str())
                                .unwrap_or("")
                                .trim()
                                .to_string()
                        })
                })
                .any(|branch_lhs| deps.contains(&branch_lhs));
            if dep_written_in_branch {
                continue;
            }

            let mut used_after = false;
            for later_line in out.iter().skip(end_idx + 1) {
                let later_trimmed = later_line.trim();
                if later_line.contains("<- function") {
                    break;
                }
                if let Some(caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                    let later_lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    if later_lhs == lhs {
                        break;
                    }
                }
                if expr_idents(later_trimmed).iter().any(|ident| ident == &lhs) {
                    used_after = true;
                    break;
                }
            }
            if used_after {
                hoisted.push(out[assign_idx].clone());
                out[assign_idx].clear();
            }
        }

        if !hoisted.is_empty() {
            for (offset, line) in hoisted.into_iter().enumerate() {
                out.insert(idx + offset, line);
            }
            idx = end_idx + 1;
            continue;
        }

        idx = end_idx + 1;
    }
    out
}

fn inline_two_use_named_scalar_index_reads_within_straight_line_region(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || !expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
            || line_is_within_loop_body(&out, idx)
        {
            continue;
        }
        let mut later_reassigned = false;
        for later_line in out.iter().skip(idx + 1) {
            let later_trimmed = later_line.trim();
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    later_reassigned = true;
                    break;
                }
            }
        }
        if later_reassigned {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let mut use_lines = Vec::new();
        let mut dep_write_lines = Vec::new();
        for (line_no, line) in out.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") || is_control_flow_boundary(line_trimmed) {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    if expr_idents(next_rhs).iter().any(|ident| ident == lhs) {
                        dep_write_lines.push(line_no);
                    }
                    break;
                }
                if deps.contains(next_lhs) {
                    dep_write_lines.push(line_no);
                }
            }
            if expr_idents(line_trimmed).iter().any(|ident| ident == lhs) {
                use_lines.push(line_no);
                if use_lines.len() > 2 {
                    break;
                }
            }
        }
        if use_lines.is_empty() || use_lines.len() > 2 {
            continue;
        }
        let last_use = *use_lines.last().unwrap_or(&idx);
        if dep_write_lines.iter().any(|dep_idx| *dep_idx < last_use) {
            continue;
        }
        let mut used_after_region = false;
        for later_line in out.iter().skip(last_use + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(later_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let later_lhs = later_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if later_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after_region = true;
                break;
            }
        }
        if used_after_region {
            continue;
        }
        let mut changed = false;
        for use_idx in use_lines {
            let rewritten = ident_re
                .replace_all(&out[use_idx], |m: &Captures<'_>| {
                    let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                    if ident == lhs {
                        rhs.to_string()
                    } else {
                        ident.to_string()
                    }
                })
                .to_string();
            if rewritten != out[use_idx] {
                out[use_idx] = rewritten;
                changed = true;
            }
        }
        if changed {
            out[idx].clear();
        }
    }
    out
}

fn inline_single_use_scalar_temps_within_straight_line_region(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".__rr_cse_") || !expr_is_exact_reusable_scalar(rhs) {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let mut use_lines = Vec::new();
        let mut blocked = false;
        for (line_no, line) in out.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") || is_control_flow_boundary(line_trimmed) {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    break;
                }
                if deps.contains(next_lhs) {
                    blocked = true;
                    break;
                }
            }
            if expr_idents(line_trimmed).iter().any(|ident| ident == lhs) {
                use_lines.push(line_no);
                if use_lines.len() > 1 {
                    break;
                }
            }
        }
        if blocked || use_lines.len() != 1 {
            continue;
        }
        let use_idx = use_lines[0];
        let mut used_after_region = false;
        for later_line in out.iter().skip(use_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after_region = true;
                break;
            }
        }
        if used_after_region {
            continue;
        }
        let rewritten = ident_re
            .replace_all(&out[use_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[use_idx] {
            out[use_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}

fn inline_two_use_scalar_temps_within_straight_line_region(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".__rr_cse_") || !expr_is_exact_reusable_scalar(rhs) {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let mut use_lines = Vec::new();
        let mut blocked = false;
        for (line_no, line) in out.iter().enumerate().skip(idx + 1) {
            let line_trimmed = line.trim();
            if line_trimmed.is_empty() {
                continue;
            }
            if line.contains("<- function") || is_control_flow_boundary(line_trimmed) {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    break;
                }
                if deps.contains(next_lhs) {
                    blocked = true;
                    break;
                }
            }
            if expr_idents(line_trimmed).iter().any(|ident| ident == lhs) {
                use_lines.push(line_no);
                if use_lines.len() > 2 {
                    break;
                }
            }
        }
        if blocked || use_lines.is_empty() || use_lines.len() > 2 {
            continue;
        }
        let last_use = *use_lines.last().unwrap_or(&idx);
        let mut used_after_region = false;
        for later_line in out.iter().skip(last_use + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if let Some(next_caps) = assign_re().and_then(|re| re.captures(later_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if next_lhs == lhs {
                    break;
                }
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after_region = true;
                break;
            }
        }
        if used_after_region {
            continue;
        }
        let mut changed = false;
        for use_idx in use_lines {
            let rewritten = ident_re
                .replace_all(&out[use_idx], |m: &Captures<'_>| {
                    let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                    if ident == lhs {
                        rhs.to_string()
                    } else {
                        ident.to_string()
                    }
                })
                .to_string();
            if rewritten != out[use_idx] {
                out[use_idx] = rewritten;
                changed = true;
            }
        }
        if changed {
            out[idx].clear();
        }
    }
    out
}

fn inline_immediate_single_use_index_temps(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".__rr_cse_") || !rhs.starts_with("rr_index_vec_floor(") {
            continue;
        }

        let Some(next_idx) = ((idx + 1)..out.len()).find(|i| !out[*i].trim().is_empty()) else {
            continue;
        };
        let next_trimmed = out[next_idx].trim().to_string();
        if out[next_idx].contains("<- function")
            || is_control_flow_boundary(&next_trimmed)
            || !expr_idents(&next_trimmed).iter().any(|ident| ident == lhs)
        {
            continue;
        }

        let mut used_after = false;
        for later_line in out.iter().skip(next_idx + 1) {
            let later_trimmed = later_line.trim();
            if later_trimmed.is_empty() {
                continue;
            }
            if later_line.contains("<- function") {
                break;
            }
            if expr_idents(later_trimmed).iter().any(|ident| ident == lhs) {
                used_after = true;
                break;
            }
        }
        if used_after {
            continue;
        }

        let rewritten = ident_re
            .replace_all(&out[next_idx], |m: &Captures<'_>| {
                let ident = m.get(0).map(|mm| mm.as_str()).unwrap_or("");
                if ident == lhs {
                    rhs.to_string()
                } else {
                    ident.to_string()
                }
            })
            .to_string();
        if rewritten != out[next_idx] {
            out[next_idx] = rewritten;
            out[idx].clear();
        }
    }
    out
}

fn expr_is_exact_reusable_scalar(rhs: &str) -> bool {
    let rhs = rhs.trim();
    let has_runtime_helper = rhs.contains("rr_") && !rhs.contains(".__rr_cse_");
    if rhs.is_empty()
        || plain_ident_re().is_some_and(|re| re.is_match(rhs))
        || scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        || has_runtime_helper
        || rhs.contains('[')
        || rhs.contains('"')
        || rhs.contains(',')
        || rhs.contains("Sym_")
    {
        return false;
    }
    true
}

fn rewrite_forward_exact_pure_call_reuse(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    let len = out.len();
    for idx in 0..len {
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
        let deps: FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
        if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
            || lhs.starts_with(".arg_")
            || lhs.starts_with(".__rr_cse_")
            || deps.contains(&lhs)
            || !rhs.contains('(')
            || !expr_has_only_pure_calls(&rhs, pure_user_calls)
        {
            continue;
        }

        let lhs_reassigned_later = (idx + 1..out.len()).any(|scan_idx| {
            let scan_trimmed = out[scan_idx].trim();
            let scan_indent = out[scan_idx].len() - out[scan_idx].trim_start().len();
            if !scan_trimmed.is_empty() && scan_indent < candidate_indent {
                return false;
            }
            if out[scan_idx].contains("<- function")
                || scan_trimmed == "repeat {"
                || scan_trimmed.starts_with("while")
                || scan_trimmed.starts_with("for")
            {
                return false;
            }
            assign_re()
                .and_then(|re| re.captures(scan_trimmed))
                .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()))
                .is_some_and(|scan_lhs| scan_lhs == lhs)
        });

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
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if next_lhs == lhs {
                    break;
                }
                if next_rhs.contains(&rhs) {
                    if lhs_reassigned_later {
                        line_no += 1;
                        continue;
                    }
                    out[line_no] = out[line_no].replacen(&rhs, &lhs, usize::MAX);
                }
                if deps.contains(&next_lhs) {
                    break;
                }
                line_no += 1;
                continue;
            }

            if line_trimmed.contains(&rhs) {
                out[line_no] = out[line_no].replacen(&rhs, &lhs, usize::MAX);
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

fn rewrite_adjacent_duplicate_pure_call_assignments(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    if out.len() < 2 {
        return out;
    }

    for idx in 0..(out.len() - 1) {
        let first = out[idx].trim().to_string();
        let second = out[idx + 1].trim().to_string();
        let Some(caps0) = assign_re().and_then(|re| re.captures(&first)) else {
            continue;
        };
        let Some(caps1) = assign_re().and_then(|re| re.captures(&second)) else {
            continue;
        };
        let lhs0 = caps0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs0 = caps0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let lhs1 = caps1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs1 = caps1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs0))
            || !plain_ident_re().is_some_and(|re| re.is_match(lhs1))
            || lhs0.starts_with(".arg_")
            || lhs1.starts_with(".arg_")
            || lhs0.starts_with(".__rr_cse_")
            || lhs1.starts_with(".__rr_cse_")
            || lhs0 == lhs1
            || rhs0 != rhs1
            || !rhs0.contains('(')
            || !expr_has_only_pure_calls(rhs0, pure_user_calls)
        {
            continue;
        }

        let indent = out[idx + 1]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        out[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
    }

    out
}

fn rewrite_adjacent_duplicate_symbol_assignments(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    if out.len() < 2 {
        return out;
    }

    for idx in 0..(out.len() - 1) {
        let first = out[idx].trim().to_string();
        let second = out[idx + 1].trim().to_string();
        let Some(caps0) = assign_re().and_then(|re| re.captures(&first)) else {
            continue;
        };
        let Some(caps1) = assign_re().and_then(|re| re.captures(&second)) else {
            continue;
        };
        let lhs0 = caps0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs0 = caps0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let lhs1 = caps1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs1 = caps1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs0))
            || !plain_ident_re().is_some_and(|re| re.is_match(lhs1))
            || !plain_ident_re().is_some_and(|re| re.is_match(rhs0))
            || lhs0.starts_with(".arg_")
            || lhs1.starts_with(".arg_")
            || rhs0.starts_with(".arg_")
            || lhs0.starts_with(".__rr_cse_")
            || lhs1.starts_with(".__rr_cse_")
            || lhs0 == lhs1
            || rhs0 != rhs1
        {
            continue;
        }

        let indent = out[idx + 1]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        out[idx + 1] = format!("{indent}{lhs1} <- {lhs0}");
    }

    out
}

fn find_matching_open_brace_line_in_emitted(lines: &[String], close_idx: usize) -> Option<usize> {
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate().take(close_idx + 1) {
        for ch in line.chars() {
            match ch {
                '{' => stack.push(idx),
                '}' => {
                    let open = stack.pop()?;
                    if idx == close_idx {
                        return Some(open);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn strip_terminal_repeat_nexts(lines: Vec<String>) -> Vec<String> {
    if lines.len() < 2 {
        return lines;
    }
    let mut out = Vec::with_capacity(lines.len());
    for idx in 0..lines.len() {
        if lines[idx].trim() == "next"
            && idx + 1 < lines.len()
            && lines[idx + 1].trim() == "}"
            && find_matching_open_brace_line_in_emitted(&lines, idx + 1)
                .is_some_and(|open_idx| lines[open_idx].trim() == "repeat {")
        {
            continue;
        }
        out.push(lines[idx].clone());
    }
    out
}

fn same_var_is_na_or_not_finite_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r"is\.na\((?P<lhs>[A-Za-z_][A-Za-z0-9_]*)\)\s*\|\s*\(!\(is\.finite\((?P<rhs>[A-Za-z_][A-Za-z0-9_]*)\)\)\)"
                .to_string(),
        )
    })
    .as_ref()
}

fn wrapped_not_finite_cond_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(r"\(\((?P<inner>!\(is\.finite\([A-Za-z_][A-Za-z0-9_]*\)\))\)\)".to_string())
    })
    .as_ref()
}

fn not_finite_or_zero_guard_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(
            r"\(\(\((?P<inner>!\(is\.finite\((?P<lhs>[A-Za-z_][A-Za-z0-9_]*)\)\))\)\s*\|\s*\((?P<rhs>[A-Za-z_][A-Za-z0-9_]*) == 0\)\)\)".to_string(),
        )
    })
    .as_ref()
}

fn simplify_same_var_is_na_or_not_finite_guards(lines: Vec<String>) -> Vec<String> {
    let Some(re) = same_var_is_na_or_not_finite_re() else {
        return lines;
    };
    lines
        .into_iter()
        .map(|line| {
            re.replace_all(&line, |caps: &Captures<'_>| {
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
                if lhs == rhs {
                    format!("!(is.finite({lhs}))")
                } else {
                    caps.get(0)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default()
                }
            })
            .into_owned()
        })
        .collect()
}

fn simplify_wrapped_not_finite_parens(lines: Vec<String>) -> Vec<String> {
    let Some(re) = wrapped_not_finite_cond_re() else {
        return lines;
    };
    lines
        .into_iter()
        .map(|line| {
            re.replace_all(&line, |caps: &Captures<'_>| {
                let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
                format!("({inner})")
            })
            .into_owned()
        })
        .collect()
}

fn simplify_not_finite_or_zero_guard_parens(lines: Vec<String>) -> Vec<String> {
    let Some(re) = not_finite_or_zero_guard_re() else {
        return lines;
    };
    lines
        .into_iter()
        .map(|line| {
            re.replace_all(&line, |caps: &Captures<'_>| {
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
                let inner = caps.name("inner").map(|m| m.as_str()).unwrap_or("");
                if lhs == rhs {
                    format!("(({inner} | ({rhs} == 0)))")
                } else {
                    caps.get(0)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default()
                }
            })
            .into_owned()
        })
        .collect()
}

fn restore_missing_scalar_loop_increments(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut idx = 0usize;
    while idx < out.len() {
        if out[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }

        let mut depth = 0usize;
        let mut loop_end = None;
        for (j, line) in out.iter().enumerate().skip(idx) {
            for ch in line.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' if depth > 0 => depth -= 1,
                    _ => {}
                }
            }
            if depth == 0 {
                loop_end = Some(j);
                break;
            }
        }
        let Some(loop_end) = loop_end else {
            idx += 1;
            continue;
        };

        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = out[*line_idx].trim();
            trimmed.starts_with("if (!(") && trimmed.ends_with(")) break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let guard = out[guard_idx].trim();
        let Some(inner) = guard
            .strip_prefix("if (!(")
            .and_then(|s| s.strip_suffix(")) break"))
        else {
            idx = loop_end + 1;
            continue;
        };
        let Some((lhs, _rhs)) = inner.split_once("<=") else {
            idx = loop_end + 1;
            continue;
        };
        let idx_var = lhs.trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(idx_var)) {
            idx = loop_end + 1;
            continue;
        }

        let mut already_updates_idx = false;
        let mut has_body_use = false;
        for line in out.iter().take(loop_end).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with(&format!("{idx_var} <-")) {
                already_updates_idx = true;
                break;
            }
            if trimmed.contains(&format!("[{idx_var}]"))
                || expr_idents(trimmed).iter().any(|ident| ident == idx_var)
            {
                has_body_use = true;
            }
        }
        if already_updates_idx || !has_body_use {
            idx = loop_end + 1;
            continue;
        }

        let inner_indent = out[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        out.insert(
            loop_end,
            format!("{inner_indent}{idx_var} <- ({idx_var} + 1L)"),
        );
        idx = loop_end + 2;
    }
    out
}

fn restore_constant_one_guard_repeat_loop_counters(lines: Vec<String>) -> Vec<String> {
    fn parse_constant_guard(line: &str) -> Option<(String, String, String)> {
        let trimmed = line.trim();
        let inner = trimmed
            .strip_prefix("if (!(")
            .or_else(|| trimmed.strip_prefix("if !("))?
            .strip_suffix(")) break")
            .or_else(|| {
                trimmed
                    .strip_prefix("if (!(")
                    .or_else(|| trimmed.strip_prefix("if !("))
                    .and_then(|s| s.strip_suffix(") break"))
            })?
            .trim();
        for op in ["<=", "<"] {
            let needle = format!(" {op} ");
            let Some((lhs, rhs)) = inner.split_once(&needle) else {
                continue;
            };
            let lhs = lhs.trim().trim_matches(|ch| ch == '(' || ch == ')');
            let rhs = rhs.trim();
            if lhs.is_empty() || rhs.is_empty() {
                continue;
            }
            let numeric = lhs.trim_end_matches('L').trim_end_matches('l');
            if numeric.parse::<f64>().ok().is_some() {
                return Some((lhs.to_string(), op.to_string(), rhs.to_string()));
            }
        }
        None
    }

    let mut out = lines;
    let mut idx = 0usize;
    while idx < out.len() {
        if out[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }

        let mut depth = 0usize;
        let mut loop_end = None;
        for (j, line) in out.iter().enumerate().skip(idx) {
            for ch in line.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' if depth > 0 => depth -= 1,
                    _ => {}
                }
            }
            if depth == 0 {
                loop_end = Some(j);
                break;
            }
        }
        let Some(loop_end) = loop_end else {
            idx += 1;
            continue;
        };

        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = out[*line_idx].trim();
            (trimmed.starts_with("if !(") || trimmed.starts_with("if (!("))
                && trimmed.ends_with("break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let Some((start_lit, cmp, bound)) = parse_constant_guard(&out[guard_idx]) else {
            idx = loop_end + 1;
            continue;
        };

        let idx_var = ".__rr_i";
        let body_mentions_idx = out.iter().take(loop_end).skip(guard_idx + 1).any(|line| {
            expr_idents(line.trim())
                .iter()
                .any(|ident| ident == idx_var)
        });
        if body_mentions_idx {
            idx = loop_end + 1;
            continue;
        }

        let indent = out[guard_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let repeat_indent = out[idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();

        out.insert(idx, format!("{repeat_indent}{idx_var} <- {start_lit}"));
        let guard_line = if cmp == "<=" {
            format!("{indent}if (!({idx_var} <= {bound})) break")
        } else {
            format!("{indent}if (!({idx_var} < {bound})) break")
        };
        out[guard_idx + 1] = guard_line;
        let one = if start_lit.contains('.') {
            "1.0"
        } else if start_lit.ends_with('L') || start_lit.ends_with('l') {
            "1L"
        } else {
            "1"
        };
        out.insert(
            loop_end + 1,
            format!("{indent}{idx_var} <- ({idx_var} + {one})"),
        );
        idx = loop_end + 3;
    }

    out
}

fn restore_missing_scalar_loop_next_increments(lines: Vec<String>) -> Vec<String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum BlockKind {
        Loop,
        Other,
    }

    fn count_loops(stack: &[BlockKind]) -> usize {
        stack
            .iter()
            .filter(|kind| matches!(kind, BlockKind::Loop))
            .count()
    }

    fn leading_close_count(line: &str) -> usize {
        line.chars().take_while(|ch| *ch == '}').count()
    }

    let mut out = lines;
    let mut idx = 0usize;
    while idx < out.len() {
        if out[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }

        let mut depth = 0usize;
        let mut loop_end = None;
        for (j, line) in out.iter().enumerate().skip(idx) {
            for ch in line.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' if depth > 0 => depth -= 1,
                    _ => {}
                }
            }
            if depth == 0 {
                loop_end = Some(j);
                break;
            }
        }
        let Some(loop_end) = loop_end else {
            idx += 1;
            continue;
        };

        let Some(guard_idx) = ((idx + 1)..loop_end).find(|line_idx| {
            let trimmed = out[*line_idx].trim();
            trimmed.starts_with("if (!(") && trimmed.ends_with(")) break")
        }) else {
            idx = loop_end + 1;
            continue;
        };
        let guard = out[guard_idx].trim();
        let Some(inner) = guard
            .strip_prefix("if (!(")
            .and_then(|s| s.strip_suffix(")) break"))
        else {
            idx = loop_end + 1;
            continue;
        };
        let Some((lhs, _rhs)) = inner.split_once("<=") else {
            idx = loop_end + 1;
            continue;
        };
        let idx_var = lhs.trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(idx_var)) {
            idx = loop_end + 1;
            continue;
        }

        let mut block_stack = vec![BlockKind::Loop];
        let mut block_depth_before = vec![0usize; out.len()];
        let mut loop_depth_before = vec![0usize; out.len()];
        for line_idx in (idx + 1)..loop_end {
            let trimmed_start = out[line_idx].trim_start();
            let close_count = leading_close_count(trimmed_start);
            for _ in 0..close_count {
                let _ = block_stack.pop();
            }
            block_depth_before[line_idx] = block_stack.len();
            loop_depth_before[line_idx] = count_loops(&block_stack);

            let remainder = trimmed_start[close_count..].trim_start();
            if remainder.ends_with('{') {
                let kind = if remainder == "repeat {"
                    || remainder.starts_with("for ")
                    || remainder.starts_with("while ")
                {
                    BlockKind::Loop
                } else {
                    BlockKind::Other
                };
                block_stack.push(kind);
            }
        }

        let mut insertions = Vec::new();
        for line_idx in (guard_idx + 1)..loop_end {
            if out[line_idx].trim() != "next" || loop_depth_before[line_idx] != 1 {
                continue;
            }

            let branch_depth = block_depth_before[line_idx];
            let mut saw_idx_update = false;
            let mut scan = line_idx;
            while scan > guard_idx + 1 {
                scan -= 1;
                let trimmed = out[scan].trim();
                if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
                    continue;
                }
                if block_depth_before[scan] < branch_depth {
                    break;
                }
                if trimmed.starts_with(&format!("{idx_var} <-")) {
                    saw_idx_update = true;
                    break;
                }
            }
            if saw_idx_update {
                continue;
            }

            let indent = out[line_idx]
                .chars()
                .take_while(|ch| ch.is_ascii_whitespace())
                .collect::<String>();
            insertions.push((line_idx, format!("{indent}{idx_var} <- ({idx_var} + 1L)")));
        }

        if insertions.is_empty() {
            idx = loop_end + 1;
            continue;
        }

        for (line_idx, increment) in insertions.into_iter().rev() {
            out.insert(line_idx, increment);
        }
        idx = loop_end + 1;
    }
    out
}

fn rewrite_canonical_counted_repeat_loops_to_for(lines: Vec<String>) -> Vec<String> {
    fn block_end(lines: &[String], start: usize) -> Option<usize> {
        let mut depth = 0usize;
        for (idx, line) in lines.iter().enumerate().skip(start) {
            for ch in line.chars() {
                match ch {
                    '{' => depth += 1,
                    '}' if depth > 0 => depth -= 1,
                    _ => {}
                }
            }
            if depth == 0 {
                return Some(idx);
            }
        }
        None
    }

    fn next_significant_line(lines: &[String], start: usize, end: usize) -> Option<usize> {
        (start..end).find(|idx| {
            let trimmed = lines[*idx].trim();
            !trimmed.is_empty() && !trimmed.starts_with("rr_mark(")
        })
    }

    fn prev_significant_line(lines: &[String], start: usize, end: usize) -> Option<usize> {
        (start..end).rev().find(|idx| {
            let trimmed = lines[*idx].trim();
            !trimmed.is_empty() && !trimmed.starts_with("rr_mark(")
        })
    }

    fn is_canonical_increment(line: &str, idx_var: &str) -> bool {
        let trimmed = line.trim();
        trimmed == format!("{idx_var} <- ({idx_var} + 1)")
            || trimmed == format!("{idx_var} <- ({idx_var} + 1L)")
    }

    fn assigns_var(line: &str, var: &str) -> bool {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == var)
            || indexed_store_base_re()
                .and_then(|re| re.captures(line.trim()))
                .is_some_and(|caps| {
                    caps.name("base").map(|m| m.as_str()).unwrap_or("").trim() == var
                })
    }

    fn references_var(line: &str, var: &str) -> bool {
        expr_idents(line).iter().any(|ident| ident == var)
    }

    let mut out = lines;
    let mut idx = 0usize;
    while idx < out.len() {
        if out[idx].trim() != "repeat {" {
            idx += 1;
            continue;
        }

        let repeat_idx = idx;
        let Some(loop_end) = block_end(&out, repeat_idx) else {
            idx += 1;
            continue;
        };

        let Some(guard_idx) = next_significant_line(&out, repeat_idx + 1, loop_end) else {
            idx = loop_end + 1;
            continue;
        };
        let Some((idx_var, end_expr)) = parse_break_guard(&out[guard_idx]) else {
            idx = loop_end + 1;
            continue;
        };
        if !plain_ident_re().is_some_and(|re| re.is_match(&idx_var)) {
            idx = loop_end + 1;
            continue;
        }

        let Some(mut last_sig_idx) = prev_significant_line(&out, repeat_idx + 1, loop_end) else {
            idx = loop_end + 1;
            continue;
        };
        let had_trailing_next = out[last_sig_idx].trim() == "next";
        if had_trailing_next {
            let Some(prev_idx) = prev_significant_line(&out, repeat_idx + 1, last_sig_idx) else {
                idx = loop_end + 1;
                continue;
            };
            last_sig_idx = prev_idx;
        }
        let incr_idx = last_sig_idx;
        if !is_canonical_increment(&out[incr_idx], &idx_var) {
            idx = loop_end + 1;
            continue;
        }

        let mut init_idx = None;
        let mut scan = repeat_idx;
        while scan > 0 {
            let prev_idx = scan - 1;
            let trimmed = out[prev_idx].trim();
            if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
                scan -= 1;
                continue;
            }
            if out[prev_idx].contains("<- function") || is_control_flow_boundary(trimmed) {
                break;
            }
            if assign_re()
                .and_then(|re| re.captures(trimmed))
                .is_some_and(|caps| {
                    caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == idx_var
                        && literal_one_re().is_some_and(|re| {
                            re.is_match(caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim())
                        })
                })
            {
                init_idx = Some(prev_idx);
                break;
            }
            scan -= 1;
        }
        let Some(init_idx) = init_idx else {
            idx = loop_end + 1;
            continue;
        };

        let prefix_references_idx = out[(init_idx + 1)..repeat_idx].iter().any(|line| {
            !line.trim().is_empty()
                && !line.trim().starts_with("rr_mark(")
                && references_var(line, &idx_var)
        });
        if prefix_references_idx {
            idx = loop_end + 1;
            continue;
        }

        let end_expr_idents = expr_idents(&end_expr);
        let mut invalid = false;
        for line in out.iter().take(incr_idx).skip(guard_idx + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
                continue;
            }
            if assigns_var(line, &idx_var) || trimmed == "next" {
                invalid = true;
                break;
            }
            if end_expr_idents.iter().any(|ident| assigns_var(line, ident)) {
                invalid = true;
                break;
            }
        }
        if invalid {
            idx = loop_end + 1;
            continue;
        }

        for line in out.iter().skip(loop_end + 1) {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("rr_mark(") {
                continue;
            }
            if assigns_var(line, &idx_var) {
                break;
            }
            if references_var(line, &idx_var) {
                invalid = true;
            }
            break;
        }
        if invalid {
            idx = loop_end + 1;
            continue;
        }

        let indent = out[repeat_idx]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();
        let mut replacement = Vec::new();
        replacement.extend(out[(init_idx + 1)..repeat_idx].iter().cloned());
        replacement.push(format!("{indent}for ({idx_var} in seq_len({end_expr})) {{"));
        replacement.extend(out[(guard_idx + 1)..incr_idx].iter().cloned());
        replacement.push(format!("{indent}}}"));

        out.splice(init_idx..=loop_end, replacement);
        idx = init_idx + 1;
    }

    out
}

fn hoist_loop_invariant_pure_assignments_from_counted_repeat_loops(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum BlockKind {
        Loop,
        Other,
    }

    fn count_loops(stack: &[BlockKind]) -> usize {
        stack
            .iter()
            .filter(|kind| matches!(kind, BlockKind::Loop))
            .count()
    }

    fn leading_close_count(line: &str) -> usize {
        line.chars().take_while(|ch| *ch == '}').count()
    }

    fn bound_literal_before(lines: &[String], idx: usize, bound: &str) -> Option<i64> {
        literal_integer_value(bound).or_else(|| {
            plain_ident_re()
                .is_some_and(|re| re.is_match(bound))
                .then(|| latest_literal_assignment_before(lines, idx, bound))
                .flatten()
        })
    }

    fn assigns_var(line: &str, var: &str) -> bool {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .is_some_and(|caps| caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim() == var)
            || indexed_store_base_re()
                .and_then(|re| re.captures(line.trim()))
                .is_some_and(|caps| {
                    caps.name("base").map(|m| m.as_str()).unwrap_or("").trim() == var
                })
    }

    fn references_var(line: &str, var: &str) -> bool {
        expr_idents(line).iter().any(|ident| ident == var)
    }

    let mut out = lines;
    let mut repeat_idx = 0usize;
    while repeat_idx < out.len() {
        let Some(next_repeat) = (repeat_idx..out.len()).find(|idx| out[*idx].trim() == "repeat {")
        else {
            break;
        };
        let Some(loop_end) = find_matching_block_end(&out, next_repeat) else {
            break;
        };
        let Some(guard_idx) = (next_repeat + 1..loop_end).find(|idx| {
            let trimmed = out[*idx].trim();
            !trimmed.is_empty() && !trimmed.starts_with("rr_mark(")
        }) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some((iter_var, op, bound)) = parse_repeat_guard_cmp_line(out[guard_idx].trim()) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some(start_value) = latest_literal_assignment_before(&out, guard_idx, &iter_var) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let Some(bound_value) = bound_literal_before(&out, guard_idx, &bound) else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        let trip_count = if op == "<=" {
            bound_value - start_value + 1
        } else if op == "<" {
            bound_value - start_value
        } else {
            repeat_idx = next_repeat + 1;
            continue;
        };
        if trip_count <= 0 || !plain_ident_re().is_some_and(|re| re.is_match(&iter_var)) {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let mut block_stack = vec![BlockKind::Loop];
        let mut block_depth_before = vec![0usize; out.len()];
        let mut loop_depth_before = vec![0usize; out.len()];
        for line_idx in (next_repeat + 1)..loop_end {
            let trimmed_start = out[line_idx].trim_start();
            let close_count = leading_close_count(trimmed_start);
            for _ in 0..close_count {
                let _ = block_stack.pop();
            }
            block_depth_before[line_idx] = block_stack.len();
            loop_depth_before[line_idx] = count_loops(&block_stack);

            let remainder = trimmed_start[close_count..].trim_start();
            if remainder.ends_with('{') {
                let kind = if remainder == "repeat {"
                    || remainder.starts_with("for ")
                    || remainder.starts_with("while ")
                {
                    BlockKind::Loop
                } else {
                    BlockKind::Other
                };
                block_stack.push(kind);
            }
        }

        let mut hoists = Vec::<(usize, String)>::new();
        for line_idx in (guard_idx + 1)..loop_end {
            if loop_depth_before[line_idx] != 1 || block_depth_before[line_idx] != 1 {
                continue;
            }
            let trimmed = out[line_idx].trim();
            if trimmed.is_empty() || trimmed.starts_with("rr_mark(") || trimmed == "next" {
                continue;
            }
            let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
                continue;
            };
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
                || !expr_has_only_pure_calls(rhs, pure_user_calls)
            {
                continue;
            }

            let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
            if deps.contains(&iter_var) || deps.contains(lhs) {
                continue;
            }

            let mut safe = true;
            for (scan_idx, scan_line) in out.iter().enumerate().take(loop_end).skip(guard_idx + 1) {
                if scan_idx == line_idx {
                    continue;
                }
                let scan_trimmed = scan_line.trim();
                if scan_trimmed.is_empty() || scan_trimmed.starts_with("rr_mark(") {
                    continue;
                }
                if assigns_var(scan_line, lhs) || deps.iter().any(|dep| assigns_var(scan_line, dep))
                {
                    safe = false;
                    break;
                }
            }
            if !safe {
                continue;
            }

            let lhs_used_before = out
                .iter()
                .take(line_idx)
                .skip(guard_idx + 1)
                .any(|line| references_var(line, lhs));
            if lhs_used_before {
                continue;
            }

            let lhs_used_later = out
                .iter()
                .take(loop_end)
                .skip(line_idx + 1)
                .any(|line| references_var(line, lhs));
            if !lhs_used_later {
                continue;
            }

            hoists.push((line_idx, out[line_idx].clone()));
        }

        if hoists.is_empty() {
            repeat_idx = next_repeat + 1;
            continue;
        }

        let mut insert_at = next_repeat;
        for line_idx in hoists.iter().rev().map(|(line_idx, _)| *line_idx) {
            out.remove(line_idx);
        }
        for (_, line) in &hoists {
            out.insert(insert_at, line.clone());
            insert_at += 1;
        }
        repeat_idx = insert_at + 1;
    }

    out
}

fn parse_singleton_list_match_cond(line: &str) -> Option<String> {
    let pattern = format!(
        r#"^if \(\(\(length\((?P<base>{})\) == 1L\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some(caps.name("base")?.as_str().to_string())
}

fn parse_single_field_record_match_cond(line: &str) -> Option<(String, String)> {
    let pattern = format!(
        r#"^if \(\(\(TRUE & rr_field_exists\((?P<base>{}), "(?P<field>[^"]+)"\)\) & TRUE\)\) \{{$"#,
        IDENT_PATTERN
    );
    let caps = compile_regex(pattern)?.captures(line.trim())?;
    Some((
        caps.name("base")?.as_str().to_string(),
        caps.name("field")?.as_str().to_string(),
    ))
}

fn restore_empty_match_single_bind_arms(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut idx = 0usize;
    while idx + 3 < out.len() {
        if out[idx + 1].trim() != "} else {" || out[idx + 3].trim() != "}" {
            idx += 1;
            continue;
        }

        let Some(phi_caps) = assign_re().and_then(|re| re.captures(out[idx + 2].trim())) else {
            idx += 1;
            continue;
        };
        let phi_lhs = phi_caps
            .name("lhs")
            .map(|m| m.as_str())
            .unwrap_or("")
            .trim();
        if !phi_lhs.starts_with(".phi_") {
            idx += 1;
            continue;
        }

        let indent = out[idx + 2]
            .chars()
            .take_while(|ch| ch.is_ascii_whitespace())
            .collect::<String>();

        if let Some(base) = parse_singleton_list_match_cond(&out[idx]) {
            out.insert(idx + 1, format!("{indent}{phi_lhs} <- {base}[1L]"));
            idx += 5;
            continue;
        }

        if let Some((base, field)) = parse_single_field_record_match_cond(&out[idx]) {
            out.insert(
                idx + 1,
                format!("{indent}{phi_lhs} <- {base}[[\"{field}\"]]"),
            );
            idx += 5;
            continue;
        }

        idx += 1;
    }
    out
}

fn rewrite_dead_zero_loop_seeds_before_for(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let mut idx = 0usize;

    while idx + 1 < out.len() {
        let trimmed = out[idx].trim();
        let Some((var, seed)) = trimmed.split_once("<-") else {
            idx += 1;
            continue;
        };
        let var = var.trim();
        let seed = seed.trim();
        if seed != "0" && seed != "1" {
            idx += 1;
            continue;
        }

        let Some(for_idx) = ((idx + 1)..out.len()).take(12).find(|line_idx| {
            out[*line_idx]
                .trim()
                .starts_with(&format!("for ({var} in seq_len("))
        }) else {
            idx += 1;
            continue;
        };

        let var_re = regex::Regex::new(&format!(r"\b{}\b", regex::escape(var))).ok();
        let used_before_for = out[(idx + 1)..for_idx]
            .iter()
            .any(|line| var_re.as_ref().is_some_and(|re| re.is_match(line)));
        if used_before_for {
            idx += 1;
            continue;
        }

        out.remove(idx);
    }

    out
}

fn rewrite_forward_exact_expr_reuse(lines: Vec<String>) -> Vec<String> {
    let mut out = lines;
    let len = out.len();
    let debug = std::env::var_os("RR_DEBUG_PEEPHOLE").is_some();
    for idx in 0..len {
        let prologue_arg_aliases = collect_prologue_arg_aliases(&out, idx);
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
        if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
            eprintln!(
                "RR_DEBUG_PEEPHOLE exact_expr_candidate line={} lhs={} rhs={}",
                idx + 1,
                lhs,
                rhs
            );
        }
        if !plain_ident_re().is_some_and(|re| re.is_match(&lhs))
            || !expr_is_exact_reusable_scalar(&rhs)
        {
            if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                eprintln!(
                    "RR_DEBUG_PEEPHOLE exact_expr_skip line={} lhs={} reusable={}",
                    idx + 1,
                    lhs,
                    expr_is_exact_reusable_scalar(&rhs)
                );
            }
            continue;
        }

        let lhs_reassigned_later = (idx + 1..out.len()).any(|scan_idx| {
            let scan_trimmed = out[scan_idx].trim();
            let scan_indent = out[scan_idx].len() - out[scan_idx].trim_start().len();
            if !scan_trimmed.is_empty() && scan_indent < candidate_indent {
                return false;
            }
            if out[scan_idx].contains("<- function")
                || scan_trimmed == "repeat {"
                || scan_trimmed.starts_with("while")
                || scan_trimmed.starts_with("for")
            {
                return false;
            }
            assign_re()
                .and_then(|re| re.captures(scan_trimmed))
                .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()))
                .is_some_and(|scan_lhs| scan_lhs == lhs)
        });

        let deps: FxHashSet<String> = expr_idents(&rhs).into_iter().collect();
        for line_no in idx + 1..out.len() {
            let line_trimmed = out[line_no].trim().to_string();
            let next_indent = out[line_no].len() - out[line_no].trim_start().len();
            if !line_trimmed.is_empty() && next_indent < candidate_indent {
                if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                    eprintln!(
                        "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=indent_drop target_line={}",
                        idx + 1,
                        lhs,
                        line_trimmed
                    );
                }
                break;
            }
            if out[line_no].contains("<- function")
                || line_trimmed == "repeat {"
                || line_trimmed.starts_with("while")
                || line_trimmed.starts_with("for")
            {
                if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                    eprintln!(
                        "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=boundary",
                        idx + 1,
                        lhs
                    );
                }
                break;
            }

            if let Some(next_caps) = assign_re().and_then(|re| re.captures(&line_trimmed)) {
                let next_lhs = next_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let next_rhs = next_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if next_lhs == lhs {
                    if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                        eprintln!(
                            "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=same_lhs next_line={}",
                            idx + 1,
                            lhs,
                            line_trimmed
                        );
                    }
                    break;
                }
                if next_rhs.contains(&rhs) {
                    if lhs_reassigned_later {
                        continue;
                    }
                    if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                        eprintln!(
                            "RR_DEBUG_PEEPHOLE exact_expr_replace line={} lhs={} target_line={}",
                            idx + 1,
                            lhs,
                            line_trimmed
                        );
                    }
                    out[line_no] = out[line_no].replacen(&rhs, &lhs, usize::MAX);
                }
                if deps.contains(&next_lhs) {
                    let mut same_rhs_as_previous = false;
                    for prev_idx in (0..line_no).rev() {
                        let prev_trimmed = out[prev_idx].trim();
                        let Some(prev_caps) = assign_re().and_then(|re| re.captures(prev_trimmed))
                        else {
                            continue;
                        };
                        let prev_lhs = prev_caps
                            .name("lhs")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .trim();
                        if prev_lhs != next_lhs {
                            continue;
                        }
                        let prev_rhs = prev_caps
                            .name("rhs")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .trim();
                        let prev_norm =
                            normalize_expr_with_aliases(prev_rhs, &prologue_arg_aliases);
                        let next_norm =
                            normalize_expr_with_aliases(&next_rhs, &prologue_arg_aliases);
                        if prev_norm == next_norm {
                            same_rhs_as_previous = true;
                        }
                        break;
                    }
                    if same_rhs_as_previous {
                        continue;
                    }
                    if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                        eprintln!(
                            "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=dep_write dep={} target_line={}",
                            idx + 1,
                            lhs,
                            next_lhs,
                            line_trimmed
                        );
                    }
                    break;
                }
                continue;
            }

            if line_trimmed.contains(&rhs) {
                out[line_no] = out[line_no].replacen(&rhs, &lhs, usize::MAX);
            }
            if line_trimmed == "return(NULL)"
                || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'))
            {
                break;
            }
        }
    }
    out
}

fn is_branch_open_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    let is_single_line_guard =
        trimmed.starts_with("if ") && (trimmed.ends_with(" break") || trimmed.ends_with(" next"));
    (trimmed.starts_with("if ") && !is_single_line_guard)
        || trimmed.starts_with("if(")
        || trimmed.starts_with("else")
        || trimmed.starts_with("} else")
}

fn is_loop_open_boundary(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed == "repeat {" || trimmed.starts_with("while") || trimmed.starts_with("for")
}

fn line_is_within_loop_body(lines: &[String], idx: usize) -> bool {
    (0..idx).rev().any(|start_idx| {
        if !is_loop_open_boundary(lines[start_idx].trim()) {
            return false;
        }
        find_matching_block_end(lines, start_idx).is_some_and(|end_idx| idx < end_idx)
    })
}

fn is_identical_pure_rebind_candidate(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(lhs))
        && !lhs.starts_with(".arg_")
        && !lhs.starts_with(".__rr_cse_")
        && rhs.contains('(')
        && expr_has_only_pure_calls(rhs, pure_user_calls)
}

fn strip_redundant_identical_pure_rebinds(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut out = lines;
    for idx in 0..out.len() {
        let trimmed = out[idx].trim().to_string();
        let Some(caps) = assign_re().and_then(|re| re.captures(&trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !is_identical_pure_rebind_candidate(lhs, rhs, pure_user_calls) {
            continue;
        }
        let rhs_canonical = strip_redundant_outer_parens(rhs);
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let cur_indent = out[idx].len() - out[idx].trim_start().len();
        let mut depth = 0usize;
        let mut crossed_enclosing_if_boundary = false;
        let mut removable = false;
        for prev_idx in (0..idx).rev() {
            let prev_line = out[prev_idx].as_str();
            let prev_trimmed = prev_line.trim();
            if prev_trimmed.is_empty() {
                continue;
            }
            if prev_line.contains("<- function") {
                break;
            }
            if prev_trimmed == "}" {
                depth += 1;
                continue;
            }
            if is_branch_open_boundary(prev_trimmed) || is_loop_open_boundary(prev_trimmed) {
                if depth > 0 {
                    depth -= 1;
                    if depth == 0 && is_branch_open_boundary(prev_trimmed) {
                        break;
                    }
                    continue;
                }
                if is_branch_open_boundary(prev_trimmed) {
                    let can_cross_current_if = !crossed_enclosing_if_boundary
                        && (prev_trimmed.starts_with("if ") || prev_trimmed.starts_with("if("));
                    if can_cross_current_if {
                        crossed_enclosing_if_boundary = true;
                        continue;
                    }
                    break;
                }
                continue;
            }
            if let Some(base) = indexed_store_base_re()
                .and_then(|re| re.captures(prev_trimmed))
                .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
            {
                if base == lhs || deps.contains(&base) {
                    break;
                }
                continue;
            }
            let Some(prev_caps) = assign_re().and_then(|re| re.captures(prev_trimmed)) else {
                continue;
            };
            let prev_lhs = prev_caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let prev_rhs = prev_caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if prev_lhs == lhs {
                if depth == 0 {
                    let prev_indent = out[prev_idx].len() - out[prev_idx].trim_start().len();
                    let same_scope_rebind = prev_indent == cur_indent;
                    let enclosing_if_rebind =
                        crossed_enclosing_if_boundary && prev_indent < cur_indent;
                    if strip_redundant_outer_parens(prev_rhs) == rhs_canonical
                        && (same_scope_rebind || enclosing_if_rebind)
                    {
                        removable = true;
                    }
                }
                break;
            }
            if deps.contains(prev_lhs) {
                break;
            }
        }
        if removable {
            out[idx].clear();
        }
    }
    out
}

fn find_if_else_bounds(lines: &[String], if_idx: usize) -> Option<(usize, usize)> {
    let mut depth = 1usize;
    let mut else_idx = None;
    for (idx, line) in lines.iter().enumerate().skip(if_idx + 1) {
        let trimmed = line.trim();
        if trimmed == "} else {" && depth == 1 {
            else_idx = Some(idx);
            continue;
        }
        if trimmed.ends_with('{') {
            depth += 1;
            continue;
        }
        if trimmed == "}" {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return else_idx.map(|else_idx| (else_idx, idx));
            }
        }
    }
    None
}

fn last_non_empty_assign_before(lines: &[String], end_exclusive: usize) -> Option<(usize, &str)> {
    for idx in (0..end_exclusive).rev() {
        let trimmed = lines[idx].trim();
        if trimmed.is_empty() {
            continue;
        }
        return assign_re()
            .and_then(|re| re.captures(trimmed))
            .map(|_| (idx, lines[idx].as_str()));
    }
    None
}

fn expr_is_fresh_allocation_like(expr: &str, fresh_user_calls: &FxHashSet<String>) -> bool {
    const FRESH_BUILTINS: &[&str] = &[
        "rep.int",
        "numeric",
        "integer",
        "logical",
        "character",
        "vector",
        "matrix",
        "c",
        "seq_len",
        "seq_along",
        "rr_named_list",
    ];
    let rhs = expr.trim();
    let Some((callee, _rest)) = rhs.split_once('(') else {
        return false;
    };
    let callee = callee.trim();
    FRESH_BUILTINS.contains(&callee) || fresh_user_calls.contains(callee)
}

fn expr_has_only_pure_calls(expr: &str, pure_user_calls: &FxHashSet<String>) -> bool {
    const PURE_CALLS: &[&str] = &[
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
        "ifelse",
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
        "rr_named_list",
        "replace",
    ];
    let expr = expr.trim();
    if expr.contains("<-")
        || expr.contains("function(")
        || expr.contains("tryCatch(")
        || expr.contains("dyn.load(")
        || expr.contains("print(")
        || expr.contains("cat(")
        || expr.contains("message(")
        || expr.contains("warning(")
        || expr.contains("stop(")
        || expr.contains("quit(")
    {
        return false;
    }
    let Some(re) = compile_regex(format!(r"(?P<callee>{})\s*\(", IDENT_PATTERN)) else {
        return false;
    };
    re.captures_iter(expr).all(|caps| {
        let callee = caps.name("callee").map(|m| m.as_str()).unwrap_or("").trim();
        PURE_CALLS.contains(&callee) || pure_user_calls.contains(callee)
    })
}

fn is_dead_pure_expr_assignment_candidate(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let lhs = lhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(lhs))
        && expr_has_only_pure_calls(rhs, pure_user_calls)
}

fn is_dead_pure_call_assignment_candidate(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let lhs = lhs.trim();
    let rhs = rhs.trim();
    if !plain_ident_re().is_some_and(|re| re.is_match(lhs)) {
        return false;
    }
    let Some((callee, _)) = rhs.split_once('(') else {
        return false;
    };
    let callee = callee.trim();
    pure_user_calls.contains(callee)
}

fn rewrite_guard_truthy_line(
    line: &str,
    no_na_vars: &FxHashSet<String>,
    scalar_consts: &FxHashMap<String, String>,
) -> String {
    let trimmed = line.trim();
    let Some(inner) = trimmed
        .strip_prefix("if (!rr_truthy1(")
        .and_then(|s| s.strip_suffix(")) break"))
    else {
        return line.to_string();
    };
    let Some(parts) = split_top_level_args(inner) else {
        return line.to_string();
    };
    if parts.len() < 2 || !expr_is_logical_comparison(&parts[0], no_na_vars, scalar_consts) {
        return line.to_string();
    }
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];
    let cond = parts[0].trim();
    let cond = cond
        .strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .unwrap_or(cond);
    format!("{indent}if (!({cond})) break")
}

fn rewrite_if_truthy_line(
    line: &str,
    no_na_vars: &FxHashSet<String>,
    scalar_consts: &FxHashMap<String, String>,
) -> String {
    let trimmed = line.trim();
    let indent_len = line.len() - line.trim_start().len();
    let indent = &line[..indent_len];

    if let Some(inner) = trimmed
        .strip_prefix("if (rr_truthy1(")
        .and_then(|s| s.strip_suffix(")) {"))
        && let Some(parts) = split_top_level_args(inner)
        && parts.len() >= 2
        && expr_is_logical_comparison(&parts[0], no_na_vars, scalar_consts)
    {
        let cond = parts[0].trim();
        let cond = cond
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(cond);
        return format!("{indent}if (({cond})) {{");
    }

    if let Some(inner) = trimmed
        .strip_prefix("if (!rr_truthy1(")
        .and_then(|s| s.strip_suffix(")) {"))
        && let Some(parts) = split_top_level_args(inner)
        && parts.len() >= 2
        && expr_is_logical_comparison(&parts[0], no_na_vars, scalar_consts)
    {
        let cond = parts[0].trim();
        let cond = cond
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .unwrap_or(cond);
        return format!("{indent}if (!({cond})) {{");
    }

    line.to_string()
}

fn strip_dead_temps(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, Vec<u32>) {
    let overwritten_dead = mark_overwritten_dead_assignments(&lines, pure_user_calls);
    let branch_local_dead = mark_branch_local_dead_inits(&lines);
    let redundant_temp_reassign = mark_redundant_identical_temp_reassigns(&lines);
    let mut ever_read_per_line: Vec<FxHashSet<String>> = vec![FxHashSet::default(); lines.len()];
    let mut current_reads: FxHashSet<String> = FxHashSet::default();
    let mut current_indices: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        if line.contains("<- function") {
            for &line_idx in &current_indices {
                ever_read_per_line[line_idx] = current_reads.clone();
            }
            current_reads.clear();
            current_indices.clear();
        }
        current_indices.push(idx);
        let trimmed = line.trim();
        if let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) {
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
            for ident in expr_idents(rhs) {
                current_reads.insert(ident);
            }
        } else {
            for ident in expr_idents(trimmed) {
                current_reads.insert(ident);
            }
        }
    }
    for &line_idx in &current_indices {
        ever_read_per_line[line_idx] = current_reads.clone();
    }

    let mut live: FxHashSet<String> = FxHashSet::default();
    let mut out = lines;
    let mut removed = vec![false; out.len()];
    for idx in (0..out.len()).rev() {
        let line = &mut out[idx];
        if line.trim().is_empty() {
            removed[idx] = true;
            continue;
        }
        if is_dead_plain_ident_eval_line(line) {
            removed[idx] = true;
            *line = String::new();
            continue;
        }
        if is_dead_parenthesized_eval_line(line) {
            removed[idx] = true;
            *line = String::new();
            continue;
        }
        if overwritten_dead[idx] || branch_local_dead[idx] || redundant_temp_reassign[idx] {
            removed[idx] = true;
            *line = String::new();
            continue;
        }
        if line.trim() == "# rr-cse-pruned" {
            removed[idx] = true;
            *line = String::new();
            continue;
        }
        if line.contains("<- function") {
            live.clear();
            continue;
        }
        let trimmed = line.trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            for ident in expr_idents(trimmed) {
                live.insert(ident);
            }
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("");
        let is_self_referential_update = expr_idents(rhs).iter().any(|ident| ident == lhs);
        let is_temp = lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_callmap_arg")
            || lhs.starts_with(".tachyon_exprmap")
            || lhs.starts_with("i_")
            || lhs.starts_with(".__rr_tmp_");
        let is_dead_helper_local = lhs.starts_with("licm_");
        let is_dead_simple_assign =
            is_dead_pure_expr_assignment_candidate(lhs, rhs, pure_user_calls)
                && !ever_read_per_line[idx].contains(lhs);
        if ((is_temp || is_dead_helper_local)
            && !live.contains(lhs)
            && !(lhs.starts_with("i_") && is_self_referential_update))
            || is_dead_simple_assign
        {
            removed[idx] = true;
            *line = String::new();
            continue;
        }
        live.remove(lhs);
        for ident in expr_idents(rhs) {
            live.insert(ident);
        }
    }
    let mut compacted = Vec::with_capacity(out.len());
    let mut line_map = vec![0u32; out.len()];
    let mut new_line = 0u32;
    for (idx, line) in out.into_iter().enumerate() {
        if removed[idx] {
            line_map[idx] = new_line.max(1);
            continue;
        }
        new_line += 1;
        line_map[idx] = new_line;
        compacted.push(line);
    }
    (compacted, line_map)
}

fn mark_redundant_identical_temp_reassigns(lines: &[String]) -> Vec<bool> {
    let mut removable = vec![false; lines.len()];
    for idx in 0..lines.len() {
        let trimmed = lines[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !lhs.starts_with(".__rr_cse_") {
            continue;
        }
        let deps: FxHashSet<String> = expr_idents(rhs).into_iter().collect();
        let cur_indent = lines[idx].len() - lines[idx].trim_start().len();
        let mut j = idx;
        while j > 0 {
            j -= 1;
            let prev = lines[j].trim();
            if prev.is_empty() {
                continue;
            }
            if lines[j].contains("<- function")
                || prev == "repeat {"
                || prev.starts_with("while")
                || prev.starts_with("for")
            {
                break;
            }
            if let Some(prev_caps) = assign_re().and_then(|re| re.captures(prev)) {
                let prev_lhs = prev_caps
                    .name("lhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                let prev_rhs = prev_caps
                    .name("rhs")
                    .map(|m| m.as_str())
                    .unwrap_or("")
                    .trim();
                if prev_lhs == lhs {
                    let prev_indent = lines[j].len() - lines[j].trim_start().len();
                    if prev_rhs == rhs && prev_indent < cur_indent {
                        removable[idx] = true;
                    }
                    break;
                }
                if deps.contains(prev_lhs) {
                    break;
                }
            }
        }
    }
    removable
}

fn mark_overwritten_dead_assignments(
    lines: &[String],
    pure_user_calls: &FxHashSet<String>,
) -> Vec<bool> {
    let mut removable = vec![false; lines.len()];
    let mut pending: FxHashMap<String, usize> = FxHashMap::default();

    let clear_pending = |pending: &mut FxHashMap<String, usize>| {
        pending.clear();
    };

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if line.contains("<- function") || is_control_flow_boundary(line) {
            clear_pending(&mut pending);
            continue;
        }

        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            for ident in expr_idents(trimmed) {
                pending.remove(&ident);
            }
            continue;
        };

        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();

        for ident in expr_idents(rhs) {
            pending.remove(&ident);
        }

        if plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && !expr_idents(rhs).iter().any(|ident| ident == lhs)
            && let Some(prev_idx) = pending.remove(lhs)
        {
            removable[prev_idx] = true;
        }

        let candidate = is_dead_pure_expr_assignment_candidate(lhs, rhs, pure_user_calls)
            || is_dead_pure_call_assignment_candidate(lhs, rhs, pure_user_calls);
        if candidate {
            pending.insert(lhs.to_string(), idx);
        } else {
            pending.remove(lhs);
        }
    }

    removable
}

fn mark_branch_local_dead_inits(lines: &[String]) -> Vec<bool> {
    let mut removable = vec![false; lines.len()];

    for idx in 0..lines.len() {
        let trimmed = lines[idx].trim();
        let Some(caps) = assign_re().and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
            || !scalar_lit_re().is_some_and(|re| re.is_match(rhs))
        {
            continue;
        }

        let mut next_idx = idx + 1;
        while next_idx < lines.len() {
            let next_trimmed = lines[next_idx].trim();
            if next_trimmed.is_empty() {
                next_idx += 1;
                continue;
            }
            let Some(next_caps) = assign_re().and_then(|re| re.captures(next_trimmed)) else {
                break;
            };
            let next_lhs = next_caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let next_rhs = next_caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if plain_ident_re().is_some_and(|re| re.is_match(next_lhs))
                && scalar_lit_re().is_some_and(|re| re.is_match(next_rhs))
            {
                next_idx += 1;
                continue;
            }
            break;
        }
        if next_idx >= lines.len() || lines[next_idx].trim() != "repeat {" {
            continue;
        }
        let Some(loop_end) = find_matching_block_end(lines, next_idx) else {
            continue;
        };

        let loop_lines = &lines[next_idx + 1..loop_end];
        let mut first_occurrence = None;
        for (off, line) in loop_lines.iter().enumerate() {
            let line_idx = next_idx + 1 + off;
            let trimmed = line.trim();
            let assigned_lhs = assign_re()
                .and_then(|re| re.captures(trimmed))
                .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()));
            let mentions = assigned_lhs.as_deref() == Some(lhs)
                || expr_idents(trimmed).iter().any(|ident| ident == lhs);
            if mentions {
                first_occurrence = Some((line_idx, assigned_lhs.as_deref() == Some(lhs)));
                break;
            }
        }
        let Some((first_line_idx, first_is_assign)) = first_occurrence else {
            continue;
        };
        if !first_is_assign {
            continue;
        }

        let Some(if_start) =
            lines[..=first_line_idx]
                .iter()
                .enumerate()
                .rev()
                .find_map(|(line_idx, line)| {
                    let trimmed = line.trim();
                    (trimmed.starts_with("if ") && trimmed.ends_with('{')).then_some(line_idx)
                })
        else {
            continue;
        };
        let Some(if_end) = find_matching_block_end(lines, if_start) else {
            continue;
        };
        if if_end > loop_end {
            continue;
        }

        let mut used_outside_if = false;
        for (line_pos, line) in lines.iter().enumerate().take(loop_end).skip(next_idx + 1) {
            let trimmed = line.trim();
            let assigned_lhs = assign_re()
                .and_then(|re| re.captures(trimmed))
                .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()));
            let mentions = assigned_lhs.as_deref() == Some(lhs)
                || expr_idents(trimmed).iter().any(|ident| ident == lhs);
            if !mentions {
                continue;
            }
            if line_pos < if_start || line_pos > if_end {
                used_outside_if = true;
                break;
            }
        }
        if !used_outside_if {
            for line in lines.iter().skip(loop_end + 1) {
                let trimmed = line.trim();
                if line.contains("<- function") {
                    break;
                }
                let assigned_lhs = assign_re()
                    .and_then(|re| re.captures(trimmed))
                    .and_then(|caps| caps.name("lhs").map(|m| m.as_str().trim().to_string()));
                let mentions = assigned_lhs.as_deref() == Some(lhs)
                    || expr_idents(trimmed).iter().any(|ident| ident == lhs);
                if !mentions {
                    continue;
                }
                if assigned_lhs.as_deref() == Some(lhs) {
                    break;
                }
                used_outside_if = true;
                break;
            }
        }
        if !used_outside_if {
            removable[idx] = true;
        }
    }

    removable
}

fn find_matching_block_end(lines: &[String], start_idx: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (idx, line) in lines.iter().enumerate().skip(start_idx) {
        let (opens, closes) = count_unquoted_braces(line);
        depth += opens;
        if closes > 0 {
            depth = depth.saturating_sub(closes);
            if depth == 0 {
                return Some(idx);
            }
        }
    }
    None
}

fn sym_ref_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| compile_regex(r"\b(?P<name>Sym_[A-Za-z0-9_]+)\b".to_string()))
        .as_ref()
}

fn unquoted_sym_refs(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let Some(re) = sym_ref_re() else {
        return out;
    };
    for caps in re.captures_iter(line) {
        let Some(mat) = caps.name("name") else {
            continue;
        };
        let start = mat.start();
        let end = mat.end();
        let prev = line[..start].chars().next_back();
        let next = line[end..].chars().next();
        if matches!(prev, Some('"') | Some('\'')) && matches!(next, Some('"') | Some('\'')) {
            continue;
        }
        out.push(mat.as_str().to_string());
    }
    out
}

fn strip_unreachable_sym_helpers(lines: Vec<String>) -> Vec<String> {
    #[derive(Clone)]
    struct FnRange {
        name: String,
        start: usize,
        end: usize,
    }

    let mut ranges = Vec::<FnRange>::new();
    let mut idx = 0usize;
    while idx < lines.len() {
        let trimmed = lines[idx].trim();
        if trimmed.starts_with("Sym_") && trimmed.contains("<- function") {
            let Some((name, _)) = trimmed.split_once("<- function") else {
                idx += 1;
                continue;
            };
            let Some(end) = find_matching_block_end(&lines, idx) else {
                break;
            };
            ranges.push(FnRange {
                name: name.trim().to_string(),
                start: idx,
                end,
            });
            idx = end + 1;
            continue;
        }
        idx += 1;
    }
    if ranges.is_empty() {
        return lines;
    }

    let mut name_to_range = FxHashMap::default();
    for range in &ranges {
        name_to_range.insert(range.name.clone(), range.clone());
    }

    let sym_top_is_empty_entrypoint = |range: &FnRange| {
        let mut saw_return_null = false;
        for line in lines.iter().take(range.end).skip(range.start + 1) {
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

    let mut roots = FxHashSet::default();
    if name_to_range.contains_key("Sym_top_0") {
        roots.insert("Sym_top_0".to_string());
    }
    let mut line_idx = 0usize;
    while line_idx < lines.len() {
        if let Some(range) = ranges.iter().find(|range| range.start == line_idx) {
            line_idx = range.end + 1;
            continue;
        }
        for name in unquoted_sym_refs(lines[line_idx].as_str()) {
            if name_to_range.contains_key(&name) {
                roots.insert(name);
            }
        }
        line_idx += 1;
    }
    if roots.is_empty() {
        return lines;
    }
    if roots.len() == 1
        && roots.contains("Sym_top_0")
        && name_to_range
            .get("Sym_top_0")
            .is_some_and(sym_top_is_empty_entrypoint)
    {
        return lines;
    }

    let mut reachable = roots.clone();
    let mut work: Vec<String> = roots.into_iter().collect();
    while let Some(name) = work.pop() {
        let Some(range) = name_to_range.get(&name) else {
            continue;
        };
        for line in lines.iter().take(range.end).skip(range.start + 1) {
            for callee in unquoted_sym_refs(line.as_str()) {
                if name_to_range.contains_key(&callee) && reachable.insert(callee.clone()) {
                    work.push(callee);
                }
            }
        }
    }

    let mut out = Vec::with_capacity(lines.len());
    let mut cursor = 0usize;
    for range in ranges {
        while cursor < range.start {
            out.push(lines[cursor].clone());
            cursor += 1;
        }
        if reachable.contains(&range.name) {
            while cursor <= range.end {
                out.push(lines[cursor].clone());
                cursor += 1;
            }
        } else {
            cursor = range.end + 1;
        }
    }
    while cursor < lines.len() {
        out.push(lines[cursor].clone());
        cursor += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::{FxHashMap, FxHashSet};

    #[test]
    fn rewrites_whole_slice_patterns() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 8\n\
  idx <- seq_len(n)\n\
  x <- idx + 1\n\
  score <- rep.int(0, n)\n\
  i <- 1L\n\
  .tmp1 <- i:8\n\
  .tmp2 <- rr_index_vec_floor(.tmp1)\n\
  .arg <- abs(rr_index1_read_vec(x, .tmp2))\n\
  score <- rr_call_map_slice_auto(score, i, 8, \"pmax\", 44L, c(1L), .arg, 0.05)\n\
  score <- rr_assign_slice(score, i, 8, rr_index1_read_vec(score, .tmp2))\n\
        }\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("abs(") || out.contains("x <- seq_len(n) + 1"),
            "{out}"
        );
        assert!(
            out.contains("score <- pmax(.arg, 0.05)") || !out.contains("score <-"),
            "{out}"
        );
        assert!(!out.contains("rr_call_map_slice_auto("));
        assert!(!out.contains("rr_index1_read_vec(score, .tmp2)"));
    }

    #[test]
    fn rewrites_nested_vector_helper_subcalls() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  idx <- seq_len(n)\n\
  x <- rr_parallel_vec_sub_f64(rr_parallel_vec_div_f64((rr_parallel_vec_mul_f64(idx, 13) %% 1000), 1000), 0.5)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("rr_parallel_vec_sub_f64"), "{out}");
        assert!(!out.contains("rr_parallel_vec_div_f64"), "{out}");
        assert!(!out.contains("rr_parallel_vec_mul_f64"), "{out}");
    }

    #[test]
    fn generic_counted_repeat_loop_rewrite_still_applies_without_benchmark_rewrites() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= TOTAL)) break\n\
    rr_mark(1545, 13);\n\
    u_stage[i] <- (u[i] + (dt * (du1[i] - adv_u[i])))\n\
    i <- (i + 1)\n\
  }\n\
}\n";
        let pure = FxHashSet::default();
        let fresh = FxHashSet::default();
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input, true, &pure, &fresh, false,
        );
        assert!(!out.contains("repeat {"), "{out}");
        assert!(!out.contains("if (!(i <= TOTAL)) break"), "{out}");
        assert!(out.contains("for (i in seq_len(TOTAL)) {"), "{out}");
    }

    #[test]
    fn generic_counted_repeat_loop_rewrite_preserves_non_iter_prefix_lines() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  seed <- 12345\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    seed <- (((seed * 1103515245) + 12345) %% 2147483648)\n\
    p[i] <- (seed / 2147483648)\n\
    i <- (i + 1)\n\
  }\n\
  return(seed)\n\
}\n";
        let pure = FxHashSet::default();
        let fresh = FxHashSet::default();
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input, true, &pure, &fresh, false,
        );
        assert!(out.contains("seed <- 12345"), "{out}");
        assert!(out.contains("for (i in seq_len(n)) {"), "{out}");
        assert!(!out.contains("repeat {"), "{out}");
    }

    #[test]
    fn restores_counter_for_constant_one_guard_repeat_loop() {
        let input = "\
Sym_1 <- function(n)\n\
{\n\
  a <- 1L\n\
  b <- 2L\n\
  repeat {\n\
    if !((1L) <= n) break\n\
    t <- a\n\
    a <- b\n\
    b <- t\n\
  }\n\
  return((a + b))\n\
}\n";
        let out = restore_constant_one_guard_repeat_loop_counters(
            input.lines().map(str::to_string).collect(),
        )
        .join("\n");
        assert!(out.contains(".__rr_i <- 1L"), "{out}");
        assert!(out.contains("if (!(.__rr_i <= n)) break"), "{out}");
        assert!(out.contains(".__rr_i <- (.__rr_i + 1L)"), "{out}");
    }

    #[test]
    fn restores_counter_for_constant_zero_guard_repeat_loop() {
        let input = "\
Sym_9 <- function(x)\n\
{\n\
  g <- ((x * 0.5) + 0.5)\n\
  repeat {\n\
    if (!((0) < 8)) break\n\
    g <- (0.5 * (g + (x / g)))\n\
  }\n\
  return(g)\n\
}\n";
        let out = restore_constant_one_guard_repeat_loop_counters(
            input.lines().map(str::to_string).collect(),
        )
        .join("\n");
        assert!(out.contains(".__rr_i <- 0"), "{out}");
        assert!(out.contains("if (!(.__rr_i < 8)) break"), "{out}");
        assert!(out.contains(".__rr_i <- (.__rr_i + 1)"), "{out}");
    }

    #[test]
    fn generic_counted_repeat_loop_rewrite_skips_when_iter_used_after_loop() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    acc[i] <- value\n\
    i <- (i + 1)\n\
  }\n\
  return(i)\n\
}\n";
        let pure = FxHashSet::default();
        let fresh = FxHashSet::default();
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input, true, &pure, &fresh, false,
        );
        assert!(out.contains("repeat {"), "{out}");
        assert!(!out.contains("for (i in seq_len(n)) {"), "{out}");
    }

    #[test]
    fn hoists_loop_invariant_pure_assignment_from_counted_repeat_loop() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  steps <- 0\n\
  total <- 0\n\
  repeat {\n\
    if (!(steps < 5)) break\n\
    heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)\n\
    total <- (total + heat[1L])\n\
    steps <- (steps + 1)\n\
  }\n\
  return(total)\n\
}\n";
        let pure = FxHashSet::from_iter([String::from("Sym_287")]);
        let fresh = FxHashSet::default();
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input, true, &pure, &fresh, false,
        );
        let heat_pos = out
            .find("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
            .expect("{out}");
        let loop_pos = out.find("repeat {").expect("{out}");
        assert!(heat_pos < loop_pos, "{out}");
        assert_eq!(
            out.matches("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
                .count(),
            1
        );
    }

    #[test]
    fn does_not_hoist_loop_invariant_pure_assignment_when_dependency_mutates() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  steps <- 0\n\
  total <- 0\n\
  repeat {\n\
    if (!(steps < 5)) break\n\
    heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)\n\
    qv <- next_qv\n\
    total <- (total + heat[1L])\n\
    steps <- (steps + 1)\n\
  }\n\
  return(total)\n\
}\n";
        let pure = FxHashSet::from_iter([String::from("Sym_287")]);
        let fresh = FxHashSet::default();
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input, true, &pure, &fresh, false,
        );
        let heat_pos = out
            .find("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
            .expect("{out}");
        let loop_pos = out.find("repeat {").expect("{out}");
        assert!(heat_pos > loop_pos, "{out}");
        assert_eq!(
            out.matches("heat <- Sym_287(temp, qv, qc, qs, qg, TOTAL)")
                .count(),
            1
        );
    }

    #[test]
    fn preserves_loop_facts_across_repeat_and_single_line_break_guard() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 250000\n\
  idx <- seq_len(n)\n\
  x <- ((((idx * 13) %% 1000) / 1000) - 0.5)\n\
  y <- (((((idx * 17) + 7) %% 1000) / 1000) - 0.5)\n\
  score <- rep.int(0, n)\n\
  clean <- rep.int(0, n)\n\
  pass <- 1\n\
  repeat {\n\
    if (!rr_truthy1((pass <= 16), \"condition\")) break\n\
    i <- 1L\n\
    .__rr_cse_159 <- i:250000\n\
    .__rr_cse_160 <- rr_index_vec_floor(.__rr_cse_159)\n\
    .tachyon_callmap_arg0_0 <- abs((((rr_index1_read_vec(x, .__rr_cse_160) * 0.65) + (rr_index1_read_vec(y, .__rr_cse_160) * 0.35)) - 0.08))\n\
    score <- rr_call_map_slice_auto(score, i, 250000, \"pmax\", 44L, c(1L), .tachyon_callmap_arg0_0, 0.05)\n\
    i_9 <- 1L\n\
    .__rr_cse_174 <- i_9:250000\n\
    .__rr_cse_175 <- rr_index_vec_floor(.__rr_cse_174)\n\
    .__rr_cse_176 <- rr_index1_read_vec(score, .__rr_cse_175)\n\
    clean <- rr_assign_slice(clean, i_9, 250000, rr_ifelse_strict((.__rr_cse_176 > 0.4), sqrt((.__rr_cse_176 + 0.1)), ((.__rr_cse_176 * 0.55) + 0.03)))\n\
    i_10 <- 1L\n\
    .__rr_cse_184 <- i_10:250000\n\
    .__rr_cse_185 <- rr_index_vec_floor(.__rr_cse_184)\n\
    x <- rr_assign_slice(x, i_10, 250000, (rr_index1_read_vec(clean, .__rr_cse_185) + (rr_index1_read_vec(y, .__rr_cse_185) * 0.15)))\n\
    i_11 <- 1L\n\
    .__rr_cse_191 <- i_11:250000\n\
    .__rr_cse_192 <- rr_index_vec_floor(.__rr_cse_191)\n\
    y <- rr_assign_slice(y, i_11, 250000, ((rr_index1_read_vec(score, .__rr_cse_192) * 0.8) + (rr_index1_read_vec(clean, .__rr_cse_192) * 0.2)))\n\
    pass <- (pass + 1)\n\
    next\n\
  }\n\
        }\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("repeat {"));
        assert!(out.contains("pass <- 1"));
        assert!(out.contains("score <- pmax("));
        assert!(out.contains(
            "clean <- ifelse((score > 0.4), sqrt((score + 0.1)), ((score * 0.55) + 0.03))"
        ));
        assert!(out.contains("x <- (clean + (y * 0.15))"));
        assert!(out.contains("y <- ((score * 0.8) + (clean * 0.2))"));
    }

    #[test]
    fn does_not_rewrite_full_slice_facts_across_branch_boundaries() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  base <- seq_len(3)\n\
  idx <- c(1L, 3L)\n\
  if (flag) {\n\
    idx <- 1:3\n\
  }\n\
  out <- rr_index1_read_vec(base, idx)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("out <- base"), "{out}");
        assert!(!out.contains("return(base)"), "{out}");
    }

    #[test]
    fn reuses_pure_user_call_binding_in_later_expression() {
        let input = "\
Sym_123 <- function() \n\
{\n\
  Ap <- Sym_119(p, n_l, n_r, n_d, n_u, size)\n\
  idx <- rr_index_vec_floor(i:size)\n\
  r <- (rr_index1_read_vec(r, idx) - (alpha * rr_index1_read_vec(Sym_119(p, n_l, n_r, n_d, n_u, size), idx)))\n\
}\n";
        let pure = FxHashSet::from_iter([String::from("Sym_119")]);
        let (out, _) = optimize_emitted_r_with_context(input, true, &pure);
        assert!(
            out.contains("rr_index1_read_vec(Ap, idx)")
                || out.matches("Sym_119(p, n_l, n_r, n_d, n_u, size)").count() <= 1,
            "{out}"
        );
        assert!(!out.contains("rr_index1_read_vec(Sym_119("));
    }

    #[test]
    fn rewrites_return_of_last_assignment_rhs_to_variable() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  x <- rr_assign_slice(x, 1, n, expr)\n\
  return(rr_assign_slice(x, 1, n, expr))\n\
        }\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("return(x)") || !out.contains("return(rr_assign_slice("));
        assert!(!out.contains("return(rr_assign_slice("));
    }

    #[test]
    fn return_rewrite_does_not_use_stale_alias_after_rhs_var_is_mutated() {
        let input = "\
Sym_5 <- function(n) \n\
{\n\
  acc <- 1L\n\
  i <- acc\n\
  acc <- prod(1L:n)\n\
  return(acc)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("return(acc)") || !out.contains("return(i)"));
        assert!(!out.contains("return(i)"));
    }

    #[test]
    fn reuses_pure_user_call_across_guarded_block_sequence() {
        let input = "\
Sym_123 <- function() \n\
{\n\
  Ap <- Sym_119(p, .arg_n_l, .arg_n_r, .arg_n_d, .arg_n_u, .arg_size)\n\
  p_Ap <- Sym_117(p, Ap, .arg_size)\n\
  if (rr_truthy1(((is.na(p_Ap) | (!(is.finite(p_Ap)))) | (p_Ap == 0)), \"condition\")) {\n\
    p_Ap <- 0.0000001\n\
  } else {\n\
  }\n\
  alpha <- (rs_old / p_Ap)\n\
  if (rr_truthy1((is.na(alpha) | (!(is.finite(alpha)))), \"condition\")) {\n\
    alpha <- 0\n\
  } else {\n\
  }\n\
  .tachyon_exprmap1_1 <- (rr_index1_read_vec(r, idx) - (alpha * rr_index1_read_vec(Sym_119(p, .arg_n_l, .arg_n_r, .arg_n_d, .arg_n_u, .arg_size), idx)))\n\
        }\n";
        let pure = FxHashSet::from_iter([String::from("Sym_119"), String::from("Sym_117")]);
        let (out, _) = optimize_emitted_r_with_context(input, true, &pure);
        assert!(
            out.matches("Sym_119(p, .arg_n_l, .arg_n_r, .arg_n_d, .arg_n_u, .arg_size)")
                .count()
                <= 1
        );
        assert!(!out.contains("rr_index1_read_vec(Sym_119("));
    }

    #[test]
    fn removes_dead_simple_alias_and_literal_assignments() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  a <- 1\n\
  b <- a\n\
  c <- 0\n\
  return((a + 1))\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("a <- 1"));
        assert!(!out.contains("b <- a"));
        assert!(!out.contains("c <- 0"));
    }

    #[test]
    fn removes_dead_unused_scalar_index_reads_and_pure_call_bindings() {
        let input = "\
Sym_287 <- function(temp, q_r, q_i, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= size)) break\n\
    T <- temp[i]\n\
    qr <- q_r[i]\n\
    qi <- q_i[i]\n\
    es_ice <- (6.11 * exp(T))\n\
    rr_mark(1, 1);\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  return(0)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("qr <- q_r[i]"), "{out}");
        assert!(!out.contains("qi <- q_i[i]"), "{out}");
        assert!(!out.contains("es_ice <- (6.11 * exp(T))"), "{out}");
        assert!(out.contains("rr_mark(1, 1);"), "{out}");
    }

    #[test]
    fn inlines_single_use_named_scalar_index_reads() {
        let input = "\
Sym_287 <- function(temp, q_v, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= size)) break\n\
    T <- temp[i]\n\
    qv <- q_v[i]\n\
    T_c <- (T - 273.15)\n\
    if ((qv > 0.01)) {\n\
      rr_mark(1, 1);\n\
      print(T_c)\n\
    }\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  return(0)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("T <- temp[i]"), "{out}");
        assert!(
            out.contains("T_c <- (temp[i] - 273.15)")
                || out.contains("T_c <- ((temp[i]) - 273.15)"),
            "{out}"
        );
        assert!(
            out.contains("if (((q_v[i]) > 0.01)) {")
                || out.contains("if ((q_v[i] > 0.01)) {")
                || out.contains("if ((qv > 0.01)) {"),
            "{out}"
        );
    }

    #[test]
    fn inlines_single_use_named_scalar_index_reads_across_if_boundaries() {
        let input = "\
Sym_287 <- function(temp, q_v, q_c, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= size)) break\n\
    T_c <- (temp[i] - 273.15)\n\
    qc <- q_c[i]\n\
    if ((T_c < (-(5)))) {\n\
      if ((qc > 0.0001)) {\n\
        rate <- (0.01 * qc)\n\
      }\n\
    }\n\
    qv <- q_v[i]\n\
    if ((T_c < (-(15)))) {\n\
      if ((qv > 0.01)) {\n\
        print(T_c)\n\
      }\n\
    }\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  return(0)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("if ((q_c[i] > 0.0001)) {")
                || out.contains("if (((q_c[i]) > 0.0001)) {")
                || out.contains("if ((qc > 0.0001)) {"),
            "{out}"
        );
        assert!(
            out.contains("if ((q_v[i] > 0.01)) {")
                || out.contains("if (((q_v[i]) > 0.01)) {")
                || out.contains("if ((qv > 0.01)) {"),
            "{out}"
        );
    }

    #[test]
    fn inlines_two_use_named_scalar_index_reads_across_if_boundaries() {
        let input = "\
Sym_287 <- function(temp, q_s, q_g, size) \n\
{\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= size)) break\n\
    T_c <- (temp[i] - 273.15)\n\
    qs <- q_s[i]\n\
    qg <- q_g[i]\n\
    if ((T_c > 0)) {\n\
      melt_rate <- 0\n\
      if ((qs > 0)) {\n\
        melt_rate <- (qs * 0.05)\n\
      }\n\
      if ((qg > 0)) {\n\
        melt_rate <- (melt_rate + (qg * 0.02))\n\
      }\n\
    }\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  return(0)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("if ((q_s[i] > 0)) {")
                || out.contains("if (((q_s[i]) > 0)) {")
                || out.contains("if ((qs > 0)) {"),
            "{out}"
        );
        assert!(
            out.contains("melt_rate <- (q_s[i] * 0.05)")
                || out.contains("melt_rate <- ((q_s[i]) * 0.05)")
                || out.contains("melt_rate <- (qs * 0.05)"),
            "{out}"
        );
        assert!(
            out.contains("if ((q_g[i] > 0)) {")
                || out.contains("if (((q_g[i]) > 0)) {")
                || out.contains("if ((qg > 0)) {"),
            "{out}"
        );
        assert!(
            out.contains("melt_rate <- (melt_rate + (q_g[i] * 0.02))")
                || out.contains("melt_rate <- (melt_rate + ((q_g[i]) * 0.02))")
                || out.contains("melt_rate <- (melt_rate + (qg * 0.02))"),
            "{out}"
        );
    }

    #[test]
    fn inlines_immediate_single_use_named_scalar_expr_into_following_assignment() {
        let input = "\
Sym_287 <- function(q_c, i) \n\
{\n\
  if ((q_c[i] > 0.0001)) {\n\
    rate <- (0.01 * q_c[i])\n\
    tendency_T <- (rate * L_f)\n\
  }\n\
  return(tendency_T)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("rate <- (0.01 * q_c[i])"), "{out}");
        assert!(
            out.contains("tendency_T <- ((0.01 * q_c[i]) * L_f)")
                || out.contains("tendency_T <- (((0.01 * q_c[i]) * L_f))")
                || out.contains("tendency_T <- ((0.01 * (q_c[i])) * L_f)"),
            "{out}"
        );
    }

    #[test]
    fn does_not_inline_named_scalar_expr_inside_repeat_loop() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  repeat {\n\
    if (!(time <= 5)) break\n\
    vy <- (vy + (g * dt))\n\
    y <- (y + (vy * dt))\n\
    time <- (time + dt)\n\
  }\n\
  return(y)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("vy <- (vy + (g * dt))"), "{out}");
        assert!(out.contains("y <- (y + (vy * dt))"), "{out}");
    }

    #[test]
    fn rewrites_adjacent_duplicate_pure_call_assignments_to_alias() {
        let input = "\
Sym_303 <- function() \n\
{\n\
  p_x <- Sym_183(1000)\n\
  p_y <- Sym_183(1000)\n\
  return(p_y)\n\
}\n";
        let pure = FxHashSet::from_iter([String::from("Sym_183")]);
        let (out, _) = optimize_emitted_r_with_context(input, true, &pure);
        assert!(out.contains("p_x <- Sym_183(1000)"), "{out}");
        assert!(out.contains("p_y <- p_x"), "{out}");
        assert!(!out.contains("p_y <- Sym_183(1000)"), "{out}");
    }

    #[test]
    fn rewrites_adjacent_duplicate_symbol_assignments_to_alias() {
        let input = "\
Sym_123 <- function(b) \n\
{\n\
  r <- b\n\
  p <- b\n\
  return(p)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("r <- b") || out.contains("return(r)"), "{out}");
        assert!(out.contains("p <- r") || out.contains("return(r)"), "{out}");
        assert!(
            !out.contains("p <- b") || out.contains("return(r)"),
            "{out}"
        );
    }

    #[test]
    fn prunes_dead_zero_loop_seeds_before_for() {
        let input = "\
Sym_top_0 <- function() \n\
{\n\
t <- 0\n\
  rr_mark(1031, 5);\n\
  print(\"  Watching the pattern emerge...\")\n\
for (t in seq_len(20)) {\n\
  print(t)\n\
}\n\
steps <- 0\n\
dt <- 0.1\n\
for (steps in seq_len(5)) {\n\
  print(steps)\n\
}\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("t <- 0"), "{out}");
        assert!(!out.contains("steps <- 0"), "{out}");
        assert!(!out.contains("k <- 1"), "{out}");
        assert!(out.contains("for (t in seq_len(20)) {"), "{out}");
        assert!(out.contains("for (steps in seq_len(5)) {"), "{out}");
    }

    #[test]
    fn simplifies_same_var_is_na_or_not_finite_guards() {
        let input = "\
Sym_123 <- function() \n\
{\n\
  if (((is.na(rs_old) | (!(is.finite(rs_old)))) | (rs_old == 0))) {\n\
    rs_old <- 0.0000001\n\
  }\n\
  if ((is.na(alpha) | (!(is.finite(alpha))))) {\n\
    alpha <- 0\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("!(is.finite(rs_old))") && out.contains("(rs_old == 0)"),
            "{out}"
        );
        assert!(out.contains("!(is.finite(alpha))"), "{out}");
        assert!(!out.contains("is.na(rs_old)"), "{out}");
        assert!(!out.contains("is.na(alpha)"), "{out}");
    }

    #[test]
    fn simplifies_not_finite_or_zero_guard_parens() {
        let input = "\
Sym_123 <- function() \n\
{\n\
  if (((!(is.finite(rs_old))) | (rs_old == 0))) {\n\
    rs_old <- 0.0000001\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("if ((!(is.finite(rs_old)) | (rs_old == 0))) {"),
            "{out}"
        );
        assert!(
            !out.contains("if (((!(is.finite(rs_old))) | (rs_old == 0))) {"),
            "{out}"
        );
    }

    #[test]
    fn simplifies_wrapped_not_finite_parens() {
        let input = "\
Sym_123 <- function() \n\
{\n\
  if ((!(is.finite(alpha)))) {\n\
    alpha <- 0\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("if (!(is.finite(alpha))) {"), "{out}");
        assert!(!out.contains("if ((!(is.finite(alpha)))) {"), "{out}");
    }

    #[test]
    fn strips_terminal_repeat_nexts_without_touching_inner_if_nexts() {
        let input = "\
Sym_83 <- function() \n\
{\n\
  repeat {\n\
    if ((flag)) {\n\
      next\n\
    }\n\
    x <- (x + 1)\n\
    next\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("if ((flag)) {\nnext\n}") || out.contains("if ((flag)) {\n  next\n}"),
            "{out}"
        );
        assert!(
            !out.contains("x <- (x + 1)\nnext\n}") && !out.contains("x <- (x + 1)\n  next\n}"),
            "{out}"
        );
    }

    #[test]
    fn inlines_single_use_named_scalar_pure_calls() {
        let input = "\
Sym_222 <- function() \n\
{\n\
  id <- rr_wrap_index_vec_i(x, y, W, H)\n\
  rr_mark(1, 1);\n\
  B[rr_index1_write(id, \"index\")] <- 1\n\
  center_idx <- rr_wrap_index_vec_i(32, 32, W, H)\n\
  print(rr_index1_read(B, center_idx, \"index\"))\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            !out.contains("id <- rr_wrap_index_vec_i(x, y, W, H)"),
            "{out}"
        );
        assert!(
            !out.contains("center_idx <- rr_wrap_index_vec_i(32, 32, W, H)"),
            "{out}"
        );
        assert!(
            out.contains("B[rr_wrap_index_vec_i(x, y, W, H)] <- 1"),
            "{out}"
        );
        assert!(
            out.contains("print(B[rr_wrap_index_vec_i(32, 32, W, H)])"),
            "{out}"
        );
    }

    #[test]
    fn rewrites_wrap_index_scalar_access_helpers_to_base_indexing() {
        let input = "\
Sym_top_0 <- function() \n\
{\n\
  B[rr_index1_write(rr_wrap_index_vec_i(x, y, W, H), \"index\")] <- 1\n\
  return(rr_index1_read(B, rr_wrap_index_vec_i(32, 32, W, H), \"index\"))\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("B[rr_wrap_index_vec_i(x, y, W, H)] <- 1"),
            "{out}"
        );
        assert!(
            out.contains("return(B[rr_wrap_index_vec_i(32, 32, W, H)])"),
            "{out}"
        );
        assert!(
            !out.contains("rr_index1_write(rr_wrap_index_vec_i("),
            "{out}"
        );
        assert!(
            !out.contains("rr_index1_read(B, rr_wrap_index_vec_i("),
            "{out}"
        );
    }

    #[test]
    fn rewrites_straight_line_sym156_helper_call() {
        let input = "\
Sym_156 <- function(u, v, n_l, n_r, n_d, n_u, size)\n\
{\n\
  .arg_n_l <- rr_index_vec_floor(n_l)\n\
  .arg_n_r <- rr_index_vec_floor(n_r)\n\
  .arg_n_d <- rr_index_vec_floor(n_d)\n\
  .arg_n_u <- rr_index_vec_floor(n_u)\n\
  Cs <- 0.15\n\
  DX <- 10000\n\
  mix_sq <- ((Cs * DX) * (Cs * DX))\n\
  visc <- (mix_sq * sqrt((((((rr_gather(u, .arg_n_r) - rr_gather(u, .arg_n_l)) / (2 * DX)) - ((rr_gather(v, .arg_n_u) - rr_gather(v, .arg_n_d)) / (2 * DX))) * (((rr_gather(u, .arg_n_r) - rr_gather(u, .arg_n_l)) / (2 * DX)) - ((rr_gather(v, .arg_n_u) - rr_gather(v, .arg_n_d)) / (2 * DX)))) + ((((rr_gather(u, .arg_n_u) - rr_gather(u, .arg_n_d)) / (2 * DX)) + ((rr_gather(v, .arg_n_r) - rr_gather(v, .arg_n_l)) / (2 * DX))) * (((rr_gather(u, .arg_n_u) - rr_gather(u, .arg_n_d)) / (2 * DX)) + ((rr_gather(v, .arg_n_r) - rr_gather(v, .arg_n_l)) / (2 * DX)))))))\n\
  return(visc)\n\
}\n\
Sym_1 <- function()\n\
{\n\
  visc <- Sym_156(u, v, adj_l, adj_r, adj_d, adj_u, TOTAL)\n\
  return(visc)\n\
}\n";
        let lines: Vec<String> = input.lines().map(str::to_string).collect();
        let mut bindings = rustc_hash::FxHashMap::default();
        for line in lines.iter().skip(2).take(8) {
            let trimmed = line.trim();
            let caps = super::assign_re()
                .and_then(|re| re.captures(trimmed))
                .expect("expected assignment in helper body");
            let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
            let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
            let expanded = super::substitute_helper_expr(rhs, &bindings);
            bindings.insert(lhs.to_string(), expanded);
        }
        let expanded_return = super::substitute_helper_expr("visc", &bindings);
        assert!(
            expanded_return.contains("rr_index_vec_floor(n_l)"),
            "{expanded_return}"
        );
        assert!(
            expanded_return.contains("rr_gather(u,"),
            "{expanded_return}"
        );
        let helpers = super::collect_simple_expr_helpers(&lines, &FxHashSet::default());
        assert!(helpers.contains_key("Sym_156"), "{helpers:#?}");
        let out = optimize_emitted_r(input, true);
        assert!(
            !out.contains("visc <- Sym_156(u, v, adj_l, adj_r, adj_d, adj_u, TOTAL)"),
            "{out}"
        );
        assert!(out.contains("return("), "{out}");
    }

    #[test]
    fn rewrites_selected_simple_expr_helper_calls_in_text() {
        let input = "\
Sym_244 <- function(v_m2, v_m1, v_c, v_p1, v_p2)\n\
{\n\
  return((v_c + v_p1))\n\
}\n\
Sym_268 <- function(field, u_vel, n_l, n_r, n_ll, n_rr)\n\
{\n\
  flux <- ifelse((u_vel > 0), (u_vel * (Sym_244(rr_gather(field, n_l), field, rr_gather(field, n_r), rr_gather(field, n_rr), rr_gather(field, n_rr)) - Sym_244(rr_gather(field, n_ll), rr_gather(field, n_l), field, rr_gather(field, n_r), rr_gather(field, n_rr)))), ((u_vel * (rr_gather(field, n_r) - rr_gather(field, n_l))) * 0.5))\n\
  return(flux)\n\
}\n\
Sym_999 <- function(x)\n\
{\n\
  return((x + 1))\n\
}\n\
Sym_303 <- function()\n\
{\n\
  adv_u <- Sym_268(u, u, adj_l, adj_r, adj_ll, adj_rr)\n\
  keep_me <- Sym_999(1)\n\
  return(adv_u)\n\
}\n";
        let out = super::rewrite_selected_simple_expr_helper_calls_in_text(
            input,
            &["Sym_244", "Sym_268"],
        );
        assert!(!out.contains("adv_u <- Sym_268("), "{out}");
        assert!(out.contains("ifelse((u > 0),"), "{out}");
        assert!(out.contains("rr_gather(u, adj_l)"), "{out}");
        assert!(out.contains("keep_me <- Sym_999(1)"), "{out}");
    }

    #[test]
    fn restores_missing_scalar_loop_increment_for_repeat_guarded_index_loop() {
        let input = "\
Sym_1 <- function(n)\n\
{\n\
  y <- seq_len(n)\n\
  i <- 1L\n\
  repeat {\n\
    if (!(i <= length(y))) break\n\
    y[i] <- 0L\n\
  }\n\
  return(y)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("i <- (i + 1L)"), "{out}");
    }

    #[test]
    fn restores_missing_scalar_loop_increment_for_repeat_guarded_reduction_loop() {
        let input = "\
Sym_1 <- function(n)\n\
{\n\
  acc <- 0L\n\
  i <- 1L\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    acc <- (acc + (i * i))\n\
  }\n\
  return(acc)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("i <- (i + 1L)") || out.contains("for (i in seq_len(n)) {"),
            "{out}"
        );
    }

    #[test]
    fn restores_missing_scalar_loop_increment_before_next_branch() {
        let input = "\
Sym_1 <- function(n)\n\
{\n\
s <- 0L\n\
i <- 1L\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    if ((i == 3L)) {\n\
i <- (i + 1L)\n\
      next\n\
    } else if ((i == 6L)) {\n\
      break\n\
    } else {\n\
s <- (s + i)\n\
      next\n\
    }\n\
  }\n\
  return(s)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("s <- (s + i)\ni <- (i + 1L)\nnext"), "{out}");
    }

    #[test]
    fn restores_missing_scalar_loop_increment_after_nested_if() {
        let input = "\
Sym_11 <- function(n, k)\n\
{\n\
x <- seq_len(n)\n\
y <- x\n\
i <- 1L\n\
  repeat {\n\
    if (!(i <= length(x))) break\n\
    if ((x[i] > k)) {\n\
      y[i] <- x[i]\n\
    } else {\n\
      y[i] <- 0L\n\
    }\n\
  }\n\
  return(y)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("}\ni <- (i + 1L)\n}") || out.contains("for (i in seq_len(length(x))) {"),
            "{out}"
        );
    }

    #[test]
    fn rewrites_known_length_calls_from_prior_vector_facts() {
        let mut vector_lens = FxHashMap::default();
        vector_lens.insert("y".to_string(), "n".to_string());
        assert_eq!(
            super::rewrite_known_length_calls("rep.int(0L, length(y))", &vector_lens),
            "rep.int(0L, n)"
        );
        assert_eq!(
            super::rewrite_known_length_calls("rep.int(i, length(y))", &vector_lens),
            "rep.int(i, n)"
        );
    }

    #[test]
    fn restores_empty_singleton_list_match_arm() {
        let input = "\
Sym_9 <- function(v)\n\
{\n\
  if ((((length(v) >= 2L) & TRUE) & TRUE)) {\n\
.phi_32 <- ((v[1L] + v[2L]) + 1L)\n\
  } else {\n\
    if (((length(v) == 1L) & TRUE)) {\n\
    } else {\n\
.phi_32 <- 0L\n\
    }\n\
  }\n\
  return(.phi_32)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains(".phi_32 <- v[1L]"), "{out}");
    }

    #[test]
    fn restores_empty_single_field_record_match_arm() {
        let input = "\
Sym_17 <- function(v)\n\
{\n\
  if (((((TRUE & rr_field_exists(v, \"a\")) & TRUE) & rr_field_exists(v, \"b\")) & TRUE)) {\n\
.phi_30 <- (v[[\"a\"]] + v[[\"b\"]])\n\
  } else {\n\
    if (((TRUE & rr_field_exists(v, \"a\")) & TRUE)) {\n\
    } else {\n\
.phi_30 <- 0L\n\
    }\n\
  }\n\
  return(.phi_30)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains(".phi_30 <- v[[\"a\"]]"), "{out}");
    }

    #[test]
    fn simplifies_nested_index_vec_floor_calls_in_text() {
        let input = "\
Sym_303 <- function()\n\
{\n\
  visc <- rr_gather(u, rr_index_vec_floor(rr_index_vec_floor(adj_r)))\n\
  return(rr_index_vec_floor(rr_index_vec_floor(adj_l)))\n\
}\n";
        let out = super::simplify_nested_index_vec_floor_calls_in_text(input);
        assert!(
            !out.contains("rr_index_vec_floor(rr_index_vec_floor("),
            "{out}"
        );
        assert!(
            out.contains("rr_gather(u, rr_index_vec_floor(adj_r))"),
            "{out}"
        );
        assert!(out.contains("return(rr_index_vec_floor(adj_l))"), "{out}");
    }

    #[test]
    fn keeps_loop_carried_assignments_that_are_read_elsewhere_in_function() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  rs_old <- 1\n\
  repeat {\n\
    if (!(iter <= 2)) break\n\
    beta <- (rs_new / rs_old)\n\
    rs_old <- rs_new\n\
    iter <- (iter + 1)\n\
    next\n\
  }\n\
  return(rs_old)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("rs_old <- rs_new"));
    }

    #[test]
    fn keeps_repeat_preheader_and_induction_assignments() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    ii <- i\n\
    out[ii] <- x[ii]\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("out[i] <- x[i]") || out.contains("out[ii] <- x[ii]"),
            "{out}"
        );
        assert!(
            out.contains("i <- 1")
                || out.contains("i <- 1L")
                || out.contains("for (i in seq_len(n)) {"),
            "{out}"
        );
        assert!(
            out.contains("i <- (i + 1)") || out.contains("for (i in seq_len(n)) {"),
            "{out}"
        );
    }

    #[test]
    fn keeps_generated_i_temp_induction_update_inside_repeat() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 10\n\
  clean <- rep.int(0, n)\n\
  score <- rep.int(1, n)\n\
  i_9 <- 1L\n\
  repeat {\n\
    if (!(i_9 <= n)) break\n\
    if ((score[i_9] > 0.4)) {\n\
      clean[i_9] <- sqrt((score[i_9] + 0.1))\n\
    } else {\n\
      clean[i_9] <- ((score[i_9] * 0.55) + 0.03)\n\
    }\n\
    i_9 <- (i_9 + 1L)\n\
    next\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("i_9 <- (i_9 + 1L)")
                || out.contains("clean <- ifelse(")
                || out.contains("for (i_9 in seq_len(n)) {")
                || (out.contains("n <- 10") && !out.contains("clean")),
            "{out}"
        );
    }

    #[test]
    fn keeps_nested_loop_reseed_inside_outer_repeat() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  i <- 1\n\
  steps <- 0\n\
  repeat {\n\
    if (!(steps < 3)) break\n\
    i <- 1\n\
    repeat {\n\
      if (!(i <= n)) break\n\
      ii <- i\n\
      out[ii] <- x[ii]\n\
      i <- (i + 1)\n\
      next\n\
    }\n\
    steps <- (steps + 1)\n\
    next\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert_eq!(out.matches("i <- 1").count(), 2);
        assert!(out.contains("steps <- (steps + 1)"));
        assert!(out.contains("out[i] <- x[i]"));
    }

    #[test]
    fn removes_dead_parenthesized_eval_lines() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  x <- 1\n\
  (floor(((x - 1) / 4)) + 1)\n\
  ((x + x) - 1)\n\
  rr_mark(10, 1);\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("(floor(((x - 1) / 4)) + 1)"));
        assert!(!out.contains("((x + x) - 1)"));
        assert!(out.contains("rr_mark(10, 1);"));
        assert!(out.contains("return(x)"));
    }

    #[test]
    fn removes_dead_plain_identifier_eval_lines() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  x <- 1\n\
  x\n\
  rr_mark(10, 1);\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("\n  x\n"));
        assert!(out.contains("rr_mark(10, 1);"));
        assert!(out.contains("return(x)"));
    }

    #[test]
    fn removes_noop_self_assignments() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_1 <- .__rr_cse_1\n\
  x <- x\n\
  y <- (x + 1)\n\
  return(y)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains(".__rr_cse_1 <- .__rr_cse_1"));
        assert!(!out.contains("x <- x"));
        assert!(
            out.contains("y <- (x + 1)") || out.contains("return((x + 1))"),
            "{out}"
        );
    }

    #[test]
    fn forwards_exact_scalar_expr_into_following_if_chain() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  u <- ((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N) + ((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)) - 1)\n\
  if ((face == 5)) {\n\
    lat <- (45 + ((1 - (((((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N) + ((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)) - 1) * ((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N) + ((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)) - 1))) * 0.25)) * 45))\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("u <- ((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N) + ((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)) - 1)")
                || out.contains("lat <- (45 + ((1 - (((u * u)")
                || out.contains("lat <- (45 + ((1 - (((((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)")
                || out.contains("if ((face == 5)) {\n}"),
            "{out}"
        );
        assert!(
            out.contains("lat <- (45 + ((1 - (((u * u)")
                || out.contains(
                    "lat <- (45 + ((1 - (((((((floor((((k - 1) %% grid_sq) / 40)) + 1) / N)"
                )
                || out.contains("if ((face == 5)) {\n}"),
            "{out}"
        );
    }

    #[test]
    fn exact_expr_reuse_does_not_rewrite_same_lhs_reassignment_to_self_copy() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_1 <- (x_curr / N)\n\
  if ((flag == 1)) {\n\
    .__rr_cse_1 <- (x_curr / N)\n\
    z <- ((.__rr_cse_1 + .__rr_cse_1) - 1)\n\
  }\n\
        }\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains(".__rr_cse_1 <- .__rr_cse_1"));
        assert!(out.matches(".__rr_cse_1 <- (x_curr / N)").count() <= 1);
    }

    #[test]
    fn exact_expr_reuse_does_not_leak_branch_local_temp_into_sibling_branch() {
        let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .__rr_cse_11 <- (y / size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  lat <- 0\n\
  if ((f == 6)) {\n\
    .__rr_cse_11 <- (y / size)\n\
    .__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
    lat <- ((-(45)) - ((1 - ((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))) * 45))\n\
  }\n\
  if ((f < 5)) {\n\
    lat <- ((.__rr_cse_13 - 1) * 45)\n\
  }\n\
  return(lat)\n\
        }\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("lat <- ((.__rr_cse_13 - 1) * 45)"));
    }

    #[test]
    fn boundary_alias_rewrite_does_not_corrupt_following_if_condition() {
        let input = "\
Sym_20 <- function(x, lo, hi) \n\
{\n\
  .arg_x <- x\n\
  .arg_lo <- lo\n\
  .arg_hi <- hi\n\
  y <- .arg_x\n\
  if ((y < .arg_lo)) {\n\
    y <- lo\n\
  }\n\
  if ((y > .arg_hi)) {\n\
    y <- hi\n\
  }\n\
  return(y)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("if ((y > hi)) {")
                || out.contains("if ((y > .arg_hi)) {")
                || out.contains("return(pmin(pmax(x, lo), hi))"),
            "{out}"
        );
        assert!(!out.contains("if ((lo > hi)) {"));
    }

    #[test]
    fn exact_expr_reuse_tolerates_prologue_arg_aliases_for_same_rhs_dep_write() {
        let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_size <- size\n\
  .__rr_cse_11 <- (y / size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  if ((f < 5)) {\n\
    .__rr_cse_11 <- (.arg_y / .arg_size)\n\
    lat <- (((.__rr_cse_11 + .__rr_cse_11) - 1) * 45)\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("lat <- (v * 45)")
                || out.contains("lat <- ((((y / size) + (y / size)) - 1) * 45)")
                || out
                    .contains("lat <- ((((.arg_y / .arg_size) + (.arg_y / .arg_size)) - 1) * 45)")
                || !out.contains(".__rr_cse_11 <- (.arg_y / .arg_size)"),
            "{out}"
        );
    }

    #[test]
    fn inlines_immediate_single_use_scalar_temp_into_following_assignment() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_642 <- (x_curr / N)\n\
  inlined_39_u <- ((.__rr_cse_642 + .__rr_cse_642) - 1)\n\
  .__rr_cse_648 <- (y_curr / N)\n\
  inlined_39_v <- ((.__rr_cse_648 + .__rr_cse_648) - 1)\n\
  return(inlined_39_v)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains(".__rr_cse_642 <- (x_curr / N)"));
        assert!(!out.contains(".__rr_cse_648 <- (y_curr / N)"));
        assert!(
            out.contains("inlined_39_u <- (((x_curr / N) + (x_curr / N)) - 1)")
                || !out.contains("inlined_39_u <-"),
            "{out}"
        );
        assert!(
            out.contains("inlined_39_v <- (((y_curr / N) + (y_curr / N)) - 1)")
                || out.contains("return((((y_curr / N) + (y_curr / N)) - 1))"),
            "{out}"
        );
    }

    #[test]
    fn inlines_immediate_single_use_index_temp_into_following_assignment() {
        let input = "\
Sym_1 <- function(size) \n\
{\n\
  i <- 1\n\
  .__rr_cse_65 <- rr_index_vec_floor(i:size)\n\
  y <- rr_assign_slice(y, i, size, (rr_index1_read_vec(x, .__rr_cse_65) + rr_index1_read_vec(z, .__rr_cse_65)))\n\
  return(y)\n\
        }\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains(".__rr_cse_65 <- rr_index_vec_floor(i:size)"));
        assert!(out.contains("(x + z)"), "{out}");
    }

    #[test]
    fn inlines_single_use_scalar_temp_across_adjacent_temp_setup() {
        let input = "\
Sym_244 <- function(v_m2, v_m1, v_c, v_p1, v_p2) \n\
{\n\
  .__rr_cse_10 <- ((v_m2 - (2 * v_m1)) + v_c)\n\
  .__rr_cse_20 <- (v_m2 - (4 * v_m1))\n\
  .__rr_cse_22 <- (3 * v_c)\n\
  b1 <- (((1.0833 * .__rr_cse_10) * .__rr_cse_10) + ((0.25 * (.__rr_cse_20 + .__rr_cse_22)) * (.__rr_cse_20 + .__rr_cse_22)))\n\
  return(b1)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("b1 <-") || out.contains("return(("), "{out}");
        assert!(out.contains("1.0833"), "{out}");
        assert!(out.contains("0.25"), "{out}");
        assert!(out.contains("v_m2"), "{out}");
        assert!(out.contains("v_c"), "{out}");
    }

    #[test]
    fn inlines_two_use_scalar_temp_within_straight_line_region() {
        let input = "\
Sym_244 <- function(v_m2, v_m1, v_c, v_p1, v_p2) \n\
{\n\
  .__rr_cse_22 <- (3 * v_c)\n\
  b1 <- ((0.25 * ((v_m2 - (4 * v_m1)) + .__rr_cse_22)) * ((v_m2 - (4 * v_m1)) + .__rr_cse_22))\n\
  b3 <- ((0.25 * ((.__rr_cse_22 - (4 * v_p1)) + v_p2)) * ((.__rr_cse_22 - (4 * v_p1)) + v_p2))\n\
  return(b3)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("b1 <-") || !out.contains("b1"), "{out}");
        assert!(out.contains("b3 <-") || out.contains("return(("), "{out}");
        assert!(out.contains("3 * v_c"), "{out}");
    }

    #[test]
    fn strips_unused_arg_aliases_but_keeps_used_ones() {
        let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_size <- size\n\
  u <- (((x / size) + (x / size)) - 1)\n\
  v <- (((y / size) + (y / size)) - 1)\n\
  lat <- (v * 45)\n\
  return(lat)\n\
}\n\
Sym_186 <- function(px, py, pf, u, v, dt, N) \n\
{\n\
  .arg_px <- px\n\
  .arg_py <- py\n\
  .arg_dt <- dt\n\
  x <- .arg_px[i]\n\
  y <- .arg_py[i]\n\
  dx <- (x * .arg_dt)\n\
  return(dx)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains(".arg_x <- x"));
        assert!(!out.contains(".arg_y <- y"));
        assert!(!out.contains(".arg_size <- size"));
        assert!(!out.contains(".arg_px"));
        assert!(!out.contains(".arg_py"));
        assert!(!out.contains(".arg_dt"));
    }

    #[test]
    fn rewrites_immediate_ii_alias_to_i_in_loop_body() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    ii <- i\n\
    out[ii] <- (x[ii] + y[ii])\n\
    if ((out[ii] > max_v)) {\n\
      max_v <- out[ii]\n\
    }\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("ii <- i"));
        assert!(out.contains("out[i] <- (x[i] + y[i])"));
        assert!(out.contains("if ((out[i] > max_v)) {"));
        assert!(out.contains("max_v <- out[i]"));
    }

    #[test]
    fn rewrites_temp_uses_after_named_copy() {
        let input = "\
Sym_287 <- function() \n\
{\n\
  .__pc_src_tmp0 <- (temp[i] - 273.15)\n\
  .__pc_src_tmp1 <- temp[i]\n\
  .__pc_src_tmp3 <- q_s[i]\n\
  .__pc_src_tmp4 <- q_g[i]\n\
  T_c <- .__pc_src_tmp0\n\
  T <- .__pc_src_tmp1\n\
  qs <- .__pc_src_tmp3\n\
  qg <- .__pc_src_tmp4\n\
  if ((.__pc_src_tmp0 < (-(15)))) {\n\
    T_c <- .__pc_src_tmp0\n\
  }\n\
  if ((.__pc_src_tmp3 > 0)) {\n\
    melt_rate <- (qs * 0.05)\n\
  }\n\
  if ((.__pc_src_tmp4 > 0)) {\n\
    melt_rate <- (melt_rate + (qg * 0.02))\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("if ((T_c < (-(15)))) {"));
        assert!(out.contains("if ((qs > 0)) {"));
        assert!(out.contains("if ((qg > 0)) {"));
        assert!(!out.contains("if ((.__pc_src_tmp0 < (-(15)))) {"));
        assert!(!out.contains("if ((.__pc_src_tmp3 > 0)) {"));
        assert!(!out.contains("if ((.__pc_src_tmp4 > 0)) {"));
    }

    #[test]
    fn branch_local_scalar_hoist_does_not_corrupt_guard_self_compare() {
        let input = "\
Sym_303 <- function() \n\
{\n\
  i <- 1\n\
  max_u <- (-(1000))\n\
  repeat {\n\
    if (!(i <= TOTAL)) break\n\
    u_new[i] <- (u_new[i] + heat[i])\n\
    if ((u_new[i] > max_u)) {\n\
      max_u <- u_new[i]\n\
    }\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  return(max_u)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("if ((u_new[i] > max_u)) {"), "{out}");
        assert!(!out.contains("if ((u_new[i] > u_new[i])) {"), "{out}");
        assert!(out.contains("max_u <- u_new[i]"), "{out}");
    }

    #[test]
    fn branch_local_named_scalar_index_read_does_not_leak_past_if() {
        let input = "\
Sym_303 <- function() \n\
{\n\
  max_u <- (-(1000))\n\
  if ((u_new[i] > max_u)) {\n\
    max_u <- u_new[i]\n\
  }\n\
  print(max_u)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("max_u <- u_new[i]"), "{out}");
        assert!(out.contains("print(max_u)"), "{out}");
        assert!(!out.contains("print(u_new[i])"), "{out}");
    }

    #[test]
    fn simple_alias_guard_rewrite_does_not_leak_branch_local_alias_to_outer_guard() {
        let input = "\
Sym_207 <- function(x, y, w, h) \n\
{\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_w <- w\n\
  .arg_h <- h\n\
  xx <- .arg_x\n\
  yy <- .arg_y\n\
  if ((xx < 1)) {\n\
    xx <- .arg_w\n\
  } else {\n\
  }\n\
  if ((xx > .arg_w)) {\n\
    xx <- 1\n\
  } else {\n\
  }\n\
  if ((yy < 1)) {\n\
    yy <- h\n\
  } else {\n\
  }\n\
  if ((yy > .arg_h)) {\n\
    yy <- 1\n\
  } else {\n\
  }\n\
  return((((yy - 1) * .arg_w) + xx))\n\
}\n";
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input,
            true,
            &FxHashSet::default(),
            &FxHashSet::default(),
            true,
        );
        assert!(out.contains("yy <- y"));
        assert!(out.contains("xx <- w"));
        assert!(out.contains("yy <- h"));
    }

    #[test]
    fn strips_arg_aliases_in_trivial_return_wrappers() {
        let input = "\
Sym_17 <- function(n, val) \n\
{\n\
  .arg_n <- n\n\
  .arg_val <- val\n\
  return(rep.int(.arg_val, .arg_n))\n\
}\n\
Sym_49__typed_impl <- function(a, b) \n\
{\n\
  .arg_a <- a\n\
  .arg_b <- b\n\
  return(rr_parallel_vec_mul_f64(rr_intrinsic_vec_add_f64(.arg_a, .arg_b), 0.5))\n\
}\n\
Sym_186 <- function(px, py, pf, u, v, dt, N) \n\
{\n\
  .arg_px <- px\n\
  .arg_py <- py\n\
  x <- .arg_px[i]\n\
  y <- .arg_py[i]\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("return(rep.int(val, n))"));
        assert!(
            out.contains("return(rr_parallel_vec_mul_f64(rr_intrinsic_vec_add_f64(a, b), 0.5))")
        );
        assert!(!out.contains(".arg_n <- n"));
        assert!(!out.contains(".arg_val <- val"));
        assert!(!out.contains(".arg_a <- a"));
        assert!(!out.contains(".arg_b <- b"));
        assert!(
            out.contains(".arg_px <- px")
                || out.contains("x <- px[i]")
                || out.contains("return(px[i])"),
            "{out}"
        );
        assert!(
            out.contains(".arg_py <- py")
                || out.contains("y <- py[i]")
                || out.contains("py[i]")
                || !out.contains("py"),
            "{out}"
        );
    }

    #[test]
    fn collapses_trivial_copy_wrapper_and_rewrites_calls() {
        let input = "\
Sym_12 <- function(xs) \n\
{\n\
  n <- length(xs)\n\
  out <- rep.int(0, n)\n\
  out <- xs\n\
  return(out)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  a <- seq_len(8)\n\
  next_a <- Sym_12(a)\n\
  a <- Sym_12(a)\n\
  return(next_a)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("Sym_12 <- function(xs)"));
        assert!(out.contains("{\nreturn(xs)\n}") || out.contains("{\n  return(xs)\n}"));
        assert!(out.contains("return(xs)"));
        assert!(out.contains("return(a)") || out.contains("return(next_a)"));
        assert!(!out.contains("next_a <- Sym_12(a)"));
        assert!(!out.contains("a <- Sym_12(a)"));
    }

    #[test]
    fn collapses_passthrough_wrapper_with_dead_length_setup() {
        let input = "\
Sym_10 <- function(xs) \n\
{\n\
  n <- length(xs)\n\
  out <- xs\n\
  return(out)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  z <- seq_len(8)\n\
  x <- Sym_10(z)\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("Sym_10 <- function(xs)"));
        assert!(out.contains("return(xs)"));
        assert!(
            out.contains("x <- z") || out.contains("return(z)") || !out.contains("Sym_10(z)"),
            "{out}"
        );
        assert!(!out.contains("x <- Sym_10(z)"));
    }

    #[test]
    fn rewrites_simple_expression_helper_chain_calls() {
        let input = "\
Sym_39 <- function(xs) \n\
{\n\
  n <- length(xs)\n\
  s <- sum(xs)\n\
  return(s)\n\
}\n\
Sym_12 <- function(xs) \n\
{\n\
  return(Sym_39(xs) / length(xs))\n\
}\n\
Sym_1 <- function() \n\
{\n\
  z <- seq_len(8)\n\
  return(Sym_12(z))\n\
}\n";
        let pure = FxHashSet::from_iter(["Sym_39".to_string(), "Sym_12".to_string()]);
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input,
            true,
            &pure,
            &FxHashSet::default(),
            false,
        );
        assert!(
            out.contains("sum(z)") || out.contains("mean(z)") || out.contains("sum(seq_len(8))"),
            "{out}"
        );
        assert!(
            out.contains("length(z)")
                || out.contains("mean(z)")
                || out.contains("length(seq_len(8))"),
            "{out}"
        );
        assert!(!out.contains("return(Sym_12(z))"));
    }

    #[test]
    fn rewrites_metric_helper_return_call_inline() {
        let input = "\
Sym_10 <- function(name, value) \n\
{\n\
  rr_mark(125, 5);\n\
  print(name)\n\
  rr_mark(126, 5);\n\
  print(value)\n\
  return(value)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  temp <- seq_len(8)\n\
  return(Sym_10(\"heat_bench_energy\", sum(temp)))\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("print(\"heat_bench_energy\")"));
        assert!(
            out.contains(".__rr_inline_metric_0 <- sum(temp)")
                || out.contains(".__rr_inline_metric_0 <- sum(seq_len(8))")
                || out.contains("sum(temp)")
                || out.contains("sum(seq_len(8))"),
            "{out}"
        );
        assert!(
            out.contains("return(.__rr_inline_metric_0)")
                || out.contains("return(sum(temp))")
                || out.contains("return(sum(seq_len(8)))"),
            "{out}"
        );
        assert!(!out.contains("return(Sym_10(\"heat_bench_energy\", sum(temp)))"));
    }

    #[test]
    fn collapses_trivial_clamp_wrapper_and_rewrites_calls() {
        let input = "\
Sym_20 <- function(x, lo, hi) \n\
{\n\
  y <- x\n\
  if ((x < lo)) {\n\
    y <- lo\n\
  }\n\
  if ((y > hi)) {\n\
    y <- hi\n\
  }\n\
  return(y)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  next_a_cell <- 1.2\n\
  return(Sym_20(next_a_cell, 0, 1))\n\
}\n";
        let pure = FxHashSet::from_iter(["Sym_20".to_string()]);
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input,
            true,
            &pure,
            &FxHashSet::default(),
            false,
        );
        assert!(
            out.contains("return((pmin(pmax(next_a_cell, 0), 1)))")
                || out.contains("return(pmin(pmax(next_a_cell, 0), 1))")
        );
        assert!(!out.contains("return(Sym_20(next_a_cell, 0, 1))"));
    }

    #[test]
    fn collapses_trivial_unit_index_wrapper_and_rewrites_calls() {
        let input = "\
Sym_14 <- function(u, n) \n\
{\n\
  idx <- (1 + floor((u * n)))\n\
  if ((idx < 1)) {\n\
    idx <- 1\n\
  }\n\
  if ((idx > n)) {\n\
    idx <- n\n\
  }\n\
  return(idx)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  return(Sym_14(draw, n))\n\
}\n";
        let pure = FxHashSet::from_iter(["Sym_14".to_string()]);
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input,
            true,
            &pure,
            &FxHashSet::default(),
            false,
        );
        assert!(
            out.contains("return((pmin(pmax((1 + floor((draw * n))), 1), n)))")
                || out.contains("return(pmin(pmax((1 + floor((draw * n))), 1), n))"),
            "{out}"
        );
        assert!(!out.contains("return(Sym_14(draw, n))"), "{out}");
    }

    #[test]
    fn collapses_trivial_dot_product_wrapper_and_rewrites_calls() {
        let input = "\
Sym_117 <- function(a, b, n) \n\
{\n\
  sum <- 0\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    sum <- (sum + (a[i] * b[i]))\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  return(sum)\n\
}\n\
Sym_1 <- function() \n\
{\n\
  return(Sym_117(r, Ap, size))\n\
}\n";
        let pure = FxHashSet::from_iter([String::from("Sym_117")]);
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input,
            true,
            &pure,
            &FxHashSet::default(),
            false,
        );
        assert!(
            out.contains("return((sum((r[seq_len(size)] * Ap[seq_len(size)]))))")
                || out.contains("return(sum((r[seq_len(size)] * Ap[seq_len(size)])))"),
            "{out}"
        );
        assert!(!out.contains("return(Sym_117(r, Ap, size))"), "{out}");
    }

    #[test]
    fn collapses_contextual_full_range_gather_replay_to_direct_gather() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  adj_ll <- (rep.int(0, TOTAL))\n\
  i <- 1\n\
  adj_ll <- rr_assign_slice(adj_ll, i, TOTAL, rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:TOTAL)))))\n\
  return(adj_ll)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("adj_ll <- rr_gather(adj_l, rr_index_vec_floor(adj_l))")
                || out.contains("return(rr_gather(adj_l, rr_index_vec_floor(adj_l)))"),
            "{out}"
        );
        assert!(
            !out.contains("adj_ll <- rr_assign_slice(adj_ll, i, TOTAL, rr_gather(adj_l, rr_index_vec_floor(rr_index1_read_vec(adj_l, rr_index_vec_floor(i:TOTAL)))))"),
            "{out}"
        );
    }

    #[test]
    fn collapses_inlined_copy_vec_sequence_to_direct_alias_and_swap() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  inlined_9_n <- length(temp)\n\
  inlined_9_out <- rep.int(0, inlined_9_n)\n\
  inlined_9_i <- 1\n\
  inlined_9_out <- temp\n\
  next_temp <- inlined_9_out\n\
  repeat {\n\
    if (!(i < n)) break\n\
    next_temp[i] <- temp[i]\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)\n\
  return(temp)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("next_temp <- temp"));
        assert!(out.contains("return(temp)") || out.contains("temp <- next_temp"));
    }

    #[test]
    fn collapses_copy_vec_sequence_after_named_copy_alias_cleanup() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  inlined_9_n <- length(temp)\n\
  inlined_9_out <- rep.int(0, inlined_9_n)\n\
  inlined_9_i <- 1\n\
  inlined_9_out <- temp\n\
  next_temp <- temp\n\
  repeat {\n\
    if (!(i < n)) break\n\
    next_temp[i] <- temp[i]\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)\n\
  return(temp)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("next_temp <- temp"));
        assert!(out.contains("temp <- next_temp"));
        assert!(
            !out.contains("temp <- rr_assign_slice(inlined_9_out, inlined_9_i, inlined_9_n, temp)")
        );
    }

    #[test]
    fn strips_unreachable_sym_helpers_after_call_rewrite() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  return(Sym_10(\"x\", Sym_11(temp)))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_1())\n\
}\n\
Sym_7 <- function(xs) \n\
{\n\
  return(xs)\n\
}\n\
Sym_11 <- function(xs) \n\
{\n\
  return(sum(xs))\n\
}\n\
Sym_10 <- function(name, value) \n\
{\n\
  print(name)\n\
  return(value)\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            (out.contains("Sym_10 <- function") && out.contains("Sym_11 <- function"))
                || (out.contains("Sym_10 <- function") && out.contains("sum(temp)"))
                || (out.contains("print(\"x\")") && out.contains("sum(temp)")),
            "{out}"
        );
        assert!(!out.contains("Sym_7 <- function"));
    }

    #[test]
    fn keeps_sym_top_entrypoint_reachable_closure() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  return(Sym_10(\"x\", Sym_11(temp)))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_1())\n\
}\n\
Sym_11 <- function(xs) \n\
{\n\
  return(sum(xs))\n\
}\n\
Sym_10 <- function(name, value) \n\
{\n\
  print(name)\n\
  return(value)\n\
}\n\
# --- RR synthesized entrypoints (auto-generated) ---\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("Sym_top_0 <- function"));
        assert!(
            (out.contains("Sym_1 <- function")
                && out.contains("Sym_10 <- function")
                && out.contains("Sym_11 <- function"))
                || (out.contains("Sym_1 <- function")
                    && out.contains("Sym_10 <- function")
                    && out.contains("sum(temp)"))
                || (out.contains("print(\"x\")") && out.contains("sum(temp)")),
            "{out}"
        );
        assert!(out.contains("Sym_top_0()"));
    }

    #[test]
    fn keeps_helper_only_sym_defs_when_synthesized_entrypoint_is_null() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  return(1)\n\
}\n\
Sym_2 <- function() \n\
{\n\
  return(2)\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(NULL)\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("Sym_1 <- function"));
        assert!(out.contains("Sym_2 <- function"));
        assert!(out.contains("Sym_top_0 <- function"));
    }

    #[test]
    fn preserve_all_defs_keeps_unreachable_sym_helpers() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  return(1)\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_1())\n\
}\n\
Sym_99 <- function() \n\
{\n\
  print(\"DROP\")\n\
  return(2)\n\
}\n\
Sym_top_0()\n";
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input,
            true,
            &FxHashSet::default(),
            &FxHashSet::default(),
            true,
        );
        assert!(out.contains("Sym_99 <- function"));
        assert!(out.contains("print(\"DROP\")"));
    }

    #[test]
    fn keeps_typed_parallel_impl_helper_referenced_as_symbol_argument() {
        let input = "\
Sym_49__typed_impl <- function(a, b) \n\
{\n\
  return(rr_parallel_vec_mul_f64(rr_intrinsic_vec_add_f64(a, b), 0.5))\n\
}\n\
Sym_49 <- function(a, b) \n\
{\n\
  return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), a, b))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_49(c(1, 2), c(2, 1)))\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("Sym_49__typed_impl <- function"));
        assert!(out.contains("rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl"));
    }

    #[test]
    fn reuses_exact_parallel_typed_vec_call_binding_inside_nested_pure_expr() {
        let input = "\
Sym_top_0 <- function() \n\
{\n\
  probe_vec <- rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2, 3, 4), c(4, 3, 2, 1))\n\
  probe_energy <- mean(abs(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2, 3, 4), c(4, 3, 2, 1))))\n\
  return(probe_energy)\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("probe_vec <- rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl"),
            "{out}"
        );
        assert!(
            out.contains("probe_energy <- mean(abs(probe_vec))")
                || out.contains("probe_energy <- (mean(abs(probe_vec)))")
                || out.contains("return(mean(abs(probe_vec)))")
                || out.contains("return((mean(abs(probe_vec))))"),
            "{out}"
        );
        assert!(
            !out.contains("probe_energy <- mean(abs(rr_parallel_typed_vec_call(\"Sym_49\"")
                && !out
                    .contains("probe_energy <- (mean(abs((rr_parallel_typed_vec_call(\"Sym_49\""),
            "{out}"
        );
    }

    #[test]
    fn drops_unreachable_typed_parallel_wrapper_when_only_string_name_remains() {
        let input = "\
Sym_49__typed_impl <- function(a, b) \n\
{\n\
  return(((a + b) * 0.5))\n\
}\n\
Sym_49 <- function(a, b) \n\
{\n\
  return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), a, b))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl, c(1L, 2L), c(1, 2), c(2, 1)))\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("Sym_49 <- function"), "{out}");
        assert!(out.contains("Sym_49__typed_impl <- function"), "{out}");
        assert!(
            out.contains("rr_parallel_typed_vec_call(\"Sym_49\", Sym_49__typed_impl"),
            "{out}"
        );
    }

    #[test]
    fn removes_redundant_identical_rr_field_get_rebind_after_loop() {
        let input = "\
Sym_top_0 <- function() \n\
{\n\
  particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N, TOTAL)\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  p_y <- rr_field_get(particles, \"py\")\n\
  p_f <- rr_field_get(particles, \"pf\")\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= TOTAL)) break\n\
    u_stage[i] <- u[i]\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  p_y <- rr_field_get(particles, \"py\")\n\
  p_f <- rr_field_get(particles, \"pf\")\n\
  return(p_f)\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.matches("p_x <-").count() <= 1, "{out}");
        assert!(out.matches("p_y <-").count() <= 1, "{out}");
        assert!(out.matches("p_f <-").count() <= 1, "{out}");
    }

    #[test]
    fn strips_unused_trailing_helper_param_and_updates_callsites() {
        let input = "\
Sym_186 <- function(px, py, pf, u, v, dt, N, total_grid) \n\
{\n\
  out_px <- px\n\
  out_py <- py\n\
  out_pf <- pf\n\
  i <- 1\n\
  if ((i == 1)) {\n\
    out_px[i] <- px[i]\n\
  }\n\
  return(rr_named_list(\"px\", out_px, \"py\", out_py, \"pf\", out_pf))\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N, TOTAL)\n\
  return(rr_field_get(particles, \"pf\"))\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("Sym_186 <- function(px, py, pf)"), "{out}");
        assert!(
            !out.contains("Sym_186 <- function(px, py, pf, u, v, dt, N, total_grid)"),
            "{out}"
        );
        assert!(out.contains("particles <- Sym_186(p_x, p_y, p_f)"), "{out}");
        assert!(
            !out.contains("particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N, TOTAL)"),
            "{out}"
        );
    }

    #[test]
    fn strips_unused_middle_helper_params_and_updates_callsites() {
        let input = "\
Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size) \n\
{\n\
  heat <- rep.int(0, size)\n\
  if ((q_c[1] > 0)) {\n\
    heat[1] <- (q_c[1] + q_v[1])\n\
  }\n\
  if ((q_s[1] > 0)) {\n\
    heat[1] <- (heat[1] + q_g[1])\n\
  }\n\
  return(heat)\n\
}\n\
Sym_top_0 <- function() \n\
{\n\
  return(Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL))\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("Sym_287 <- function(q_v, q_c, q_s, q_g, size)"),
            "{out}"
        );
        assert!(
            !out.contains("Sym_287 <- function(temp, q_v, q_c, q_r, q_i, q_s, q_g, size)"),
            "{out}"
        );
        assert!(
            out.contains("return(Sym_287(qv, qc, qs, qg, TOTAL))"),
            "{out}"
        );
        assert!(
            !out.contains("return(Sym_287(temp, qv, qc, qr, qi, qs, qg, TOTAL))"),
            "{out}"
        );
    }

    #[test]
    fn rewrites_literal_field_get_calls_to_base_indexing() {
        let input = "\
Sym_top_0 <- function() \n\
{\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  return(rr_field_get(particles, \"pf\"))\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("return(particles[[\"pf\"]])"), "{out}");
        assert!(!out.contains("rr_field_get(particles, \"px\")"), "{out}");
        assert!(!out.contains("rr_field_get(particles, \"pf\")"), "{out}");
    }

    #[test]
    fn rewrites_literal_named_list_calls_to_base_list() {
        let input = "\
Sym_top_0 <- function() \n\
{\n\
  return(rr_named_list(\"px\", px, \"py\", py, \"pf\", pf))\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("return(list(px = px, py = py, pf = pf))"),
            "{out}"
        );
        assert!(
            !out.contains("rr_named_list(\"px\", px, \"py\", py, \"pf\", pf)"),
            "{out}"
        );
    }

    #[test]
    fn rewrites_safe_loop_index_write_calls_to_base_indexing() {
        let input = "\
Sym_1 <- function(n, xs) \n\
{\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    xs[rr_index1_write(i, \"index\")] <- 0\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  return(xs)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("xs[i] <- 0"), "{out}");
        assert!(!out.contains("rr_index1_write(i, \"index\")"), "{out}");
    }

    #[test]
    fn keeps_index_write_helper_when_loop_index_is_reassigned_non_canonically() {
        let input = "\
Sym_1 <- function(n, xs) \n\
{\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    xs[rr_index1_write(i, \"index\")] <- 0\n\
    i <- (i * 2)\n\
    next\n\
  }\n\
  return(xs)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("rr_index1_write(i, \"index\")"), "{out}");
    }

    #[test]
    fn rewrites_safe_named_index_read_calls_to_base_indexing() {
        let input = "\
Sym_1 <- function(u, f, ix, iy, N) \n\
{\n\
  idx <- rr_idx_cube_vec_i(f, ix, iy, N)\n\
  dx <- ((rr_index1_read(u, idx, \"index\") * 0.1) / 4)\n\
  return(dx)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("dx <- ((u[idx] * 0.1) / 4)")
                || out.contains("dx <- ((u[rr_idx_cube_vec_i(f, ix, iy, N)] * 0.1) / 4)"),
            "{out}"
        );
        assert!(!out.contains("rr_index1_read(u, idx, \"index\")"), "{out}");
    }

    #[test]
    fn rewrites_floor_clamped_named_index_reads_to_base_indexing() {
        let input = "\
Sym_1 <- function(samples, draws, n, inner, resample) \n\
{\n\
  idx <- (pmin(pmax((1 + floor((rr_index1_read(draws, (((resample - 1) * n) + inner), \"index\") * n))), 1), n))\n\
  s <- (0 + rr_index1_read(samples, idx, \"index\"))\n\
  return(s)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("s <- (0 + samples[idx])")
                || out.contains("s <- (samples[idx])")
                || out.contains("return((0 + samples[idx]))")
                || out.contains("return(samples[idx])"),
            "{out}"
        );
        assert!(
            !out.contains("rr_index1_read(samples, idx, \"index\")"),
            "{out}"
        );
    }

    #[test]
    fn rewrites_flat_positive_loop_index_reads_to_base_indexing() {
        let input = "\
Sym_1 <- function(draws, n, resamples) \n\
{\n\
  resample <- 1\n\
  repeat {\n\
    if (!(resample <= resamples)) break\n\
    inner <- 1\n\
    repeat {\n\
      if (!(inner <= n)) break\n\
      draw <- rr_index1_read(draws, (((resample - 1) * n) + inner), \"index\")\n\
      inner <- (inner + 1)\n\
      next\n\
    }\n\
    resample <- (resample + 1)\n\
    next\n\
  }\n\
  return(draw)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("draw <- draws[(((resample - 1) * n) + inner)]"),
            "{out}"
        );
        assert!(
            !out.contains("rr_index1_read(draws, (((resample - 1) * n) + inner), \"index\")"),
            "{out}"
        );
    }

    #[test]
    fn rewrites_same_len_tail_scalar_reads_to_base_indexing() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 8\n\
  clean <- rep.int(0, n)\n\
  clean <- ifelse((clean > 0.4), sqrt((clean + 0.1)), ((clean * 0.55) + 0.03))\n\
  print(rr_index1_read(clean, n, \"index\"))\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("print(clean[n])"), "{out}");
        assert!(
            !out.contains("rr_index1_read(clean, n, \"index\")"),
            "{out}"
        );
    }

    #[test]
    fn rewrites_safe_interior_loop_neighbor_reads_to_base_indexing() {
        let input = "\
Sym_1 <- function(n, a, next_a) \n\
{\n\
  i <- 2\n\
  repeat {\n\
    if (!(i < n)) break\n\
    lap_a <- ((rr_index1_read(a, (i - 1), \"index\") - (2 * rr_index1_read(a, i, \"index\"))) + rr_index1_read(a, (i + 1), \"index\"))\n\
    next_a[rr_index1_write(i, \"index\")] <- lap_a\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  return(next_a)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("lap_a <- ((a[(i - 1)] - (2 * a[i])) + a[(i + 1)])")
                || out.contains("next_a[i] <- ((a[(i - 1)] - (2 * a[i])) + a[(i + 1)])"),
            "{out}"
        );
        assert!(
            out.contains("next_a[i] <- lap_a")
                || out.contains("next_a[i] <- ((a[(i - 1)] - (2 * a[i])) + a[(i + 1)])"),
            "{out}"
        );
        assert!(!out.contains("rr_index1_read(a, i, \"index\")"), "{out}");
        assert!(!out.contains("rr_index1_write(i, \"index\")"), "{out}");
    }

    #[test]
    fn keeps_identical_rr_field_get_rebind_when_particles_change_inside_loop() {
        let input = "\
Sym_top_0 <- function() \n\
{\n\
  particles <- Sym_186(p_x, p_y, p_f, u, v, dt, N, TOTAL)\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= TOTAL)) break\n\
    particles <- rr_named_list(\"px\", p_x, \"py\", p_y, \"pf\", p_f)\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  p_x <- rr_field_get(particles, \"px\")\n\
  return(p_x)\n\
}\n\
Sym_top_0()\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("particles <- rr_named_list(\"px\", p_x, \"py\", p_y, \"pf\", p_f)")
                || out.contains("particles <- list(px = p_x, py = p_y, pf = p_f)"),
            "{out}"
        );
        assert!(
            out.contains("return(rr_field_get(particles, \"px\"))")
                || out
                    .matches("p_x <- rr_field_get(particles, \"px\")")
                    .count()
                    == 2
                || out.contains("return(particles[[\"px\"]])")
                || out.matches("p_x <- particles[[\"px\"]]").count() >= 1,
            "{out}"
        );
    }

    #[test]
    fn does_not_inline_singleton_assign_slice_as_whole_range_copy() {
        let input = "\
Sym_78 <- function(f, ys) \n\
{\n\
  rot <- rep.int(0, length(ys))\n\
  if ((f == 2)) {\n\
    rot <- rr_assign_slice(rot, 1, 1, rep.int(1, 1))\n\
    return(rot)\n\
  } else {\n\
    return(rot)\n\
  }\n\
}\n";
        let (out, _) = optimize_emitted_r_with_context_and_fresh_with_options(
            input,
            true,
            &FxHashSet::default(),
            &FxHashSet::default(),
            true,
        );
        assert!(out.contains("rot <- replace(rot, 1, 1)"));
        assert!(!out.contains("rot <- rep.int(1, 1)"));
        assert!(!out.contains("rot <- rr_assign_slice(rot, 1, 1, rep.int(1, 1))"));
    }

    #[test]
    fn rewrites_readonly_param_aliases_and_index_only_mutated_param_shadows() {
        let input = "\
Sym_60 <- function(f, x, ys, size) \n\
{\n\
  .arg_f <- f\n\
  .arg_x <- x\n\
  .arg_ys <- ys\n\
  .arg_size <- size\n\
  if ((.arg_f == 1)) {\n\
    return(rr_idx_cube_vec_i(rep.int(4, length(ys)), rep.int(.arg_size, length(ys)), .arg_ys, .arg_size))\n\
  }\n\
  return(rr_idx_cube_vec_i(rep.int(.arg_f, length(ys)), rep.int((.arg_x - 1), length(ys)), .arg_ys, .arg_size))\n\
}\n\
Sym_186 <- function(px, py, pf, u, v, dt, N) \n\
{\n\
  .arg_px <- px\n\
  .arg_py <- py\n\
  .arg_dt <- dt\n\
  x <- .arg_px[i]\n\
  .arg_px[i] <- x\n\
  return(.arg_dt)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains(".arg_f <- f"));
        assert!(!out.contains(".arg_x <- x"));
        assert!(!out.contains(".arg_ys <- ys"));
        assert!(!out.contains(".arg_size <- size"));
        assert!(out.contains("if ((f == 1)) {"));
        assert!(out.contains("rep.int(size, length(ys))"));
        assert!(out.contains("return(rr_idx_cube_vec_i(rep.int(f, length(ys)), rep.int((x - 1), length(ys)), ys, size))"));
        assert!(!out.contains(".arg_px"));
        assert!(!out.contains(".arg_py"));
        assert!(!out.contains(".arg_dt"));
    }

    #[test]
    fn does_not_rewrite_mutated_param_shadow_aliases() {
        let input = "\
Sym_13 <- function(n, acc) \n\
{\n\
  .arg_n <- n\n\
  .arg_acc <- acc\n\
  repeat {\n\
    if ((.arg_n <= 0L)) break\n\
    .__pc_src_tmp0 <- (.arg_n - 1L)\n\
    .__pc_src_tmp1 <- (.arg_acc + .arg_n)\n\
    .arg_n <- .__pc_src_tmp0\n\
    .arg_acc <- .__pc_src_tmp1\n\
    next\n\
  }\n\
  return(.arg_acc)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains(".arg_n <- n"));
        assert!(out.contains(".arg_n <- .__pc_src_tmp0"));
        assert!(out.contains("if ((.arg_n <= 0L)) break"));
    }

    #[test]
    fn readonly_param_alias_rewrite_keeps_mutated_tco_shadow_aliases() {
        let input = vec![
            "Sym_13 <- function(n, acc) ".to_string(),
            "{".to_string(),
            "  .arg_n <- n".to_string(),
            "  .arg_acc <- acc".to_string(),
            "  repeat {".to_string(),
            "    if ((.arg_n <= 0L)) break".to_string(),
            "    .__pc_src_tmp0 <- (.arg_n - 1L)".to_string(),
            "    .__pc_src_tmp1 <- (.arg_acc + .arg_n)".to_string(),
            "    .arg_n <- .__pc_src_tmp0".to_string(),
            "    .arg_acc <- .__pc_src_tmp1".to_string(),
            "    next".to_string(),
            "  }".to_string(),
            "  return(.arg_acc)".to_string(),
            "}".to_string(),
        ];
        let out = super::rewrite_readonly_param_aliases(input).join("\n");
        assert!(out.contains(".arg_n <- n"));
        assert!(out.contains("if ((.arg_n <= 0L)) break"));
        assert!(out.contains(".arg_n <- .__pc_src_tmp0"));
    }

    #[test]
    fn strip_unused_arg_aliases_keeps_mutated_tco_shadow_aliases() {
        let input = vec![
            "Sym_13 <- function(n, acc) ".to_string(),
            "{".to_string(),
            "  .arg_n <- n".to_string(),
            "  .arg_acc <- acc".to_string(),
            "  repeat {".to_string(),
            "    if ((.arg_n <= 0L)) break".to_string(),
            "    .__pc_src_tmp0 <- (.arg_n - 1L)".to_string(),
            "    .__pc_src_tmp1 <- (.arg_acc + .arg_n)".to_string(),
            "    .arg_n <- .__pc_src_tmp0".to_string(),
            "    .arg_acc <- .__pc_src_tmp1".to_string(),
            "    next".to_string(),
            "  }".to_string(),
            "  return(.arg_acc)".to_string(),
            "}".to_string(),
        ];
        let out = super::strip_unused_arg_aliases(input).join("\n");
        assert!(out.contains(".arg_n <- n"));
        assert!(out.contains(".arg_n <- .__pc_src_tmp0"));
    }

    #[test]
    fn readonly_param_alias_rewrite_fully_rewrites_recursive_param_uses() {
        let input = vec![
            "Sym_13 <- function(n, acc) ".to_string(),
            "{".to_string(),
            "  .arg_n <- n".to_string(),
            "  .arg_acc <- acc".to_string(),
            "  if ((.arg_n <= 0L)) {".to_string(),
            "    return(.arg_acc)".to_string(),
            "  } else {".to_string(),
            "    return(Sym_13((.arg_n - 1L), (.arg_acc + .arg_n)))".to_string(),
            "  }".to_string(),
            "}".to_string(),
        ];
        let out = super::rewrite_readonly_param_aliases(input).join("\n");
        assert!(!out.contains(".arg_n <- n"));
        assert!(!out.contains(".arg_acc <- acc"));
        assert!(out.contains("if ((n <= 0L)) {"));
        assert!(out.contains("return(acc)"));
        assert!(out.contains("return(Sym_13((n - 1L), (acc + n)))"));
    }

    #[test]
    fn shifted_square_reuse_collapses_recent_temp_chain_to_named_scalar() {
        let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .__rr_cse_5 <- (x / size)\n\
  u <- ((.__rr_cse_5 + .__rr_cse_5) - 1)\n\
  .__rr_cse_11 <- (y / size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  if ((f == 6)) {\n\
    .__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)\n\
    .__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
    lat <- ((-(45)) - ((1 - ((((.__rr_cse_7 - 1) * (.__rr_cse_7 - 1)) + ((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))) * 0.25)) * 45))\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("lat <-") || out.contains("if ((f == 6)) {\n}"),
            "{out}"
        );
        assert!(
            out.contains("* 45") || out.contains("if ((f == 6)) {\n}"),
            "{out}"
        );
        assert!(
            !out.contains(".__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)"),
            "{out}"
        );
        assert!(
            !out.contains(".__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)"),
            "{out}"
        );
    }

    #[test]
    fn shifted_square_reuse_handles_actual_sym37_shape() {
        let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_size <- size\n\
  .__rr_cse_5 <- (x / size)\n\
  u <- ((.__rr_cse_5 + .__rr_cse_5) - 1)\n\
  .__rr_cse_11 <- (y / size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  if ((f == 6)) {\n\
    .__rr_cse_5 <- (.arg_x / .arg_size)\n\
    .__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)\n\
    .__rr_cse_11 <- (.arg_y / .arg_size)\n\
    .__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
    lat <- ((-(45)) - ((1 - ((((.__rr_cse_7 - 1) * (.__rr_cse_7 - 1)) + ((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))) * 0.25)) * 45))\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(
            out.contains("lat <-") || out.contains("if ((f == 6)) {\n}"),
            "{out}"
        );
        assert!(
            out.contains("* 45") || out.contains("if ((f == 6)) {\n}"),
            "{out}"
        );
        assert!(
            !out.contains(".__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)"),
            "{out}"
        );
        assert!(
            !out.contains(".__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)"),
            "{out}"
        );
    }

    #[test]
    fn shifted_square_reuse_handles_raw_emitted_sym37_shape() {
        let input = "\
Sym_37 <- function(f, x, y, size) \n\
{\n\
  .arg_f <- f\n\
  .arg_x <- x\n\
  .arg_y <- y\n\
  .arg_size <- size\n\
  .__rr_cse_5 <- (.arg_x / .arg_size)\n\
  u <- ((.__rr_cse_5 + .__rr_cse_5) - 1)\n\
  .__rr_cse_11 <- (.arg_y / .arg_size)\n\
  v <- ((.__rr_cse_11 + .__rr_cse_11) - 1)\n\
  lat <- 0\n\
  if ((.arg_f == 5)) {\n\
    .__rr_cse_5 <- (.arg_x / .arg_size)\n\
    .__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)\n\
    .__rr_cse_11 <- (.arg_y / .arg_size)\n\
    .__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
    lat <- (45 + ((1 - (((u * u) + (v * v)) * 0.25)) * 45))\n\
  } else {\n\
  }\n\
  if ((.arg_f == 6)) {\n\
    .__rr_cse_5 <- (.arg_x / .arg_size)\n\
    .__rr_cse_7 <- (.__rr_cse_5 + .__rr_cse_5)\n\
    .__rr_cse_11 <- (.arg_y / .arg_size)\n\
    .__rr_cse_13 <- (.__rr_cse_11 + .__rr_cse_11)\n\
    lat <- ((-(45)) - ((1 - ((((.__rr_cse_7 - 1) * (.__rr_cse_7 - 1)) + ((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))) * 0.25)) * 45))\n\
  } else {\n\
  }\n\
  if ((.arg_f < 5)) {\n\
    .__rr_cse_11 <- (.arg_y / .arg_size)\n\
    lat <- (((.__rr_cse_11 + .__rr_cse_11) - 1) * 45)\n\
  } else {\n\
  }\n\
  return(lat)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("((u * u) + (v * v))"));
        assert!(!out.contains("((.__rr_cse_7 - 1) * (.__rr_cse_7 - 1))"));
        assert!(!out.contains("((.__rr_cse_13 - 1) * (.__rr_cse_13 - 1))"));
        assert!(out.contains("lat <-"));
        assert!(out.contains("* 45)"));
    }

    #[test]
    fn removes_redundant_identical_nested_temp_reassign() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_1 <- (x_curr / N)\n\
  if ((f_curr == 6)) {\n\
    .__rr_cse_1 <- (x_curr / N)\n\
    y <- ((.__rr_cse_1 + .__rr_cse_1) - 1)\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.matches(".__rr_cse_1 <- (x_curr / N)").count() <= 1);
        assert!(
            out.contains("y <-")
                || out.contains("return(")
                || out.contains("if ((f_curr == 6)) {\n}"),
            "{out}"
        );
    }

    #[test]
    fn removes_redundant_temp_reassign_even_with_intermediate_self_copy() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  .__rr_cse_1 <- (x_curr / N)\n\
  if ((flag == 1)) {\n\
    .__rr_cse_1 <- .__rr_cse_1\n\
    .__rr_cse_1 <- (x_curr / N)\n\
    z <- (.__rr_cse_1 + 1)\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.matches(".__rr_cse_1 <- (x_curr / N)").count() <= 1);
    }

    #[test]
    fn strips_empty_else_blocks() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  if (flag) {\n\
    x <- 1\n\
  } else {\n\
  }\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("} else {\n}"));
        assert!(!out.contains("} else {\n  }"));
        assert!(out.contains("if (flag) {"));
        assert!(out.contains("x <- 1"));
    }

    #[test]
    fn collapses_common_if_else_tail_assignments() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  if ((f == 5)) {\n\
    lat <- 1\n\
    arg_f <- f\n\
  } else {\n\
    arg_f <- f\n\
  }\n\
  if ((arg_f == 6)) {\n\
    lat <- 2\n\
  }\n\
        }\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.matches("arg_f <- f").count() <= 1);
    }

    #[test]
    fn collapses_common_if_else_tail_assignment_sequences() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  if (flag) {\n\
    x <- 1\n\
    a <- foo\n\
    b <- bar\n\
  } else {\n\
    a <- foo\n\
    b <- bar\n\
  }\n\
  return(b)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.matches("a <- foo").count() <= 1);
        assert!(out.matches("b <- bar").count() <= 1);
        assert!(out.contains("return(bar)") || out.contains("b <- bar"));
    }

    #[test]
    fn collapses_common_if_else_tail_assignments_for_sym287_shape() {
        let input = "\
Sym_287 <- function() \n\
{\n\
  if ((T_c < (-(5)))) {\n\
    if ((qc > 0.0001)) {\n\
      rate <- (0.01 * qc)\n\
      tendency_T <- (rate * L_f)\n\
    }\n\
    .__pc_src_tmp0 <- (temp[i] - 273.15)\n\
    .__pc_src_tmp1 <- temp[i]\n\
    .__pc_src_tmp2 <- q_v[i]\n\
    .__pc_src_tmp3 <- q_s[i]\n\
    .__pc_src_tmp4 <- q_g[i]\n\
    T_c <- .__pc_src_tmp0\n\
    T <- .__pc_src_tmp1\n\
    qs <- .__pc_src_tmp3\n\
    qg <- .__pc_src_tmp4\n\
  } else {\n\
    .__pc_src_tmp0 <- (temp[i] - 273.15)\n\
    .__pc_src_tmp1 <- temp[i]\n\
    .__pc_src_tmp2 <- q_v[i]\n\
    .__pc_src_tmp3 <- q_s[i]\n\
    .__pc_src_tmp4 <- q_g[i]\n\
    T_c <- .__pc_src_tmp0\n\
    T <- .__pc_src_tmp1\n\
    qs <- .__pc_src_tmp3\n\
    qg <- .__pc_src_tmp4\n\
  }\n\
  return(qg)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.matches(".__pc_src_tmp0 <- (temp[i] - 273.15)").count() <= 1);
        assert!(out.matches(".__pc_src_tmp1 <- temp[i]").count() <= 1);
        assert!(out.matches(".__pc_src_tmp2 <- q_v[i]").count() <= 1);
        assert!(out.matches(".__pc_src_tmp3 <- q_s[i]").count() <= 1);
        assert!(out.matches(".__pc_src_tmp4 <- q_g[i]").count() <= 1);
        assert!(out.matches("T_c <- .__pc_src_tmp0").count() <= 1);
        assert!(out.matches("T <- .__pc_src_tmp1").count() <= 1);
        assert!(out.matches("qs <- .__pc_src_tmp3").count() <= 1);
        assert!(out.matches("qg <- .__pc_src_tmp4").count() <= 1);
        assert!(
            out.contains("return(qg)") || out.contains("return(q_g[i])"),
            "{out}"
        );
    }

    #[test]
    fn rewrites_if_truthy_scalar_guards() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  f <- 5\n\
  if (rr_truthy1((f == 5), \"condition\")) {\n\
    x <- 1\n\
  }\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("if ((f == 5)) {"));
        assert!(!out.contains("if (rr_truthy1("));
    }

    #[test]
    fn forwards_simple_alias_into_following_guards_only() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  arg_f <- f_curr\n\
  if ((arg_f == 6)) {\n\
    lat <- 1\n\
  }\n\
  if ((arg_f < 5)) {\n\
    lat <- 2\n\
  }\n\
  return(lat)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("arg_f <- f_curr"));
        assert!(out.contains("if ((f_curr == 6)) {"));
        assert!(
            out.contains("if ((f_curr < 5)) {") || out.contains("if ((f_curr <= 4)) {"),
            "{out}"
        );
    }

    #[test]
    fn removes_dead_pure_helper_call_assignment() {
        let input = "\
Sym_top_0 <- function() \n\
{\n\
  rot_l <- Sym_91(1, N)\n\
  used <- 1\n\
  return(used)\n\
}\n";
        let pure = FxHashSet::from_iter([String::from("Sym_91")]);
        let (out, _) = optimize_emitted_r_with_context(input, true, &pure);
        assert!(!out.contains("rot_l <- Sym_91(1, N)"));
        assert!(out.contains("used <- 1"));
    }

    #[test]
    fn removes_simple_init_overwritten_before_first_read() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  k0 <- 0\n\
  rem <- 0\n\
  k0 <- (k - 1)\n\
  rem <- (k0 %% grid_sq)\n\
  return(rem)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("k0 <- 0"));
        assert!(!out.contains("rem <- 0"));
        assert!(
            out.contains("k0 <- (k - 1)") || out.contains("return(((k - 1) %% grid_sq))"),
            "{out}"
        );
        assert!(
            out.contains("rem <- (k0 %% grid_sq)") || out.contains("return(((k - 1) %% grid_sq))"),
            "{out}"
        );
    }

    #[test]
    fn indexed_write_invalidates_stale_return_rhs_rewrite() {
        let input = "\
Sym_183 <- function(n) \n\
{\n\
  p <- seq_len(n)\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    p[i] <- (seed / 2147483648)\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  return(p)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("return(p)"));
        assert!(!out.contains("return(seq_len(n))"));
    }

    #[test]
    fn removes_branch_local_init_overwritten_before_first_read_in_loop() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  dist <- 0\n\
  repeat {\n\
    if (!(k <= n)) break\n\
    if ((flag == 1)) {\n\
      dist <- ((dx * dx) + (dy * dy))\n\
      out[k] <- dist\n\
    }\n\
    k <- (k + 1)\n\
    next\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("dist <- 0"));
        assert!(
            out.contains("dist <- ((dx * dx) + (dy * dy))")
                || out.contains("out[k] <- ((dx * dx) + (dy * dy))"),
            "{out}"
        );
        assert!(
            out.contains("out[k] <- dist") || out.contains("out[k] <- ((dx * dx) + (dy * dy))"),
            "{out}"
        );
    }

    #[test]
    fn keeps_loop_accumulator_init_used_after_inner_repeat() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  sum1 <- 0\n\
  count1 <- 0\n\
  i <- 1\n\
  repeat {\n\
    if (!(i <= n)) break\n\
    if ((flag == 1)) {\n\
      sum1 <- (sum1 + x)\n\
      count1 <- (count1 + 1)\n\
    }\n\
    i <- (i + 1)\n\
    next\n\
  }\n\
  if ((count1 > 0)) {\n\
    out <- (sum1 / count1)\n\
  }\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("sum1 <- 0"));
        assert!(out.contains("count1 <- 0"));
        assert!(
            out.contains("out <- (sum1 / count1)")
                || out.contains("return((sum1 / count1))")
                || out.contains("if ((count1 > 0)) {\n}"),
            "{out}"
        );
    }

    #[test]
    fn removes_redundant_tail_assign_slice_after_non_empty_repeat() {
        let input = "\
Sym_123 <- function() \n\
{\n\
  x <- rep.int(0, n)\n\
  iter <- 1\n\
  repeat {\n\
    if (!(iter <= 20)) break\n\
    i <- 1\n\
    .tachyon_exprmap0_1 <- expr\n\
    x <- rr_assign_slice(x, i, n, expr)\n\
    iter <- (iter + 1)\n\
    next\n\
  }\n\
  x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)"));
        assert!(out.contains("return(x)"));
    }

    #[test]
    fn removes_redundant_tail_assign_slice_for_sym123_shape() {
        let input = "\
Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  r <- rep.int(0, size)\n\
  p <- rep.int(0, size)\n\
  k <- 1\n\
  r <- rr_assign_slice(r, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  p <- rr_assign_slice(p, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  rs_old <- Sym_117(r, r, size)\n\
  rs_new <- 0\n\
  alpha <- 0\n\
  beta <- 0\n\
  i <- 1\n\
  iter <- 1\n\
  repeat {\n\
    if (!(iter <= 20)) break\n\
    Ap <- Sym_119(p, .arg_n_l, .arg_n_r, .arg_n_d, .arg_n_u, .arg_size)\n\
    p_Ap <- Sym_117(p, Ap, .arg_size)\n\
    alpha <- (rs_old / p_Ap)\n\
    i <- 1\n\
    .tachyon_exprmap0_1 <- (rr_index1_read_vec(x, rr_index_vec_floor(i:.arg_size)) + (alpha * rr_index1_read_vec(p, rr_index_vec_floor(i:.arg_size))))\n\
    x <- rr_assign_slice(x, i, .arg_size, (rr_index1_read_vec(x, rr_index_vec_floor(i:.arg_size)) + (alpha * rr_index1_read_vec(p, rr_index_vec_floor(i:.arg_size)))))\n\
    r <- rr_assign_slice(r, i, .arg_size, (rr_index1_read_vec(r, rr_index_vec_floor(i:.arg_size)) - (alpha * rr_index1_read_vec(Ap, rr_index_vec_floor(i:.arg_size)))))\n\
    rs_new <- Sym_117(r, r, .arg_size)\n\
    beta <- (rs_new / rs_old)\n\
    i <- 1\n\
    p <- rr_assign_slice(p, i, .arg_size, (rr_index1_read_vec(r, rr_index_vec_floor(i:.arg_size)) + (beta * rr_index1_read_vec(p, rr_index_vec_floor(i:.arg_size)))))\n\
    rs_old <- rs_new\n\
    iter <- (iter + 1)\n\
    next\n\
  }\n\
  x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)"));
        assert!(out.contains("return(x)"));
        assert!(
            out.contains("rs_old <- (sum((r[seq_len(size)] * r[seq_len(size)])))")
                || out.contains("rs_old <- sum((r[seq_len(size)] * r[seq_len(size)]))")
                || out.contains("rs_old <- Sym_117(r, r, size)")
        );
        assert!(!out.contains("rs_old <- Sym_117(rep.int(0, size), rep.int(0, size), size)"));
    }

    #[test]
    fn removes_redundant_tail_assign_slice_for_actual_sym123_raw_shape() {
        let input = "\
Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  r <- rep.int(0, size)\n\
  p <- rep.int(0, size)\n\
  k <- 1\n\
  r <- rr_assign_slice(r, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  p <- rr_assign_slice(p, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  rs_old <- Sym_117(r, r, size)\n\
  rs_new <- 0\n\
  alpha <- 0\n\
  beta <- 0\n\
  i <- 1\n\
  iter <- 1\n\
  repeat {\n\
    if (!(iter <= 20)) break\n\
    p_Ap <- Sym_117(p, Ap, .arg_size)\n\
    alpha <- (rs_old / p_Ap)\n\
    if ((is.na(alpha) | (!(is.finite(alpha))))) {\n\
      alpha <- 0\n\
    } else {\n\
    }\n\
    i <- 1\n\
    .__rr_cse_217 <- i:.arg_size\n\
    .__rr_cse_218 <- rr_index_vec_floor(.__rr_cse_217)\n\
    .tachyon_exprmap0_1 <- (rr_index1_read_vec(x, .__rr_cse_218) + (alpha * rr_index1_read_vec(p, .__rr_cse_218)))\n\
    .tachyon_exprmap1_1 <- (rr_index1_read_vec(r, .__rr_cse_218) - (alpha * rr_index1_read_vec(Ap, .__rr_cse_218)))\n\
    x <- rr_assign_slice(x, i, .arg_size, .tachyon_exprmap0_1)\n\
    r <- rr_assign_slice(r, i, .arg_size, .tachyon_exprmap1_1)\n\
    rs_new <- Sym_117(r, r, .arg_size)\n\
    beta <- (rs_new / rs_old)\n\
    .__rr_cse_231 <- 1:.arg_size\n\
    .__rr_cse_232 <- rr_index_vec_floor(.__rr_cse_231)\n\
    p <- rr_assign_slice(p, 1, .arg_size, (rr_index1_read_vec(r, .__rr_cse_232) + (beta * rr_index1_read_vec(p, .__rr_cse_232))))\n\
    rs_old <- rs_new\n\
    iter <- (iter + 1)\n\
    next\n\
  }\n\
  x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)"));
        assert!(out.contains("return(x)"));
    }

    #[test]
    fn removes_redundant_tail_assign_slice_for_actual_sym123_raw_shape_with_context() {
        let input = "\
Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  r <- rep.int(0, size)\n\
  p <- rep.int(0, size)\n\
  k <- 1\n\
  r <- rr_assign_slice(r, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  p <- rr_assign_slice(p, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  rs_old <- Sym_117(r, r, size)\n\
  rs_new <- 0\n\
  alpha <- 0\n\
  beta <- 0\n\
  i <- 1\n\
  iter <- 1\n\
  repeat {\n\
    if (!(iter <= 20)) break\n\
    p_Ap <- Sym_117(p, Ap, .arg_size)\n\
    alpha <- (rs_old / p_Ap)\n\
    if ((is.na(alpha) | (!(is.finite(alpha))))) {\n\
      alpha <- 0\n\
    } else {\n\
    }\n\
    i <- 1\n\
    .__rr_cse_217 <- i:.arg_size\n\
    .__rr_cse_218 <- rr_index_vec_floor(.__rr_cse_217)\n\
    .tachyon_exprmap0_1 <- (rr_index1_read_vec(x, .__rr_cse_218) + (alpha * rr_index1_read_vec(p, .__rr_cse_218)))\n\
    .tachyon_exprmap1_1 <- (rr_index1_read_vec(r, .__rr_cse_218) - (alpha * rr_index1_read_vec(Ap, .__rr_cse_218)))\n\
    x <- rr_assign_slice(x, i, .arg_size, .tachyon_exprmap0_1)\n\
    r <- rr_assign_slice(r, i, .arg_size, .tachyon_exprmap1_1)\n\
    rs_new <- Sym_117(r, r, .arg_size)\n\
    beta <- (rs_new / rs_old)\n\
    .__rr_cse_231 <- 1:.arg_size\n\
    .__rr_cse_232 <- rr_index_vec_floor(.__rr_cse_231)\n\
    p <- rr_assign_slice(p, 1, .arg_size, (rr_index1_read_vec(r, .__rr_cse_232) + (beta * rr_index1_read_vec(p, .__rr_cse_232))))\n\
    rs_old <- rs_new\n\
    iter <- (iter + 1)\n\
    next\n\
  }\n\
  x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
        let pure = FxHashSet::from_iter([String::from("Sym_117")]);
        let fresh = FxHashSet::from_iter([String::from("Sym_17")]);
        let (out, _) = optimize_emitted_r_with_context_and_fresh(input, true, &pure, &fresh);
        assert!(!out.contains("x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)"));
        assert!(out.contains("return(x)"));
    }

    #[test]
    fn removes_branch_local_identical_pure_rebind_after_same_outer_init() {
        let input = "\
Sym_123 <- function(b, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  rs_old <- (sum((b[seq_len(size)] * b[seq_len(size)])))\n\
  if (((is.na(rs_old) | (!(is.finite(rs_old)))) | (rs_old == 0))) {\n\
    rs_old <- 0.0000001\n\
    x <- (rep.int(0, size))\n\
  } else {\n\
  }\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains("x <- rep.int(0, size)"));
        assert!(!out.contains("x <- (rep.int(0, size))"), "{out}");
        assert!(out.contains("return(x)"));
    }

    #[test]
    fn removes_redundant_tail_assign_after_runtime_trycatch_helper() {
        let input = "\
rr_native_try_load <- function() {\n\
  ok <- tryCatch({\n\
    dyn.load(.rr_env$native_lib)\n\
    TRUE\n\
  }, error = function(e) FALSE)\n\
  .rr_env$native_loaded <- isTRUE(ok)\n\
  isTRUE(ok)\n\
}\n\
Sym_123 <- function(b, n_l, n_r, n_d, n_u, size) \n\
{\n\
  x <- rep.int(0, size)\n\
  r <- rep.int(0, size)\n\
  p <- rep.int(0, size)\n\
  k <- 1\n\
  r <- rr_assign_slice(r, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  p <- rr_assign_slice(p, k, size, rr_index1_read_vec(b, rr_index_vec_floor(k:size)))\n\
  rs_old <- Sym_117(r, r, size)\n\
  rs_new <- 0\n\
  alpha <- 0\n\
  beta <- 0\n\
  i <- 1\n\
  iter <- 1\n\
  repeat {\n\
    if (!(iter <= 20)) break\n\
    p_Ap <- Sym_117(p, Ap, .arg_size)\n\
    alpha <- (rs_old / p_Ap)\n\
    i <- 1\n\
    .__rr_cse_217 <- i:.arg_size\n\
    .__rr_cse_218 <- rr_index_vec_floor(.__rr_cse_217)\n\
    .tachyon_exprmap0_1 <- (rr_index1_read_vec(x, .__rr_cse_218) + (alpha * rr_index1_read_vec(p, .__rr_cse_218)))\n\
    x <- rr_assign_slice(x, i, .arg_size, .tachyon_exprmap0_1)\n\
    r <- rr_assign_slice(r, i, .arg_size, (rr_index1_read_vec(r, .__rr_cse_218) - (alpha * rr_index1_read_vec(Ap, .__rr_cse_218))))\n\
    rs_new <- Sym_117(r, r, .arg_size)\n\
    beta <- (rs_new / rs_old)\n\
    .__rr_cse_231 <- 1:.arg_size\n\
    .__rr_cse_232 <- rr_index_vec_floor(.__rr_cse_231)\n\
    p <- rr_assign_slice(p, 1, .arg_size, (rr_index1_read_vec(r, .__rr_cse_232) + (beta * rr_index1_read_vec(p, .__rr_cse_232))))\n\
    rs_old <- rs_new\n\
    iter <- (iter + 1)\n\
    next\n\
  }\n\
  x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)\n\
  return(x)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(!out.contains("x <- rr_assign_slice(x, 1, .arg_size, .tachyon_exprmap0_1)"));
        assert!(out.contains("return(x)"));
    }

    #[test]
    fn hoists_repeated_vector_helper_calls_within_single_assignment_rhs() {
        let input = vec![String::from(
            "  .tachyon_exprmap0_0 <- (((rr_index1_read_vec(x, rr_index_vec_floor(i:n)) + rr_index1_read_vec(x, rr_index_vec_floor(i:n))) + rr_index1_read_vec(x, rr_index_vec_floor(i:n))) + ((rr_index1_read_vec(y, rr_index_vec_floor(i:n)) * rr_index1_read_vec(y, rr_index_vec_floor(i:n))) + rr_index1_read_vec(y, rr_index_vec_floor(i:n))))",
        )];
        let out = hoist_repeated_vector_helper_calls_within_lines(input);
        let joined = out.join("\n");
        assert!(
            joined.contains(".__rr_cse_0 <- rr_index1_read_vec(x, rr_index_vec_floor(i:n))"),
            "{joined}"
        );
        assert!(
            joined.contains(".__rr_cse_1 <- rr_index1_read_vec(y, rr_index_vec_floor(i:n))"),
            "{joined}"
        );
        assert_eq!(
            joined
                .matches("rr_index1_read_vec(x, rr_index_vec_floor(i:n))")
                .count(),
            1,
            "{joined}"
        );
        assert_eq!(
            joined
                .matches("rr_index1_read_vec(y, rr_index_vec_floor(i:n))")
                .count(),
            1,
            "{joined}"
        );
        assert!(joined.contains(".tachyon_exprmap0_0 <- (((.__rr_cse_0 + .__rr_cse_0) + .__rr_cse_0) + ((.__rr_cse_1 * .__rr_cse_1) + .__rr_cse_1))"), "{joined}");
    }

    #[test]
    fn forward_exact_vector_helper_reuse_rewrites_later_lines() {
        let input = vec![
            String::from("  .__rr_cse_3 <- rr_index1_read_vec(a, idx)"),
            String::from("  .tachyon_exprmap0_0 <- (rr_index1_read_vec(a, idx) + 1)"),
        ];
        let out = rewrite_forward_exact_vector_helper_reuse(input);
        let joined = out.join("\n");
        assert!(
            joined.contains(".__rr_cse_3 <- rr_index1_read_vec(a, idx)"),
            "{joined}"
        );
        assert!(
            joined.contains(".tachyon_exprmap0_0 <- (.__rr_cse_3 + 1)"),
            "{joined}"
        );
    }

    #[test]
    fn forward_temp_aliases_rewrite_later_uses_to_original_temp() {
        let input = vec![
            String::from("  .__rr_cse_0 <- rr_index1_read_vec(a, idx)"),
            String::from("  .__rr_cse_3 <- .__rr_cse_0"),
            String::from("  next <- (.__rr_cse_3 + 1)"),
        ];
        let out = rewrite_forward_temp_aliases(input);
        let joined = out.join("\n");
        assert!(joined.contains(".__rr_cse_3 <- .__rr_cse_0"), "{joined}");
        assert!(joined.contains("next <- (.__rr_cse_0 + 1)"), "{joined}");
    }

    #[test]
    fn does_not_hoist_two_use_vector_helper_calls_within_single_assignment_rhs() {
        let input = vec![String::from(
            "  score <- (rr_index1_read_vec(x, rr_index_vec_floor(i:n)) + rr_index1_read_vec(x, rr_index_vec_floor(i:n)))",
        )];
        let out = hoist_repeated_vector_helper_calls_within_lines(input);
        let joined = out.join("\n");
        assert!(
            !joined.contains(".__rr_cse_0 <- rr_index1_read_vec(x, rr_index_vec_floor(i:n))"),
            "{joined}"
        );
        assert_eq!(
            joined
                .matches("rr_index1_read_vec(x, rr_index_vec_floor(i:n))")
                .count(),
            2,
            "{joined}"
        );
    }
}
