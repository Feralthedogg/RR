use super::{
    FunctionFacts, FunctionLineFacts, PeepholeAnalysisCache, assign_re, build_function_text_index,
    count_unquoted_braces, expr_has_only_pure_calls, expr_idents, find_matching_block_end,
    is_control_flow_boundary, is_dead_parenthesized_eval_line, is_dead_plain_ident_eval_line,
    is_loop_open_boundary, plain_ident_re, scalar_lit_re,
};
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FunctionDeadTempPlan {
    pub(crate) has_trivial_removals: bool,
    pub(crate) has_temp_like_assign: bool,
    pub(crate) has_dead_assign_candidates: bool,
    pub(crate) has_branch_local_candidates: bool,
    pub(crate) has_repeat_loop: bool,
    pub(crate) has_if_open: bool,
    pub(crate) has_redundant_temp_candidates: bool,
}

impl FunctionDeadTempPlan {
    pub(crate) fn has_heavy_candidates(self) -> bool {
        self.has_temp_like_assign
            || self.has_dead_assign_candidates
            || (self.has_branch_local_candidates && self.has_repeat_loop && self.has_if_open)
            || self.has_redundant_temp_candidates
    }
}

#[derive(Debug)]
pub(crate) struct DeadTempScanSummary {
    pub(crate) functions: Vec<super::IndexedFunction>,
    pub(crate) function_plans: Vec<FunctionDeadTempPlan>,
    pub(crate) has_trivial_removals: bool,
    pub(crate) has_temp_like_assign: bool,
    pub(crate) has_dead_assign_candidates: bool,
    pub(crate) has_branch_local_candidates: bool,
    pub(crate) has_redundant_temp_candidates: bool,
}

impl DeadTempScanSummary {
    pub(crate) fn has_heavy_candidates(&self) -> bool {
        self.has_temp_like_assign
            || self.has_dead_assign_candidates
            || self.has_branch_local_candidates
            || self.has_redundant_temp_candidates
    }
}

pub(crate) struct DeadTempMarks {
    pub(crate) overwritten_dead: Vec<bool>,
    pub(crate) branch_local_dead: Vec<bool>,
    pub(crate) redundant_temp_reassign: Vec<bool>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct DeadTempProfile {
    pub(crate) facts_elapsed_ns: u128,
    pub(crate) mark_elapsed_ns: u128,
    pub(crate) reverse_elapsed_ns: u128,
    pub(crate) compact_elapsed_ns: u128,
}

pub(crate) fn is_trivial_dead_temp_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty()
        || trimmed == "# rr-cse-pruned"
        || is_dead_plain_ident_eval_line(line)
        || is_dead_parenthesized_eval_line(line)
}

pub(crate) fn is_dead_pure_expr_assignment_candidate(
    lhs: &str,
    rhs: &str,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let lhs = lhs.trim();
    plain_ident_re().is_some_and(|re| re.is_match(lhs))
        && expr_has_only_pure_calls(rhs, pure_user_calls)
}

