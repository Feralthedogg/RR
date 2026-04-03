use super::{
    FxHashMap, FxHashSet, IDENT_PATTERN, assign_re, compile_regex, expr_idents, floor_re,
    indexed_store_base_re, length_call_re, plain_ident_re, range_re, rep_int_re, scalar_lit_re,
    seq_len_re, split_top_level_args,
};
use regex::{Captures, Regex};
use std::sync::OnceLock;

pub(super) fn read_vec_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>{}|rr_index_vec_floor\([^\)]*\)|[^,\)]*:[^\)]*)\)",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(super) fn normalize_expr(expr: &str, scalar_consts: &FxHashMap<String, String>) -> String {
    let trimmed = expr.trim();
    scalar_consts
        .get(trimmed)
        .cloned()
        .unwrap_or_else(|| trimmed.to_string())
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

pub(super) fn expr_proven_no_na(
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

pub(super) fn expr_is_logical_comparison(
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

pub(super) fn rewrite_strict_ifelse_expr(
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

pub(super) fn helper_heavy_runtime_auto_args(args: &str) -> bool {
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

pub(super) fn helper_heavy_runtime_auto_args_with_temps(
    args: &str,
    helper_heavy_vars: &FxHashSet<String>,
) -> bool {
    helper_heavy_runtime_auto_args(args)
        || expr_idents(args)
            .into_iter()
            .any(|ident| helper_heavy_vars.contains(&ident))
}

pub(super) fn is_one(expr: &str, scalar_consts: &FxHashMap<String, String>) -> bool {
    matches!(
        normalize_expr(expr, scalar_consts).as_str(),
        "1" | "1L" | "1.0"
    )
}

pub(super) fn infer_len_from_expr(
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

pub(super) fn rewrite_known_length_calls(
    expr: &str,
    vector_lens: &FxHashMap<String, String>,
) -> String {
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

pub(super) fn identity_index_end_expr(
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

pub(super) fn clear_linear_facts(
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

pub(super) fn clear_loop_boundary_facts(
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

pub(super) fn is_control_flow_boundary(line: &str) -> bool {
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

pub(super) fn is_dead_parenthesized_eval_line(line: &str) -> bool {
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

pub(super) fn is_dead_plain_ident_eval_line(line: &str) -> bool {
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

pub(super) fn collect_mutated_arg_aliases(code: &str) -> FxHashSet<String> {
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
