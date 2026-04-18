use super::{
    FunctionFacts, FunctionLineFacts, PeepholeAnalysisCache, assign_re, build_function_text_index,
    count_unquoted_braces, expr_has_only_pure_calls, expr_idents, find_matching_block_end,
    is_control_flow_boundary, is_dead_parenthesized_eval_line, is_dead_plain_ident_eval_line,
    is_loop_open_boundary, plain_ident_re, scalar_lit_re,
};
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Copy, Debug, Default)]
struct FunctionDeadTempPlan {
    has_trivial_removals: bool,
    has_temp_like_assign: bool,
    has_dead_assign_candidates: bool,
    has_branch_local_candidates: bool,
    has_repeat_loop: bool,
    has_if_open: bool,
    has_redundant_temp_candidates: bool,
}

impl FunctionDeadTempPlan {
    fn has_heavy_candidates(self) -> bool {
        self.has_temp_like_assign
            || self.has_dead_assign_candidates
            || (self.has_branch_local_candidates && self.has_repeat_loop && self.has_if_open)
            || self.has_redundant_temp_candidates
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct DeadTempProfile {
    pub(super) facts_elapsed_ns: u128,
    pub(super) mark_elapsed_ns: u128,
    pub(super) reverse_elapsed_ns: u128,
    pub(super) compact_elapsed_ns: u128,
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
    pure_user_calls.contains(callee.trim())
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

fn build_dead_temp_function_facts(
    lines: &[String],
    functions: &[super::IndexedFunction],
    plans: &[FunctionDeadTempPlan],
) -> Vec<FunctionFacts> {
    let Some(assign_re) = assign_re() else {
        return Vec::new();
    };
    functions
        .iter()
        .cloned()
        .zip(plans.iter().copied())
        .map(|(function, plan)| {
            let fn_start = function.start;
            let fn_end = function.end;
            let collect_uses = plan.has_dead_assign_candidates;
            if !plan.has_heavy_candidates() {
                return FunctionFacts {
                    function,
                    line_facts: if plan.has_trivial_removals {
                        (fn_start..=fn_end)
                            .map(|line_idx| FunctionLineFacts {
                                line_idx,
                                ..Default::default()
                            })
                            .collect()
                    } else {
                        Vec::new()
                    },
                    defs: FxHashMap::default(),
                    uses: FxHashMap::default(),
                    helper_call_lines: Vec::new(),
                    param_set: FxHashSet::default(),
                    mutated_arg_aliases: FxHashSet::default(),
                    prologue_arg_alias_defs: Vec::new(),
                    non_prologue_assigned_idents: FxHashSet::default(),
                    stored_bases: FxHashSet::default(),
                    mentioned_arg_aliases: FxHashSet::default(),
                    exact_reuse_candidates: Default::default(),
                };
            }
            let mut line_facts =
                Vec::with_capacity(function.end.saturating_sub(function.start) + 1);
            let mut uses = FxHashMap::<String, Vec<usize>>::default();
            for line_idx in function.start..=function.end {
                let line = &lines[line_idx];
                let trimmed = line.trim();
                let indent = line.len() - line.trim_start().len();
                let mut fact = FunctionLineFacts {
                    line_idx,
                    indent,
                    is_control_boundary: is_control_flow_boundary(trimmed),
                    ..Default::default()
                };
                if let Some(caps) = assign_re.captures(trimmed) {
                    let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                    let idents = expr_idents(rhs);
                    fact.is_assign = true;
                    fact.lhs = Some(lhs.to_string());
                    fact.rhs = Some(rhs.to_string());
                    fact.idents = idents.clone();
                    if collect_uses {
                        for ident in idents {
                            uses.entry(ident).or_default().push(line_idx);
                        }
                    }
                } else {
                    fact.idents = expr_idents(trimmed);
                    if collect_uses {
                        for ident in &fact.idents {
                            uses.entry(ident.clone()).or_default().push(line_idx);
                        }
                    }
                }
                line_facts.push(fact);
            }
            FunctionFacts {
                function,
                line_facts,
                defs: FxHashMap::default(),
                uses,
                helper_call_lines: Vec::new(),
                param_set: FxHashSet::default(),
                mutated_arg_aliases: FxHashSet::default(),
                prologue_arg_alias_defs: Vec::new(),
                non_prologue_assigned_idents: FxHashSet::default(),
                stored_bases: FxHashSet::default(),
                mentioned_arg_aliases: FxHashSet::default(),
                exact_reuse_candidates: Default::default(),
            }
        })
        .collect()
}

fn line_fact_for_idx<'a>(
    facts: &'a [FunctionFacts],
    line_to_fn: &[Option<usize>],
    idx: usize,
) -> Option<&'a FunctionLineFacts> {
    let fn_idx = line_to_fn.get(idx).and_then(|entry| *entry)?;
    let function_facts = &facts[fn_idx];
    function_facts
        .line_facts
        .get(idx.saturating_sub(function_facts.function.start))
}

fn loop_body_references_var_before(lines: &[String], idx: usize, var: &str) -> bool {
    (0..idx)
        .rev()
        .find_map(|start_idx| {
            is_loop_open_boundary(lines[start_idx].trim())
                .then(|| find_matching_block_end(lines, start_idx).map(|end| (start_idx, end)))
                .flatten()
                .filter(|(_, end)| idx < *end)
        })
        .is_some_and(|(start, _end)| {
            lines
                .iter()
                .take(idx)
                .skip(start + 1)
                .any(|line| expr_idents(line).iter().any(|ident| ident == var))
        })
}

fn build_enclosing_loop_starts(lines: &[String]) -> Vec<Option<usize>> {
    let mut out = vec![None; lines.len()];
    let mut block_stack: Vec<Option<usize>> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        out[idx] = block_stack.iter().rev().find_map(|entry| *entry);
        let trimmed = line.trim();
        let (opens, closes) = count_unquoted_braces(trimmed);
        for _ in 0..closes {
            let _ = block_stack.pop();
        }
        let loop_open = is_loop_open_boundary(trimmed);
        for open_idx in 0..opens {
            block_stack.push(if loop_open && open_idx == 0 {
                Some(idx)
            } else {
                None
            });
        }
    }
    out
}

