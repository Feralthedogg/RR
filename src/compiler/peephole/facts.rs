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

#[derive(Debug, Clone, Default)]
pub(in super::super) struct FunctionLineFacts {
    pub(super) line_idx: usize,
    pub(super) indent: usize,
    pub(super) region_end: usize,
    pub(super) inline_region_end: usize,
    pub(super) next_non_empty_line: Option<usize>,
    pub(super) in_loop_body: bool,
    pub(super) is_assign: bool,
    pub(super) is_control_boundary: bool,
    pub(super) lhs: Option<String>,
    pub(super) rhs: Option<String>,
    pub(super) idents: Vec<String>,
}

#[derive(Debug, Clone)]
pub(in super::super) struct ExactReuseCandidate {
    pub(super) line_idx: usize,
    pub(super) indent: usize,
    pub(super) region_end: usize,
    pub(super) lhs: String,
    pub(super) rhs: String,
    pub(super) idents: Vec<String>,
}

#[derive(Debug, Clone)]
pub(in super::super) struct ArgAliasDef {
    pub(super) line_idx: usize,
    pub(super) alias: String,
    pub(super) target: String,
}

#[derive(Debug, Clone, Default)]
pub(in super::super) struct ExactReuseCandidateSets {
    pub(super) call_assign_candidates: Vec<ExactReuseCandidate>,
    pub(super) exact_expr_candidates: Vec<ExactReuseCandidate>,
    pub(super) pure_rebind_candidates: Vec<ExactReuseCandidate>,
}

#[derive(Debug, Clone)]
pub(in super::super) struct FunctionFacts {
    pub(super) function: IndexedFunction,
    pub(super) line_facts: Vec<FunctionLineFacts>,
    pub(super) defs: FxHashMap<String, Vec<usize>>,
    pub(super) uses: FxHashMap<String, Vec<usize>>,
    pub(super) helper_call_lines: Vec<usize>,
    pub(super) param_set: FxHashSet<String>,
    pub(super) mutated_arg_aliases: FxHashSet<String>,
    pub(super) prologue_arg_alias_defs: Vec<ArgAliasDef>,
    pub(super) non_prologue_assigned_idents: FxHashSet<String>,
    pub(super) stored_bases: FxHashSet<String>,
    pub(super) mentioned_arg_aliases: FxHashSet<String>,
    pub(super) exact_reuse_candidates: ExactReuseCandidateSets,
}

#[derive(Debug, Clone, Default)]
pub(in super::super) struct PeepholeAnalysisCache {
    signature: Option<u64>,
    function_facts: Vec<FunctionFacts>,
}

fn lines_signature(lines: &[String]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for line in lines {
        line.hash(&mut hasher);
    }
    hasher.finish()
}

