use super::*;
use crate::diagnostic::DiagnosticBuilder;
use crate::error::{RRCode, RRException, Stage};
use crate::utils::Span;
use rustc_hash::FxHashSet;

pub(crate) struct RuntimeSafetyNeeds {
    pub(crate) needs_na: bool,
    pub(crate) needs_range: bool,
    pub(crate) needs_dataflow: bool,
}

pub(crate) fn runtime_safety_needs(fn_ir: &FnIR) -> RuntimeSafetyNeeds {
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
                needs.needs_dataflow = true;
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
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                needs.needs_dataflow = true;
                needs.needs_range = true;
            }
            ValueKind::Index1D { .. } | ValueKind::Index2D { .. } | ValueKind::Index3D { .. } => {
                needs.needs_range = true;
                needs.needs_dataflow = true;
            }
            _ => {}
        }
    }

    needs
}

pub(crate) fn division_by_zero_diagnostic(
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

pub(crate) fn seq_len_negative_diagnostic(use_span: Span, origin_span: Span) -> RRException {
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

// Proof correspondence:
// `proof/lean/RRProofs/RuntimeSafetyFieldRangeSubset.lean` and the Coq
// `RuntimeSafetyFieldRangeSubset.v` companion establish the reduced bridge
// from exact field-read intervals to concrete hazards. These helpers are the
// Rust-side projection from those interval facts into the `< 1` / `< 0`
// predicates consumed by runtime-safety diagnostics.
pub(crate) fn interval_guarantees_below_one(
    intv: &crate::mir::analyze::range::RangeInterval,
) -> bool {
    upper_const(intv).is_some_and(|hi| hi < 1)
}

pub(crate) fn interval_guarantees_negative(
    intv: &crate::mir::analyze::range::RangeInterval,
) -> bool {
    upper_const(intv).is_some_and(|hi| hi < 0)
}

pub(crate) fn interval_guarantees_above_base_len(
    intv: &crate::mir::analyze::range::RangeInterval,
    base: ValueId,
) -> bool {
    matches!(
        intv.lo,
        crate::mir::analyze::range::SymbolicBound::LenOf(b, off) if b == base && off > 0
    )
}

pub(crate) fn upper_const(intv: &crate::mir::analyze::range::RangeInterval) -> Option<i64> {
    match intv.hi {
        crate::mir::analyze::range::SymbolicBound::Const(v) => Some(v),
        _ => None,
    }
}

pub(crate) fn lower_const(intv: &crate::mir::analyze::range::RangeInterval) -> Option<i64> {
    match intv.lo {
        crate::mir::analyze::range::SymbolicBound::Const(v) => Some(v),
        _ => None,
    }
}

pub(crate) fn interval_guarantees_above_const(
    intv: &crate::mir::analyze::range::RangeInterval,
    limit: i64,
) -> bool {
    lower_const(intv).is_some_and(|lo| lo > limit)
}

pub(crate) fn format_interval(intv: &crate::mir::analyze::range::RangeInterval) -> String {
    format!("[{}, {}]", format_bound(&intv.lo), format_bound(&intv.hi))
}

pub(crate) fn format_bound(bound: &crate::mir::analyze::range::SymbolicBound) -> String {
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

pub(crate) fn interval_guarantees_zero(interval: Option<crate::mir::flow::Interval>) -> bool {
    interval.is_some_and(|intv| intv.min == 0 && intv.max == 0)
}

pub(crate) fn flow_interval_guarantees_negative(
    interval: Option<crate::mir::flow::Interval>,
) -> bool {
    interval.is_some_and(|intv| !intv.is_empty() && intv.max < 0)
}

pub(crate) fn flow_interval_guarantees_below_one(
    interval: Option<crate::mir::flow::Interval>,
) -> bool {
    interval.is_some_and(|intv| !intv.is_empty() && intv.max < 1)
}

pub(crate) fn range_interval_to_fact_interval(
    range_in: &[crate::mir::analyze::range::RangeFacts],
    bid: BlockId,
    vid: ValueId,
) -> Option<crate::mir::flow::Interval> {
    let intv = range_in.get(bid)?.get(vid);
    let lo = upper_const_from_bound(&intv.lo)?;
    let hi = upper_const_from_bound(&intv.hi)?;
    Some(crate::mir::flow::Interval::new(lo, hi))
}

pub(crate) fn upper_const_from_bound(
    bound: &crate::mir::analyze::range::SymbolicBound,
) -> Option<i64> {
    match bound {
        crate::mir::analyze::range::SymbolicBound::Const(v) => Some(*v),
        _ => None,
    }
}

pub(crate) fn bid_for_value(fn_ir: &FnIR, vid: ValueId) -> BlockId {
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
                Instr::UnsafeRBlock { .. } => {}
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

pub(crate) fn root_depends_on_value(
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
    let depends = value_dependencies(&fn_ir.values[root].kind)
        .into_iter()
        .any(|dep| root_depends_on_value(fn_ir, dep, target, seen));
    seen.remove(&root);
    depends
}

#[cfg(test)]
mod tests {
    use super::{
        bid_for_value, flow_interval_guarantees_below_one, flow_interval_guarantees_negative,
        root_depends_on_value,
    };
    use crate::mir::flow::Interval;
    use crate::mir::{Facts, FnIR, Instr, IntrinsicOp, Lit, Terminator, ValueKind};
    use crate::utils::Span;
    use rustc_hash::FxHashSet;

    #[test]
    fn flow_bottom_interval_does_not_prove_negative_or_below_one() {
        let bottom = Some(Interval::BOTTOM);
        assert!(!flow_interval_guarantees_negative(bottom));
        assert!(!flow_interval_guarantees_below_one(bottom));

        let negative = Some(Interval::new(-5, -1));
        assert!(flow_interval_guarantees_negative(negative));
        assert!(flow_interval_guarantees_below_one(negative));
    }

    #[test]
    fn root_depends_on_value_tracks_record_intrinsic_phi_chain() {
        let mut f = FnIR::new("runtime_proofs_dep_chain".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(Lit::Float(1.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(Lit::Float(2.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (two, right)],
            },
            Span::dummy(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);
        let intrinsic = f.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecAbsF64,
                args: vec![phi],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let record = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), intrinsic)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].instrs.push(Instr::Eval {
            val: record,
            span: Span::dummy(),
        });
        f.blocks[merge].term = Terminator::Return(Some(record));

        assert!(root_depends_on_value(
            &f,
            record,
            phi,
            &mut FxHashSet::default()
        ));
    }

    #[test]
    fn bid_for_value_finds_use_through_record_intrinsic_phi_chain() {
        let mut f = FnIR::new("runtime_proofs_bid_chain".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(Lit::Float(1.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(Lit::Float(2.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (two, right)],
            },
            Span::dummy(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);
        let intrinsic = f.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecAbsF64,
                args: vec![phi],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let record = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), intrinsic)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].instrs.push(Instr::Eval {
            val: record,
            span: Span::dummy(),
        });
        f.blocks[merge].term = Terminator::Return(Some(record));

        assert_eq!(bid_for_value(&f, phi), merge);
    }
}
