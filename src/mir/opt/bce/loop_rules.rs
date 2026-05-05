use super::*;
use crate::mir::analyze::na::{self, NaState};
use crate::mir::analyze::range::{analyze_ranges, ensure_value_range, transfer_instr};
use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo};
use rustc_hash::FxHashSet;

#[derive(Clone)]
pub(crate) struct CanonicalIvRule<'a> {
    pub(crate) body: &'a FxHashSet<BlockId>,
    pub(crate) iv: ValueId,
    // iv <= len(base) + limit_off
    pub(crate) limit: Option<(ValueId, i64)>,
}

#[derive(Clone)]
pub(crate) struct OneBasedIvRule<'a> {
    pub(crate) body: &'a FxHashSet<BlockId>,
    pub(crate) iv: ValueId,
}

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    let loops = LoopAnalyzer::new(fn_ir).find_loops();
    optimize_with_loop_info(fn_ir, &loops)
}

pub fn optimize_with_loop_info(fn_ir: &mut FnIR, loops: &[LoopInfo]) -> bool {
    let mut changed = false;
    let bb_facts = analyze_ranges(fn_ir);
    let na_states = na::compute_na_states(fn_ir);
    let canonical_ivs = canonical_loop_ivs(fn_ir, loops);
    let one_based_ivs = one_based_loop_ivs(fn_ir, loops);
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
    macro_rules! collect_safety_for {
        ($bid:expr, $facts:expr, $($value:expr),+ $(,)?) => {{
            let mut seen = FxHashSet::default();
            let mut collector = IndexSafetyCollector {
                bid: $bid,
                facts: &mut $facts,
                fn_ir,
                canonical_ivs: &canonical_ivs,
                one_based_ivs: &one_based_ivs,
                na_states: &na_states,
                safe_values: &mut safe_values,
                non_na_values: &mut non_na_values,
                one_based_values: &mut one_based_indices,
                seen: &mut seen,
                node_visits: &mut node_visits,
                visit_limit,
            };
            $(collector.collect($value);)+
        }};
    }
    for (bid, facts_at_bid) in bb_facts.iter().enumerate().take(fn_ir.blocks.len()) {
        let mut cur_facts = facts_at_bid.clone();
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    collect_safety_for!(bid, cur_facts, *src);
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    collect_safety_for!(bid, cur_facts, *base, *idx, *val);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    collect_safety_for!(bid, cur_facts, *base, *r, *c, *val);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    collect_safety_for!(bid, cur_facts, *base, *i, *j, *k, *val);
                }
                Instr::UnsafeRBlock { .. } => {}
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

pub(crate) fn bce_visit_limit() -> usize {
    200_000
}

pub(crate) fn canonical_loop_ivs<'a>(
    fn_ir: &FnIR,
    loops: &'a [LoopInfo],
) -> Vec<CanonicalIvRule<'a>> {
    let mut out = Vec::new();
    for lp in loops {
        let iv = match &lp.iv {
            Some(iv) => iv,
            None => continue,
        };
        let init_is_one = const_int(fn_ir, iv.init_val) == Some(1);
        let canonical =
            init_is_one && iv.step == 1 && iv.step_op == BinOp::Add && lp.is_seq_len.is_some();
        if canonical {
            let limit = lp.limit.and_then(|v| extract_len_limit(fn_ir, v));
            out.push(CanonicalIvRule {
                body: &lp.body,
                iv: iv.phi_val,
                limit,
            });
        }
    }
    out
}

pub(crate) fn one_based_loop_ivs<'a>(
    fn_ir: &FnIR,
    loops: &'a [LoopInfo],
) -> Vec<OneBasedIvRule<'a>> {
    let mut out = Vec::new();
    for lp in loops {
        let iv = match &lp.iv {
            Some(iv) => iv,
            None => continue,
        };
        let init_is_one = const_int(fn_ir, iv.init_val) == Some(1);
        let one_based = init_is_one && iv.step == 1 && iv.step_op == BinOp::Add;
        if one_based {
            out.push(OneBasedIvRule {
                body: &lp.body,
                iv: iv.phi_val,
            });
        }
    }
    out
}

pub(crate) fn extract_len_limit(fn_ir: &FnIR, limit_val: ValueId) -> Option<(ValueId, i64)> {
    let mut seen_vals = FxHashSet::default();
    let mut seen_vars = FxHashSet::default();
    extract_len_limit_rec(fn_ir, limit_val, &mut seen_vals, &mut seen_vars)
}

pub(crate) fn extract_len_limit_rec(
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

pub(crate) fn extract_len_limit_from_var(
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

pub(crate) fn const_int(fn_ir: &FnIR, vid: ValueId) -> Option<i64> {
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

pub(crate) fn iv_offset_for_idx(fn_ir: &FnIR, idx: ValueId, iv: ValueId) -> Option<i64> {
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

pub(crate) fn is_iv_equivalent(fn_ir: &FnIR, candidate: ValueId, iv: ValueId) -> bool {
    let mut seen = vec![false; fn_ir.values.len()];
    let mut seen_vars = FxHashSet::default();
    is_iv_equivalent_rec(fn_ir, candidate, iv, &mut seen, &mut seen_vars)
}

pub(crate) fn is_iv_equivalent_rec(
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

pub(crate) fn load_var_is_iv_equivalent(
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

pub(crate) fn iv_non_na_in_block(
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

pub(crate) fn iv_exact_in_block(
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

pub(crate) fn iv_in_bounds_for_base(
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
