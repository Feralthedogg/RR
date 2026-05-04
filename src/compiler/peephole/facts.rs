use super::{
    FxHashMap, FxHashSet, IDENT_PATTERN, IndexedFunction, assign_re, build_function_text_index,
    compile_regex, count_unquoted_braces, expr_idents, expr_is_exact_reusable_scalar, floor_re,
    indexed_store_base_re, is_loop_open_boundary, length_call_re, plain_ident_re, range_re,
    rep_int_re, scalar_lit_re, seq_len_re, split_top_level_args,
};
use regex::{Captures, Regex};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::OnceLock;

#[path = "facts/collection.rs"]
pub(crate) mod collection;

use self::collection::collect_function_facts;

#[derive(Debug, Clone, Default)]
pub(crate) struct FunctionLineFacts {
    pub(crate) line_idx: usize,
    pub(crate) indent: usize,
    pub(crate) region_end: usize,
    pub(crate) inline_region_end: usize,
    pub(crate) next_non_empty_line: Option<usize>,
    pub(crate) in_loop_body: bool,
    pub(crate) is_assign: bool,
    pub(crate) is_control_boundary: bool,
    pub(crate) lhs: Option<String>,
    pub(crate) rhs: Option<String>,
    pub(crate) idents: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ExactReuseCandidate {
    pub(crate) line_idx: usize,
    pub(crate) indent: usize,
    pub(crate) region_end: usize,
    pub(crate) lhs: String,
    pub(crate) rhs: String,
    pub(crate) idents: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ArgAliasDef {
    pub(crate) line_idx: usize,
    pub(crate) alias: String,
    pub(crate) target: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ExactReuseCandidateSets {
    pub(crate) call_assign_candidates: Vec<ExactReuseCandidate>,
    pub(crate) exact_expr_candidates: Vec<ExactReuseCandidate>,
    pub(crate) pure_rebind_candidates: Vec<ExactReuseCandidate>,
}

#[derive(Debug, Clone)]
pub(crate) struct FunctionFacts {
    pub(crate) function: IndexedFunction,
    pub(crate) line_facts: Vec<FunctionLineFacts>,
    pub(crate) defs: FxHashMap<String, Vec<usize>>,
    pub(crate) uses: FxHashMap<String, Vec<usize>>,
    pub(crate) helper_call_lines: Vec<usize>,
    pub(crate) param_set: FxHashSet<String>,
    pub(crate) mutated_arg_aliases: FxHashSet<String>,
    pub(crate) prologue_arg_alias_defs: Vec<ArgAliasDef>,
    pub(crate) non_prologue_assigned_idents: FxHashSet<String>,
    pub(crate) stored_bases: FxHashSet<String>,
    pub(crate) mentioned_arg_aliases: FxHashSet<String>,
    pub(crate) exact_reuse_candidates: ExactReuseCandidateSets,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct PeepholeAnalysisCache {
    pub(crate) signature: Option<u64>,
    pub(crate) function_facts: Vec<FunctionFacts>,
}

pub(crate) fn lines_signature(lines: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for line in lines {
        line.hash(&mut hasher);
    }
    hasher.finish()
}

pub(crate) fn parse_function_header(line: &str) -> Option<(String, Vec<String>)> {
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

pub(crate) fn literal_field_read_expr_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r#"^(?P<base>{})\[\["(?P<field>[A-Za-z_][A-Za-z0-9_]*)"\]\]$"#,
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(crate) fn is_literal_field_read_expr(rhs: &str) -> bool {
    literal_field_read_expr_re().is_some_and(|re| re.is_match(rhs.trim()))
}

pub(crate) fn cached_function_facts<'a>(
    cache: &'a mut PeepholeAnalysisCache,
    lines: &[String],
) -> &'a [FunctionFacts] {
    let signature = lines_signature(lines);
    if cache.signature != Some(signature) {
        cache.function_facts = collect_function_facts(lines);
        cache.signature = Some(signature);
    }
    &cache.function_facts
}

pub(crate) fn helper_call_candidate_lines(
    cache: &mut PeepholeAnalysisCache,
    lines: &[String],
) -> Vec<usize> {
    let facts = cached_function_facts(cache, lines);
    let mut covered = vec![false; lines.len()];
    let mut out = Vec::new();
    for function_facts in facts {
        for idx in function_facts.function.start..=function_facts.function.end {
            if idx < covered.len() {
                covered[idx] = true;
            }
        }
        out.extend(function_facts.helper_call_lines.iter().copied());
    }
    for (idx, line) in lines.iter().enumerate() {
        if covered.get(idx).copied().unwrap_or(false) {
            continue;
        }
        if !line.contains("<- function") && line.contains("Sym_") && line.contains('(') {
            out.push(idx);
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

pub(crate) fn next_def_after(
    facts: &FunctionFacts,
    symbol: &str,
    after_line: usize,
    region_end: usize,
) -> Option<usize> {
    facts.defs.get(symbol).and_then(|defs| {
        let start = defs.partition_point(|line_idx| *line_idx <= after_line);
        defs.get(start)
            .copied()
            .filter(|line_idx| *line_idx < region_end)
    })
}

pub(crate) fn first_use_after(
    facts: &FunctionFacts,
    symbol: &str,
    after_line: usize,
    region_end: usize,
) -> Option<usize> {
    uses_in_region(facts, symbol, after_line, region_end)
        .first()
        .copied()
}

pub(crate) fn uses_in_region<'a>(
    facts: &'a FunctionFacts,
    symbol: &str,
    after_line: usize,
    region_end: usize,
) -> &'a [usize] {
    let Some(uses) = facts.uses.get(symbol) else {
        return &[];
    };
    let start = uses.partition_point(|line_idx| *line_idx <= after_line);
    let end = start + uses[start..].partition_point(|line_idx| *line_idx < region_end);
    &uses[start..end]
}

pub(crate) fn function_facts_for_line<'a>(
    cache: &'a mut PeepholeAnalysisCache,
    lines: &[String],
    line_idx: usize,
) -> Option<&'a FunctionFacts> {
    cached_function_facts(cache, lines)
        .iter()
        .find(|facts| line_idx >= facts.function.start && line_idx <= facts.function.end)
}

pub(crate) fn next_def_after_in_facts(
    facts: &[FunctionFacts],
    line_idx: usize,
    symbol: &str,
    region_end: usize,
) -> Option<usize> {
    let function_facts = facts
        .iter()
        .find(|facts| line_idx >= facts.function.start && line_idx <= facts.function.end)?;
    next_def_after(function_facts, symbol, line_idx, region_end)
}

pub(crate) fn next_def_after_cached(
    cache: &mut PeepholeAnalysisCache,
    lines: &[String],
    line_idx: usize,
    symbol: &str,
    region_end: usize,
) -> Option<usize> {
    let facts = function_facts_for_line(cache, lines, line_idx)?;
    next_def_after(facts, symbol, line_idx, region_end)
}

pub(crate) fn first_use_after_cached(
    cache: &mut PeepholeAnalysisCache,
    lines: &[String],
    line_idx: usize,
    symbol: &str,
    region_end: usize,
) -> Option<usize> {
    let facts = function_facts_for_line(cache, lines, line_idx)?;
    first_use_after(facts, symbol, line_idx, region_end)
}

pub(crate) fn read_vec_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*(?P<idx>{}|rr_index_vec_floor\([^\)]*\)|[^,\)]*:[^\)]*)\)",
            IDENT_PATTERN, IDENT_PATTERN
        ))
    })
    .as_ref()
}

