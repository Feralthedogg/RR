use crate::mir::*;
use crate::typeck::{PrimTy, ShapeTy};

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;

    for i in 0..fn_ir.values.len() {
        let idx_for_na = match fn_ir.values[i].kind {
            ValueKind::Index1D { idx, .. } => Some(idx),
            _ => None,
        };
        if let Some(idx) = idx_for_na {
            let idx_ty = fn_ir.values[idx].value_ty;
            if idx_ty.is_int_scalar_non_na()
                && let ValueKind::Index1D {
                    ref mut is_na_safe, ..
                } = fn_ir.values[i].kind
                && !*is_na_safe
            {
                *is_na_safe = true;
                changed = true;
            }
            continue;
        }

        let replacement = match fn_ir.values[i].kind.clone() {
            ValueKind::Binary { op, lhs, rhs } => {
                let lhs_ty = fn_ir.values[lhs].value_ty;
                let rhs_ty = fn_ir.values[rhs].value_ty;
                let both_vec = lhs_ty.shape == ShapeTy::Vector && rhs_ty.shape == ShapeTy::Vector;
                let both_numeric = matches!(lhs_ty.prim, PrimTy::Double | PrimTy::Int)
                    && matches!(rhs_ty.prim, PrimTy::Double | PrimTy::Int);
                if both_vec && both_numeric {
                    let op = match op {
                        crate::syntax::ast::BinOp::Add => Some(IntrinsicOp::VecAddF64),
                        crate::syntax::ast::BinOp::Sub => Some(IntrinsicOp::VecSubF64),
                        crate::syntax::ast::BinOp::Mul => Some(IntrinsicOp::VecMulF64),
                        crate::syntax::ast::BinOp::Div => Some(IntrinsicOp::VecDivF64),
                        _ => None,
                    };
                    op.map(|intr| ValueKind::Intrinsic {
                        op: intr,
                        args: vec![lhs, rhs],
                    })
                } else {
                    None
                }
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                if names.iter().any(|n| n.is_some()) {
                    None
                } else {
                    let arg0_ty = args
                        .first()
                        .map(|a| fn_ir.values[*a].value_ty)
                        .unwrap_or(TypeState::unknown());
                    let numeric_vec0 = arg0_ty.shape == ShapeTy::Vector
                        && matches!(arg0_ty.prim, PrimTy::Double | PrimTy::Int);
                    let mk = match (callee.as_str(), args.len()) {
                        ("abs", 1) if numeric_vec0 => Some(IntrinsicOp::VecAbsF64),
                        ("log", 1) if numeric_vec0 => Some(IntrinsicOp::VecLogF64),
                        ("sqrt", 1) if numeric_vec0 => Some(IntrinsicOp::VecSqrtF64),
                        ("pmax", 2) if numeric_vec0 => Some(IntrinsicOp::VecPmaxF64),
                        ("pmin", 2) if numeric_vec0 => Some(IntrinsicOp::VecPminF64),
                        ("sum", 1) if numeric_vec0 => Some(IntrinsicOp::VecSumF64),
                        ("mean", 1) if numeric_vec0 => Some(IntrinsicOp::VecMeanF64),
                        _ => None,
                    };
                    mk.map(|op| ValueKind::Intrinsic { op, args })
                }
            }
            _ => None,
        };

        if let Some(new_kind) = replacement {
            fn_ir.values[i].kind = new_kind;
            changed = true;
        }
    }

    for bb in 0..fn_ir.blocks.len() {
        for ins in 0..fn_ir.blocks[bb].instrs.len() {
            if let Instr::StoreIndex1D {
                idx,
                ref mut is_na_safe,
                ..
            } = fn_ir.blocks[bb].instrs[ins]
                && !*is_na_safe
                && fn_ir.values[idx].value_ty.is_int_scalar_non_na()
            {
                *is_na_safe = true;
                changed = true;
            }
        }
    }

    changed
}

use crate::typeck::TypeState;
