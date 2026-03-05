use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo};
use crate::mir::{FnIR, Instr, Terminator, ValueId, ValueKind};

pub struct MirLoopOptimizer;

impl Default for MirLoopOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl MirLoopOptimizer {
    pub fn new() -> Self {
        Self
    }

    pub fn optimize(&self, fn_ir: &mut FnIR) -> bool {
        self.optimize_with_count(fn_ir) > 0
    }

    pub fn optimize_with_count(&self, fn_ir: &mut FnIR) -> usize {
        let mut count = 0usize;
        let loops = LoopAnalyzer::new(fn_ir).find_loops();
        for lp in loops {
            if self.canonicalize_loop(fn_ir, &lp) {
                count += 1;
            }
            if self.vectorize_loop(fn_ir, &lp) {
                count += 1;
            }
        }
        count
    }

    fn canonicalize_loop(&self, _fn_ir: &mut FnIR, _lp: &LoopInfo) -> bool {
        false
    }

    fn vectorize_loop(&self, fn_ir: &mut FnIR, lp: &LoopInfo) -> bool {
        // Vectorize only canonical loops.
        if lp.is_seq_len.is_none() {
            return false;
        }

        // Require a single body block plus header.
        if lp.body.len() != 2 {
            return false;
        } // Header + Body block

        // Find the body block (not the header)
        let body_bb = *lp.body.iter().find(|&&b| b != lp.header).unwrap();

        // Verify body block instructions
        // We look for: x[i] <- x[i] op y[i]
        // where i is the IV.
        let iv = match lp.iv.as_ref() {
            Some(iv) => iv.phi_val,
            None => return false,
        };

        let mut vectorized_instrs = Vec::new();
        let mut is_pure_vectorizable = true;

        // For simplicity: look at all StoreIndex1D in the body block
        let instrs = fn_ir.blocks[body_bb].instrs.clone();
        for instr in &instrs {
            match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    span,
                    ..
                } => {
                    if *idx == iv {
                        // Check if val is a Binary(Add, Index1D(A, iv), Index1D(B, iv))
                        let attempt = self.try_vectorize_value(fn_ir, *val, iv);
                        if let Some(transformed) = attempt {
                            vectorized_instrs.push(Instr::StoreIndex1D {
                                base: *base,
                                idx: iv,
                                val: transformed,
                                is_safe: true,
                                is_na_safe: false,
                                is_vector: true,
                                span: *span,
                            });
                        } else {
                            is_pure_vectorizable = false;
                        }
                    } else {
                        is_pure_vectorizable = false;
                    }
                }
                _ => is_pure_vectorizable = false,
            }
        }

        if !is_pure_vectorizable || vectorized_instrs.is_empty() {
            return false;
        }

        // 3. Construct new body block (vectorized)
        let new_body_bb = fn_ir.add_block();
        fn_ir.blocks[new_body_bb].instrs = vectorized_instrs;

        // 4. Update CFG
        // Header -> NewBody
        fn_ir.blocks[lp.header].term = Terminator::Goto(new_body_bb);

        // NewBody -> Exit
        let exit_bb = if !lp.exits.is_empty() {
            lp.exits[0]
        } else {
            return false;
        };
        fn_ir.blocks[new_body_bb].term = Terminator::Goto(exit_bb);

        // Ensure predecessors are updated?
        // simplify_cfg will handle reachability of old body.

        true
    }

    fn try_vectorize_value(
        &self,
        fn_ir: &mut FnIR,
        val_id: ValueId,
        iv_id: ValueId,
    ) -> Option<ValueId> {
        let val_kind = fn_ir.values[val_id].kind.clone();
        let span = fn_ir.values[val_id].span;
        let facts = fn_ir.values[val_id].facts;

        match val_kind {
            ValueKind::Binary { op, lhs, rhs } => {
                let v_lhs = self.try_vectorize_value(fn_ir, lhs, iv_id)?;
                let v_rhs = self.try_vectorize_value(fn_ir, rhs, iv_id)?;

                let new_id = fn_ir.add_value(
                    ValueKind::Binary {
                        op,
                        lhs: v_lhs,
                        rhs: v_rhs,
                    },
                    span,
                    facts,
                    None,
                );
                fn_ir.values[new_id].value_ty = fn_ir.values[val_id].value_ty;
                fn_ir.values[new_id].value_term = fn_ir.values[val_id].value_term.clone();
                Some(new_id)
            }
            ValueKind::Index1D { base, idx, .. } => {
                if idx == iv_id {
                    // Vectorization: Return the base array directly
                    // This transforms a[i] -> a
                    Some(base)
                } else {
                    None
                }
            }
            ValueKind::Const(_) => Some(val_id),
            _ => None,
        }
    }
}
