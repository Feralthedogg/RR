use super::{
    FunctionFacts, PeepholeAnalysisCache, assign_re, cached_function_facts, count_unquoted_braces,
    expr_has_only_pure_calls, expr_idents, ident_re, is_control_flow_boundary, plain_ident_re,
    scalar_lit_re, uses_in_region,
};
use regex::Captures;
use rustc_hash::FxHashSet;
use std::sync::OnceLock;
use std::time::Instant;

#[derive(Default)]
pub(in super::super) struct ImmediateInlineProfile {
    pub(super) immediate_scalar_elapsed_ns: u128,
    pub(super) named_expr_elapsed_ns: u128,
    pub(super) immediate_index_elapsed_ns: u128,
}

#[derive(Default)]
pub(in super::super) struct StraightLineInlineProfile {
    pub(super) named_index_elapsed_ns: u128,
    pub(super) scalar_region_elapsed_ns: u128,
}

fn build_line_to_function(facts: &[FunctionFacts], total_lines: usize) -> Vec<Option<usize>> {
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

fn build_block_end_map(lines: &[String]) -> Vec<Option<usize>> {
    let mut ends = vec![None; lines.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let (opens, closes) = count_unquoted_braces(line.trim());
        for _ in 0..closes {
            let Some(open_idx) = stack.pop() else {
                break;
            };
            ends[open_idx] = Some(idx);
        }
        for _ in 0..opens {
            stack.push(idx);
        }
    }
    ends
}

fn expr_is_simple_scalar_index_read(rhs: &str) -> bool {
    static RE: OnceLock<Option<regex::Regex>> = OnceLock::new();
    RE.get_or_init(|| super::compile_regex(format!(r"^{}\[[^\],]+\]$", super::IDENT_PATTERN)))
        .as_ref()
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

pub(in super::super) fn rewrite_temp_uses_after_named_copy_with_cache(
    lines: Vec<String>,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    if !lines
        .iter()
        .any(|line| line.contains(".__pc_src_tmp") || line.contains(".__rr_cse_"))
    {
        return lines;
    }
    let mut out = lines;
    let function_facts = cached_function_facts(cache, &out);
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for facts in function_facts.iter() {
        for fact in &facts.line_facts {
            if !fact.is_assign {
                continue;
            }
            let Some(lhs) = fact.lhs.as_ref() else {
                continue;
            };
            let Some(rhs) = fact.rhs.as_ref() else {
                continue;
            };
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
                || !(rhs.starts_with(".__pc_src_tmp") || rhs.starts_with(".__rr_cse_"))
            {
                continue;
            }
            let idx = fact.line_idx;
            let temp = rhs.as_str();
            let next_temp_def = facts
                .defs
                .get(temp)
                .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > idx))
                .unwrap_or(facts.function.end + 1);
            let next_lhs_def = facts
                .defs
                .get(lhs)
                .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > idx))
                .unwrap_or(facts.function.end + 1);
            let region_end = next_temp_def.min(next_lhs_def);
            let use_lines = uses_in_region(facts, temp, idx, region_end);
            if use_lines.is_empty() {
                continue;
            }
            for &line_no in use_lines {
                let line = &mut out[line_no];
                let line_trimmed = line.trim();
                if line_trimmed.is_empty() || !line.contains(temp) {
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
    }
    out
}

pub(super) fn rewrite_temp_uses_after_named_copy(lines: Vec<String>) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    rewrite_temp_uses_after_named_copy_with_cache(lines, &mut analysis_cache)
}

