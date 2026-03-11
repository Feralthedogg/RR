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
    RE.get_or_init(|| {
        compile_regex(format!(
            r"^rr_index_vec_floor\((?P<src>{})\)$",
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

fn read_vec_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>{})\)",
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

pub(crate) fn optimize_emitted_r(code: &str, direct_builtin_call_map: bool) -> String {
    let mut scalar_consts: FxHashMap<String, String> = FxHashMap::default();
    let mut vector_lens: FxHashMap<String, String> = FxHashMap::default();
    let mut identity_indices: FxHashMap<String, String> = FxHashMap::default();
    let mut aliases: FxHashMap<String, String> = FxHashMap::default();
    let mut no_na_vars: FxHashSet<String> = FxHashSet::default();
    let mut helper_heavy_vars: FxHashSet<String> = FxHashSet::default();
    let mut out_lines = Vec::new();

    for line in code.lines() {
        if line.contains("<- function") {
            scalar_consts.clear();
            vector_lens.clear();
            identity_indices.clear();
            aliases.clear();
            no_na_vars.clear();
            out_lines.push(line.to_string());
            continue;
        }

        let Some(caps) = assign_re().and_then(|re| re.captures(line)) else {
            out_lines.push(line.to_string());
            continue;
        };

        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();

        let rewritten_rhs = if let Some(re) = read_vec_re() {
            re.replace_all(rhs, |caps: &Captures<'_>| {
                let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                let idx = caps.name("idx").map(|m| m.as_str()).unwrap_or("");
                match (identity_indices.get(idx), vector_lens.get(base)) {
                    (Some(end), Some(base_len)) if end == base_len => base.to_string(),
                    _ => caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
                }
            })
            .to_string()
        } else {
            rhs.to_string()
        };
        let rewritten_rhs = rewrite_known_aliases(&rewritten_rhs, &aliases);
        let rewritten_rhs = rewrite_direct_vec_helper_expr(&rewritten_rhs, direct_builtin_call_map);

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

        let rewritten_rhs = rewrite_strict_ifelse_expr(&rewritten_rhs, &no_na_vars, &scalar_consts);

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

        let rewritten_rhs = rewrite_known_aliases(&rewritten_rhs, &aliases);
        let rewritten_rhs = rewrite_direct_vec_helper_expr(&rewritten_rhs, direct_builtin_call_map);
        let rewritten_rhs = rewrite_strict_ifelse_expr(&rewritten_rhs, &no_na_vars, &scalar_consts);

        if scalar_lit_re().is_some_and(|re| re.is_match(rewritten_rhs.trim())) {
            scalar_consts.insert(lhs.to_string(), rewritten_rhs.trim().to_string());
        } else {
            scalar_consts.remove(lhs);
        }

        invalidate_aliases_for_write(lhs, &mut aliases);
        if is_peephole_temp(lhs)
            && plain_ident_re().is_some_and(|re| re.is_match(rewritten_rhs.trim()))
            && rewritten_rhs.trim() != lhs
        {
            aliases.insert(
                lhs.to_string(),
                resolve_alias(rewritten_rhs.trim(), &aliases),
            );
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

        out_lines.push(format!("{indent}{lhs} <- {rewritten_rhs}"));
    }

    let out_lines = strip_dead_temps(out_lines);
    let mut out = out_lines.join("\n");
    if code.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn strip_dead_temps(lines: Vec<String>) -> Vec<String> {
    let mut live: FxHashSet<String> = FxHashSet::default();
    let mut out = lines;
    for line in out.iter_mut().rev() {
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
        let is_temp = lhs.starts_with(".__rr_cse_")
            || lhs.starts_with(".tachyon_callmap_arg")
            || lhs.starts_with("i_")
            || lhs.starts_with(".__rr_tmp_");
        if is_temp && !live.contains(lhs) {
            *line = String::new();
            continue;
        }
        live.remove(lhs);
        for ident in expr_idents(rhs) {
            if ident != lhs {
                live.insert(ident);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::optimize_emitted_r;

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
        assert!(out.contains(".arg <- abs(x)"));
        assert!(out.contains("score <- pmax(.arg, 0.05)"));
        assert!(!out.contains("rr_index1_read_vec(score, .tmp2)"));
    }

    #[test]
    fn rewrites_signal_pipeline_like_full_range_slice() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 250000\n\
  idx <- seq_len(n)\n\
  x <- rr_parallel_vec_sub_f64(rr_parallel_vec_div_f64((rr_parallel_vec_mul_f64(idx, 13) %% 1000), 1000), 0.5)\n\
  y <- rr_parallel_vec_sub_f64(rr_parallel_vec_div_f64((rr_parallel_vec_add_f64(rr_parallel_vec_mul_f64(idx, 17), 7) %% 1000), 1000), 0.5)\n\
  score <- rep.int(0, n)\n\
  i <- 1L\n\
  .__rr_cse_155 <- i:250000\n\
  .__rr_cse_156 <- rr_index_vec_floor(.__rr_cse_155)\n\
  .tachyon_callmap_arg0_0 <- abs((((rr_index1_read_vec(x, .__rr_cse_156) * 0.65) + (rr_index1_read_vec(y, .__rr_cse_156) * 0.35)) - 0.08))\n\
  score <- rr_call_map_slice_auto(score, i, 250000, \"pmax\", 44L, c(1L), .tachyon_callmap_arg0_0, 0.05)\n\
}\n";
        let out = optimize_emitted_r(input, true);
        println!("{}", out);
        assert!(out.contains(".tachyon_callmap_arg0_0 <- abs((((x * 0.65) + (y * 0.35)) - 0.08))"));
        assert!(out.contains("score <- pmax(.tachyon_callmap_arg0_0, 0.05)"));
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
        assert!(out.contains("x <- ((((idx * 13) %% 1000) / 1000) - 0.5)"));
    }

    #[test]
    fn rewrites_full_signal_pipeline_block() {
        let input = "\
Sym_1 <- function() \n\
{\n\
  n <- 250000\n\
  idx <- seq_len(n)\n\
  x <- rr_parallel_vec_sub_f64(rr_parallel_vec_div_f64((rr_parallel_vec_mul_f64(idx, 13) %% 1000), 1000), 0.5)\n\
  y <- rr_parallel_vec_sub_f64(rr_parallel_vec_div_f64((rr_parallel_vec_add_f64(rr_parallel_vec_mul_f64(idx, 17), 7) %% 1000), 1000), 0.5)\n\
  score <- rep.int(0, n)\n\
  clean <- rep.int(0, n)\n\
  i <- 1L\n\
  .__rr_cse_155 <- i:250000\n\
  .__rr_cse_156 <- rr_index_vec_floor(.__rr_cse_155)\n\
  .tachyon_callmap_arg0_0 <- abs((((rr_index1_read_vec(x, .__rr_cse_156) * 0.65) + (rr_index1_read_vec(y, .__rr_cse_156) * 0.35)) - 0.08))\n\
  score <- rr_call_map_slice_auto(score, i, 250000, \"pmax\", 44L, c(1L), .tachyon_callmap_arg0_0, 0.05)\n\
  i_9 <- 1L\n\
  .__rr_cse_170 <- i_9:250000\n\
  .__rr_cse_171 <- rr_index_vec_floor(.__rr_cse_170)\n\
  .__rr_cse_172 <- rr_index1_read_vec(score, .__rr_cse_171)\n\
  clean <- rr_assign_slice(clean, i_9, 250000, rr_ifelse_strict((.__rr_cse_172 > 0.4), sqrt((.__rr_cse_172 + 0.1)), ((.__rr_cse_172 * 0.55) + 0.03)))\n\
  i_10 <- 1L\n\
  .__rr_cse_180 <- i_10:250000\n\
  .__rr_cse_181 <- rr_index_vec_floor(.__rr_cse_180)\n\
  x <- rr_assign_slice(x, i_10, 250000, (rr_index1_read_vec(clean, .__rr_cse_181) + (rr_index1_read_vec(y, .__rr_cse_181) * 0.15)))\n\
  i_11 <- 1L\n\
  .__rr_cse_187 <- i_11:250000\n\
  .__rr_cse_188 <- rr_index_vec_floor(.__rr_cse_187)\n\
  y <- rr_assign_slice(y, i_11, 250000, ((rr_index1_read_vec(score, .__rr_cse_188) * 0.8) + (rr_index1_read_vec(clean, .__rr_cse_188) * 0.2)))\n\
}\n";
        let out = optimize_emitted_r(input, true);
        assert!(out.contains(".tachyon_callmap_arg0_0 <- abs((((x * 0.65) + (y * 0.35)) - 0.08))"));
        assert!(out.contains("score <- pmax(.tachyon_callmap_arg0_0, 0.05)"));
        assert!(out.contains(
            "clean <- ifelse((score > 0.4), sqrt((score + 0.1)), ((score * 0.55) + 0.03))"
        ));
        assert!(out.contains("x <- (clean + (y * 0.15))"));
        assert!(out.contains("y <- ((score * 0.8) + (clean * 0.2))"));
        assert!(!out.contains(".__rr_cse_172 <-"));
    }
}