fn collect_function_facts(lines: &[String]) -> Vec<FunctionFacts> {
    let Some(assign_re) = assign_re() else {
        return Vec::new();
    };
    let plain_ident_re = plain_ident_re();
    build_function_text_index(lines, parse_function_header)
        .into_iter()
        .map(|function| {
            let mut line_facts =
                Vec::with_capacity(function.end.saturating_sub(function.start) + 1);
            let mut defs = FxHashMap::<String, Vec<usize>>::default();
            let mut uses = FxHashMap::<String, Vec<usize>>::default();
            let mut helper_call_lines = Vec::<usize>::new();
            let mut prologue_arg_alias_defs = Vec::<ArgAliasDef>::new();
            let mut non_prologue_assigned_idents = FxHashSet::<String>::default();
            let mut stored_bases = FxHashSet::<String>::default();
            let mut mentioned_arg_aliases = FxHashSet::<String>::default();
            let mut exact_reuse_candidates = ExactReuseCandidateSets::default();
            let mut in_prologue_arg_aliases = true;
            let mut block_stack: Vec<bool> = Vec::new();
            let param_set: FxHashSet<String> = function.params.iter().cloned().collect();
            let mutated_arg_aliases =
                collect_mutated_arg_aliases_in_lines(lines, function.start, function.end);
            for line_idx in function.start..=function.end {
                let line = &lines[line_idx];
                let trimmed = line.trim();
                let indent = line.len() - line.trim_start().len();
                let mut facts = FunctionLineFacts {
                    line_idx,
                    indent,
                    region_end: next_straight_line_region_end(lines, &function, line_idx, indent),
                    inline_region_end: function.end + 1,
                    next_non_empty_line: None,
                    in_loop_body: block_stack.iter().any(|is_loop| *is_loop),
                    is_assign: false,
                    is_control_boundary: is_control_flow_boundary(trimmed),
                    lhs: None,
                    rhs: None,
                    idents: Vec::new(),
                };
                if line_idx > function.start
                    && !line.contains("<- function")
                    && line.contains("Sym_")
                    && line.contains('(')
                {
                    helper_call_lines.push(line_idx);
                }
                if let Some(caps) = assign_re.captures(trimmed) {
                    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let idents = expr_idents(rhs);
                    facts.is_assign = true;
                    facts.lhs = Some(lhs.to_string());
                    facts.rhs = Some(rhs.to_string());
                    facts.idents = idents.clone();
                    defs.entry(lhs.to_string()).or_default().push(line_idx);
                    for ident in &idents {
                        uses.entry(ident.clone()).or_default().push(line_idx);
                        if ident.starts_with(".arg_") {
                            mentioned_arg_aliases.insert(ident.clone());
                        }
                    }
                    let is_prologue_alias_def = line_idx >= function.body_start
                        && in_prologue_arg_aliases
                        && lhs.starts_with(".arg_")
                        && plain_ident_re.is_some_and(|re| re.is_match(rhs));
                    if is_prologue_alias_def {
                        prologue_arg_alias_defs.push(ArgAliasDef {
                            line_idx,
                            alias: lhs.to_string(),
                            target: rhs.to_string(),
                        });
                    } else if line_idx >= function.body_start
                        && !trimmed.is_empty()
                        && trimmed != "{"
                    {
                        in_prologue_arg_aliases = false;
                        non_prologue_assigned_idents.insert(lhs.to_string());
                    }
                    let region_end = facts.region_end;
                    if line_idx >= function.body_start
                        && line_idx < function.end
                        && plain_ident_re.is_some_and(|re| re.is_match(lhs))
                        && !lhs.starts_with(".arg_")
                        && !lhs.starts_with(".__rr_cse_")
                    {
                        let candidate = ExactReuseCandidate {
                            line_idx,
                            indent,
                            region_end,
                            lhs: lhs.to_string(),
                            rhs: rhs.to_string(),
                            idents,
                        };
                        if rhs.contains('(') {
                            exact_reuse_candidates
                                .call_assign_candidates
                                .push(candidate.clone());
                            exact_reuse_candidates
                                .pure_rebind_candidates
                                .push(candidate);
                        } else if is_literal_field_read_expr(rhs) {
                            exact_reuse_candidates
                                .pure_rebind_candidates
                                .push(candidate);
                        }
                    }
                    if line_idx >= function.body_start
                        && line_idx < function.end
                        && plain_ident_re.is_some_and(|re| re.is_match(lhs))
                        && expr_is_exact_reusable_scalar(rhs)
                    {
                        exact_reuse_candidates
                            .exact_expr_candidates
                            .push(ExactReuseCandidate {
                                line_idx,
                                indent,
                                region_end,
                                lhs: lhs.to_string(),
                                rhs: rhs.to_string(),
                                idents: expr_idents(rhs),
                            });
                    }
                } else {
                    facts.idents = expr_idents(trimmed);
                    for ident in &facts.idents {
                        uses.entry(ident.clone()).or_default().push(line_idx);
                        if ident.starts_with(".arg_") {
                            mentioned_arg_aliases.insert(ident.clone());
                        }
                    }
                    if line_idx >= function.body_start && !trimmed.is_empty() && trimmed != "{" {
                        in_prologue_arg_aliases = false;
                    }
                }
                if let Some(caps) = indexed_store_base_re().and_then(|re| re.captures(trimmed)) {
                    let base = caps.name("base").map(|m| m.as_str()).unwrap_or("").trim();
                    stored_bases.insert(base.to_string());
                }
                if trimmed == "} else {" {
                    let _ = block_stack.pop();
                    block_stack.push(false);
                } else {
                    let (opens, closes) = count_unquoted_braces(trimmed);
                    for _ in 0..closes {
                        let _ = block_stack.pop();
                    }
                    let loop_open = is_loop_open_boundary(trimmed);
                    for open_idx in 0..opens {
                        block_stack.push(loop_open && open_idx == 0);
                    }
                }
                line_facts.push(facts);
            }
            populate_inline_region_ends(lines, &function, &mut line_facts);
            FunctionFacts {
                function,
                line_facts,
                defs,
                uses,
                helper_call_lines,
                param_set,
                mutated_arg_aliases,
                prologue_arg_alias_defs,
                non_prologue_assigned_idents,
                stored_bases,
                mentioned_arg_aliases,
                exact_reuse_candidates,
            }
        })
        .collect()
}

