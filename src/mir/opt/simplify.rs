use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit, UnaryOp};
use crate::typeck::{PrimTy, TypeTerm};

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;
    changed |= algebraic_simplify(fn_ir);
    changed |= name_propagation(fn_ir);
    changed
}

fn name_propagation(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;
    for b in &fn_ir.blocks {
        for instr in &b.instrs {
            if let Instr::Assign { dst, src, .. } = instr {
                // If the value doesn't have a name yet, or it's a temp, use the assigned name
                let val = &mut fn_ir.values[*src];
                if val.origin_var.is_none() {
                    val.origin_var = Some(dst.clone());
                    changed = true;
                }
            }
        }
    }
    changed
}

fn algebraic_simplify(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;

    // We iterate over all values and try to simplify their kinds.
    for val_id in 0..fn_ir.values.len() {
        let new_kind = {
            let val = &fn_ir.values[val_id];
            simplify_kind(&val.kind, fn_ir)
        };

        if let Some(kind) = new_kind
            && fn_ir.values[val_id].kind != kind
        {
            fn_ir.values[val_id].kind = kind;
            changed = true;
        }
    }

    changed
}

fn simplify_kind(kind: &ValueKind, fn_ir: &FnIR) -> Option<ValueKind> {
    match kind {
        ValueKind::Binary { op, lhs, rhs } => {
            let l_kind = &fn_ir.values[*lhs].kind;
            let r_kind = &fn_ir.values[*rhs].kind;

            match op {
                BinOp::Add => {
                    // x + 0 -> x
                    if is_const_zero(r_kind) && is_numeric_value(fn_ir, *lhs) {
                        return Some(fn_ir.values[*lhs].kind.clone());
                    }
                    // 0 + x -> x
                    if is_const_zero(l_kind) && is_numeric_value(fn_ir, *rhs) {
                        return Some(fn_ir.values[*rhs].kind.clone());
                    }
                }
                BinOp::Sub
                    // x - 0 -> x
                    if is_const_zero(r_kind) && is_numeric_value(fn_ir, *lhs) => {
                        return Some(fn_ir.values[*lhs].kind.clone());
                    }
                BinOp::Mul => {
                    // x * 1 -> x
                    if is_const_one(r_kind) && is_numeric_value(fn_ir, *lhs) {
                        return Some(fn_ir.values[*lhs].kind.clone());
                    }
                    // 1 * x -> x
                    if is_const_one(l_kind) && is_numeric_value(fn_ir, *rhs) {
                        return Some(fn_ir.values[*rhs].kind.clone());
                    }
                    // x * 2 -> x + x, but only when rendering the duplicated
                    // value cannot duplicate calls, checks, or mutable reads.
                    if is_const_two(r_kind)
                        && is_numeric_value(fn_ir, *lhs)
                        && is_duplicate_safe_value(fn_ir, *lhs)
                    {
                        return Some(ValueKind::Binary {
                            op: BinOp::Add,
                            lhs: *lhs,
                            rhs: *lhs,
                        });
                    }
                    // 2 * x -> x + x under the same duplication guard.
                    if is_const_two(l_kind)
                        && is_numeric_value(fn_ir, *rhs)
                        && is_duplicate_safe_value(fn_ir, *rhs)
                    {
                        return Some(ValueKind::Binary {
                            op: BinOp::Add,
                            lhs: *rhs,
                            rhs: *rhs,
                        });
                    }
                }
                BinOp::Div
                    // x / 1 -> x
                    if is_const_one(r_kind) && is_numeric_value(fn_ir, *lhs) => {
                        return Some(fn_ir.values[*lhs].kind.clone());
                    }
                _ => {}
            }
        }
        ValueKind::Unary { op, rhs } => {
            let r_kind = &fn_ir.values[*rhs].kind;
            if op == &UnaryOp::Not {
                // !!x -> x
                if let ValueKind::Unary {
                    op: UnaryOp::Not,
                    rhs: inner_rhs,
                } = r_kind
                    && is_logical_value(fn_ir, *inner_rhs)
                {
                    return Some(fn_ir.values[*inner_rhs].kind.clone());
                }
            }
        }
        _ => {}
    }
    None
}

fn is_const_zero(kind: &ValueKind) -> bool {
    matches!(
        kind,
        ValueKind::Const(Lit::Int(0)) | ValueKind::Const(Lit::Float(0.0))
    )
}

fn is_const_one(kind: &ValueKind) -> bool {
    matches!(
        kind,
        ValueKind::Const(Lit::Int(1)) | ValueKind::Const(Lit::Float(1.0))
    )
}

fn is_const_two(kind: &ValueKind) -> bool {
    matches!(
        kind,
        ValueKind::Const(Lit::Int(2)) | ValueKind::Const(Lit::Float(2.0))
    )
}

fn is_duplicate_safe_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    is_duplicate_safe_value_inner(fn_ir, vid, &mut Vec::new())
}

