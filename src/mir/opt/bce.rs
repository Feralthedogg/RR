use crate::mir::analyze::na::{self, NaState};
use crate::mir::analyze::range::{RangeInterval, SymbolicBound};
use crate::mir::analyze::range::{analyze_ranges, ensure_value_range, transfer_instr};
use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::mir::*;
use rustc_hash::FxHashSet;
use std::env;

#[derive(Clone)]
struct CanonicalIvRule {
    body: FxHashSet<BlockId>,
    iv: ValueId,
    // iv <= len(base) + limit_off
    limit: Option<(ValueId, i64)>,
}

#[derive(Clone)]
struct OneBasedIvRule {
    body: FxHashSet<BlockId>,
    iv: ValueId,
}

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;
    let bb_facts = analyze_ranges(fn_ir);
    let na_states = na::compute_na_states(fn_ir);
    let canonical_ivs = canonical_loop_ivs(fn_ir);
    let one_based_ivs = one_based_loop_ivs(fn_ir);
    let mut node_visits = 0usize;
    let visit_limit = bce_visit_limit();
    let mut one_based_indices = FxHashSet::default();

    // Pass 1: Handle StoreIndex1D instructions
    for (bid, facts_at_bid) in bb_facts.iter().enumerate().take(fn_ir.blocks.len()) {
        let mut cur_facts = facts_at_bid.clone();
        let num_instrs = fn_ir.blocks[bid].instrs.len();

        for i in 0..num_instrs {
            let (in_bounds, non_na) = {
                let instr = &fn_ir.blocks[bid].instrs[i];
                if let Instr::StoreIndex1D { base, idx, .. } = instr {
                    ensure_value_range(*idx, &fn_ir.values, &mut cur_facts);
                    let iv_proven = iv_exact_in_block(bid, *idx, &canonical_ivs, fn_ir);
                    let idx_intv = cur_facts.get(*idx);
                    let in_bounds = interval_proves_in_bounds(fn_ir, &idx_intv, *base) || iv_proven;
                    if iv_non_na_in_block(bid, *idx, &canonical_ivs, &one_based_ivs, fn_ir) {
                        one_based_indices.insert(*idx);
                    }
                    // If bounds are proven in 1-based domain, index cannot be NA.
                    let non_na =
                        in_bounds || matches!(na_states[*idx], NaState::Never) || iv_proven;
                    (in_bounds, non_na)
                } else {
                    (false, false)
                }
            };

            if let Instr::StoreIndex1D {
                ref mut is_safe,
                ref mut is_na_safe,
                ..
            } = fn_ir.blocks[bid].instrs[i]
            {
                if in_bounds && !*is_safe {
                    *is_safe = true;
                    changed = true;
                }
                if non_na && !*is_na_safe {
                    *is_na_safe = true;
                    changed = true;
                }
            }

            // Re-borrow for transfer
            let values = &fn_ir.values;
            let instr = &fn_ir.blocks[bid].instrs[i];
            transfer_instr(instr, values, &mut cur_facts);
        }
    }

    // Pass 2: Handle Index1D loads, including nested loads inside expression trees.
    let mut safe_values = FxHashSet::default();
    let mut non_na_values = FxHashSet::default();
    for (bid, facts_at_bid) in bb_facts.iter().enumerate().take(fn_ir.blocks.len()) {
        let mut cur_facts = facts_at_bid.clone();
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    let mut seen = FxHashSet::default();
                    collect_index_safety(
                        *src,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &one_based_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut one_based_indices,
                        &mut seen,
                        &mut node_visits,
                        visit_limit,
                    );
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    let mut seen = FxHashSet::default();
                    collect_index_safety(
                        *base,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &one_based_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut one_based_indices,
                        &mut seen,
                        &mut node_visits,
                        visit_limit,
                    );
                    collect_index_safety(
                        *idx,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &one_based_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut one_based_indices,
                        &mut seen,
                        &mut node_visits,
                        visit_limit,
                    );
                    collect_index_safety(
                        *val,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &one_based_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut one_based_indices,
                        &mut seen,
                        &mut node_visits,
                        visit_limit,
                    );
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    let mut seen = FxHashSet::default();
                    collect_index_safety(
                        *base,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &one_based_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut one_based_indices,
                        &mut seen,
                        &mut node_visits,
                        visit_limit,
                    );
                    collect_index_safety(
                        *r,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &one_based_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut one_based_indices,
                        &mut seen,
                        &mut node_visits,
                        visit_limit,
                    );
                    collect_index_safety(
                        *c,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &one_based_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut one_based_indices,
                        &mut seen,
                        &mut node_visits,
                        visit_limit,
                    );
                    collect_index_safety(
                        *val,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &one_based_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut one_based_indices,
                        &mut seen,
                        &mut node_visits,
                        visit_limit,
                    );
                }
            }
            transfer_instr(instr, &fn_ir.values, &mut cur_facts);
        }
    }

    for vid in one_based_indices {
        if let Some(v) = fn_ir.values.get_mut(vid) {
            let old = v.facts.flags;
            v.facts
                .add(Facts::ONE_BASED | Facts::INT_SCALAR | Facts::NON_NA);
            if v.facts.flags != old {
                changed = true;
            }
        }
    }

    for vid in safe_values {
        if let ValueKind::Index1D {
            ref mut is_safe, ..
        } = fn_ir.values[vid].kind
            && !*is_safe
        {
            *is_safe = true;
            changed = true;
        }
    }

    for vid in non_na_values {
        if let ValueKind::Index1D {
            ref mut is_na_safe, ..
        } = fn_ir.values[vid].kind
            && !*is_na_safe
        {
            *is_na_safe = true;
            changed = true;
        }
    }

    changed
}

