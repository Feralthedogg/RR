use super::super::{
    PeepholeAnalysisCache, assign_re, cached_function_facts, collect_prologue_arg_aliases,
    expr_has_only_pure_calls, expr_idents, ident_re, is_peephole_temp, next_def_after_in_facts,
    plain_ident_re, rewrite_forward_exact_expr_reuse_ir, rewrite_forward_exact_pure_call_reuse_ir,
    scalar_lit_re, strip_terminal_repeat_nexts_ir, uses_in_region,
};
use crate::compiler::peephole::alias::normalize_expr_with_aliases;
use regex::Captures;
use rustc_hash::{FxHashMap, FxHashSet};

fn line_might_mention_any(text: &str, names: &FxHashSet<String>) -> bool {
    names.iter().any(|name| text.contains(name))
}

fn build_line_to_function(
    facts: &[super::super::FunctionFacts],
    total_lines: usize,
) -> Vec<Option<usize>> {
    let mut line_to_fn = vec![None; total_lines];
    for (fn_idx, function_facts) in facts.iter().enumerate() {
        for line_idx in function_facts.function.start..=function_facts.function.end {
            if line_idx < total_lines {
                line_to_fn[line_idx] = Some(fn_idx);
            }
        }
    }
    line_to_fn
}

fn extend_lines_in_region(
    out: &mut Vec<usize>,
    lines: Option<&Vec<usize>>,
    after_line: usize,
    region_end: usize,
) {
    let Some(lines) = lines else {
        return;
    };
    let start = lines.partition_point(|line_idx| *line_idx <= after_line);
    let end = start + lines[start..].partition_point(|line_idx| *line_idx < region_end);
    out.extend_from_slice(&lines[start..end]);
}

fn reuse_candidate_lines(
    facts: &super::super::FunctionFacts,
    lhs: &str,
    deps: &FxHashSet<String>,
    after_line: usize,
    region_end: usize,
) -> Vec<usize> {
    let mut lines = Vec::new();
    extend_lines_in_region(&mut lines, facts.defs.get(lhs), after_line, region_end);
    for dep in deps {
        extend_lines_in_region(&mut lines, facts.defs.get(dep), after_line, region_end);
        extend_lines_in_region(&mut lines, facts.uses.get(dep), after_line, region_end);
    }
    lines.sort_unstable();
    lines.dedup();
    lines
}

fn compare_exact_forward_ir(
    pass: &str,
    input: &[String],
    legacy: &[String],
    pure_user_calls: &FxHashSet<String>,
) {
    let Some(mode) = std::env::var_os("RR_COMPARE_EXACT_FORWARD_IR") else {
        return;
    };
    let input_vec = input.to_vec();
    let ir = match pass {
        "pure_call" => rewrite_forward_exact_pure_call_reuse_ir(input_vec, pure_user_calls),
        "exact_expr" => rewrite_forward_exact_expr_reuse_ir(input_vec),
        _ => return,
    };
    let compare_only = mode == "1" || mode == "compare" || mode == "verbose";
    if compare_only && ir != legacy {
        let mismatch_idx = legacy
            .iter()
            .zip(ir.iter())
            .position(|(lhs, rhs)| lhs != rhs)
            .unwrap_or_else(|| legacy.len().min(ir.len()));
        let legacy_line = legacy
            .get(mismatch_idx)
            .map(|line| line.trim())
            .unwrap_or("<eof>");
        let ir_line = ir
            .get(mismatch_idx)
            .map(|line| line.trim())
            .unwrap_or("<eof>");
        eprintln!(
            "RR_COMPARE_EXACT_FORWARD_IR diff pass={pass} legacy_lines={} ir_lines={} first_mismatch={} legacy=`{}` ir=`{}`",
            legacy.len(),
            ir.len(),
            mismatch_idx + 1,
            legacy_line,
            ir_line
        );
        if mode == "verbose" {
            let start = mismatch_idx.saturating_sub(2);
            let end = (mismatch_idx + 3)
                .max(start)
                .min(legacy.len().max(ir.len()));
            for idx in start..end {
                let legacy_line = legacy.get(idx).map(|line| line.trim()).unwrap_or("<eof>");
                let ir_line = ir.get(idx).map(|line| line.trim()).unwrap_or("<eof>");
                eprintln!(
                    "RR_COMPARE_EXACT_FORWARD_IR ctx line={} legacy=`{}` ir=`{}`",
                    idx + 1,
                    legacy_line,
                    ir_line
                );
            }
            let _ = input;
        }
    }
}