pub(in super::super) fn run_immediate_single_use_inline_bundle_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut PeepholeAnalysisCache,
) -> (Vec<String>, ImmediateInlineProfile) {
    if !lines.iter().any(|line| line.contains("<-")) {
        return (lines, ImmediateInlineProfile::default());
    }
    let mut out = lines;
    let function_facts = cached_function_facts(cache, &out);
    let Some(ident_re) = ident_re() else {
        return (out, ImmediateInlineProfile::default());
    };
    let Some(plain_ident_re) = plain_ident_re() else {
        return (out, ImmediateInlineProfile::default());
    };
    let mut profile = ImmediateInlineProfile::default();
    for facts in function_facts.iter() {
        for line_fact in &facts.line_facts {
            if !line_fact.is_assign {
                continue;
            }
            let idx = line_fact.line_idx;
            let current_trimmed = out[idx].trim();
            let Some(current_caps) = assign_re().and_then(|re| re.captures(current_trimmed)) else {
                continue;
            };
            let lhs = current_caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let rhs = current_caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if lhs.is_empty() || rhs.is_empty() {
                continue;
            }

            enum ImmediateKind {
                ScalarTemp,
                NamedExpr,
                IndexTemp,
            }

            let kind = if lhs.starts_with(".__rr_cse_") && super::expr_is_exact_reusable_scalar(rhs)
            {
                ImmediateKind::ScalarTemp
            } else if lhs.starts_with(".__rr_cse_") && rhs.starts_with("rr_index_vec_floor(") {
                ImmediateKind::IndexTemp
            } else if plain_ident_re.is_match(lhs)
                && !lhs.starts_with(".arg_")
                && !lhs.starts_with(".__rr_cse_")
                && !line_fact.in_loop_body
                && expr_is_inlineable_named_scalar_expr(rhs, pure_user_calls)
            {
                ImmediateKind::NamedExpr
            } else {
                continue;
            };

            let started = Instant::now();
            let Some(next_idx) = line_fact
                .next_non_empty_line
                .filter(|line_no| *line_no <= facts.function.end)
                .or_else(|| ((idx + 1)..=facts.function.end).find(|i| !out[*i].trim().is_empty()))
            else {
                continue;
            };
            let next_trimmed = out[next_idx].trim();
            if next_trimmed.is_empty()
                || out[next_idx].contains("<- function")
                || is_control_flow_boundary(next_trimmed)
            {
                continue;
            }

            let next_def = facts
                .defs
                .get(lhs)
                .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > idx))
                .unwrap_or(facts.function.end + 1);
            let use_lines = uses_in_region(facts, lhs, idx, next_def);
            if use_lines != [next_idx] {
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

            let elapsed = started.elapsed().as_nanos();
            match kind {
                ImmediateKind::ScalarTemp => profile.immediate_scalar_elapsed_ns += elapsed,
                ImmediateKind::NamedExpr => profile.named_expr_elapsed_ns += elapsed,
                ImmediateKind::IndexTemp => profile.immediate_index_elapsed_ns += elapsed,
            }
        }
    }
    (out, profile)
}

pub(in super::super) fn inline_immediate_single_use_scalar_temps_with_cache(
    lines: Vec<String>,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    run_immediate_single_use_inline_bundle_with_cache(lines, &FxHashSet::default(), cache).0
}

pub(super) fn inline_immediate_single_use_scalar_temps(lines: Vec<String>) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    inline_immediate_single_use_scalar_temps_with_cache(lines, &mut analysis_cache)
}

fn inline_named_scalar_index_reads_within_straight_line_region_impl(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    min_uses: usize,
    max_uses: usize,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    if !lines
        .iter()
        .any(|line| line.contains("rr_") || line.contains('['))
    {
        return lines;
    }
    let mut out = lines;
    let function_facts = cached_function_facts(cache, &out);
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for facts in function_facts.iter() {
        for line_fact in &facts.line_facts {
            if !line_fact.is_assign {
                continue;
            }
            let idx = line_fact.line_idx;
            let current_trimmed = out[idx].trim().to_string();
            let Some(current_caps) = assign_re().and_then(|re| re.captures(&current_trimmed))
            else {
                continue;
            };
            let lhs = current_caps
                .name("lhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            let rhs = current_caps
                .name("rhs")
                .map(|m| m.as_str())
                .unwrap_or("")
                .trim();
            if lhs.is_empty() || rhs.is_empty() {
                continue;
            }
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
                || lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || !expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
            {
                continue;
            }
            let next_def = facts
                .defs
                .get(lhs)
                .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > idx))
                .unwrap_or(facts.function.end + 1);
            let region_end = line_fact.inline_region_end.min(next_def);
            let use_lines = uses_in_region(facts, lhs, idx, region_end);
            let Some(&first_use) = use_lines.first() else {
                continue;
            };
            if use_lines.len() < min_uses || use_lines.len() > max_uses {
                continue;
            }
            let second_use = use_lines.get(1).copied();
            let last_use = second_use.unwrap_or(first_use);
            let dep_write_before_use = line_fact.idents.iter().any(|dep| {
                facts.defs.get(dep).is_some_and(|defs| {
                    defs.iter()
                        .copied()
                        .any(|line_idx| line_idx > idx && line_idx < last_use)
                })
            });
            if dep_write_before_use {
                continue;
            }
            let mut changed = false;
            for use_idx in [Some(first_use), second_use].into_iter().flatten() {
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
    }
    out
}

pub(super) fn inline_single_use_named_scalar_index_reads_within_straight_line_region(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    inline_named_scalar_index_reads_within_straight_line_region_impl(
        lines,
        pure_user_calls,
        1,
        1,
        &mut analysis_cache,
    )
}

pub(super) fn inline_one_or_two_use_named_scalar_index_reads_within_straight_line_region(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    inline_named_scalar_index_reads_within_straight_line_region_impl(
        lines,
        pure_user_calls,
        1,
        2,
        &mut analysis_cache,
    )
}

pub(in super::super) fn inline_one_or_two_use_named_scalar_index_reads_within_straight_line_region_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    inline_named_scalar_index_reads_within_straight_line_region_impl(
        lines,
        pure_user_calls,
        1,
        2,
        cache,
    )
}