pub(crate) fn is_dead_pure_call_assignment_candidate(
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

pub(crate) fn scan_dead_temp_candidates(
    lines: &[String],
    pure_user_calls: &FxHashSet<String>,
) -> DeadTempScanSummary {
    let assign_re_cached = assign_re();
    let plain_ident_re_cached = plain_ident_re();
    let scalar_lit_re_cached = scalar_lit_re();
    let functions = build_function_text_index(lines, |_| None);
    let function_plans = functions
        .iter()
        .map(|function| {
            scan_dead_temp_function_plan(
                lines,
                function.start,
                function.end,
                pure_user_calls,
                assign_re_cached,
                plain_ident_re_cached,
                scalar_lit_re_cached,
            )
        })
        .collect::<Vec<_>>();

    let mut summary = DeadTempScanSummary {
        functions,
        function_plans,
        has_trivial_removals: false,
        has_temp_like_assign: false,
        has_dead_assign_candidates: false,
        has_branch_local_candidates: false,
        has_redundant_temp_candidates: false,
    };
    for plan in &summary.function_plans {
        summary.has_trivial_removals |= plan.has_trivial_removals;
        summary.has_temp_like_assign |= plan.has_temp_like_assign;
        summary.has_dead_assign_candidates |= plan.has_dead_assign_candidates;
        summary.has_branch_local_candidates |=
            plan.has_branch_local_candidates && plan.has_repeat_loop && plan.has_if_open;
        summary.has_redundant_temp_candidates |= plan.has_redundant_temp_candidates;
    }
    summary
}

pub(crate) fn scan_dead_temp_function_plan(
    lines: &[String],
    start: usize,
    end: usize,
    pure_user_calls: &FxHashSet<String>,
    assign_re_cached: Option<&regex::Regex>,
    plain_ident_re_cached: Option<&regex::Regex>,
    scalar_lit_re_cached: Option<&regex::Regex>,
) -> FunctionDeadTempPlan {
    let mut plan = FunctionDeadTempPlan::default();
    for line in lines.iter().take(end + 1).skip(start) {
        let trimmed = line.trim();
        if is_trivial_dead_temp_line(line) {
            plan.has_trivial_removals = true;
        }
        if trimmed == "repeat {" {
            plan.has_repeat_loop = true;
        }
        if trimmed.starts_with("if ") && trimmed.ends_with('{') {
            plan.has_if_open = true;
        }
        let Some(caps) = assign_re_cached.and_then(|re| re.captures(trimmed)) else {
            continue;
        };
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        if is_temp_like_dead_assignment(lhs) {
            plan.has_temp_like_assign = true;
        }
        if lhs.starts_with(".__rr_cse_") {
            plan.has_redundant_temp_candidates = true;
        }
        if plain_ident_re_cached.is_some_and(|re| re.is_match(lhs))
            && scalar_lit_re_cached.is_some_and(|re| re.is_match(rhs))
        {
            plan.has_branch_local_candidates = true;
        }
        if plain_ident_re_cached.is_some_and(|re| re.is_match(lhs))
            && (is_dead_pure_expr_assignment_candidate(lhs, rhs, pure_user_calls)
                || is_dead_pure_call_assignment_candidate(lhs, rhs, pure_user_calls))
        {
            plan.has_dead_assign_candidates = true;
        }
    }
    plan
}

pub(crate) fn is_temp_like_dead_assignment(lhs: &str) -> bool {
    lhs.starts_with(".__rr_cse_")
        || lhs.starts_with(".tachyon_callmap_arg")
        || lhs.starts_with(".tachyon_exprmap")
        || lhs.starts_with("i_")
        || lhs.starts_with(".__rr_tmp_")
        || lhs.starts_with("licm_")
}

pub(crate) fn build_line_to_function(
    facts: &[FunctionFacts],
    total_lines: usize,
) -> Vec<Option<usize>> {
    let mut line_to_fn = vec![None; total_lines];
    for (fn_idx, function_facts) in facts.iter().enumerate() {
        for slot in line_to_fn
            .iter_mut()
            .take(function_facts.function.end.saturating_add(1))
            .skip(function_facts.function.start)
        {
            *slot = Some(fn_idx);
        }
    }
    line_to_fn
}

pub(crate) fn build_dead_temp_function_facts(
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
            for (line_idx, line) in lines
                .iter()
                .enumerate()
                .take(function.end.saturating_add(1))
                .skip(function.start)
            {
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

pub(crate) fn line_fact_for_idx<'a>(
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

pub(crate) fn loop_body_references_var_before(lines: &[String], idx: usize, var: &str) -> bool {
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

pub(crate) fn build_enclosing_loop_starts(lines: &[String]) -> Vec<Option<usize>> {
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

pub(crate) fn build_enclosing_if_starts(lines: &[String]) -> Vec<Option<usize>> {
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

pub(crate) fn build_block_end_map(lines: &[String]) -> Vec<Option<usize>> {
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

pub(crate) fn loop_body_references_var_before_cached(
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

pub(crate) fn mark_redundant_identical_temp_reassigns_with_cache(
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
            let Some(lhs) = fact.lhs.as_deref() else {
                continue;
            };
            let Some(rhs) = fact.rhs.as_deref() else {
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
                let Some(prev_lhs) = prev_fact.lhs.as_deref() else {
                    continue;
                };
                let Some(prev_rhs) = prev_fact.rhs.as_deref() else {
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

pub(crate) fn mark_overwritten_dead_assignments_with_cache(
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

            let lhs = fact.lhs.as_deref().unwrap_or("");
            let rhs = fact.rhs.as_deref().unwrap_or("");
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

pub(crate) fn mark_branch_local_dead_inits_with_cache(
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
            let lhs = fact.lhs.as_deref().unwrap_or("");
            let rhs = fact.rhs.as_deref().unwrap_or("");
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
                let assigned_lhs = loop_fact.lhs.as_deref();
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
                let assigned_lhs = scan_fact.lhs.as_deref();
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
                    let assigned_lhs = scan_fact.lhs.as_deref();
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

pub(crate) fn finish_without_heavy_dead_temp_analysis(
    lines: Vec<String>,
    has_trivial_removals: bool,
    mut profile: DeadTempProfile,
) -> ((Vec<String>, Vec<u32>), DeadTempProfile) {
    let compact_started = std::time::Instant::now();
    if !has_trivial_removals {
        let line_map = (1..=lines.len() as u32).collect::<Vec<_>>();
        profile.compact_elapsed_ns = compact_started.elapsed().as_nanos();
        return ((lines, line_map), profile);
    }

    let result =
        compact_unmarked_dead_temp_lines(lines, |line, _idx| is_trivial_dead_temp_line(line));
    profile.compact_elapsed_ns = compact_started.elapsed().as_nanos();
    (result, profile)
}

pub(crate) fn build_dead_temp_marks(
    lines: &[String],
    pure_user_calls: &FxHashSet<String>,
    function_facts: &[FunctionFacts],
    function_plans: &[FunctionDeadTempPlan],
    scan: &DeadTempScanSummary,
    block_end_map: Option<&[Option<usize>]>,
    enclosing_if_starts: Option<&[Option<usize>]>,
) -> DeadTempMarks {
    let overwritten_dead = if scan.has_dead_assign_candidates {
        mark_overwritten_dead_assignments_with_cache(
            lines,
            pure_user_calls,
            function_facts,
            function_plans,
        )
    } else {
        vec![false; lines.len()]
    };
    let branch_local_dead = if scan.has_branch_local_candidates {
        mark_branch_local_dead_inits_with_cache(
            lines,
            function_facts,
            function_plans,
            block_end_map.unwrap_or(&[]),
            enclosing_if_starts.unwrap_or(&[]),
        )
    } else {
        vec![false; lines.len()]
    };
    let redundant_temp_reassign = if scan.has_redundant_temp_candidates {
        mark_redundant_identical_temp_reassigns_with_cache(lines, function_facts, function_plans)
    } else {
        vec![false; lines.len()]
    };
    DeadTempMarks {
        overwritten_dead,
        branch_local_dead,
        redundant_temp_reassign,
    }
}

pub(crate) struct DeadTempReverseContext<'a> {
    pub(crate) function_plans: &'a [FunctionDeadTempPlan],
    pub(crate) function_facts: &'a [FunctionFacts],
    pub(crate) line_to_fn: &'a [Option<usize>],
    pub(crate) marks: &'a DeadTempMarks,
    pub(crate) enclosing_loop_starts: Option<&'a [Option<usize>]>,
    pub(crate) pure_user_calls: &'a FxHashSet<String>,
    pub(crate) has_dead_assign_candidates: bool,
}

pub(crate) fn remove_dead_temp_lines(
    mut out: Vec<String>,
    ctx: DeadTempReverseContext<'_>,
) -> (Vec<String>, Vec<bool>) {
    let mut removed = vec![false; out.len()];
    remove_trivial_lines_outside_functions(&mut out, &mut removed, ctx.line_to_fn);

    for (fn_idx, function) in ctx.function_facts.iter().enumerate().rev() {
        let plan = ctx.function_plans[fn_idx];
        if !plan.has_heavy_candidates() {
            remove_trivial_function_lines(function, &mut out, &mut removed, plan);
            continue;
        }

        if !plan.has_temp_like_assign {
            remove_dead_lines_without_temp_liveness(function, &mut out, &mut removed, &ctx);
            continue;
        }

        remove_dead_lines_with_temp_liveness(function, &mut out, &mut removed, &ctx);
    }
    (out, removed)
}

pub(crate) fn remove_trivial_lines_outside_functions(
    out: &mut [String],
    removed: &mut [bool],
    line_to_fn: &[Option<usize>],
) {
    for idx in (0..out.len()).rev() {
        if line_to_fn.get(idx).and_then(|entry| *entry).is_some() {
            continue;
        }
        if is_trivial_dead_temp_line(&out[idx]) {
            clear_removed_dead_temp_line(out, removed, idx);
        }
    }
}

pub(crate) fn remove_trivial_function_lines(
    function: &FunctionFacts,
    out: &mut [String],
    removed: &mut [bool],
    plan: FunctionDeadTempPlan,
) {
    if !plan.has_trivial_removals {
        return;
    }
    for fact in &function.line_facts {
        let idx = fact.line_idx;
        if is_trivial_dead_temp_line(&out[idx]) {
            clear_removed_dead_temp_line(out, removed, idx);
        }
    }
}

pub(crate) fn remove_dead_lines_without_temp_liveness(
    function: &FunctionFacts,
    out: &mut [String],
    removed: &mut [bool],
    ctx: &DeadTempReverseContext<'_>,
) {
    for fact in function.line_facts.iter().rev() {
        let idx = fact.line_idx;
        if is_trivial_dead_temp_line(&out[idx]) || is_marked_dead_temp_line(ctx.marks, idx) {
            clear_removed_dead_temp_line(out, removed, idx);
            continue;
        }

        let (Some(lhs), Some(rhs)) = (fact.lhs.as_deref(), fact.rhs.as_deref()) else {
            continue;
        };
        if is_dead_simple_assignment(function, idx, lhs, rhs, ctx) {
            clear_removed_dead_temp_line(out, removed, idx);
        }
    }
}

pub(crate) fn remove_dead_lines_with_temp_liveness(
    function: &FunctionFacts,
    out: &mut [String],
    removed: &mut [bool],
    ctx: &DeadTempReverseContext<'_>,
) {
    let mut live: FxHashSet<String> = FxHashSet::default();
    for fact in function.line_facts.iter().rev() {
        let idx = fact.line_idx;
        if is_trivial_dead_temp_line(&out[idx]) || is_marked_dead_temp_line(ctx.marks, idx) {
            clear_removed_dead_temp_line(out, removed, idx);
            continue;
        }

        let (lhs, rhs, idents) =
            if let (Some(lhs), Some(rhs)) = (fact.lhs.as_deref(), fact.rhs.as_deref()) {
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
        if ((is_temp || is_dead_helper_local)
            && !live.contains(lhs)
            && !(lhs.starts_with("i_") && is_self_referential_update))
            || is_dead_simple_assignment(function, idx, lhs, rhs, ctx)
        {
            clear_removed_dead_temp_line(out, removed, idx);
            continue;
        }
        live.remove(lhs);
        for ident in idents {
            live.insert(ident.clone());
        }
    }
}

pub(crate) fn is_marked_dead_temp_line(marks: &DeadTempMarks, idx: usize) -> bool {
    marks.overwritten_dead[idx]
        || marks.branch_local_dead[idx]
        || marks.redundant_temp_reassign[idx]
}

pub(crate) fn is_dead_simple_assignment(
    function: &FunctionFacts,
    idx: usize,
    lhs: &str,
    rhs: &str,
    ctx: &DeadTempReverseContext<'_>,
) -> bool {
    ctx.has_dead_assign_candidates
        && is_dead_pure_expr_assignment_candidate(lhs, rhs, ctx.pure_user_calls)
        && !is_loop_carried_state_update(function, idx, lhs, ctx.enclosing_loop_starts)
        && !function.uses.contains_key(lhs)
}

pub(crate) fn is_loop_carried_state_update(
    function: &FunctionFacts,
    idx: usize,
    lhs: &str,
    enclosing_loop_starts: Option<&[Option<usize>]>,
) -> bool {
    enclosing_loop_starts.is_some_and(|loop_starts| {
        let Some(loop_start) = loop_starts[idx] else {
            return false;
        };
        function.uses.get(lhs).is_some_and(|use_lines| {
            use_lines
                .iter()
                .any(|&use_idx| use_idx > loop_start && use_idx < idx)
        })
    })
}

pub(crate) fn clear_removed_dead_temp_line(out: &mut [String], removed: &mut [bool], idx: usize) {
    removed[idx] = true;
    out[idx].clear();
}

pub(crate) fn compact_removed_dead_temp_lines(
    lines: Vec<String>,
    removed: &[bool],
) -> (Vec<String>, Vec<u32>) {
    compact_unmarked_dead_temp_lines(lines, |_line, idx| removed[idx])
}

pub(crate) fn compact_unmarked_dead_temp_lines(
    lines: Vec<String>,
    should_remove: impl Fn(&str, usize) -> bool,
) -> (Vec<String>, Vec<u32>) {
    let mut compacted = Vec::with_capacity(lines.len());
    let mut line_map = vec![0u32; lines.len()];
    let mut new_line = 0u32;
    for (idx, line) in lines.into_iter().enumerate() {
        if should_remove(&line, idx) {
            line_map[idx] = new_line.max(1);
            continue;
        }
        new_line += 1;
        line_map[idx] = new_line;
        compacted.push(line);
    }
    (compacted, line_map)
}

pub(crate) fn strip_dead_temps(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, Vec<u32>) {
    let mut cache = PeepholeAnalysisCache::default();
    strip_dead_temps_with_cache(lines, pure_user_calls, &mut cache)
}

pub(crate) fn strip_dead_temps_with_cache_and_profile(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    _cache: &mut PeepholeAnalysisCache,
) -> ((Vec<String>, Vec<u32>), DeadTempProfile) {
    let mut profile = DeadTempProfile::default();
    let scan = scan_dead_temp_candidates(&lines, pure_user_calls);

    if !scan.has_heavy_candidates() {
        return finish_without_heavy_dead_temp_analysis(lines, scan.has_trivial_removals, profile);
    }

    let facts_started = std::time::Instant::now();
    let function_facts =
        build_dead_temp_function_facts(&lines, &scan.functions, &scan.function_plans);
    let line_to_fn = build_line_to_function(&function_facts, lines.len());
    profile.facts_elapsed_ns = facts_started.elapsed().as_nanos();

    let enclosing_loop_starts = if scan.has_dead_assign_candidates {
        Some(build_enclosing_loop_starts(&lines))
    } else {
        None
    };
    let block_end_map = if scan.has_branch_local_candidates {
        Some(build_block_end_map(&lines))
    } else {
        None
    };
    let enclosing_if_starts = if scan.has_branch_local_candidates {
        Some(build_enclosing_if_starts(&lines))
    } else {
        None
    };
    let mark_started = std::time::Instant::now();
    let marks = build_dead_temp_marks(
        &lines,
        pure_user_calls,
        &function_facts,
        &scan.function_plans,
        &scan,
        block_end_map.as_deref(),
        enclosing_if_starts.as_deref(),
    );
    profile.mark_elapsed_ns = mark_started.elapsed().as_nanos();

    let reverse_started = std::time::Instant::now();
    let (out, removed) = remove_dead_temp_lines(
        lines,
        DeadTempReverseContext {
            function_plans: &scan.function_plans,
            function_facts: &function_facts,
            line_to_fn: &line_to_fn,
            marks: &marks,
            enclosing_loop_starts: enclosing_loop_starts.as_deref(),
            pure_user_calls,
            has_dead_assign_candidates: scan.has_dead_assign_candidates,
        },
    );
    profile.reverse_elapsed_ns = reverse_started.elapsed().as_nanos();

    let compact_started = std::time::Instant::now();
    let result = compact_removed_dead_temp_lines(out, &removed);
    profile.compact_elapsed_ns = compact_started.elapsed().as_nanos();
    (result, profile)
}

pub(crate) fn strip_dead_temps_with_cache(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    cache: &mut PeepholeAnalysisCache,
) -> (Vec<String>, Vec<u32>) {
    strip_dead_temps_with_cache_and_profile(lines, pure_user_calls, cache).0
}