fn build_enclosing_if_starts(lines: &[String]) -> Vec<Option<usize>> {
    let mut out = vec![None; lines.len()];
    let mut block_stack: Vec<Option<usize>> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        out[idx] = block_stack.iter().rev().find_map(|entry| *entry);
        let trimmed = line.trim();
        let (opens, closes) = count_unquoted_braces(trimmed);
        for _ in 0..closes {
            let _ = block_stack.pop();
        }
        let if_open = trimmed.starts_with("if ") && trimmed.ends_with('{');
        for open_idx in 0..opens {
            block_stack.push(if if_open && open_idx == 0 {
                Some(idx)
            } else {
                None
            });
        }
    }
    out
}

fn build_block_end_map(lines: &[String]) -> Vec<Option<usize>> {
    let mut out = vec![None; lines.len()];
    let mut block_stack: Vec<usize> = Vec::new();
    for (idx, line) in lines.iter().enumerate() {
        let (opens, closes) = count_unquoted_braces(line.trim());
        for _ in 0..closes {
            let Some(open_idx) = block_stack.pop() else {
                break;
            };
            out[open_idx] = Some(idx);
        }
        for _ in 0..opens {
            block_stack.push(idx);
        }
    }
    out
}

fn loop_body_references_var_before_cached(
    facts: &[FunctionFacts],
    line_to_fn: &[Option<usize>],
    enclosing_loop_starts: &[Option<usize>],
    idx: usize,
    var: &str,
) -> bool {
    let Some(loop_start) = enclosing_loop_starts.get(idx).and_then(|entry| *entry) else {
        return false;
    };
    let Some(fn_idx) = line_to_fn.get(idx).and_then(|entry| *entry) else {
        return false;
    };
    facts[fn_idx].uses.get(var).is_some_and(|use_lines| {
        use_lines
            .iter()
            .any(|&use_idx| use_idx > loop_start && use_idx < idx)
    })
}