pub(in super::super) fn inline_immediate_single_use_named_scalar_exprs_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    run_immediate_single_use_inline_bundle_with_cache(lines, pure_user_calls, cache).0
}

pub(super) fn inline_immediate_single_use_named_scalar_exprs(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    inline_immediate_single_use_named_scalar_exprs_with_cache(
        lines,
        pure_user_calls,
        &mut analysis_cache,
    )
}

pub(in super::super) fn hoist_branch_local_named_scalar_assigns_used_after_branch_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    if !lines
        .iter()
        .any(|line| line.trim_start().starts_with("if ") && line.trim_end().ends_with('{'))
    {
        return lines;
    }
    let mut out = lines;
    let function_facts = cached_function_facts(cache, &out);
    let line_to_fn = build_line_to_function(&function_facts, out.len());
    let block_end_map = build_block_end_map(&out);
    let function_has_branch_hoist_candidate = function_facts
        .iter()
        .map(|facts| {
            facts.line_facts.iter().any(|fact| {
                let Some(lhs) = fact.lhs.as_deref() else {
                    return false;
                };
                let Some(rhs) = fact.rhs.as_deref() else {
                    return false;
                };
                plain_ident_re().is_some_and(|re| re.is_match(lhs))
                    && !lhs.starts_with(".arg_")
                    && !lhs.starts_with(".__rr_cse_")
                    && expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
                    && facts
                        .uses
                        .get(lhs)
                        .is_some_and(|uses| uses.iter().any(|line_idx| *line_idx > fact.line_idx))
            })
        })
        .collect::<Vec<_>>();
    let mut idx = 0usize;
    while idx < out.len() {
        if let Some(fn_idx) = line_to_fn.get(idx).and_then(|entry| *entry)
            && !function_has_branch_hoist_candidate[fn_idx]
        {
            idx = function_facts[fn_idx].function.end + 1;
            continue;
        }
        let trimmed = out[idx].trim();
        if !(trimmed.starts_with("if ") && trimmed.ends_with('{')) {
            idx += 1;
            continue;
        }
        let Some(fn_idx) = line_to_fn[idx] else {
            idx += 1;
            continue;
        };
        let facts = &function_facts[fn_idx];
        let guard_idents = expr_idents(trimmed);
        let Some(end_idx) = block_end_map[idx] else {
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
            let Some(scan_fact) = facts
                .line_facts
                .get(scan.saturating_sub(facts.function.start))
            else {
                break;
            };
            if !scan_fact.is_assign {
                break;
            }
            let Some(lhs) = scan_fact.lhs.as_deref() else {
                break;
            };
            let Some(rhs) = scan_fact.rhs.as_deref() else {
                break;
            };
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
                || lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || !expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
            {
                break;
            }
            trailing.push(scan);
        }
        if trailing.is_empty() {
            idx = end_idx + 1;
            continue;
        }
        trailing.reverse();

        let mut hoisted = Vec::new();
        for assign_idx in trailing {
            let Some(assign_fact) = facts
                .line_facts
                .get(assign_idx.saturating_sub(facts.function.start))
            else {
                continue;
            };
            let Some(lhs) = assign_fact.lhs.as_deref() else {
                continue;
            };
            let Some(_rhs) = assign_fact.rhs.as_deref() else {
                continue;
            };
            if guard_idents.iter().any(|ident| ident == &lhs) {
                continue;
            }
            let dep_written_in_branch = assign_fact.idents.iter().any(|dep| {
                facts.defs.get(dep).is_some_and(|defs| {
                    defs.iter()
                        .copied()
                        .any(|line_idx| line_idx > idx && line_idx < assign_idx)
                })
            });
            if dep_written_in_branch {
                continue;
            }

            let next_def = facts
                .defs
                .get(lhs)
                .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > assign_idx))
                .unwrap_or(facts.function.end + 1);
            let used_after = facts.uses.get(lhs).is_some_and(|uses| {
                uses.iter()
                    .copied()
                    .any(|line_idx| line_idx > end_idx && line_idx < next_def)
            });
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

