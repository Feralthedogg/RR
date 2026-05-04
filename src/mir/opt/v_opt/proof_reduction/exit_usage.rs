use super::*;
use crate::mir::value_dependencies;
pub(crate) fn var_read_after_loop_exit(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> bool {
    fn value_has_direct_post_exit_load(fn_ir: &FnIR, root: ValueId, var: &str) -> bool {
        let mut stack = vec![root];
        let mut seen = FxHashSet::default();
        while let Some(value) = stack.pop() {
            if !seen.insert(value) {
                continue;
            }
            let Some(row) = fn_ir.values.get(value) else {
                continue;
            };
            match &row.kind {
                ValueKind::Load { var: load_var } if load_var == var => return true,
                ValueKind::Phi { .. }
                | ValueKind::Const(_)
                | ValueKind::Param { .. }
                | ValueKind::RSymbol { .. }
                | ValueKind::Load { .. } => {}
                _ => stack.extend(value_dependencies(&row.kind)),
            }
        }
        false
    }

    let mut seen = FxHashSet::default();
    let mut stack = lp.exits.clone();
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        let Some(block) = fn_ir.blocks.get(bid) else {
            continue;
        };
        for instr in &block.instrs {
            let reads = match instr {
                Instr::Assign { dst, src, .. } => {
                    dst != var && value_has_direct_post_exit_load(fn_ir, *src, var)
                }
                Instr::Eval { val, .. } => value_has_direct_post_exit_load(fn_ir, *val, var),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    value_has_direct_post_exit_load(fn_ir, *base, var)
                        || value_has_direct_post_exit_load(fn_ir, *idx, var)
                        || value_has_direct_post_exit_load(fn_ir, *val, var)
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    value_has_direct_post_exit_load(fn_ir, *base, var)
                        || value_has_direct_post_exit_load(fn_ir, *r, var)
                        || value_has_direct_post_exit_load(fn_ir, *c, var)
                        || value_has_direct_post_exit_load(fn_ir, *val, var)
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    value_has_direct_post_exit_load(fn_ir, *base, var)
                        || value_has_direct_post_exit_load(fn_ir, *i, var)
                        || value_has_direct_post_exit_load(fn_ir, *j, var)
                        || value_has_direct_post_exit_load(fn_ir, *k, var)
                        || value_has_direct_post_exit_load(fn_ir, *val, var)
                }
                Instr::UnsafeRBlock { .. } => false,
            };
            if reads {
                return true;
            }
        }
        match block.term {
            Terminator::Goto(next) => stack.push(next),
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                if value_has_direct_post_exit_load(fn_ir, cond, var) {
                    return true;
                }
                stack.push(then_bb);
                stack.push(else_bb);
            }
            Terminator::Return(Some(ret)) => {
                if value_has_direct_post_exit_load(fn_ir, ret, var) {
                    return true;
                }
            }
            Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }
    false
}