fn use_ir_forward_pure_call() -> bool {
    !matches!(
        std::env::var("RR_USE_IR_FORWARD_PURE_CALL").ok().as_deref(),
        Some("0") | Some("false") | Some("no")
    )
}

fn use_ir_forward_exact_expr() -> bool {
    !matches!(
        std::env::var("RR_USE_IR_FORWARD_EXACT_EXPR")
            .ok()
            .as_deref(),
        Some("0") | Some("false") | Some("no")
    )
}

pub(in super::super) fn rewrite_forward_simple_alias_guards(lines: Vec<String>) -> Vec<String> {
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

pub(in super::super) fn rewrite_loop_index_alias_ii(lines: Vec<String>) -> Vec<String> {
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

pub(in super::super) fn expr_is_exact_reusable_scalar(rhs: &str) -> bool {
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

pub(in super::super) fn rewrite_forward_exact_pure_call_reuse(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    if use_ir_forward_pure_call() {
        return rewrite_forward_exact_pure_call_reuse_ir(lines, pure_user_calls);
    }
    let mut cache = PeepholeAnalysisCache::default();
    let input = lines.clone();
    let out = rewrite_forward_exact_pure_call_reuse_with_cache(lines, pure_user_calls, &mut cache);
    compare_exact_forward_ir("pure_call", &input, &out, pure_user_calls);
    out
}

pub(in super::super) fn rewrite_forward_exact_pure_call_reuse_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    if use_ir_forward_pure_call() {
        return rewrite_forward_exact_pure_call_reuse_ir(lines, pure_user_calls);
    }
    if !lines
        .iter()
        .any(|line| line.contains("<-") && line.contains('('))
    {
        return lines;
    }
    let Some(assign_re) = assign_re() else {
        return lines;
    };
    let Some(plain_ident_re) = plain_ident_re() else {
        return lines;
    };
    let mut out = lines;
    let function_facts = cached_function_facts(cache, &out);
    let line_to_fn = build_line_to_function(&function_facts, out.len());
    let candidates = function_facts
        .iter()
        .flat_map(|function| {
            function
                .exact_reuse_candidates
                .call_assign_candidates
                .iter()
                .cloned()
        })
        .collect::<Vec<_>>();
    for candidate in candidates {
        let idx = candidate.line_idx;
        let region_end = candidate.region_end.min(out.len());
        let lhs = candidate.lhs.as_str();
        let rhs = candidate.rhs.as_str();
        if !plain_ident_re.is_match(lhs) || !expr_has_only_pure_calls(rhs, pure_user_calls) {
            continue;
        }
        let deps: FxHashSet<String> = candidate.idents.into_iter().collect();
        if deps.contains(lhs) {
            continue;
        }
        let next_lhs_def =
            next_def_after_in_facts(function_facts, idx, lhs, region_end).unwrap_or(region_end);
        let lhs_reassigned_later = next_lhs_def < region_end;
        let scan_end = next_lhs_def.min(region_end);
        if scan_end <= idx + 1 {
            continue;
        }
        let current_facts = line_to_fn[idx].and_then(|fn_idx| function_facts.get(fn_idx));
        let deps_used_in_region = current_facts
            .map(|facts| {
                deps.iter()
                    .any(|ident| !uses_in_region(facts, ident, idx, scan_end).is_empty())
            })
            .unwrap_or(false);
        if !deps_used_in_region && !out[idx + 1..scan_end].iter().any(|line| line.contains(rhs)) {
            continue;
        }
        let candidate_lines = current_facts
            .filter(|_| !deps.is_empty())
            .map(|facts| reuse_candidate_lines(facts, lhs, &deps, idx, scan_end))
            .filter(|lines| !lines.is_empty());
        let mut process_line = |line_no: usize| {
            let mut replacement = None;
            let mut should_break = false;
            let mut should_continue = false;
            {
                let line_trimmed = out[line_no].trim();
                let is_return = line_trimmed == "return(NULL)"
                    || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'));
                if !line_trimmed.contains(" <- ") && !line_trimmed.contains(rhs) && !is_return {
                    return (false, false);
                }
                if let Some(next_caps) = assign_re.captures(line_trimmed) {
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
                        should_break = true;
                    } else {
                        if !line_trimmed.contains(rhs)
                            && next_lhs != lhs
                            && !deps.contains(next_lhs)
                        {
                            return (false, false);
                        }
                        if next_rhs.contains(rhs) {
                            if lhs_reassigned_later {
                                should_continue = true;
                            } else {
                                replacement = Some(out[line_no].replacen(rhs, lhs, usize::MAX));
                            }
                        }
                        if deps.contains(next_lhs) {
                            should_break = true;
                        }
                    }
                } else {
                    if line_trimmed.contains(rhs) {
                        replacement = Some(out[line_no].replacen(rhs, lhs, usize::MAX));
                    }
                    if is_return {
                        should_break = true;
                    }
                }
            }
            if let Some(new_line) = replacement {
                out[line_no] = new_line;
            }
            (should_break, should_continue)
        };

        if let Some(candidate_lines) = candidate_lines.as_deref() {
            for &line_no in candidate_lines {
                let (should_break, should_continue) = process_line(line_no);
                if should_break {
                    break;
                }
                if should_continue {
                    continue;
                }
            }
        } else {
            for line_no in idx + 1..scan_end {
                let (should_break, should_continue) = process_line(line_no);
                if should_break {
                    break;
                }
                if should_continue {
                    continue;
                }
            }
        }
    }
    out
}

pub(in super::super) fn rewrite_adjacent_duplicate_pure_call_assignments(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    rewrite_adjacent_duplicate_assignments_impl(lines, pure_user_calls, true, false)
}

pub(in super::super) fn rewrite_adjacent_duplicate_symbol_assignments(
    lines: Vec<String>,
) -> Vec<String> {
    rewrite_adjacent_duplicate_assignments_impl(lines, &FxHashSet::default(), false, true)
}

pub(in super::super) fn rewrite_adjacent_duplicate_assignments(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    rewrite_adjacent_duplicate_assignments_impl(lines, pure_user_calls, true, true)
}

fn rewrite_adjacent_duplicate_assignments_impl(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    include_pure_call: bool,
    include_symbol: bool,
) -> Vec<String> {
    let mut out = lines;
    if out.len() < 2 {
        return out;
    }

    let Some(assign_re) = assign_re() else {
        return out;
    };
    let Some(plain_ident_re) = plain_ident_re() else {
        return out;
    };

    for idx in 0..(out.len() - 1) {
        let first = out[idx].trim();
        let second = out[idx + 1].trim();
        let Some(caps0) = assign_re.captures(first) else {
            continue;
        };
        let Some(caps1) = assign_re.captures(second) else {
            continue;
        };
        let lhs0 = caps0.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs0 = caps0.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let lhs1 = caps1.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs1 = caps1.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if !plain_ident_re.is_match(lhs0)
            || !plain_ident_re.is_match(lhs1)
            || lhs0.starts_with(".arg_")
            || lhs1.starts_with(".arg_")
            || lhs0.starts_with(".__rr_cse_")
            || lhs1.starts_with(".__rr_cse_")
            || lhs0 == lhs1
            || rhs0 != rhs1
        {
            continue;
        }

        let can_rewrite_pure_call = include_pure_call
            && rhs0.contains('(')
            && expr_has_only_pure_calls(rhs0, pure_user_calls);
        let can_rewrite_symbol =
            include_symbol && plain_ident_re.is_match(rhs0) && !rhs0.starts_with(".arg_");
        if !can_rewrite_pure_call && !can_rewrite_symbol {
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

pub(in super::super) fn strip_terminal_repeat_nexts(lines: Vec<String>) -> Vec<String> {
    strip_terminal_repeat_nexts_ir(lines)
}

pub(in super::super) fn rewrite_forward_exact_expr_reuse(lines: Vec<String>) -> Vec<String> {
    if use_ir_forward_exact_expr() {
        return rewrite_forward_exact_expr_reuse_ir(lines);
    }
    let mut cache = PeepholeAnalysisCache::default();
    let input = lines.clone();
    let out = rewrite_forward_exact_expr_reuse_with_cache(lines, &mut cache);
    compare_exact_forward_ir("exact_expr", &input, &out, &FxHashSet::default());
    out
}

pub(in super::super) fn rewrite_forward_exact_expr_reuse_with_cache(
    lines: Vec<String>,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    if use_ir_forward_exact_expr() {
        return rewrite_forward_exact_expr_reuse_ir(lines);
    }
    let Some(assign_re) = assign_re() else {
        return lines;
    };
    let Some(plain_ident_re) = plain_ident_re() else {
        return lines;
    };
    let mut out = lines;
    let debug = std::env::var_os("RR_DEBUG_PEEPHOLE").is_some();
    let function_facts = cached_function_facts(cache, &out);
    let line_to_fn = build_line_to_function(&function_facts, out.len());
    let candidates = function_facts
        .iter()
        .flat_map(|function| {
            function
                .exact_reuse_candidates
                .exact_expr_candidates
                .iter()
                .cloned()
        })
        .collect::<Vec<_>>();
    for candidate in candidates {
        let idx = candidate.line_idx;
        let region_end = candidate.region_end.min(out.len());
        let lhs = candidate.lhs.as_str();
        let rhs = candidate.rhs.as_str();
        if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
            eprintln!(
                "RR_DEBUG_PEEPHOLE exact_expr_candidate line={} lhs={} rhs={}",
                idx + 1,
                lhs,
                rhs
            );
        }
        if !plain_ident_re.is_match(lhs) || !expr_is_exact_reusable_scalar(rhs) {
            if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                eprintln!(
                    "RR_DEBUG_PEEPHOLE exact_expr_skip line={} lhs={} reusable={}",
                    idx + 1,
                    lhs,
                    expr_is_exact_reusable_scalar(rhs)
                );
            }
            continue;
        }
        let mut prologue_arg_aliases = None;
        let next_lhs_def =
            next_def_after_in_facts(function_facts, idx, lhs, region_end).unwrap_or(region_end);
        let lhs_reassigned_later = next_lhs_def < region_end;
        let scan_end = next_lhs_def.min(region_end);
        if scan_end <= idx + 1 {
            continue;
        }
        let rhs_idents = candidate.idents;
        let deps: FxHashSet<String> = rhs_idents.iter().cloned().collect();
        let current_facts = line_to_fn[idx].and_then(|fn_idx| function_facts.get(fn_idx));
        let rhs_used_in_region = current_facts
            .map(|facts| {
                rhs_idents
                    .iter()
                    .any(|ident| !uses_in_region(facts, ident, idx, scan_end).is_empty())
            })
            .unwrap_or(false);
        if !rhs_used_in_region && !out[idx + 1..scan_end].iter().any(|line| line.contains(rhs)) {
            continue;
        }
        let candidate_lines = current_facts
            .filter(|_| !deps.is_empty())
            .map(|facts| reuse_candidate_lines(facts, lhs, &deps, idx, scan_end))
            .filter(|lines| !lines.is_empty());
        let mut process_line = |line_no: usize| {
            let mut replacement = None;
            let mut should_break = false;
            let mut should_continue = false;
            {
                let line_trimmed = out[line_no].trim();
                let is_return = line_trimmed == "return(NULL)"
                    || (line_trimmed.starts_with("return(") && line_trimmed.ends_with(')'));
                let may_mention_dep = line_might_mention_any(line_trimmed, &deps);
                let may_mention_lhs = line_trimmed.contains(lhs);
                if !line_trimmed.contains(" <- ")
                    && !line_trimmed.contains(rhs)
                    && !may_mention_lhs
                    && !is_return
                {
                    return (false, false);
                }
                if let Some(next_caps) = assign_re.captures(line_trimmed) {
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
                        if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                            eprintln!(
                                "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=same_lhs next_line={}",
                                idx + 1,
                                lhs,
                                line_trimmed
                            );
                        }
                        should_break = true;
                    } else {
                        if !line_trimmed.contains(rhs) && !may_mention_dep && !may_mention_lhs {
                            return (false, false);
                        }
                        if next_rhs.contains(rhs) {
                            if lhs_reassigned_later {
                                should_continue = true;
                            } else {
                                if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                                    eprintln!(
                                        "RR_DEBUG_PEEPHOLE exact_expr_replace line={} lhs={} target_line={}",
                                        idx + 1,
                                        lhs,
                                        line_trimmed
                                    );
                                }
                                replacement = Some(out[line_no].replacen(rhs, lhs, usize::MAX));
                            }
                        }
                        if deps.contains(next_lhs) {
                            let mut same_rhs_as_previous = false;
                            for prev_idx in (idx + 1..line_no).rev() {
                                let prev_trimmed = out[prev_idx].trim();
                                let Some(prev_caps) = assign_re.captures(prev_trimmed) else {
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
                                if prev_rhs == next_rhs {
                                    same_rhs_as_previous = true;
                                } else if prev_rhs.contains(".arg_") || next_rhs.contains(".arg_") {
                                    let aliases = prologue_arg_aliases.get_or_insert_with(|| {
                                        collect_prologue_arg_aliases(&out, idx)
                                    });
                                    let prev_norm = normalize_expr_with_aliases(prev_rhs, aliases);
                                    let next_norm = normalize_expr_with_aliases(next_rhs, aliases);
                                    if prev_norm == next_norm {
                                        same_rhs_as_previous = true;
                                    }
                                }
                                if same_rhs_as_previous {
                                    same_rhs_as_previous = true;
                                }
                                break;
                            }
                            if same_rhs_as_previous {
                                should_continue = true;
                            } else {
                                if debug && (lhs == "inlined_39_u" || lhs == "inlined_39_v") {
                                    eprintln!(
                                        "RR_DEBUG_PEEPHOLE exact_expr_stop line={} lhs={} reason=dep_write dep={} target_line={}",
                                        idx + 1,
                                        lhs,
                                        next_lhs,
                                        line_trimmed
                                    );
                                }
                                should_break = true;
                            }
                        }
                    }
                } else {
                    if line_trimmed.contains(rhs) {
                        replacement = Some(out[line_no].replacen(rhs, lhs, usize::MAX));
                    }
                    if is_return {
                        should_break = true;
                    }
                }
            }
            if let Some(new_line) = replacement {
                out[line_no] = new_line;
            }
            (should_break, should_continue)
        };
        if let Some(candidate_lines) = candidate_lines.as_deref() {
            for &line_no in candidate_lines {
                let (should_break, should_continue) = process_line(line_no);
                if should_break {
                    break;
                }
                if should_continue {
                    continue;
                }
            }
        } else {
            for line_no in idx + 1..scan_end {
                let (should_break, should_continue) = process_line(line_no);
                if should_break {
                    break;
                }
                if should_continue {
                    continue;
                }
            }
        }
    }
    out
}