fn mark_redundant_identical_temp_reassigns_with_cache(
    lines: &[String],
    facts: &[FunctionFacts],
    plans: &[FunctionDeadTempPlan],
) -> Vec<bool> {
    let mut removable = vec![false; lines.len()];
    for (function, plan) in facts.iter().zip(plans.iter()) {
        if !plan.has_redundant_temp_candidates {
            continue;
        }
        for fact in &function.line_facts {
            if !fact.is_assign {
                continue;
            }
            let Some(lhs) = fact.lhs.as_ref().map(String::as_str) else {
                continue;
            };
            let Some(rhs) = fact.rhs.as_ref().map(String::as_str) else {
                continue;
            };
            if !lhs.starts_with(".__rr_cse_") {
                continue;
            }
            let idx = fact.line_idx;
            let deps: FxHashSet<String> = fact.idents.iter().cloned().collect();
            let cur_indent = fact.indent;
            let mut j = idx;
            while j > function.function.start {
                j -= 1;
                let prev = lines[j].trim();
                if prev.is_empty() {
                    continue;
                }
                if prev == "repeat {" || prev.starts_with("while") || prev.starts_with("for") {
                    break;
                }
                let prev_fact = &function.line_facts[j - function.function.start];
                if !prev_fact.is_assign {
                    continue;
                }
                let Some(prev_lhs) = prev_fact.lhs.as_ref().map(String::as_str) else {
                    continue;
                };
                let Some(prev_rhs) = prev_fact.rhs.as_ref().map(String::as_str) else {
                    continue;
                };
                if prev_lhs == lhs {
                    if prev_rhs == rhs && prev_fact.indent < cur_indent {
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

fn mark_overwritten_dead_assignments_with_cache(
    lines: &[String],
    pure_user_calls: &FxHashSet<String>,
    facts: &[FunctionFacts],
    plans: &[FunctionDeadTempPlan],
) -> Vec<bool> {
    let mut removable = vec![false; lines.len()];
    for (function, plan) in facts.iter().zip(plans.iter()) {
        if !plan.has_dead_assign_candidates {
            continue;
        }
        let mut pending: FxHashMap<String, usize> = FxHashMap::default();
        for fact in &function.line_facts {
            let idx = fact.line_idx;
            let line = &lines[idx];
            if line.contains("<- function") || fact.is_control_boundary {
                pending.clear();
                continue;
            }

            if !fact.is_assign {
                for ident in &fact.idents {
                    pending.remove(ident);
                }
                continue;
            }

            let lhs = fact.lhs.as_ref().map(String::as_str).unwrap_or("");
            let rhs = fact.rhs.as_ref().map(String::as_str).unwrap_or("");
            for ident in &fact.idents {
                pending.remove(ident);
            }

            if plain_ident_re().is_some_and(|re| re.is_match(lhs))
                && !fact.idents.iter().any(|ident| ident == lhs)
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
    }

    removable
}

fn mark_branch_local_dead_inits_with_cache(
    lines: &[String],
    facts: &[FunctionFacts],
    plans: &[FunctionDeadTempPlan],
    block_end_map: &[Option<usize>],
    enclosing_if_starts: &[Option<usize>],
) -> Vec<bool> {
    let mut removable = vec![false; lines.len()];

    for (function, plan) in facts.iter().zip(plans.iter()) {
        if !(plan.has_branch_local_candidates && plan.has_repeat_loop && plan.has_if_open) {
            continue;
        }
        for fact in &function.line_facts {
            if !fact.is_assign {
                continue;
            }
            let lhs = fact.lhs.as_ref().map(String::as_str).unwrap_or("");
            let rhs = fact.rhs.as_ref().map(String::as_str).unwrap_or("");
            if !plain_ident_re().is_some_and(|re| re.is_match(lhs))
                || !scalar_lit_re().is_some_and(|re| re.is_match(rhs))
            {
                continue;
            }

            let idx = fact.line_idx;
            let mut next_idx = idx + 1;
            while next_idx <= function.function.end {
                let next_fact = &function.line_facts[next_idx - function.function.start];
                if !next_fact.is_assign {
                    break;
                }
                let next_lhs = next_fact.lhs.as_deref().unwrap_or("");
                let next_rhs = next_fact.rhs.as_deref().unwrap_or("");
                if plain_ident_re().is_some_and(|re| re.is_match(next_lhs))
                    && scalar_lit_re().is_some_and(|re| re.is_match(next_rhs))
                {
                    next_idx += 1;
                    continue;
                }
                break;
            }
            if next_idx > function.function.end || lines[next_idx].trim() != "repeat {" {
                continue;
            }
            let Some(loop_end) = block_end_map.get(next_idx).and_then(|entry| *entry) else {
                continue;
            };

            let mut first_occurrence = None;
            for line_idx in next_idx + 1..loop_end {
                let loop_fact = &function.line_facts[line_idx - function.function.start];
                let assigned_lhs = loop_fact.lhs.as_ref().map(String::as_str);
                let mentions =
                    assigned_lhs == Some(lhs) || loop_fact.idents.iter().any(|ident| ident == lhs);
                if mentions {
                    first_occurrence = Some((line_idx, assigned_lhs == Some(lhs)));
                    break;
                }
            }
            let Some((first_line_idx, first_is_assign)) = first_occurrence else {
                continue;
            };
            if !first_is_assign {
                continue;
            }

            let Some(if_start) = enclosing_if_starts
                .get(first_line_idx)
                .and_then(|entry| *entry)
            else {
                continue;
            };
            let Some(if_end) = block_end_map.get(if_start).and_then(|entry| *entry) else {
                continue;
            };
            if if_end > loop_end {
                continue;
            }

            let mut used_outside_if = false;
            for line_pos in next_idx + 1..loop_end {
                let scan_fact = &function.line_facts[line_pos - function.function.start];
                let assigned_lhs = scan_fact.lhs.as_ref().map(String::as_str);
                let mentions =
                    assigned_lhs == Some(lhs) || scan_fact.idents.iter().any(|ident| ident == lhs);
                if !mentions {
                    continue;
                }
                if line_pos < if_start || line_pos > if_end {
                    used_outside_if = true;
                    break;
                }
            }
            if !used_outside_if {
                for line_pos in loop_end + 1..=function.function.end {
                    let scan_fact = &function.line_facts[line_pos - function.function.start];
                    let assigned_lhs = scan_fact.lhs.as_ref().map(String::as_str);
                    let mentions = assigned_lhs == Some(lhs)
                        || scan_fact.idents.iter().any(|ident| ident == lhs);
                    if !mentions {
                        continue;
                    }
                    if assigned_lhs == Some(lhs) {
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
    }

    removable
}

pub(super) fn strip_dead_temps(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, Vec<u32>) {
    let mut cache = PeepholeAnalysisCache::default();
    strip_dead_temps_with_cache(lines, pure_user_calls, &mut cache)
}

pub(super) fn strip_dead_temps_with_cache_and_profile(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    _cache: &mut PeepholeAnalysisCache,
) -> ((Vec<String>, Vec<u32>), DeadTempProfile) {
    let mut profile = DeadTempProfile::default();

    let mut has_trivial_removals = false;
    let mut has_temp_like_assign = false;
    let mut has_dead_assign_candidates = false;
    let mut has_branch_local_candidates = false;
    let mut has_redundant_temp_candidates = false;
    let assign_re_cached = assign_re();
    let plain_ident_re_cached = plain_ident_re();
    let scalar_lit_re_cached = scalar_lit_re();
    let functions = build_function_text_index(&lines, |_| None);
    let function_plans = functions
        .iter()
        .map(|function| {
            let mut plan = FunctionDeadTempPlan::default();
            for line in lines.iter().take(function.end + 1).skip(function.start) {
                let trimmed = line.trim();
                if trimmed.is_empty()
                    || trimmed == "# rr-cse-pruned"
                    || is_dead_plain_ident_eval_line(line)
                    || is_dead_parenthesized_eval_line(line)
                {
                    plan.has_trivial_removals = true;
                }
                if trimmed == "repeat {" {
                    plan.has_repeat_loop = true;
                }
                if trimmed.starts_with("if ") && trimmed.ends_with('{') {
                    plan.has_if_open = true;
                }
                let Some(caps) = assign_re_cached
                    .as_ref()
                    .and_then(|re| re.captures(trimmed))
                else {
                    continue;
                };
                let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
                let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
                if lhs.starts_with(".__rr_cse_")
                    || lhs.starts_with(".tachyon_callmap_arg")
                    || lhs.starts_with(".tachyon_exprmap")
                    || lhs.starts_with("i_")
                    || lhs.starts_with(".__rr_tmp_")
                    || lhs.starts_with("licm_")
                {
                    plan.has_temp_like_assign = true;
                }
                if lhs.starts_with(".__rr_cse_") {
                    plan.has_redundant_temp_candidates = true;
                }
                if plain_ident_re_cached
                    .as_ref()
                    .is_some_and(|re| re.is_match(lhs))
                    && scalar_lit_re_cached
                        .as_ref()
                        .is_some_and(|re| re.is_match(rhs))
                {
                    plan.has_branch_local_candidates = true;
                }
                if plain_ident_re_cached
                    .as_ref()
                    .is_some_and(|re| re.is_match(lhs))
                    && (is_dead_pure_expr_assignment_candidate(lhs, rhs, pure_user_calls)
                        || is_dead_pure_call_assignment_candidate(lhs, rhs, pure_user_calls))
                {
                    plan.has_dead_assign_candidates = true;
                }
            }
            plan
        })
        .collect::<Vec<_>>();

    for plan in &function_plans {
        has_trivial_removals |= plan.has_trivial_removals;
        has_temp_like_assign |= plan.has_temp_like_assign;
        has_dead_assign_candidates |= plan.has_dead_assign_candidates;
        has_branch_local_candidates |=
            plan.has_branch_local_candidates && plan.has_repeat_loop && plan.has_if_open;
        has_redundant_temp_candidates |= plan.has_redundant_temp_candidates;
    }

    let has_heavy_candidates = has_temp_like_assign
        || has_dead_assign_candidates
        || has_branch_local_candidates
        || has_redundant_temp_candidates;

    if !has_heavy_candidates {
        let compact_started = std::time::Instant::now();
        if !has_trivial_removals {
            let line_map = (1..=lines.len() as u32).collect::<Vec<_>>();
            profile.compact_elapsed_ns = compact_started.elapsed().as_nanos();
            return ((lines, line_map), profile);
        }
        let mut compacted = Vec::with_capacity(lines.len());
        let mut line_map = vec![0u32; lines.len()];
        let mut new_line = 0u32;
        for (idx, line) in lines.into_iter().enumerate() {
            let remove = line.trim().is_empty()
                || line.trim() == "# rr-cse-pruned"
                || is_dead_plain_ident_eval_line(&line)
                || is_dead_parenthesized_eval_line(&line);
            if remove {
                line_map[idx] = new_line.max(1);
                continue;
            }
            new_line += 1;
            line_map[idx] = new_line;
            compacted.push(line);
        }
        profile.compact_elapsed_ns = compact_started.elapsed().as_nanos();
        return ((compacted, line_map), profile);
    }

    let facts_started = std::time::Instant::now();
    let function_facts = build_dead_temp_function_facts(&lines, &functions, &function_plans);
    let line_to_fn = build_line_to_function(&function_facts, lines.len());
    profile.facts_elapsed_ns = facts_started.elapsed().as_nanos();

    let enclosing_loop_starts = if has_dead_assign_candidates {
        Some(build_enclosing_loop_starts(&lines))
    } else {
        None
    };
    let block_end_map = if has_branch_local_candidates {
        Some(build_block_end_map(&lines))
    } else {
        None
    };
    let enclosing_if_starts = if has_branch_local_candidates {
        Some(build_enclosing_if_starts(&lines))
    } else {
        None
    };
    let mark_started = std::time::Instant::now();
    let overwritten_dead = if has_dead_assign_candidates {
        mark_overwritten_dead_assignments_with_cache(
            &lines,
            pure_user_calls,
            &function_facts,
            &function_plans,
        )
    } else {
        vec![false; lines.len()]
    };
    let branch_local_dead = if has_branch_local_candidates {
        mark_branch_local_dead_inits_with_cache(
            &lines,
            &function_facts,
            &function_plans,
            block_end_map.as_deref().unwrap_or(&[]),
            enclosing_if_starts.as_deref().unwrap_or(&[]),
        )
    } else {
        vec![false; lines.len()]
    };
    let redundant_temp_reassign = if has_redundant_temp_candidates {
        mark_redundant_identical_temp_reassigns_with_cache(&lines, &function_facts, &function_plans)
    } else {
        vec![false; lines.len()]
    };
    profile.mark_elapsed_ns = mark_started.elapsed().as_nanos();

    let reverse_started = std::time::Instant::now();
    let mut out = lines;
    let mut removed = vec![false; out.len()];
    for idx in (0..out.len()).rev() {
        if line_to_fn.get(idx).and_then(|entry| *entry).is_some() {
            continue;
        }
        let line = out[idx].clone();
        if line.trim().is_empty()
            || line.trim() == "# rr-cse-pruned"
            || is_dead_plain_ident_eval_line(&line)
            || is_dead_parenthesized_eval_line(&line)
        {
            removed[idx] = true;
            out[idx] = String::new();
        }
    }

    for (fn_idx, function) in function_facts.iter().enumerate().rev() {
        let plan = function_plans[fn_idx];
        if !plan.has_heavy_candidates() {
            if plan.has_trivial_removals {
                for fact in &function.line_facts {
                    let idx = fact.line_idx;
                    let remove = {
                        let line = out[idx].trim();
                        line.is_empty()
                            || line == "# rr-cse-pruned"
                            || is_dead_plain_ident_eval_line(&out[idx])
                            || is_dead_parenthesized_eval_line(&out[idx])
                    };
                    if remove {
                        removed[idx] = true;
                        out[idx].clear();
                    }
                }
            }
            continue;
        }

        if !plan.has_temp_like_assign {
            for fact in function.line_facts.iter().rev() {
                let idx = fact.line_idx;
                let remove_trivial = {
                    let line = out[idx].trim();
                    line.is_empty()
                        || line == "# rr-cse-pruned"
                        || is_dead_plain_ident_eval_line(&out[idx])
                        || is_dead_parenthesized_eval_line(&out[idx])
                };
                if remove_trivial
                    || overwritten_dead[idx]
                    || branch_local_dead[idx]
                    || redundant_temp_reassign[idx]
                {
                    removed[idx] = true;
                    out[idx].clear();
                    continue;
                }

                let (Some(lhs), Some(rhs)) = (
                    fact.lhs.as_ref().map(String::as_str),
                    fact.rhs.as_ref().map(String::as_str),
                ) else {
                    continue;
                };
                let is_loop_carried_state_update = has_dead_assign_candidates
                    && enclosing_loop_starts.as_ref().is_some_and(|loop_starts| {
                        let Some(loop_start) = loop_starts[idx] else {
                            return false;
                        };
                        function.uses.get(lhs).is_some_and(|use_lines| {
                            use_lines
                                .iter()
                                .any(|&use_idx| use_idx > loop_start && use_idx < idx)
                        })
                    });
                let ever_read_in_function = function.uses.contains_key(lhs);
                let is_dead_simple_assign = has_dead_assign_candidates
                    && is_dead_pure_expr_assignment_candidate(lhs, rhs, pure_user_calls)
                    && !is_loop_carried_state_update
                    && !ever_read_in_function;
                if is_dead_simple_assign {
                    removed[idx] = true;
                    out[idx].clear();
                }
            }
            continue;
        }

        let mut live: FxHashSet<String> = FxHashSet::default();
        for fact in function.line_facts.iter().rev() {
            let idx = fact.line_idx;
            let remove_trivial = {
                let line = out[idx].trim();
                line.is_empty()
                    || line == "# rr-cse-pruned"
                    || is_dead_plain_ident_eval_line(&out[idx])
                    || is_dead_parenthesized_eval_line(&out[idx])
            };
            if remove_trivial {
                removed[idx] = true;
                out[idx].clear();
                continue;
            }
            if overwritten_dead[idx] || branch_local_dead[idx] || redundant_temp_reassign[idx] {
                removed[idx] = true;
                out[idx].clear();
                continue;
            }

            let (lhs, rhs, idents) = if let (Some(lhs), Some(rhs)) = (
                fact.lhs.as_ref().map(String::as_str),
                fact.rhs.as_ref().map(String::as_str),
            ) {
                (lhs, rhs, fact.idents.as_slice())
            } else {
                live.extend(fact.idents.iter().cloned());
                continue;
            };

            let is_self_referential_update = idents.iter().any(|ident| ident == lhs);
            let is_temp = lhs.starts_with(".__rr_cse_")
                || lhs.starts_with(".tachyon_callmap_arg")
                || lhs.starts_with(".tachyon_exprmap")
                || lhs.starts_with("i_")
                || lhs.starts_with(".__rr_tmp_");
            let is_dead_helper_local = lhs.starts_with("licm_");
            let is_loop_carried_state_update = has_dead_assign_candidates
                && enclosing_loop_starts.as_ref().is_some_and(|loop_starts| {
                    let Some(loop_start) = loop_starts[idx] else {
                        return false;
                    };
                    function.uses.get(lhs).is_some_and(|use_lines| {
                        use_lines
                            .iter()
                            .any(|&use_idx| use_idx > loop_start && use_idx < idx)
                    })
                });
            let ever_read_in_function = function.uses.contains_key(lhs);
            let is_dead_simple_assign = has_dead_assign_candidates
                && is_dead_pure_expr_assignment_candidate(lhs, rhs, pure_user_calls)
                && !is_loop_carried_state_update
                && !ever_read_in_function;
            if ((is_temp || is_dead_helper_local)
                && !live.contains(lhs)
                && !(lhs.starts_with("i_") && is_self_referential_update))
                || is_dead_simple_assign
            {
                removed[idx] = true;
                out[idx].clear();
                continue;
            }
            live.remove(lhs);
            for ident in idents {
                live.insert(ident.clone());
            }
        }
    }
    profile.reverse_elapsed_ns = reverse_started.elapsed().as_nanos();

    let compact_started = std::time::Instant::now();
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
    profile.compact_elapsed_ns = compact_started.elapsed().as_nanos();
    ((compacted, line_map), profile)
}

pub(super) fn strip_dead_temps_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut PeepholeAnalysisCache,
) -> (Vec<String>, Vec<u32>) {
    strip_dead_temps_with_cache_and_profile(lines, pure_user_calls, cache).0
}