fn bce_visit_limit() -> usize {
    env::var("RR_BCE_VISIT_LIMIT")
        .ok()
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(200_000)
        .max(10_000)
}

fn canonical_loop_ivs(fn_ir: &FnIR) -> Vec<CanonicalIvRule> {
    let mut out = Vec::new();
    for lp in LoopAnalyzer::new(fn_ir).find_loops() {
        let iv = match lp.iv {
            Some(iv) => iv,
            None => continue,
        };
        let init_is_one = const_int(fn_ir, iv.init_val) == Some(1);
        let canonical =
            init_is_one && iv.step == 1 && iv.step_op == BinOp::Add && lp.is_seq_len.is_some();
        if canonical {
            let limit = lp.limit.and_then(|v| extract_len_limit(fn_ir, v));
            out.push(CanonicalIvRule {
                body: lp.body,
                iv: iv.phi_val,
                limit,
            });
        }
    }
    out
}

fn one_based_loop_ivs(fn_ir: &FnIR) -> Vec<OneBasedIvRule> {
    let mut out = Vec::new();
    for lp in LoopAnalyzer::new(fn_ir).find_loops() {
        let iv = match lp.iv {
            Some(iv) => iv,
            None => continue,
        };
        let init_is_one = const_int(fn_ir, iv.init_val) == Some(1);
        let one_based = init_is_one && iv.step == 1 && iv.step_op == BinOp::Add;
        if one_based {
            out.push(OneBasedIvRule {
                body: lp.body,
                iv: iv.phi_val,
            });
        }
    }
    out
}

fn extract_len_limit(fn_ir: &FnIR, limit_val: ValueId) -> Option<(ValueId, i64)> {
    let mut seen_vals = FxHashSet::default();
    let mut seen_vars = FxHashSet::default();
    extract_len_limit_rec(fn_ir, limit_val, &mut seen_vals, &mut seen_vars)
}

