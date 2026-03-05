use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit, UnaryOp};

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
            && fn_ir.values[val_id].kind != kind {
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
                    if is_const_zero(r_kind) {
                        return Some(fn_ir.values[*lhs].kind.clone());
                    }
                    // 0 + x -> x
                    if is_const_zero(l_kind) {
                        return Some(fn_ir.values[*rhs].kind.clone());
                    }
                }
                BinOp::Sub => {
                    // x - 0 -> x
                    if is_const_zero(r_kind) {
                        return Some(fn_ir.values[*lhs].kind.clone());
                    }
                    // x - x -> 0 (if pure)
                    // Purity checks are handled conservatively at pass boundaries.
                    if lhs == rhs {
                        return Some(ValueKind::Const(Lit::Int(0)));
                    }
                }
                BinOp::Mul => {
                    // x * 1 -> x
                    if is_const_one(r_kind) {
                        return Some(fn_ir.values[*lhs].kind.clone());
                    }
                    // 1 * x -> x
                    if is_const_one(l_kind) {
                        return Some(fn_ir.values[*rhs].kind.clone());
                    }
                    // x * 0 -> 0
                    if is_const_zero(r_kind) {
                        return Some(ValueKind::Const(Lit::Int(0)));
                    }
                    // 0 * x -> 0
                    if is_const_zero(l_kind) {
                        return Some(ValueKind::Const(Lit::Int(0)));
                    }
                    // x * 2 -> x + x
                    if is_const_two(r_kind) {
                        return Some(ValueKind::Binary {
                            op: BinOp::Add,
                            lhs: *lhs,
                            rhs: *lhs,
                        });
                    }
                }
                BinOp::Div => {
                    // x / 1 -> x
                    if is_const_one(r_kind) {
                        return Some(fn_ir.values[*lhs].kind.clone());
                    }
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