pub(crate) fn normalize_expr(expr: &str, scalar_consts: &FxHashMap<String, String>) -> String {
    let trimmed = expr.trim();
    scalar_consts
        .get(trimmed)
        .cloned()
        .unwrap_or_else(|| trimmed.to_string())
}

pub(crate) fn is_expr_builtin_name(name: &str) -> bool {
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

pub(crate) fn expr_proven_no_na(
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

pub(crate) fn expr_is_logical_comparison(
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

pub(crate) fn rewrite_strict_ifelse_expr(
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

pub(crate) fn helper_heavy_runtime_auto_args(args: &str) -> bool {
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

pub(crate) fn helper_heavy_runtime_auto_args_with_temps(
    args: &str,
    helper_heavy_vars: &FxHashSet<String>,
) -> bool {
    helper_heavy_runtime_auto_args(args)
        || expr_idents(args)
            .into_iter()
            .any(|ident| helper_heavy_vars.contains(&ident))
}

pub(crate) fn is_one(expr: &str, scalar_consts: &FxHashMap<String, String>) -> bool {
    matches!(
        normalize_expr(expr, scalar_consts).as_str(),
        "1" | "1L" | "1.0"
    )
}

pub(crate) fn infer_len_from_expr(
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

pub(crate) fn rewrite_known_length_calls(
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

pub(crate) fn identity_index_end_expr(
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

pub(crate) fn clear_linear_facts(
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

pub(crate) fn clear_loop_boundary_facts(
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

pub(crate) fn is_control_flow_boundary(line: &str) -> bool {
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

pub(crate) fn is_dead_parenthesized_eval_line(line: &str) -> bool {
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

pub(crate) fn is_dead_plain_ident_eval_line(line: &str) -> bool {
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

pub(crate) fn collect_mutated_arg_aliases_iter<'a>(
    lines: impl IntoIterator<Item = &'a str>,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let mut seen_initial_aliases = FxHashSet::default();

    for line in lines {
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

pub(crate) fn collect_mutated_arg_aliases(code: &str) -> FxHashSet<String> {
    collect_mutated_arg_aliases_iter(code.lines())
}

pub(crate) fn collect_mutated_arg_aliases_in_lines(
    lines: &[String],
    start: usize,
    end_inclusive: usize,
) -> FxHashSet<String> {
    collect_mutated_arg_aliases_iter(
        lines
            .iter()
            .take(end_inclusive.saturating_add(1))
            .skip(start)
            .map(|line| line.as_str()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_function_facts_collect_exact_reuse_candidates() {
        let lines = vec![
            "foo <- function(x) {".to_string(),
            "  out <- pure_call(x)".to_string(),
            "  expr <- (x + 1)".to_string(),
            "  return(expr)".to_string(),
            "}".to_string(),
        ];

        let mut cache = PeepholeAnalysisCache::default();
        let facts = cached_function_facts(&mut cache, &lines);
        assert_eq!(facts.len(), 1);
        assert_eq!(
            facts[0].exact_reuse_candidates.call_assign_candidates.len(),
            2
        );
        assert_eq!(
            facts[0].exact_reuse_candidates.pure_rebind_candidates.len(),
            2
        );
        assert_eq!(
            facts[0].exact_reuse_candidates.exact_expr_candidates.len(),
            2
        );
        assert_eq!(
            facts[0].exact_reuse_candidates.call_assign_candidates[0].line_idx,
            1
        );
        let exact_expr_lines: FxHashSet<usize> = facts[0]
            .exact_reuse_candidates
            .exact_expr_candidates
            .iter()
            .map(|candidate| candidate.line_idx)
            .collect();
        assert_eq!(exact_expr_lines, FxHashSet::from_iter([1usize, 2usize]));
    }

    #[test]
    fn cached_function_facts_invalidates_on_input_change() {
        let lines_before = vec![
            "foo <- function(x) {".to_string(),
            "  out <- pure_call(x)".to_string(),
            "}".to_string(),
        ];
        let lines_after = vec![
            "foo <- function(x) {".to_string(),
            "  out <- x".to_string(),
            "}".to_string(),
        ];

        let mut cache = PeepholeAnalysisCache::default();
        let before_count = cached_function_facts(&mut cache, &lines_before)[0]
            .exact_reuse_candidates
            .call_assign_candidates
            .len();
        let after_count = cached_function_facts(&mut cache, &lines_after)[0]
            .exact_reuse_candidates
            .call_assign_candidates
            .len();

        assert_eq!(before_count, 1);
        assert_eq!(after_count, 0);
    }
}
