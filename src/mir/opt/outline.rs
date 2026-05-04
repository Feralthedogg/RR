use super::*;

#[path = "outline/analysis.rs"]
mod analysis;
#[path = "outline/extract.rs"]
mod extract;
#[path = "outline/policy.rs"]
mod policy;
#[path = "outline/rewrite.rs"]
mod rewrite;

use analysis::OutlineCandidate;
use policy::OutlinePolicy;

pub(crate) fn optimize_program(
    all_fns: &mut FxHashMap<String, FnIR>,
    engine: &TachyonEngine,
    stats: &mut TachyonPulseStats,
) -> usize {
    let policy = OutlinePolicy::for_engine(engine);
    if !policy.enabled {
        return 0;
    }

    let mut applied = 0usize;
    let mut names: Vec<_> = all_fns.keys().cloned().collect();
    names.sort();

    for name in names {
        let Some(parent) = all_fns.get(&name) else {
            continue;
        };
        let Some(candidate) = OutlineCandidate::find(parent, &policy) else {
            continue;
        };
        stats.outline_candidates += 1;
        if !candidate.is_profitable(parent, &policy) {
            stats.outline_skipped += 1;
            continue;
        }

        let helper_name = deterministic_helper_name(all_fns, &name);
        let Some(parent) = all_fns.get_mut(&name) else {
            stats.outline_skipped += 1;
            continue;
        };
        let Some(helper) = extract::extract_helper(parent, &candidate, helper_name) else {
            stats.outline_skipped += 1;
            continue;
        };
        let helper_name = helper.name.clone();
        all_fns.insert(helper_name, helper);
        stats.outline_applied += 1;
        applied += 1;
    }

    applied
}

fn deterministic_helper_name(all_fns: &FxHashMap<String, FnIR>, parent: &str) -> String {
    let base = sanitize_internal_name(parent);
    let mut index = 0usize;
    loop {
        let name = format!("__rr_outline_{base}_{index}");
        if !all_fns.contains_key(&name) {
            return name;
        }
        index += 1;
    }
}

fn sanitize_internal_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "fn".to_string()
    } else {
        out
    }
}

#[cfg(test)]
#[path = "outline/tests.rs"]
mod tests;
