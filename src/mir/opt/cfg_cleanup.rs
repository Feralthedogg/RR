use super::TachyonEngine;
use crate::mir::*;
use rustc_hash::FxHashSet;

impl TachyonEngine {
    pub(super) fn simplify_cfg(&self, fn_ir: &mut FnIR) -> bool {
        let mut changed = false;

        let mut reachable = FxHashSet::default();
        let mut queue = vec![fn_ir.entry];
        reachable.insert(fn_ir.entry);

        let mut head = 0;
        while head < queue.len() {
            let bid = queue[head];
            head += 1;

            if let Some(blk) = fn_ir.blocks.get(bid) {
                match &blk.term {
                    Terminator::Goto(target) => {
                        if reachable.insert(*target) {
                            queue.push(*target);
                        }
                    }
                    Terminator::If {
                        then_bb, else_bb, ..
                    } => {
                        if reachable.insert(*then_bb) {
                            queue.push(*then_bb);
                        }
                        if reachable.insert(*else_bb) {
                            queue.push(*else_bb);
                        }
                    }
                    _ => {}
                }
            }
        }

        for bid in 0..fn_ir.blocks.len() {
            if !reachable.contains(&bid) {
                let blk = &mut fn_ir.blocks[bid];
                if !blk.instrs.is_empty() || !matches!(blk.term, Terminator::Unreachable) {
                    blk.instrs.clear();
                    blk.term = Terminator::Unreachable;
                    changed = true;
                }
            }
        }

        changed
    }

    pub(super) fn dce(&self, fn_ir: &mut FnIR) -> bool {
        let reachable = Self::reachable_blocks(fn_ir);
        let mut live_in: Vec<FxHashSet<VarId>> = vec![FxHashSet::default(); fn_ir.blocks.len()];
        let mut live_out: Vec<FxHashSet<VarId>> = vec![FxHashSet::default(); fn_ir.blocks.len()];

        let mut dataflow_changed = true;
        while dataflow_changed {
            dataflow_changed = false;
            for bid in (0..fn_ir.blocks.len()).rev() {
                if !reachable.contains(&bid) {
                    continue;
                }

                let succ_live = Self::successor_live_vars(fn_ir, bid, &live_in);
                let block_live = self.compute_block_live_in(fn_ir, bid, &succ_live, &fn_ir.values);
                if live_out[bid] != succ_live {
                    live_out[bid] = succ_live.clone();
                    dataflow_changed = true;
                }
                if live_in[bid] != block_live {
                    live_in[bid] = block_live;
                    dataflow_changed = true;
                }
            }
        }

        let mut changed = false;
        for bid in 0..fn_ir.blocks.len() {
            if !reachable.contains(&bid) {
                continue;
            }
            let mut live = Self::successor_live_vars(fn_ir, bid, &live_in);
            self.collect_term_live_vars(&fn_ir.blocks[bid].term, &fn_ir.values, &mut live);

            let mut new_instrs_rev = Vec::with_capacity(fn_ir.blocks[bid].instrs.len());
            for instr in fn_ir.blocks[bid].instrs.iter().rev() {
                match instr {
                    Instr::Assign { dst, src, span } => {
                        let pinned = Self::is_pinned_live_var(dst);
                        let removable = Self::can_eliminate_assign_dst(dst);
                        if pinned || !removable || live.remove(dst) {
                            self.collect_value_live_vars(*src, &fn_ir.values, &mut live);
                            new_instrs_rev.push(instr.clone());
                        } else if self.has_side_effect_val(*src, &fn_ir.values) {
                            self.collect_value_live_vars(*src, &fn_ir.values, &mut live);
                            new_instrs_rev.push(Instr::Eval {
                                val: *src,
                                span: *span,
                            });
                            changed = true;
                        } else {
                            changed = true;
                        }
                    }
                    Instr::Eval { val, .. } => {
                        if self.has_side_effect_val(*val, &fn_ir.values) {
                            self.collect_value_live_vars(*val, &fn_ir.values, &mut live);
                            new_instrs_rev.push(instr.clone());
                        } else {
                            changed = true;
                        }
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        self.collect_value_live_vars(*base, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*idx, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*val, &fn_ir.values, &mut live);
                        new_instrs_rev.push(instr.clone());
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        self.collect_value_live_vars(*base, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*r, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*c, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*val, &fn_ir.values, &mut live);
                        new_instrs_rev.push(instr.clone());
                    }
                    Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        self.collect_value_live_vars(*base, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*i, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*j, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*k, &fn_ir.values, &mut live);
                        self.collect_value_live_vars(*val, &fn_ir.values, &mut live);
                        new_instrs_rev.push(instr.clone());
                    }
                }
            }
            new_instrs_rev.reverse();
            if fn_ir.blocks[bid].instrs != new_instrs_rev {
                fn_ir.blocks[bid].instrs = new_instrs_rev;
                changed = true;
            }
        }