pub(super) fn hoist_branch_local_named_scalar_assigns_used_after_branch(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    hoist_branch_local_named_scalar_assigns_used_after_branch_with_cache(
        lines,
        pure_user_calls,
        &mut analysis_cache,
    )
}

pub(super) fn inline_two_use_named_scalar_index_reads_within_straight_line_region(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    inline_named_scalar_index_reads_within_straight_line_region_impl(
        lines,
        pure_user_calls,
        2,
        2,
        &mut analysis_cache,
    )
}

fn inline_scalar_temps_within_straight_line_region_impl(
    lines: Vec<String>,
    min_uses: usize,
    max_uses: usize,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    if !lines.iter().any(|line| line.contains(".__rr_cse_")) {
        return lines;
    }
    let mut out = lines;
    let function_facts = cached_function_facts(cache, &out);
    let Some(ident_re) = ident_re() else {
        return out;
    };
    for facts in function_facts.iter() {
        for line_fact in &facts.line_facts {
            if !line_fact.is_assign {
                continue;
            }
            let Some(lhs) = line_fact.lhs.as_deref() else {
                continue;
            };
            let Some(rhs) = line_fact.rhs.as_deref() else {
                continue;
            };
            if !lhs.starts_with(".__rr_cse_") || !super::expr_is_exact_reusable_scalar(rhs) {
                continue;
            }
            let idx = line_fact.line_idx;
            let region_end = line_fact.inline_region_end;
            let next_def = facts
                .defs
                .get(lhs)
                .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > idx))
                .unwrap_or(facts.function.end + 1);
            let next_dep_write = line_fact
                .idents
                .iter()
                .filter_map(|dep| {
                    facts
                        .defs
                        .get(dep)
                        .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > idx))
                })
                .min()
                .unwrap_or(facts.function.end + 1);
            let use_lines = uses_in_region(facts, lhs, idx, region_end.min(next_def));
            let Some(&first_use) = use_lines.first() else {
                continue;
            };
            if use_lines.len() < min_uses || use_lines.len() > max_uses {
                continue;
            }
            let second_use = use_lines.get(1).copied();
            let last_use = second_use.unwrap_or(first_use);
            if next_dep_write < last_use {
                continue;
            }
            let mut changed = false;
            for use_idx in [Some(first_use), second_use].into_iter().flatten() {
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
    }
    out
}

pub(super) fn inline_one_or_two_use_scalar_temps_within_straight_line_region(
    lines: Vec<String>,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    inline_scalar_temps_within_straight_line_region_impl(lines, 1, 2, &mut analysis_cache)
}

pub(in super::super) fn inline_one_or_two_use_scalar_temps_within_straight_line_region_with_cache(
    lines: Vec<String>,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    inline_scalar_temps_within_straight_line_region_impl(lines, 1, 2, cache)
}