fn extract_len_limit_rec(
    fn_ir: &FnIR,
    vid: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> Option<(ValueId, i64)> {
    if !seen_vals.insert(vid) {
        return None;
    }
    let out = match &fn_ir.values[vid].kind {
        ValueKind::Len { base } => Some((*base, 0)),
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            let k = const_int(fn_ir, *rhs)?;
            let (base, off) = extract_len_limit_rec(fn_ir, *lhs, seen_vals, seen_vars)?;
            let off = off.checked_sub(k)?;
            Some((base, off))
        }
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            if let Some(k) = const_int(fn_ir, *rhs)
                && let Some((base, off)) = extract_len_limit_rec(fn_ir, *lhs, seen_vals, seen_vars)
            {
                let off = off.checked_add(k)?;
                return Some((base, off));
            }
            if let Some(k) = const_int(fn_ir, *lhs)
                && let Some((base, off)) = extract_len_limit_rec(fn_ir, *rhs, seen_vals, seen_vars)
            {
                let off = off.checked_add(k)?;
                return Some((base, off));
            }
            None
        }
        ValueKind::Load { var } => extract_len_limit_from_var(fn_ir, var, seen_vals, seen_vars),
        ValueKind::Phi { args } if !args.is_empty() => {
            let mut unique = None;
            for (arg, _) in args {
                let candidate = extract_len_limit_rec(fn_ir, *arg, seen_vals, seen_vars)?;
                match unique {
                    None => unique = Some(candidate),
                    Some(prev) if prev == candidate => {}
                    Some(_) => return None,
                }
            }
            unique
        }
        _ => None,
    };
    seen_vals.remove(&vid);
    out
}

fn extract_len_limit_from_var(
    fn_ir: &FnIR,
    var: &str,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> Option<(ValueId, i64)> {
    if !seen_vars.insert(var.to_string()) {
        return None;
    }
    let mut unique = None;
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let candidate = extract_len_limit_rec(fn_ir, *src, seen_vals, seen_vars)?;
            match unique {
                None => unique = Some(candidate),
                Some(prev) if prev == candidate => {}
                Some(_) => {
                    seen_vars.remove(var);
                    return None;
                }
            }
        }
    }
    seen_vars.remove(var);
    unique
}

fn const_int(fn_ir: &FnIR, vid: ValueId) -> Option<i64> {
    match &fn_ir.values[vid].kind {
        ValueKind::Const(Lit::Int(n)) => Some(*n),
        ValueKind::Const(Lit::Float(f))
            if f.is_finite()
                && (*f - f.trunc()).abs() < f64::EPSILON
                && *f >= i64::MIN as f64
                && *f <= i64::MAX as f64 =>
        {
            Some(*f as i64)
        }
        _ => None,
    }
}

fn iv_offset_for_idx(fn_ir: &FnIR, idx: ValueId, iv: ValueId) -> Option<i64> {
    if is_iv_equivalent(fn_ir, idx, iv) {
        return Some(0);
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv) {
                return const_int(fn_ir, *rhs);
            }
            if is_iv_equivalent(fn_ir, *rhs, iv) {
                return const_int(fn_ir, *lhs);
            }
            None
        }
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv) {
                return const_int(fn_ir, *rhs).map(|k| -k);
            }
            None
        }
        _ => None,
    }
}

fn is_iv_equivalent(fn_ir: &FnIR, candidate: ValueId, iv: ValueId) -> bool {
    let mut seen = vec![false; fn_ir.values.len()];
    let mut seen_vars = FxHashSet::default();
    is_iv_equivalent_rec(fn_ir, candidate, iv, &mut seen, &mut seen_vars)
}

fn is_iv_equivalent_rec(
    fn_ir: &FnIR,
    candidate: ValueId,
    iv: ValueId,
    seen: &mut [bool],
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    if candidate >= fn_ir.values.len() {
        return false;
    }
    if candidate == iv {
        return true;
    }
    if seen[candidate] {
        return false;
    }
    seen[candidate] = true;
    match &fn_ir.values[candidate].kind {
        ValueKind::Load { var } => {
            if fn_ir.values[iv].origin_var.as_deref() == Some(var.as_str()) {
                return true;
            }
            load_var_is_iv_equivalent(fn_ir, var, iv, seen, seen_vars)
        }
        ValueKind::Phi { args } if args.is_empty() => {
            match (
                fn_ir.values[candidate].origin_var.as_deref(),
                fn_ir.values[iv].origin_var.as_deref(),
            ) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        ValueKind::Phi { args } => args
            .iter()
            .all(|(v, _)| is_iv_equivalent_rec(fn_ir, *v, iv, seen, seen_vars)),
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let floor_like = matches!(callee.as_str(), "floor" | "ceiling" | "trunc");
            let single_positional = args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_equivalent_rec(fn_ir, args[0], iv, seen, seen_vars)
        }
        _ => false,
    }
}

