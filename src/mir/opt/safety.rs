use super::TachyonEngine;
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::FxHashMap;

impl TachyonEngine {
    pub(super) fn check_elimination(&self, fn_ir: &mut FnIR) -> bool {
        let mut changed = false;
        let facts = crate::mir::flow::DataflowSolver::analyze_function(fn_ir);

        for val_idx in 0..fn_ir.values.len() {
            let mut is_proven_safe = false;
            {
                let val = &fn_ir.values[val_idx];
                if let ValueKind::Index1D {
                    base, idx, is_safe, ..
                } = &val.kind
                    && !*is_safe
                    && self.is_safe_access(fn_ir, *base, *idx, &facts)
                {
                    is_proven_safe = true;
                }
            }
            if is_proven_safe
                && let ValueKind::Index1D {
                    ref mut is_safe, ..
                } = fn_ir.values[val_idx].kind
            {
                *is_safe = true;
                changed = true;
            }
        }

        for blk_idx in 0..fn_ir.blocks.len() {
            for instr_idx in 0..fn_ir.blocks[blk_idx].instrs.len() {
                let mut is_proven_safe = false;
                {
                    let instr = &fn_ir.blocks[blk_idx].instrs[instr_idx];
                    if let Instr::StoreIndex1D {
                        base, idx, is_safe, ..
                    } = instr
                        && !*is_safe
                        && self.is_safe_access(fn_ir, *base, *idx, &facts)
                    {
                        is_proven_safe = true;
                    }
                }
                if is_proven_safe
                    && let Instr::StoreIndex1D {
                        ref mut is_safe, ..
                    } = fn_ir.blocks[blk_idx].instrs[instr_idx]
                {
                    *is_safe = true;
                    changed = true;
                }
            }
        }

        changed
    }

    pub(super) fn is_safe_access(
        &self,
        fn_ir: &FnIR,
        base_id: ValueId,
        idx_id: ValueId,
        facts: &FxHashMap<ValueId, crate::mir::flow::Facts>,
    ) -> bool {
        let f = facts.get(&idx_id).cloned().unwrap_or(Facts::empty());
        if f.has(Facts::ONE_BASED) && self.is_derived_from_len(fn_ir, idx_id, base_id, facts) {
            return true;
        }
        false
    }

    pub(super) fn is_derived_from_len(
        &self,
        fn_ir: &FnIR,
        val_id: ValueId,
        base_id: ValueId,
        facts: &FxHashMap<ValueId, crate::mir::flow::Facts>,
    ) -> bool {
        let _ = facts;
        let val = &fn_ir.values[val_id];
        match &val.kind {
            ValueKind::Indices { base } => *base == base_id,
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            } => {
                if let ValueKind::Const(Lit::Int(1)) = &fn_ir.values[*rhs].kind {
                    return self.is_loop_induction(fn_ir, *lhs, base_id);
                }
                false
            }
            ValueKind::Phi { args } => args
                .iter()
                .any(|(id, _)| self.is_derived_from_len(fn_ir, *id, base_id, facts)),
            _ => false,
        }
    }

    pub(super) fn is_loop_induction(
        &self,
        fn_ir: &FnIR,
        val_id: ValueId,
        _base_id: ValueId,
    ) -> bool {
        let val = &fn_ir.values[val_id];
        if let ValueKind::Phi { args } = &val.kind {
            for (arg_id, _) in args {
                let arg_val = &fn_ir.values[*arg_id];
                if let ValueKind::Const(Lit::Int(0)) = &arg_val.kind {
                    return true;
                }
            }
        }
        false
    }
}
