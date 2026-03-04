use crate::mir::analyze::na::{self, NaState};
use crate::mir::analyze::range::{RangeInterval, SymbolicBound};
use crate::mir::analyze::range::{analyze_ranges, ensure_value_range, transfer_instr};
use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::mir::*;
use rustc_hash::FxHashSet;

#[derive(Clone)]
struct CanonicalIvRule {
    body: FxHashSet<BlockId>,
    iv: ValueId,
    // iv <= len(base) + limit_off
    limit: Option<(ValueId, i64)>,
}

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;
    let bb_facts = analyze_ranges(fn_ir);
    let na_states = na::compute_na_states(fn_ir);
    let canonical_ivs = canonical_loop_ivs(fn_ir);

    // Pass 1: Handle StoreIndex1D instructions
    for bid in 0..fn_ir.blocks.len() {
        let mut cur_facts = bb_facts[bid].clone();
        let num_instrs = fn_ir.blocks[bid].instrs.len();

        for i in 0..num_instrs {
            let (in_bounds, non_na) = {
                let instr = &fn_ir.blocks[bid].instrs[i];
                if let Instr::StoreIndex1D { base, idx, .. } = instr {
                    ensure_value_range(*idx, &fn_ir.values, &mut cur_facts);
                    let iv_proven = iv_exact_in_block(bid, *idx, &canonical_ivs, fn_ir);
                    let idx_intv = cur_facts.get(*idx);
                    let in_bounds = interval_proves_in_bounds(fn_ir, &idx_intv, *base) || iv_proven;
                    let non_na = matches!(na_states[*idx], NaState::Never) || iv_proven;
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
    for bid in 0..fn_ir.blocks.len() {
        let mut cur_facts = bb_facts[bid].clone();
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
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut seen,
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
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut seen,
                    );
                    collect_index_safety(
                        *idx,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut seen,
                    );
                    collect_index_safety(
                        *val,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut seen,
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
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut seen,
                    );
                    collect_index_safety(
                        *r,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut seen,
                    );
                    collect_index_safety(
                        *c,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut seen,
                    );
                    collect_index_safety(
                        *val,
                        bid,
                        &mut cur_facts,
                        fn_ir,
                        &canonical_ivs,
                        &na_states,
                        &mut safe_values,
                        &mut non_na_values,
                        &mut seen,
                    );
                }
            }
            transfer_instr(instr, &fn_ir.values, &mut cur_facts);
        }
    }

    for vid in safe_values {
        if let ValueKind::Index1D {
            ref mut is_safe, ..
        } = fn_ir.values[vid].kind
        {
            if !*is_safe {
                *is_safe = true;
                changed = true;
            }
        }
    }

    for vid in non_na_values {
        if let ValueKind::Index1D {
            ref mut is_na_safe, ..
        } = fn_ir.values[vid].kind
        {
            if !*is_na_safe {
                *is_na_safe = true;
                changed = true;
            }
        }
    }

    changed
}

fn canonical_loop_ivs(fn_ir: &FnIR) -> Vec<CanonicalIvRule> {
    let mut out = Vec::new();
    for lp in LoopAnalyzer::new(fn_ir).find_loops() {
        let iv = match lp.iv {
            Some(iv) => iv,
            None => continue,
        };
        let init_is_one = matches!(
            fn_ir.values[iv.init_val].kind,
            ValueKind::Const(Lit::Int(1))
        );
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

fn extract_len_limit(fn_ir: &FnIR, limit_val: ValueId) -> Option<(ValueId, i64)> {
    match &fn_ir.values[limit_val].kind {
        ValueKind::Len { base } => Some((*base, 0)),
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            if let ValueKind::Len { base } = fn_ir.values[*lhs].kind {
                if let Some(k) = const_int(fn_ir, *rhs) {
                    return Some((base, -k));
                }
            }
            None
        }
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            if let ValueKind::Len { base } = fn_ir.values[*lhs].kind {
                if let Some(k) = const_int(fn_ir, *rhs) {
                    return Some((base, k));
                }
            }
            if let ValueKind::Len { base } = fn_ir.values[*rhs].kind {
                if let Some(k) = const_int(fn_ir, *lhs) {
                    return Some((base, k));
                }
            }
            None
        }
        _ => None,
    }
}

fn const_int(fn_ir: &FnIR, vid: ValueId) -> Option<i64> {
    match &fn_ir.values[vid].kind {
        ValueKind::Const(Lit::Int(n)) => Some(*n),
        _ => None,
    }
}

fn iv_offset_for_idx(fn_ir: &FnIR, idx: ValueId, iv: ValueId) -> Option<i64> {
    if idx == iv {
        return Some(0);
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            if *lhs == iv {
                return const_int(fn_ir, *rhs);
            }
            if *rhs == iv {
                return const_int(fn_ir, *lhs);
            }
            None
        }
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            if *lhs == iv {
                return const_int(fn_ir, *rhs).map(|k| -k);
            }
            None
        }
        _ => None,
    }
}

fn iv_non_na_in_block(
    bid: BlockId,
    idx: ValueId,
    canonical_ivs: &[CanonicalIvRule],
    fn_ir: &FnIR,
) -> bool {
    for rule in canonical_ivs {
        if !rule.body.contains(&bid) {
            continue;
        }
        if let Some(off) = iv_offset_for_idx(fn_ir, idx, rule.iv) {
            if off >= 0 {
                return true;
            }
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

fn collect_index_safety(
    vid: ValueId,
    bid: BlockId,
    facts: &mut crate::mir::analyze::range::RangeFacts,
    fn_ir: &FnIR,
    canonical_ivs: &[CanonicalIvRule],
    na_states: &[NaState],
    safe_values: &mut FxHashSet<ValueId>,
    non_na_values: &mut FxHashSet<ValueId>,
    seen: &mut FxHashSet<ValueId>,
) {
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
            if !*is_safe && (interval_proves_in_bounds(fn_ir, &idx_intv, *base) || iv_proven) {
                safe_values.insert(vid);
            }
            if !*is_na_safe
                && (matches!(na_states[*idx], NaState::Never)
                    || iv_non_na_in_block(bid, *idx, canonical_ivs, fn_ir))
            {
                non_na_values.insert(vid);
            }

            collect_index_safety(
                *base,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
            );
            collect_index_safety(
                *idx,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
            );
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            collect_index_safety(
                *lhs,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
            );
            collect_index_safety(
                *rhs,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
            );
        }
        ValueKind::Unary { rhs, .. } => {
            collect_index_safety(
                *rhs,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
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
                    na_states,
                    safe_values,
                    non_na_values,
                    seen,
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
                    na_states,
                    safe_values,
                    non_na_values,
                    seen,
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
                    na_states,
                    safe_values,
                    non_na_values,
                    seen,
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
                na_states,
                safe_values,
                non_na_values,
                seen,
            );
        }
        ValueKind::Range { start, end } => {
            collect_index_safety(
                *start,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
            );
            collect_index_safety(
                *end,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
            );
        }
        ValueKind::Index2D { base, r, c } => {
            collect_index_safety(
                *base,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
            );
            collect_index_safety(
                *r,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
            );
            collect_index_safety(
                *c,
                bid,
                facts,
                fn_ir,
                canonical_ivs,
                na_states,
                safe_values,
                non_na_values,
                seen,
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
