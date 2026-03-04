use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit};
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub header: BlockId,
    pub latch: BlockId,           // The block that jumps back to header
    pub exits: Vec<BlockId>,      // Blocks outside loop targeted by loop blocks
    pub body: FxHashSet<BlockId>, // All blocks in the loop

    pub is_seq_len: Option<ValueId>,   // If it's 1:N, stores N
    pub is_seq_along: Option<ValueId>, // If it's seq_along(X), stores X
    pub iv: Option<InductionVar>,
    pub limit: Option<ValueId>,
}

#[derive(Debug, Clone)]
pub struct InductionVar {
    pub phi_val: ValueId,
    pub init_val: ValueId,
    pub step: i64,      // +1, -1, etc.
    pub step_op: BinOp, // Add/Sub
}

pub struct LoopAnalyzer<'a> {
    fn_ir: &'a FnIR,
    preds: FxHashMap<BlockId, Vec<BlockId>>,
}

impl<'a> LoopAnalyzer<'a> {
    pub fn new(fn_ir: &'a FnIR) -> Self {
        let preds = build_pred_map(fn_ir);
        Self { fn_ir, preds }
    }

    pub fn find_loops(&self) -> Vec<LoopInfo> {
        // 1. Compute Dominators (Simplified for structured/reducible CFG)
        // For standard "natural loops", we look for back-edges A->B where B dominates A.
        // B is header, A is latch.

        let doms = self.compute_dominators();
        let mut loops = Vec::new();

        // 2. Find Back-edges
        for (src, targets) in self.get_cfg_edges() {
            for &dst in &targets {
                if self.dominates(&doms, dst, src) {
                    // Back-edge src -> dst
                    // dst is Header, src is Latch
                    if let Some(loop_info) = self.analyze_natural_loop(dst, src) {
                        loops.push(loop_info);
                    }
                }
            }
        }

        loops
    }

    fn analyze_natural_loop(&self, header: BlockId, latch: BlockId) -> Option<LoopInfo> {
        // Collect body blocks (Reach backwards from latch to header)
        let mut body = FxHashSet::default();
        let mut stack = vec![latch];
        body.insert(header);
        body.insert(latch);

        while let Some(node) = stack.pop() {
            if let Some(node_preds) = self.preds.get(&node) {
                for &pred in node_preds {
                    if !body.contains(&pred) {
                        body.insert(pred);
                        stack.push(pred);
                    }
                }
            }
        }

        // Find exits (successors of body blocks NOT in body)
        let mut exits = Vec::new();
        for &block in &body {
            let succs = self.get_block_successors(block);
            for succ in succs {
                if !body.contains(&succ) {
                    exits.push(succ);
                }
            }
        }

        // Analyze IV
        let (iv, limit) = self.find_induction_variable(header, latch, &exits);

        // Detect if it's 1:N (Canonical seq_len loop)
        let mut is_seq_len = None;
        let mut is_seq_along = None;
        if let (Some(iv_val), Some(limit_val)) = (&iv, limit) {
            let init_is_1 = match &self.fn_ir.values[iv_val.init_val].kind {
                ValueKind::Const(Lit::Int(1)) => true,
                _ => false,
            };

            if init_is_1 && iv_val.step == 1 && iv_val.step_op == BinOp::Add {
                // Check if condition is Le (<=)
                if let Terminator::If { cond, .. } = &self.fn_ir.blocks[header].term {
                    if let ValueKind::Binary { op: BinOp::Le, .. } = self.fn_ir.values[*cond].kind {
                        is_seq_len = Some(limit_val);

                        // NEW: Check if limit is length(X)
                        if let ValueKind::Len { base } = &self.fn_ir.values[limit_val].kind {
                            is_seq_along = Some(*base);
                        }
                    }
                }
            }
        }

        Some(LoopInfo {
            header,
            latch,
            body,
            exits,
            iv,
            limit,
            is_seq_len,
            is_seq_along,
        })
    }