pub(in super::super) fn run_named_index_scalar_region_inline_bundle_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut PeepholeAnalysisCache,
) -> (Vec<String>, StraightLineInlineProfile) {
    if !lines
        .iter()
        .any(|line| line.contains("rr_") || line.contains('[') || line.contains(".__rr_cse_"))
    {
        return (lines, StraightLineInlineProfile::default());
    }
    let mut out = lines;
    let function_facts = cached_function_facts(cache, &out);
    let Some(ident_re) = ident_re() else {
        return (out, StraightLineInlineProfile::default());
    };
    let Some(plain_ident_re) = plain_ident_re() else {
        return (out, StraightLineInlineProfile::default());
    };
    let mut profile = StraightLineInlineProfile::default();

    let started = Instant::now();
    for facts in function_facts.iter() {
        for line_fact in &facts.line_facts {
            if !line_fact.is_assign {
                continue;
            }
            let idx = line_fact.line_idx;
            let Some(lhs) = line_fact.lhs.as_deref() else {
                continue;
            };
            let Some(rhs) = line_fact.rhs.as_deref() else {
                continue;
            };
            if !plain_ident_re.is_match(lhs)
                || lhs.starts_with(".arg_")
                || lhs.starts_with(".__rr_cse_")
                || !expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
            {
                continue;
            }
            let next_def = facts
                .defs
                .get(lhs)
                .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > idx))
                .unwrap_or(facts.function.end + 1);
            let region_end = line_fact.inline_region_end.min(next_def);
            let use_lines = uses_in_region(facts, lhs, idx, region_end);
            let Some(&first_use) = use_lines.first() else {
                continue;
            };
            if use_lines.len() > 2 {
                continue;
            }
            let second_use = use_lines.get(1).copied();
            let last_use = second_use.unwrap_or(first_use);
            let dep_write_before_use = line_fact.idents.iter().any(|dep| {
                facts.defs.get(dep).is_some_and(|defs| {
                    defs.iter()
                        .copied()
                        .any(|line_idx| line_idx > idx && line_idx < last_use)
                })
            });
            if dep_write_before_use {
                continue;
            }
            let mut changed = false;
            for use_idx in [Some(first_use), second_use].into_iter().flatten() {
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
    }
    profile.named_index_elapsed_ns = started.elapsed().as_nanos();

    let started = Instant::now();
    for facts in function_facts.iter() {
        for line_fact in &facts.line_facts {
            if !line_fact.is_assign {
                continue;
            }
            let Some(lhs) = line_fact.lhs.as_deref() else {
                continue;
            };
            let Some(rhs) = line_fact.rhs.as_deref() else {
                continue;
            };
            if !lhs.starts_with(".__rr_cse_") || !super::expr_is_exact_reusable_scalar(rhs) {
                continue;
            }
            let idx = line_fact.line_idx;
            let region_end = line_fact.inline_region_end;
            let next_def = facts
                .defs
                .get(lhs)
                .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > idx))
                .unwrap_or(facts.function.end + 1);
            let next_dep_write = line_fact
                .idents
                .iter()
                .filter_map(|dep| {
                    facts
                        .defs
                        .get(dep)
                        .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > idx))
                })
                .min()
                .unwrap_or(facts.function.end + 1);
            let use_lines = uses_in_region(facts, lhs, idx, region_end.min(next_def));
            let Some(&first_use) = use_lines.first() else {
                continue;
            };
            if use_lines.len() > 2 {
                continue;
            }
            let second_use = use_lines.get(1).copied();
            let last_use = second_use.unwrap_or(first_use);
            if next_dep_write < last_use {
                continue;
            }
            let mut changed = false;
            for use_idx in [Some(first_use), second_use].into_iter().flatten() {
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
    }
    profile.scalar_region_elapsed_ns = started.elapsed().as_nanos();

    (out, profile)
}

pub(super) fn inline_single_use_scalar_temps_within_straight_line_region(
    lines: Vec<String>,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    inline_scalar_temps_within_straight_line_region_impl(lines, 1, 1, &mut analysis_cache)
}

pub(super) fn inline_two_use_scalar_temps_within_straight_line_region(
    lines: Vec<String>,
) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    inline_scalar_temps_within_straight_line_region_impl(lines, 2, 2, &mut analysis_cache)
}

pub(in super::super) fn inline_immediate_single_use_index_temps_with_cache(
    lines: Vec<String>,
    cache: &mut PeepholeAnalysisCache,
) -> Vec<String> {
    run_immediate_single_use_inline_bundle_with_cache(lines, &FxHashSet::default(), cache).0
}

pub(super) fn inline_immediate_single_use_index_temps(lines: Vec<String>) -> Vec<String> {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    inline_immediate_single_use_index_temps_with_cache(lines, &mut analysis_cache)
}
