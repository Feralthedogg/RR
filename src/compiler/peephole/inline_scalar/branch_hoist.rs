use super::super::{FunctionFacts, FunctionLineFacts};
use super::{expr_is_inlineable_named_scalar_rhs, plain_ident_re};
use rustc_hash::FxHashSet;

pub(crate) fn is_named_scalar_hoist_lhs(lhs: &str) -> bool {
    plain_ident_re().is_some_and(|re| re.is_match(lhs))
        && !lhs.starts_with(".arg_")
        && !lhs.starts_with(".__rr_cse_")
}

pub(crate) fn is_named_scalar_hoist_assignment(
    fact: &FunctionLineFacts,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    let Some(lhs) = fact.lhs.as_deref() else {
        return false;
    };
    let Some(rhs) = fact.rhs.as_deref() else {
        return false;
    };
    is_named_scalar_hoist_lhs(lhs) && expr_is_inlineable_named_scalar_rhs(rhs, pure_user_calls)
}

pub(crate) fn has_later_use(facts: &FunctionFacts, lhs: &str, line_idx: usize) -> bool {
    facts
        .uses
        .get(lhs)
        .is_some_and(|uses| uses.iter().any(|use_idx| *use_idx > line_idx))
}

pub(crate) fn function_has_branch_hoist_candidate(
    facts: &FunctionFacts,
    pure_user_calls: &FxHashSet<String>,
) -> bool {
    facts.line_facts.iter().any(|fact| {
        let Some(lhs) = fact.lhs.as_deref() else {
            return false;
        };
        is_named_scalar_hoist_assignment(fact, pure_user_calls)
            && has_later_use(facts, lhs, fact.line_idx)
    })
}

pub(crate) fn line_starts_if_block(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("if ") && line.trim_end().ends_with('{')
}

pub(crate) fn has_if_block(lines: &[String]) -> bool {
    lines.iter().any(|line| line_starts_if_block(line))
}

pub(crate) fn branch_trailing_hoist_assignments(
    lines: &[String],
    facts: &FunctionFacts,
    branch_start: usize,
    branch_end: usize,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<usize> {
    let mut trailing = Vec::new();
    let mut scan = branch_end;
    while scan > branch_start + 1 {
        scan -= 1;
        if lines[scan].trim().is_empty() {
            continue;
        }
        let Some(scan_fact) = facts
            .line_facts
            .get(scan.saturating_sub(facts.function.start))
        else {
            break;
        };
        if !scan_fact.is_assign || !is_named_scalar_hoist_assignment(scan_fact, pure_user_calls) {
            break;
        }
        trailing.push(scan);
    }
    trailing.reverse();
    trailing
}

pub(crate) fn assignment_dep_written_in_branch(
    facts: &FunctionFacts,
    assign_fact: &FunctionLineFacts,
    branch_start: usize,
    assign_idx: usize,
) -> bool {
    assign_fact.idents.iter().any(|dep| {
        facts.defs.get(dep).is_some_and(|defs| {
            defs.iter()
                .copied()
                .any(|line_idx| line_idx > branch_start && line_idx < assign_idx)
        })
    })
}

pub(crate) fn assignment_used_after_branch(
    facts: &FunctionFacts,
    lhs: &str,
    assign_idx: usize,
    branch_end: usize,
) -> bool {
    let next_def = facts
        .defs
        .get(lhs)
        .and_then(|defs| defs.iter().copied().find(|line_idx| *line_idx > assign_idx))
        .unwrap_or(facts.function.end + 1);
    facts.uses.get(lhs).is_some_and(|uses| {
        uses.iter()
            .copied()
            .any(|line_idx| line_idx > branch_end && line_idx < next_def)
    })
}

pub(crate) fn assignment_can_hoist_from_branch(
    facts: &FunctionFacts,
    guard_idents: &[String],
    branch_start: usize,
    branch_end: usize,
    assign_idx: usize,
) -> bool {
    let Some(assign_fact) = facts
        .line_facts
        .get(assign_idx.saturating_sub(facts.function.start))
    else {
        return false;
    };
    let Some(lhs) = assign_fact.lhs.as_deref() else {
        return false;
    };
    if guard_idents.iter().any(|ident| ident == lhs) {
        return false;
    }
    !assignment_dep_written_in_branch(facts, assign_fact, branch_start, assign_idx)
        && assignment_used_after_branch(facts, lhs, assign_idx, branch_end)
}

pub(crate) fn collect_branch_hoists(
    lines: &mut [String],
    facts: &FunctionFacts,
    guard_idents: &[String],
    branch_start: usize,
    branch_end: usize,
    trailing_assignments: Vec<usize>,
) -> Vec<String> {
    let mut hoisted = Vec::new();
    for assign_idx in trailing_assignments {
        if assignment_can_hoist_from_branch(
            facts,
            guard_idents,
            branch_start,
            branch_end,
            assign_idx,
        ) {
            hoisted.push(lines[assign_idx].clone());
            lines[assign_idx].clear();
        }
    }
    hoisted
}

pub(crate) fn insert_hoisted_branch_assignments(
    lines: &mut Vec<String>,
    branch_start: usize,
    hoisted: Vec<String>,
) {
    for (offset, line) in hoisted.into_iter().enumerate() {
        lines.insert(branch_start + offset, line);
    }
}