fn load_var_is_iv_equivalent(
    fn_ir: &FnIR,
    var: &str,
    iv: ValueId,
    seen_vals: &mut [bool],
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    if !seen_vars.insert(var.to_string()) {
        return false;
    }

    let mut found = false;
    let mut all_match = true;
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            found = true;
            if !is_iv_equivalent_rec(fn_ir, *src, iv, seen_vals, seen_vars) {
                all_match = false;
                break;
            }
        }
        if !all_match {
            break;
        }
    }

    seen_vars.remove(var);
    found && all_match
}

fn iv_non_na_in_block(
    bid: BlockId,
    idx: ValueId,
    canonical_ivs: &[CanonicalIvRule],
    one_based_ivs: &[OneBasedIvRule],
    fn_ir: &FnIR,
) -> bool {
    for rule in canonical_ivs {
        if !rule.body.contains(&bid) {
            continue;
        }
        if let Some(off) = iv_offset_for_idx(fn_ir, idx, rule.iv)
            && off >= 0
        {
            return true;
        }
    }
    for rule in one_based_ivs {
        if !rule.body.contains(&bid) {
            continue;
        }
        if let Some(off) = iv_offset_for_idx(fn_ir, idx, rule.iv)
            && off >= 0
        {
            return true;
        }
    }
    false
}

fn iv_exact_in_block(
    bid: BlockId,
    idx: ValueId,
    canonical_ivs: &[CanonicalIvRule],
    fn_ir: &FnIR,
) -> bool {
    for rule in canonical_ivs {
        if !rule.body.contains(&bid) {
            continue;
        }
        if iv_offset_for_idx(fn_ir, idx, rule.iv) == Some(0) {
            return true;
        }
    }
    false
}

