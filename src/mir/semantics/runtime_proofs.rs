use crate::diagnostic::DiagnosticBuilder;
use crate::error::{RRCode, RRException, Stage};
use crate::mir::*;
use crate::utils::Span;
use rustc_hash::FxHashSet;

pub(super) struct RuntimeSafetyNeeds {
    pub(super) needs_na: bool,
    pub(super) needs_range: bool,
    pub(super) needs_dataflow: bool,
}

pub(super) fn runtime_safety_needs(fn_ir: &FnIR) -> RuntimeSafetyNeeds {
    let mut needs = RuntimeSafetyNeeds {
        needs_na: false,
        needs_range: false,
        needs_dataflow: false,
    };

    for block in &fn_ir.blocks {
        if matches!(block.term, Terminator::If { .. }) {
            needs.needs_na = true;
        }
        for instr in &block.instrs {
            if matches!(
                instr,
                Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
            ) {
                needs.needs_na = true;
                needs.needs_range = true;
            }
        }
    }

    for value in &fn_ir.values {
        match &value.kind {
            ValueKind::Binary {
                op: BinOp::Div | BinOp::Mod,
                ..
            } => {
                needs.needs_dataflow = true;
                needs.needs_range = true;
            }
            ValueKind::Index1D { .. } | ValueKind::Index2D { .. } | ValueKind::Index3D { .. } => {
                needs.needs_range = true;
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                needs.needs_range = true;
            }
            _ => {}
        }
    }

    needs
}

pub(super) fn division_by_zero_diagnostic(
    use_span: Span,
    origin_span: Span,
    message: &str,
) -> RRException {
    DiagnosticBuilder::new(
        "RR.RuntimeError",
        RRCode::E2001,
        Stage::Mir,
        message.to_string(),
    )
    .at(use_span)
    .origin(
        origin_span,
        "divisor originates here and is proven to be zero",
    )
    .constraint(use_span, "division and modulo require a non-zero divisor")
    .use_site(use_span, "used here as a divisor")
    .fix("guard the divisor or clamp it away from zero before division")
    .build()
}

pub(super) fn seq_len_negative_diagnostic(use_span: Span, origin_span: Span) -> RRException {
    DiagnosticBuilder::new(
        "RR.RuntimeError",
        RRCode::E2007,
        Stage::Mir,
        "seq_len() with negative length is guaranteed to fail".to_string(),
    )
    .at(use_span)
    .origin(
        origin_span,
        "length value originates here and is proven negative",
    )
    .constraint(use_span, "seq_len() requires an argument >= 0")
    .use_site(use_span, "used here as the seq_len() length")
    .fix("clamp the length to 0 or prove it non-negative before calling seq_len()")
    .build()
}

pub(super) fn interval_guarantees_below_one(
    intv: &crate::mir::analyze::range::RangeInterval,
) -> bool {
    upper_const(intv).is_some_and(|hi| hi < 1)
}

pub(super) fn interval_guarantees_negative(
    intv: &crate::mir::analyze::range::RangeInterval,
) -> bool {
    upper_const(intv).is_some_and(|hi| hi < 0)
}

pub(super) fn interval_guarantees_above_base_len(
    intv: &crate::mir::analyze::range::RangeInterval,
    base: ValueId,
) -> bool {
    matches!(
        intv.lo,
        crate::mir::analyze::range::SymbolicBound::LenOf(b, off) if b == base && off > 0
    )
}

pub(super) fn upper_const(intv: &crate::mir::analyze::range::RangeInterval) -> Option<i64> {
    match intv.hi {
        crate::mir::analyze::range::SymbolicBound::Const(v) => Some(v),
        _ => None,
    }
}

pub(super) fn lower_const(intv: &crate::mir::analyze::range::RangeInterval) -> Option<i64> {
    match intv.lo {
        crate::mir::analyze::range::SymbolicBound::Const(v) => Some(v),
        _ => None,
    }
}

pub(super) fn interval_guarantees_above_const(
    intv: &crate::mir::analyze::range::RangeInterval,
    limit: i64,
) -> bool {
    lower_const(intv).is_some_and(|lo| lo > limit)
}

pub(super) fn format_interval(intv: &crate::mir::analyze::range::RangeInterval) -> String {
    format!("[{}, {}]", format_bound(&intv.lo), format_bound(&intv.hi))
}