        changed
    }

    pub(super) fn has_side_effect_instr(&self, instr: &Instr, values: &[Value]) -> bool {
        match instr {
            Instr::StoreIndex1D { .. } => true,
            Instr::StoreIndex2D { .. } => true,
            Instr::StoreIndex3D { .. } => true,
            Instr::Assign { src, .. } => self.has_side_effect_val(*src, values),
            Instr::Eval { val, .. } => self.has_side_effect_val(*val, values),
        }
    }

    pub(super) fn reachable_blocks(fn_ir: &FnIR) -> FxHashSet<BlockId> {
        let mut reachable = FxHashSet::default();
        let mut queue = vec![fn_ir.entry];
        reachable.insert(fn_ir.entry);
        let mut head = 0;
        while head < queue.len() {
            let bid = queue[head];
            head += 1;
            for succ in Self::block_successors(fn_ir, bid) {
                if reachable.insert(succ) {
                    queue.push(succ);
                }
            }
        }
        reachable
    }

    pub(super) fn block_successors(fn_ir: &FnIR, bid: BlockId) -> Vec<BlockId> {
        match &fn_ir.blocks[bid].term {
            Terminator::Goto(target) => vec![*target],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
        }
    }

    pub(super) fn successor_live_vars(
        fn_ir: &FnIR,
        bid: BlockId,
        live_in: &[FxHashSet<VarId>],
    ) -> FxHashSet<VarId> {
        let mut live = FxHashSet::default();
        for succ in Self::block_successors(fn_ir, bid) {
            live.extend(live_in[succ].iter().cloned());
        }
        live
    }

    pub(super) fn compute_block_live_in(
        &self,
        fn_ir: &FnIR,
        bid: BlockId,
        succ_live: &FxHashSet<VarId>,
        values: &[Value],
    ) -> FxHashSet<VarId> {
        let blk = &fn_ir.blocks[bid];
        let mut live = succ_live.clone();
        self.collect_term_live_vars(&blk.term, values, &mut live);
        for instr in blk.instrs.iter().rev() {
            match instr {
                Instr::Assign { dst, src, .. } => {
                    let removable = Self::can_eliminate_assign_dst(dst);
                    if Self::is_pinned_live_var(dst)
                        || !removable
                        || live.remove(dst)
                        || self.has_side_effect_val(*src, values)
                    {
                        self.collect_value_live_vars(*src, values, &mut live);
                    }
                }
                Instr::Eval { val, .. } => {
                    if self.has_side_effect_val(*val, values) {
                        self.collect_value_live_vars(*val, values, &mut live);
                    }
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    self.collect_value_live_vars(*base, values, &mut live);
                    self.collect_value_live_vars(*idx, values, &mut live);
                    self.collect_value_live_vars(*val, values, &mut live);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    self.collect_value_live_vars(*base, values, &mut live);
                    self.collect_value_live_vars(*r, values, &mut live);
                    self.collect_value_live_vars(*c, values, &mut live);
                    self.collect_value_live_vars(*val, values, &mut live);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    self.collect_value_live_vars(*base, values, &mut live);
                    self.collect_value_live_vars(*i, values, &mut live);
                    self.collect_value_live_vars(*j, values, &mut live);
                    self.collect_value_live_vars(*k, values, &mut live);
                    self.collect_value_live_vars(*val, values, &mut live);
                }
            }
        }
        live
    }

    pub(super) fn collect_term_live_vars(
        &self,
        term: &Terminator,
        values: &[Value],
        live: &mut FxHashSet<VarId>,
    ) {
        match term {
            Terminator::If { cond, .. } => self.collect_value_live_vars(*cond, values, live),
            Terminator::Return(Some(val)) => self.collect_value_live_vars(*val, values, live),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    pub(super) fn collect_value_live_vars(
        &self,
        root: ValueId,
        values: &[Value],
        live: &mut FxHashSet<VarId>,
    ) {
        let mut stack = vec![root];
        let mut seen = FxHashSet::default();
        while let Some(vid) = stack.pop() {
            if !seen.insert(vid) {
                continue;
            }
            if let Some(origin_var) = &values[vid].origin_var {
                live.insert(origin_var.clone());
            }
            match &values[vid].kind {
                ValueKind::Load { var } => {
                    live.insert(var.clone());
                }
                ValueKind::Binary { lhs, rhs, .. } => {
                    stack.push(*lhs);
                    stack.push(*rhs);
                }
                ValueKind::Unary { rhs, .. } => stack.push(*rhs),
                ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                    stack.extend(args.iter().copied());
                }
                ValueKind::RecordLit { fields } => {
                    stack.extend(fields.iter().map(|(_, value)| *value));
                }
                ValueKind::FieldGet { base, .. } => stack.push(*base),
                ValueKind::FieldSet { base, value, .. } => {
                    stack.push(*base);
                    stack.push(*value);
                }
                ValueKind::Index1D { base, idx, .. } => {
                    stack.push(*base);
                    stack.push(*idx);
                }
                ValueKind::Index2D { base, r, c } => {
                    stack.push(*base);
                    stack.push(*r);
                    stack.push(*c);
                }
                ValueKind::Index3D { base, i, j, k } => {
                    stack.push(*base);
                    stack.push(*i);
                    stack.push(*j);
                    stack.push(*k);
                }
                ValueKind::Range { start, end } => {
                    stack.push(*start);
                    stack.push(*end);
                }
                ValueKind::Len { base } | ValueKind::Indices { base } => stack.push(*base),
                ValueKind::Phi { args } => {
                    stack.extend(args.iter().map(|(arg, _)| *arg));
                }
                ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => {}
            }
        }
    }

    pub(super) fn is_pinned_live_var(var: &str) -> bool {
        var.starts_with(".arg_")
    }

    pub(super) fn can_eliminate_assign_dst(var: &str) -> bool {
        var.starts_with(".tachyon_") || var.starts_with(".__rr_") || var.starts_with("inlined_")
    }

    pub(super) fn has_side_effect_val(&self, val_id: ValueId, values: &[Value]) -> bool {
        let val = &values[val_id];
        match &val.kind {
            ValueKind::Call { callee, .. } => {
                let pure = [
                    "length",
                    "c",
                    "seq_along",
                    "list",
                    "sum",
                    "mean",
                    "min",
                    "max",
                    "rr_field_get",
                    "rr_named_list",
                ];
                if !pure.contains(&callee.as_str()) {
                    return true;
                }
                value_dependencies(&val.kind)
                    .into_iter()
                    .any(|dep| self.has_side_effect_val(dep, values))
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                self.has_side_effect_val(*lhs, values) || self.has_side_effect_val(*rhs, values)
            }
            ValueKind::Unary { rhs, .. } => self.has_side_effect_val(*rhs, values),
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .any(|arg| self.has_side_effect_val(*arg, values)),
            ValueKind::RecordLit { fields } => fields
                .iter()
                .any(|(_, value)| self.has_side_effect_val(*value, values)),
            ValueKind::FieldGet { base, .. } => self.has_side_effect_val(*base, values),
            ValueKind::FieldSet { base, value, .. } => {
                self.has_side_effect_val(*base, values) || self.has_side_effect_val(*value, values)
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                self.has_side_effect_val(*base, values)
            }
            ValueKind::Range { start, end } => {
                self.has_side_effect_val(*start, values) || self.has_side_effect_val(*end, values)
            }
            ValueKind::Index1D { base, idx, .. } => {
                self.has_side_effect_val(*base, values) || self.has_side_effect_val(*idx, values)
            }
            ValueKind::Index2D { base, r, c } => {
                self.has_side_effect_val(*base, values)
                    || self.has_side_effect_val(*r, values)
                    || self.has_side_effect_val(*c, values)
            }
            ValueKind::Index3D { base, i, j, k } => {
                self.has_side_effect_val(*base, values)
                    || self.has_side_effect_val(*i, values)
                    || self.has_side_effect_val(*j, values)
                    || self.has_side_effect_val(*k, values)
            }
            ValueKind::Phi { args } => args
                .iter()
                .any(|(arg, _)| self.has_side_effect_val(*arg, values)),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::flow::Facts;
    use crate::syntax::ast::Lit;
    use crate::utils::Span;

    fn one_block_fn(name: &str) -> FnIR {
        let mut f = FnIR::new(name.to_string(), vec![]);
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;
        f
    }

    #[test]
    fn dce_preserves_eval_with_nested_side_effect_inside_pure_call() {
        let mut fn_ir = one_block_fn("dce_nested_pure_call_eval");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Call {
                callee: "length".to_string(),
                args: vec![impure],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(
            !changed,
            "the eval should stay live rather than being deleted or rewritten"
        );
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_demotes_dead_assign_with_nested_record_field_side_effect_to_eval() {
        let mut fn_ir = one_block_fn("dce_nested_record_field_assign");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let record = fn_ir.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), impure)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let field = fn_ir.add_value(
            ValueKind::FieldGet {
                base: record,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: ".tachyon_dead".to_string(),
            src: field,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed, "dead assign should be rewritten to eval, not dropped");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == field
        ));
    }

    #[test]
    fn dce_preserves_eval_with_nested_side_effect_inside_intrinsic() {
        let mut fn_ir = one_block_fn("dce_nested_intrinsic_eval");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecAbsF64,
                args: vec![impure],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(
            !changed,
            "the eval should stay live when an intrinsic argument has side effects"
        );
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_demotes_dead_assign_with_nested_fieldset_side_effect_to_eval() {
        let mut fn_ir = one_block_fn("dce_nested_fieldset_assign");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let record = fn_ir.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), one)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let updated = fn_ir.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "x".to_string(),
                value: impure,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: ".tachyon_dead".to_string(),
            src: updated,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed, "dead assign should be rewritten to eval, not dropped");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == updated
        ));
    }

    #[test]
    fn dce_preserves_eval_with_nested_side_effect_inside_index1d() {
        let mut fn_ir = one_block_fn("dce_nested_index1d_eval");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Index1D {
                base: one,
                idx: impure,
                is_safe: false,
                is_na_safe: false,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(
            !changed,
            "the eval should stay live when an Index1D operand has side effects"
        );
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_demotes_dead_assign_with_nested_index1d_side_effect_to_eval() {
        let mut fn_ir = one_block_fn("dce_nested_index1d_assign");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Index1D {
                base: one,
                idx: impure,
                is_safe: false,
                is_na_safe: false,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: ".tachyon_dead".to_string(),
            src: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed, "dead assign should be rewritten to eval, not dropped");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_preserves_eval_with_nested_side_effect_inside_phi() {
        let mut fn_ir = FnIR::new("dce_nested_phi_eval".to_string(), vec![]);
        let entry = fn_ir.add_block();
        let left = fn_ir.add_block();
        let right = fn_ir.add_block();
        let merge = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(impure, left), (one, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        fn_ir.values[phi].phi_block = Some(merge);

        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        fn_ir.blocks[left].term = Terminator::Goto(merge);
        fn_ir.blocks[right].term = Terminator::Goto(merge);
        fn_ir.blocks[merge].instrs.push(Instr::Eval {
            val: phi,
            span: Span::default(),
        });
        fn_ir.blocks[merge].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(
            !changed,
            "the eval should stay live when a Phi arm has side effects"
        );
        assert!(matches!(
            fn_ir.blocks[merge].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == phi
        ));
    }

    #[test]
    fn dce_demotes_dead_assign_with_nested_phi_side_effect_to_eval() {
        let mut fn_ir = FnIR::new("dce_nested_phi_assign".to_string(), vec![]);
        let entry = fn_ir.add_block();
        let left = fn_ir.add_block();
        let right = fn_ir.add_block();
        let merge = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(impure, left), (one, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        fn_ir.values[phi].phi_block = Some(merge);

        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        fn_ir.blocks[left].term = Terminator::Goto(merge);
        fn_ir.blocks[right].term = Terminator::Goto(merge);
        fn_ir.blocks[merge].instrs.push(Instr::Assign {
            dst: ".tachyon_dead".to_string(),
            src: phi,
            span: Span::default(),
        });
        fn_ir.blocks[merge].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed, "dead assign should be rewritten to eval, not dropped");
        assert!(matches!(
            fn_ir.blocks[merge].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == phi
        ));
    }

    #[test]
    fn dce_preserves_eval_with_nested_side_effect_inside_len() {
        let mut fn_ir = one_block_fn("dce_nested_len_eval");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Len { base: impure },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(
            !changed,
            "the eval should stay live when a Len base has side effects"
        );
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_demotes_dead_assign_with_nested_range_side_effect_to_eval() {
        let mut fn_ir = one_block_fn("dce_nested_range_assign");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Range {
                start: impure,
                end: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: ".tachyon_dead".to_string(),
            src: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed, "dead assign should be rewritten to eval, not dropped");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_preserves_eval_with_nested_side_effect_inside_indices() {
        let mut fn_ir = one_block_fn("dce_nested_indices_eval");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Indices { base: impure },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(
            !changed,
            "the eval should stay live when an Indices base has side effects"
        );
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_demotes_dead_assign_with_nested_indices_side_effect_to_eval() {
        let mut fn_ir = one_block_fn("dce_nested_indices_assign");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Indices { base: impure },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: ".tachyon_dead".to_string(),
            src: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed, "dead assign should be rewritten to eval, not dropped");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_preserves_eval_with_nested_side_effect_inside_index2d() {
        let mut fn_ir = one_block_fn("dce_nested_index2d_eval");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Index2D {
                base: one,
                r: impure,
                c: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(
            !changed,
            "the eval should stay live when an Index2D operand has side effects"
        );
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_demotes_dead_assign_with_nested_index2d_side_effect_to_eval() {
        let mut fn_ir = one_block_fn("dce_nested_index2d_assign");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Index2D {
                base: one,
                r: impure,
                c: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: ".tachyon_dead".to_string(),
            src: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed, "dead assign should be rewritten to eval, not dropped");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_preserves_eval_with_nested_side_effect_inside_index3d() {
        let mut fn_ir = one_block_fn("dce_nested_index3d_eval");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Index3D {
                base: one,
                i: impure,
                j: one,
                k: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(
            !changed,
            "the eval should stay live when an Index3D operand has side effects"
        );
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }

    #[test]
    fn dce_demotes_dead_assign_with_nested_index3d_side_effect_to_eval() {
        let mut fn_ir = one_block_fn("dce_nested_index3d_assign");
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let wrapped = fn_ir.add_value(
            ValueKind::Index3D {
                base: one,
                i: impure,
                j: one,
                k: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: ".tachyon_dead".to_string(),
            src: wrapped,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(zero));

        let changed = TachyonEngine::new().dce(&mut fn_ir);
        assert!(changed, "dead assign should be rewritten to eval, not dropped");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].instrs.as_slice(),
            [Instr::Eval { val, .. }] if *val == wrapped
        ));
    }
}