fn iv_in_bounds_for_base(
    bid: BlockId,
    idx: ValueId,
    base: ValueId,
    canonical_ivs: &[CanonicalIvRule],
    fn_ir: &FnIR,
) -> bool {
    for rule in canonical_ivs {
        if !rule.body.contains(&bid) {
            continue;
        }
        let Some(off) = iv_offset_for_idx(fn_ir, idx, rule.iv) else {
            continue;
        };
        if off < 0 {
            continue;
        }
        match rule.limit {
            Some((lim_base, lim_off)) => {
                // Need: iv + off <= len(base), given iv <= len(base) + lim_off.
                if same_base_for_len(fn_ir, lim_base, base) && off <= -lim_off {
                    return true;
                }
            }
            None => {
                if off == 0 {
                    return true;
                }
            }
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn collect_index_safety(
    vid: ValueId,
    bid: BlockId,
    facts: &mut crate::mir::analyze::range::RangeFacts,
    fn_ir: &FnIR,
    canonical_ivs: &[CanonicalIvRule],
    one_based_ivs: &[OneBasedIvRule],
    na_states: &[NaState],
    safe_values: &mut FxHashSet<ValueId>,
    non_na_values: &mut FxHashSet<ValueId>,
    one_based_values: &mut FxHashSet<ValueId>,
    seen: &mut FxHashSet<ValueId>,
    node_visits: &mut usize,
    visit_limit: usize,
) {
    if *node_visits >= visit_limit {
        return;
    }
    *node_visits += 1;

    if !seen.insert(vid) {
        return;
    }

    match &fn_ir.values[vid].kind {
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => {
            ensure_value_range(*idx, &fn_ir.values, facts);
            let iv_proven = iv_in_bounds_for_base(bid, *idx, *base, canonical_ivs, fn_ir);
            let idx_intv = facts.get(*idx);
            if iv_non_na_in_block(bid, *idx, canonical_ivs, one_based_ivs, fn_ir) {
                one_based_values.insert(*idx);
            }
            if !*is_safe && (interval_proves_in_bounds(fn_ir, &idx_intv, *base) || iv_proven) {
                safe_values.insert(vid);
            }
            let na_proven = matches!(na_states[*idx], NaState::Never)
                || iv_non_na_in_block(bid, *idx, canonical_ivs, one_based_ivs, fn_ir);
            if !*is_na_safe && (na_proven || interval_proves_in_bounds(fn_ir, &idx_intv, *base)) {
                non_na_values.insert(vid);
            }

            collect_index_safety(
                *base,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
            collect_index_safety(
                *idx,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            collect_index_safety(
                *lhs,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
            collect_index_safety(
                *rhs,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
        }
        ValueKind::Unary { rhs, .. } => {
            collect_index_safety(
                *rhs,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
        }
        ValueKind::Call { args, .. } => {
            for a in args {
                collect_index_safety(
                    *a,
                    bid,
                    facts,
                    fn_ir,
                    canonical_ivs,
                    one_based_ivs,
                    na_states,
                    safe_values,
                    non_na_values,
                    one_based_values,
                    seen,
                    node_visits,
                    visit_limit,
                );
            }
        }
        ValueKind::Intrinsic { args, .. } => {
            for a in args {
                collect_index_safety(
                    *a,
                    bid,
                    facts,
                    fn_ir,
                    canonical_ivs,
                    one_based_ivs,
                    na_states,
                    safe_values,
                    non_na_values,
                    one_based_values,
                    seen,
                    node_visits,
                    visit_limit,
                );
            }
        }
        ValueKind::Phi { args } => {
            for (a, _) in args {
                collect_index_safety(
                    *a,
                    bid,
                    facts,
                    fn_ir,
                    canonical_ivs,
                    one_based_ivs,
                    na_states,
                    safe_values,
                    non_na_values,
                    one_based_values,
                    seen,
                    node_visits,
                    visit_limit,
                );
            }
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            collect_index_safety(
                *base,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
        }
        ValueKind::Range { start, end } => {
            collect_index_safety(
                *start,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
            collect_index_safety(
                *end,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
        }
        ValueKind::Index2D { base, r, c } => {
            collect_index_safety(
                *base,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
            collect_index_safety(
                *r,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
            collect_index_safety(
                *c,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                one_based_ivs,
                na_states,
                safe_values,
                non_na_values,
                one_based_values,
                seen,
                node_visits,
                visit_limit,
            );
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => {}
    }
}

fn interval_proves_in_bounds(fn_ir: &FnIR, intv: &RangeInterval, base: ValueId) -> bool {
    let lo_safe = match &intv.lo {
        SymbolicBound::Const(n) => *n >= 1,
        SymbolicBound::LenOf(_, off) => *off >= 1,
        _ => false,
    };
    let hi_safe = match &intv.hi {
        SymbolicBound::LenOf(b, off) => *off <= 0 && same_base_for_len(fn_ir, *b, base),
        SymbolicBound::Const(_) => false,
        _ => false,
    };
    lo_safe && hi_safe
}

fn same_base_for_len(fn_ir: &FnIR, len_base: ValueId, index_base: ValueId) -> bool {
    if len_base == index_base {
        return true;
    }
    let len_ty = fn_ir.values[len_base].value_ty;
    let idx_ty = fn_ir.values[index_base].value_ty;
    if len_ty.len_sym.is_some() && len_ty.len_sym == idx_ty.len_sym {
        return true;
    }
    let a = value_base_name(fn_ir, len_base);
    let b = value_base_name(fn_ir, index_base);
    match (a, b) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

fn value_base_name(fn_ir: &FnIR, vid: ValueId) -> Option<&str> {
    if let Some(name) = fn_ir.values[vid].origin_var.as_deref() {
        return Some(name);
    }
    match &fn_ir.values[vid].kind {
        ValueKind::Load { var } => Some(var.as_str()),
        _ => None,
    }
}
