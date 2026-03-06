use crate::mir::*;
use crate::syntax::ast::BinOp;

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;

    for vid in 0..fn_ir.values.len() {
        let kind = fn_ir.values[vid].kind.clone();
        match kind {
            ValueKind::Binary {
                op: BinOp::Div,
                lhs,
                rhs,
            } => {
                if let Some(base) = sum_len_pattern(fn_ir, lhs, rhs) {
                    fn_ir.values[vid].kind = ValueKind::Call {
                        callee: "mean".to_string(),
                        args: vec![base],
                        names: vec![None],
                    };
                    changed = true;
                }
            }
            ValueKind::Call {
                ref callee,
                ref args,
                ..
            } if callee == "sqrt" && args.len() == 1 => {
                if let ValueKind::Call {
                    callee: inner,
                    args: inner_args,
                    ..
                } = &fn_ir.values[args[0]].kind
                    && inner == "var"
                    && inner_args.len() == 1
                {
                    let base = inner_args[0];
                    fn_ir.values[vid].kind = ValueKind::Call {
                        callee: "sd".to_string(),
                        args: vec![base],
                        names: vec![None],
                    };
                    changed = true;
                }
            }
            _ => {}
        }
    }

    changed
}

fn sum_len_pattern(fn_ir: &FnIR, lhs: ValueId, rhs: ValueId) -> Option<ValueId> {
    if let Some(base) = sum_len_ordered(fn_ir, lhs, rhs) {
        return Some(base);
    }
    if let Some(base) = sum_len_ordered(fn_ir, rhs, lhs) {
        return Some(base);
    }
    None
}

fn sum_len_ordered(fn_ir: &FnIR, sum_id: ValueId, len_id: ValueId) -> Option<ValueId> {
    let sum_kind = &fn_ir.values[sum_id].kind;
    let len_kind = &fn_ir.values[len_id].kind;

    let base = if let ValueKind::Call { callee, args, .. } = sum_kind {
        if callee == "sum" && args.len() == 1 {
            Some(args[0])
        } else {
            None
        }
    } else {
        None
    }?;

    if let ValueKind::Len { base: len_base } = len_kind
        && *len_base == base
    {
        return Some(base);
    }
    None
}