fn populate_inline_region_ends(
    lines: &[String],
    function: &IndexedFunction,
    line_facts: &mut [FunctionLineFacts],
) {
    let mut next_boundary = function.end + 1;
    let mut next_non_empty_line = None;
    for fact in line_facts.iter_mut().rev() {
        fact.inline_region_end = next_boundary;
        fact.next_non_empty_line = next_non_empty_line;
        let trimmed = lines[fact.line_idx].trim();
        if !trimmed.is_empty() {
            next_non_empty_line = Some(fact.line_idx);
        }
        if !trimmed.is_empty()
            && (lines[fact.line_idx].contains("<- function")
                || fact.is_control_boundary
                || trimmed == "}")
        {
            next_boundary = fact.line_idx;
        }
    }
}

fn next_straight_line_region_end(
    lines: &[String],
    function: &IndexedFunction,
    start_idx: usize,
    indent: usize,
) -> usize {
    for line_idx in start_idx + 1..=function.end {
        let line = &lines[line_idx];
        let trimmed = line.trim();
        let next_indent = line.len() - line.trim_start().len();
        if line.contains("<- function")
            || trimmed == "repeat {"
            || trimmed.starts_with("while")
            || trimmed.starts_with("for")
            || (!trimmed.is_empty() && next_indent < indent)
        {
            return line_idx;
        }
    }
    function.end + 1
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

fn literal_field_read_expr_re() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        compile_regex(format!(
            r#"^(?P<base>{})\[\["(?P<field>[A-Za-z_][A-Za-z0-9_]*)"\]\]$"#,
            IDENT_PATTERN
        ))
    })
    .as_ref()
}

fn is_literal_field_read_expr(rhs: &str) -> bool {
    literal_field_read_expr_re().is_some_and(|re| re.is_match(rhs.trim()))
}

pub(in super::super) fn cached_function_facts<'a>(
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

pub(in super::super) fn helper_call_candidate_lines(
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

pub(in super::super) fn next_def_after(
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

pub(in super::super) fn first_use_after(
    facts: &FunctionFacts,
    symbol: &str,
    after_line: usize,
    region_end: usize,
) -> Option<usize> {
    uses_in_region(facts, symbol, after_line, region_end)
        .first()
        .copied()
}

pub(in super::super) fn uses_in_region<'a>(
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

fn function_facts_for_line<'a>(
    cache: &'a mut PeepholeAnalysisCache,
    lines: &[String],
    line_idx: usize,
) -> Option<&'a FunctionFacts> {
    cached_function_facts(cache, lines)
        .iter()
        .find(|facts| line_idx >= facts.function.start && line_idx <= facts.function.end)
}

pub(in super::super) fn next_def_after_in_facts(
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

pub(in super::super) fn next_def_after_cached(
    cache: &mut PeepholeAnalysisCache,
    lines: &[String],
    line_idx: usize,
    symbol: &str,
    region_end: usize,
) -> Option<usize> {
    let facts = function_facts_for_line(cache, lines, line_idx)?;
    next_def_after(facts, symbol, line_idx, region_end)
}

pub(in super::super) fn first_use_after_cached(
    cache: &mut PeepholeAnalysisCache,
    lines: &[String],
    line_idx: usize,
    symbol: &str,
    region_end: usize,
) -> Option<usize> {
    let facts = function_facts_for_line(cache, lines, line_idx)?;
    first_use_after(facts, symbol, line_idx, region_end)
}

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

fn collect_mutated_arg_aliases_iter<'a>(
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

pub(super) fn collect_mutated_arg_aliases(code: &str) -> FxHashSet<String> {
    collect_mutated_arg_aliases_iter(code.lines())
}

pub(super) fn collect_mutated_arg_aliases_in_lines(
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