pub(super) fn format_bound(bound: &crate::mir::analyze::range::SymbolicBound) -> String {
    match bound {
        crate::mir::analyze::range::SymbolicBound::NegInf => "-inf".to_string(),
        crate::mir::analyze::range::SymbolicBound::PosInf => "+inf".to_string(),
        crate::mir::analyze::range::SymbolicBound::Const(v) => v.to_string(),
        crate::mir::analyze::range::SymbolicBound::VarPlus(v, off) => {
            format!("v{}+{}", v, off)
        }
        crate::mir::analyze::range::SymbolicBound::LenOf(v, off) => {
            format!("len(v{})+{}", v, off)
        }
    }
}

pub(super) fn interval_guarantees_zero(interval: Option<crate::mir::flow::Interval>) -> bool {
    interval.is_some_and(|intv| intv.min == 0 && intv.max == 0)
}

pub(super) fn range_interval_to_fact_interval(
    range_in: &[crate::mir::analyze::range::RangeFacts],
    bid: BlockId,
    vid: ValueId,
) -> Option<crate::mir::flow::Interval> {
    let intv = range_in.get(bid)?.get(vid);
    let lo = upper_const_from_bound(&intv.lo)?;
    let hi = upper_const_from_bound(&intv.hi)?;
    Some(crate::mir::flow::Interval::new(lo, hi))
}

pub(super) fn upper_const_from_bound(
    bound: &crate::mir::analyze::range::SymbolicBound,
) -> Option<i64> {
    match bound {
        crate::mir::analyze::range::SymbolicBound::Const(v) => Some(*v),
        _ => None,
    }
}

pub(super) fn bid_for_value(fn_ir: &FnIR, vid: ValueId) -> BlockId {
    let mut seen = FxHashSet::default();
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        for ins in &block.instrs {
            match ins {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    if root_depends_on_value(fn_ir, *src, vid, &mut seen) {
                        return bid;
                    }
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    if root_depends_on_value(fn_ir, *base, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *idx, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *val, vid, &mut seen)
                    {
                        return bid;
                    }
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    if root_depends_on_value(fn_ir, *base, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *r, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *c, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *val, vid, &mut seen)
                    {
                        return bid;
                    }
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    if root_depends_on_value(fn_ir, *base, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *i, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *j, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *k, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *val, vid, &mut seen)
                    {
                        return bid;
                    }
                }
            }
        }
        match block.term {
            Terminator::If { cond, .. } => {
                if root_depends_on_value(fn_ir, cond, vid, &mut seen) {
                    return bid;
                }
            }
            Terminator::Return(Some(ret)) => {
                if root_depends_on_value(fn_ir, ret, vid, &mut seen) {
                    return bid;
                }
            }
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }
    fn_ir.entry
}

pub(super) fn root_depends_on_value(
    fn_ir: &FnIR,
    root: ValueId,
    target: ValueId,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    if root == target {
        return true;
    }
    if !seen.insert(root) {
        return false;
    }
    let depends = match &fn_ir.values[root].kind {
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
        ValueKind::Phi { args } => args
            .iter()
            .any(|(src, _)| root_depends_on_value(fn_ir, *src, target, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            root_depends_on_value(fn_ir, *base, target, seen)
        }
        ValueKind::Range { start, end } => {
            root_depends_on_value(fn_ir, *start, target, seen)
                || root_depends_on_value(fn_ir, *end, target, seen)
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            root_depends_on_value(fn_ir, *lhs, target, seen)
                || root_depends_on_value(fn_ir, *rhs, target, seen)
        }
        ValueKind::Unary { rhs, .. } => root_depends_on_value(fn_ir, *rhs, target, seen),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| root_depends_on_value(fn_ir, *arg, target, seen)),
        ValueKind::Index1D { base, idx, .. } => {
            root_depends_on_value(fn_ir, *base, target, seen)
                || root_depends_on_value(fn_ir, *idx, target, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            root_depends_on_value(fn_ir, *base, target, seen)
                || root_depends_on_value(fn_ir, *r, target, seen)
                || root_depends_on_value(fn_ir, *c, target, seen)
        }
        ValueKind::Index3D { base, i, j, k } => {
            root_depends_on_value(fn_ir, *base, target, seen)
                || root_depends_on_value(fn_ir, *i, target, seen)
                || root_depends_on_value(fn_ir, *j, target, seen)
                || root_depends_on_value(fn_ir, *k, target, seen)
        }
    };
    seen.remove(&root);
    depends
}