    fn find_induction_variable(
        &self,
        header: BlockId,
        latch: BlockId,
        _exits: &[BlockId],
    ) -> (Option<InductionVar>, Option<ValueId>) {
        let mut iv_candidate: Option<InductionVar> = None;

        for (val_id, val) in self.fn_ir.values.iter().enumerate() {
            if let ValueKind::Phi { args } = &val.kind {
                let has_latch_input = args.iter().any(|(_, b)| *b == latch);
                if has_latch_input && args.len() == 2 {
                    let (next_val, _) = args.iter().find(|(_, b)| *b == latch).unwrap();
                    let (init_val, _) = args.iter().find(|(_, b)| *b != latch).unwrap();

                    if let Some(step_info) = self.analyze_step(*next_val, val_id) {
                        iv_candidate = Some(InductionVar {
                            phi_val: val_id,
                            init_val: *init_val,
                            step: step_info.0,
                            step_op: step_info.1,
                        });
                        break;
                    }
                }
            }
        }

        let mut limit: Option<ValueId> = None;
        if let Some(iv) = &iv_candidate {
            if let Terminator::If { cond, .. } = &self.fn_ir.blocks[header].term {
                let cond_val = &self.fn_ir.values[*cond];
                if let ValueKind::Binary { op: _, lhs, rhs } = &cond_val.kind {
                    if *lhs == iv.phi_val {
                        limit = Some(*rhs);
                    } else if *rhs == iv.phi_val {
                        limit = Some(*lhs);
                    }
                }
            }
        }

        (iv_candidate, limit)
    }

    fn analyze_step(&self, val_id: ValueId, phi_id: ValueId) -> Option<(i64, BinOp)> {
        let val = &self.fn_ir.values[val_id];
        if let ValueKind::Binary { op, lhs, rhs } = &val.kind {
            let simple_const = |vid: ValueId| -> Option<i64> {
                if let ValueKind::Const(Lit::Int(n)) = &self.fn_ir.values[vid].kind {
                    Some(*n)
                } else {
                    None
                }
            };

            if *lhs == phi_id {
                if let Some(n) = simple_const(*rhs) {
                    return Some((n, *op));
                }
            }
            if *op == BinOp::Add && *rhs == phi_id {
                if let Some(n) = simple_const(*lhs) {
                    return Some((n, *op));
                }
            }
        }
        None
    }

    // Helpers
    fn get_cfg_edges(&self) -> Vec<(BlockId, Vec<BlockId>)> {
        self.fn_ir
            .blocks
            .iter()
            .map(|b| {
                let succs = self.get_block_successors(b.id);
                (b.id, succs)
            })
            .collect()
    }

    fn get_block_successors(&self, bid: BlockId) -> Vec<BlockId> {
        match &self.fn_ir.blocks[bid].term {
            Terminator::Goto(t) => vec![*t],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            _ => vec![],
        }
    }

    fn compute_dominators(&self) -> FxHashMap<BlockId, FxHashSet<BlockId>> {
        // Naive Iterative Dominators
        // Dom(n) = {n} U (Inter(Dom(p)) for p in preds(n))
        // Init: Dom(entry) = {entry}, Dom(others) = All Blocks

        let all_blocks: FxHashSet<BlockId> = (0..self.fn_ir.blocks.len()).collect();
        let mut doms: FxHashMap<BlockId, FxHashSet<BlockId>> = FxHashMap::default();

        // Init
        doms.insert(
            self.fn_ir.entry,
            std::iter::once(self.fn_ir.entry).collect(),
        );
        for b in &all_blocks {
            if *b != self.fn_ir.entry {
                doms.insert(*b, all_blocks.clone());
            }
        }

        let mut changed = true;
        while changed {
            changed = false;
            for bb in 0..self.fn_ir.blocks.len() {
                if bb == self.fn_ir.entry {
                    continue;
                }

                let preds = self.preds.get(&bb).cloned().unwrap_or_default();
                if preds.is_empty() {
                    continue;
                } // Unreachable

                // Intersect preds
                let mut new_dom: Option<FxHashSet<BlockId>> = None;
                for p in preds {
                    if let Some(p_dom) = doms.get(&p) {
                        match new_dom {
                            None => new_dom = Some(p_dom.clone()),
                            Some(ref mut set) => set.retain(|x| p_dom.contains(x)),
                        }
                    }
                }

                if let Some(mut set) = new_dom {
                    set.insert(bb);
                    if set != *doms.get(&bb).unwrap() {
                        doms.insert(bb, set);
                        changed = true;
                    }
                }
            }
        }
        doms
    }

    fn dominates(
        &self,
        doms: &FxHashMap<BlockId, FxHashSet<BlockId>>,
        master: BlockId,
        slave: BlockId,
    ) -> bool {
        if let Some(set) = doms.get(&slave) {
            set.contains(&master)
        } else {
            false
        }
    }
}

pub fn build_pred_map(fn_ir: &FnIR) -> FxHashMap<BlockId, Vec<BlockId>> {
    let mut map = FxHashMap::default();
    for (src, blk) in fn_ir.blocks.iter().enumerate() {
        let targets = match &blk.term {
            Terminator::Goto(t) => vec![*t],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            _ => vec![],
        };
        for t in targets {
            map.entry(t).or_insert_with(Vec::new).push(src);
        }
    }
    map
}