fn is_duplicate_safe_value_inner(fn_ir: &FnIR, vid: ValueId, seen: &mut Vec<ValueId>) -> bool {
    if seen.contains(&vid) {
        return false;
    }
    let Some(value) = fn_ir.values.get(vid) else {
        return false;
    };
    seen.push(vid);
    let safe = match &value.kind {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => true,
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            is_duplicate_safe_value_inner(fn_ir, *base, seen)
        }
        ValueKind::Range { start, end } => {
            is_duplicate_safe_value_inner(fn_ir, *start, seen)
                && is_duplicate_safe_value_inner(fn_ir, *end, seen)
        }
        ValueKind::Unary { op, rhs } => {
            matches!(op, UnaryOp::Neg | UnaryOp::Not)
                && is_duplicate_safe_value_inner(fn_ir, *rhs, seen)
        }
        ValueKind::Binary { op, lhs, rhs } => {
            matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul)
                && is_duplicate_safe_value_inner(fn_ir, *lhs, seen)
                && is_duplicate_safe_value_inner(fn_ir, *rhs, seen)
        }
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => {
            *is_safe
                && *is_na_safe
                && is_duplicate_safe_value_inner(fn_ir, *base, seen)
                && is_duplicate_safe_value_inner(fn_ir, *idx, seen)
        }
        ValueKind::Phi { .. }
        | ValueKind::Call { .. }
        | ValueKind::RecordLit { .. }
        | ValueKind::FieldGet { .. }
        | ValueKind::FieldSet { .. }
        | ValueKind::Intrinsic { .. }
        | ValueKind::Index2D { .. }
        | ValueKind::Index3D { .. }
        | ValueKind::RSymbol { .. } => false,
    };
    seen.pop();
    safe
}

fn is_numeric_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    let Some(value) = fn_ir.values.get(vid) else {
        return false;
    };
    if matches!(
        &value.kind,
        ValueKind::Const(Lit::Int(_)) | ValueKind::Const(Lit::Float(_))
    ) {
        return true;
    }
    if matches!(value.value_ty.prim, PrimTy::Int | PrimTy::Double) {
        return true;
    }
    type_term_is_numeric(&value.value_term)
}

fn is_logical_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    let Some(value) = fn_ir.values.get(vid) else {
        return false;
    };
    if matches!(&value.kind, ValueKind::Const(Lit::Bool(_))) {
        return true;
    }
    value.value_ty.prim == PrimTy::Logical || type_term_is_logical(&value.value_term)
}

fn type_term_is_numeric(term: &TypeTerm) -> bool {
    match term {
        TypeTerm::Int | TypeTerm::Double => true,
        TypeTerm::Vector(inner)
        | TypeTerm::VectorLen(inner, _)
        | TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _) => type_term_is_numeric(inner),
        _ => false,
    }
}

fn type_term_is_logical(term: &TypeTerm) -> bool {
    match term {
        TypeTerm::Logical => true,
        TypeTerm::Vector(inner)
        | TypeTerm::VectorLen(inner, _)
        | TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _) => type_term_is_logical(inner),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::flow::Facts;
    use crate::utils::Span;

    fn one_block_fn(name: &str) -> FnIR {
        let mut f = FnIR::new(name.to_string(), vec![]);
        let b0 = f.add_block();
        f.entry = b0;
        f.body_head = b0;
        f
    }

    #[test]
    fn simplify_does_not_drop_operand_for_zero_multiply() {
        let mut fn_ir = one_block_fn("mul_zero");
        let call = fn_ir.add_value(
            ValueKind::Call {
                callee: "side_effect".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let product = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Mul,
                lhs: call,
                rhs: zero,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(product));

        optimize(&mut fn_ir);
        assert!(matches!(
            &fn_ir.values[product].kind,
            ValueKind::Binary {
                op: BinOp::Mul,
                lhs,
                rhs
            } if *lhs == call && *rhs == zero
        ));
    }

    #[test]
    fn simplify_rewrites_safe_multiply_by_two_to_add() {
        let mut fn_ir = one_block_fn("mul_two_safe");
        let param = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.values[param].value_term = TypeTerm::Int;
        let two = fn_ir.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let product = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Mul,
                lhs: param,
                rhs: two,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(product));

        assert!(optimize(&mut fn_ir));
        assert!(matches!(
            &fn_ir.values[product].kind,
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs
            } if *lhs == param && *rhs == param
        ));
    }

    #[test]
    fn simplify_does_not_duplicate_calls_for_multiply_by_two() {
        let mut fn_ir = one_block_fn("mul_two_call");
        let call = fn_ir.add_value(
            ValueKind::Call {
                callee: "numeric_side_effect".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.values[call].value_term = TypeTerm::Double;
        let two = fn_ir.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let product = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Mul,
                lhs: call,
                rhs: two,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(product));

        optimize(&mut fn_ir);
        assert!(matches!(
            &fn_ir.values[product].kind,
            ValueKind::Binary {
                op: BinOp::Mul,
                lhs,
                rhs
            } if *lhs == call && *rhs == two
        ));
    }

    #[test]
    fn simplify_does_not_rewrite_numeric_double_not_to_numeric_value() {
        let mut fn_ir = one_block_fn("double_not_numeric");
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let not_one = fn_ir.add_value(
            ValueKind::Unary {
                op: UnaryOp::Not,
                rhs: one,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let double_not = fn_ir.add_value(
            ValueKind::Unary {
                op: UnaryOp::Not,
                rhs: not_one,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(double_not));

        optimize(&mut fn_ir);
        assert!(matches!(
            &fn_ir.values[double_not].kind,
            ValueKind::Unary {
                op: UnaryOp::Not,
                rhs
            } if *rhs == not_one
        ));
    }

    #[test]
    fn simplify_rewrites_logical_double_not() {
        let mut fn_ir = one_block_fn("double_not_bool");
        let truth = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let not_truth = fn_ir.add_value(
            ValueKind::Unary {
                op: UnaryOp::Not,
                rhs: truth,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let double_not = fn_ir.add_value(
            ValueKind::Unary {
                op: UnaryOp::Not,
                rhs: not_truth,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(double_not));

        assert!(optimize(&mut fn_ir));
        assert!(matches!(
            &fn_ir.values[double_not].kind,
            ValueKind::Const(Lit::Bool(true))
        ));
    }
}
