use super::*;
pub(crate) fn has_reachable_assignment_to_var_after(
    fn_ir: &FnIR,
    start: BlockId,
    var: &str,
) -> bool {
    let mut seen = FxHashSet::default();
    let mut stack = vec![start];
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        let Some(block) = fn_ir.blocks.get(bid) else {
            continue;
        };
        for instr in &block.instrs {
            match instr {
                Instr::Assign { dst, .. } if dst == var => return true,
                Instr::StoreIndex1D { base, .. }
                | Instr::StoreIndex2D { base, .. }
                | Instr::StoreIndex3D { base, .. } => {
                    if resolve_base_var(fn_ir, *base).as_deref() == Some(var) {
                        return true;
                    }
                }
                Instr::Assign { .. } | Instr::Eval { .. } | Instr::UnsafeRBlock { .. } => {}
            }
        }
        match block.term {
            Terminator::Goto(next) => stack.push(next),
            Terminator::If {
                then_bb, else_bb, ..
            } => {
                stack.push(then_bb);
                stack.push(else_bb);
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }
    false
}

pub(crate) fn rewrite_reachable_value_uses_for_var_after(
    fn_ir: &mut FnIR,
    start: BlockId,
    var: &str,
    replacement: ValueId,
) {
    let mut seen = FxHashSet::default();
    let mut stack = vec![start];
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        let succs = reachable_successors_after(fn_ir, bid);
        let mut memo = FxHashMap::default();
        let mut visiting = FxHashSet::default();
        rewrite_block_instrs_after(fn_ir, bid, var, replacement, &mut memo, &mut visiting);
        rewrite_block_term_after(fn_ir, bid, var, replacement, &mut memo, &mut visiting);
        stack.extend(succs);
    }
}

pub(crate) struct VersionedExitValueRewriter<'a> {
    pub(crate) fn_ir: &'a mut FnIR,
    pub(crate) var: &'a str,
    pub(crate) replacement: ValueId,
    pub(crate) memo: &'a mut FxHashMap<ValueId, ValueId>,
    pub(crate) visiting: &'a mut FxHashSet<ValueId>,
}

impl VersionedExitValueRewriter<'_> {
    pub(crate) fn rewrite(&mut self, root: ValueId) -> ValueId {
        let root = canonical_value(self.fn_ir, root);
        if let Some(mapped) = self.memo.get(&root) {
            return *mapped;
        }
        if !self.visiting.insert(root) {
            return root;
        }

        let mapped = self.rewrite_kind(root, self.fn_ir.values[root].kind.clone());
        self.visiting.remove(&root);
        self.memo.insert(root, mapped);
        mapped
    }

    pub(crate) fn rewrite_kind(&mut self, root: ValueId, kind: ValueKind) -> ValueId {
        match kind {
            ValueKind::Phi { .. }
                if self.fn_ir.values[root].origin_var.as_deref() == Some(self.var) =>
            {
                self.replacement
            }
            ValueKind::Load { .. }
            | ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::RSymbol { .. } => root,
            ValueKind::Unary { op, rhs } => self.rewrite_unary(root, op, rhs),
            ValueKind::Binary { op, lhs, rhs } => self.rewrite_binary(root, op, lhs, rhs),
            ValueKind::RecordLit { fields } => self.rewrite_record_lit(root, fields),
            ValueKind::FieldGet { base, field } => self.rewrite_field_get(root, base, field),
            ValueKind::FieldSet { base, field, value } => {
                self.rewrite_field_set(root, base, field, value)
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => self.rewrite_call(root, callee, args, names),
            ValueKind::Intrinsic { op, args } => self.rewrite_intrinsic(root, op, args),
            ValueKind::Phi { args } => self.rewrite_phi(root, args),
            ValueKind::Len { base } => {
                self.rewrite_single_base(root, base, |base| ValueKind::Len { base })
            }
            ValueKind::Indices { base } => {
                self.rewrite_single_base(root, base, |base| ValueKind::Indices { base })
            }
            ValueKind::Range { start, end } => self.rewrite_range(root, start, end),
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => self.rewrite_index1d(
                root,
                base,
                idx,
                IndexRewriteSafety {
                    is_safe,
                    is_na_safe,
                },
            ),
            ValueKind::Index2D { base, r, c } => self.rewrite_index2d(root, base, r, c),
            ValueKind::Index3D { base, i, j, k } => self.rewrite_index3d(root, base, i, j, k),
        }
    }

    pub(crate) fn add_rewritten_value(&mut self, root: ValueId, kind: ValueKind) -> ValueId {
        self.fn_ir.add_value(
            kind,
            self.fn_ir.values[root].span,
            self.fn_ir.values[root].facts,
            self.fn_ir.values[root].origin_var.clone(),
        )
    }

    pub(crate) fn rewrite_unary(&mut self, root: ValueId, op: UnaryOp, rhs: ValueId) -> ValueId {
        let rhs_new = self.rewrite(rhs);
        if rhs_new == rhs {
            root
        } else {
            self.add_rewritten_value(root, ValueKind::Unary { op, rhs: rhs_new })
        }
    }

    pub(crate) fn rewrite_binary(
        &mut self,
        root: ValueId,
        op: BinOp,
        lhs: ValueId,
        rhs: ValueId,
    ) -> ValueId {
        let lhs_new = self.rewrite(lhs);
        let rhs_new = self.rewrite(rhs);
        if lhs_new == lhs && rhs_new == rhs {
            root
        } else {
            self.add_rewritten_value(
                root,
                ValueKind::Binary {
                    op,
                    lhs: lhs_new,
                    rhs: rhs_new,
                },
            )
        }
    }

    pub(crate) fn rewrite_record_lit(
        &mut self,
        root: ValueId,
        fields: Vec<(String, ValueId)>,
    ) -> ValueId {
        let new_fields = fields
            .iter()
            .map(|(name, value)| (name.clone(), self.rewrite(*value)))
            .collect::<Vec<_>>();
        if new_fields == fields {
            root
        } else {
            self.add_rewritten_value(root, ValueKind::RecordLit { fields: new_fields })
        }
    }

    pub(crate) fn rewrite_field_get(
        &mut self,
        root: ValueId,
        base: ValueId,
        field: String,
    ) -> ValueId {
        let base_new = self.rewrite(base);
        if base_new == base {
            root
        } else {
            self.add_rewritten_value(
                root,
                ValueKind::FieldGet {
                    base: base_new,
                    field,
                },
            )
        }
    }

    pub(crate) fn rewrite_field_set(
        &mut self,
        root: ValueId,
        base: ValueId,
        field: String,
        value: ValueId,
    ) -> ValueId {
        let base_new = self.rewrite(base);
        let value_new = self.rewrite(value);
        if base_new == base && value_new == value {
            root
        } else {
            self.add_rewritten_value(
                root,
                ValueKind::FieldSet {
                    base: base_new,
                    field,
                    value: value_new,
                },
            )
        }
    }

    pub(crate) fn rewrite_call(
        &mut self,
        root: ValueId,
        callee: String,
        args: Vec<ValueId>,
        names: Vec<Option<String>>,
    ) -> ValueId {
        let new_args = self.rewrite_args(&args);
        if new_args == args {
            root
        } else {
            self.add_rewritten_value(
                root,
                ValueKind::Call {
                    callee,
                    args: new_args,
                    names,
                },
            )
        }
    }

    pub(crate) fn rewrite_intrinsic(
        &mut self,
        root: ValueId,
        op: IntrinsicOp,
        args: Vec<ValueId>,
    ) -> ValueId {
        let new_args = self.rewrite_args(&args);
        if new_args == args {
            root
        } else {
            self.add_rewritten_value(root, ValueKind::Intrinsic { op, args: new_args })
        }
    }

    pub(crate) fn rewrite_phi(&mut self, root: ValueId, args: Vec<(ValueId, BlockId)>) -> ValueId {
        let new_args = args
            .iter()
            .map(|(arg, bid)| (self.rewrite(*arg), *bid))
            .collect::<Vec<_>>();
        if new_args == args {
            return root;
        }

        let phi = self.add_rewritten_value(root, ValueKind::Phi { args: new_args });
        self.fn_ir.values[phi].phi_block = self.fn_ir.values[root].phi_block;
        phi
    }

    pub(crate) fn rewrite_single_base(
        &mut self,
        root: ValueId,
        base: ValueId,
        build: fn(ValueId) -> ValueKind,
    ) -> ValueId {
        let base_new = self.rewrite(base);
        if base_new == base {
            root
        } else {
            self.add_rewritten_value(root, build(base_new))
        }
    }

    pub(crate) fn rewrite_range(&mut self, root: ValueId, start: ValueId, end: ValueId) -> ValueId {
        let start_new = self.rewrite(start);
        let end_new = self.rewrite(end);
        if start_new == start && end_new == end {
            root
        } else {
            self.add_rewritten_value(
                root,
                ValueKind::Range {
                    start: start_new,
                    end: end_new,
                },
            )
        }
    }

    pub(crate) fn rewrite_index1d(
        &mut self,
        root: ValueId,
        base: ValueId,
        idx: ValueId,
        safety: IndexRewriteSafety,
    ) -> ValueId {
        let base_new = self.rewrite(base);
        let idx_new = self.rewrite(idx);
        if base_new == base && idx_new == idx {
            root
        } else {
            self.add_rewritten_value(
                root,
                ValueKind::Index1D {
                    base: base_new,
                    idx: idx_new,
                    is_safe: safety.is_safe,
                    is_na_safe: safety.is_na_safe,
                },
            )
        }
    }

    pub(crate) fn rewrite_index2d(
        &mut self,
        root: ValueId,
        base: ValueId,
        row: ValueId,
        col: ValueId,
    ) -> ValueId {
        let base_new = self.rewrite(base);
        let row_new = self.rewrite(row);
        let col_new = self.rewrite(col);
        if base_new == base && row_new == row && col_new == col {
            root
        } else {
            self.add_rewritten_value(
                root,
                ValueKind::Index2D {
                    base: base_new,
                    r: row_new,
                    c: col_new,
                },
            )
        }
    }

    pub(crate) fn rewrite_index3d(
        &mut self,
        root: ValueId,
        base: ValueId,
        dim1: ValueId,
        dim2: ValueId,
        dim3: ValueId,
    ) -> ValueId {
        let base_new = self.rewrite(base);
        let dim1_new = self.rewrite(dim1);
        let dim2_new = self.rewrite(dim2);
        let dim3_new = self.rewrite(dim3);
        if base_new == base && dim1_new == dim1 && dim2_new == dim2 && dim3_new == dim3 {
            root
        } else {
            self.add_rewritten_value(
                root,
                ValueKind::Index3D {
                    base: base_new,
                    i: dim1_new,
                    j: dim2_new,
                    k: dim3_new,
                },
            )
        }
    }

    pub(crate) fn rewrite_args(&mut self, args: &[ValueId]) -> Vec<ValueId> {
        args.iter().map(|arg| self.rewrite(*arg)).collect()
    }
}

pub(crate) fn rewrite_value_tree_for_var_after(
    fn_ir: &mut FnIR,
    root: ValueId,
    var: &str,
    replacement: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
) -> ValueId {
    VersionedExitValueRewriter {
        fn_ir,
        var,
        replacement,
        memo,
        visiting,
    }
    .rewrite(root)
}

pub(crate) struct VersionedExitRewriteEnv<'a> {
    pub(crate) var: &'a str,
    pub(crate) replacement: ValueId,
    pub(crate) memo: &'a mut FxHashMap<ValueId, ValueId>,
    pub(crate) visiting: &'a mut FxHashSet<ValueId>,
}

impl VersionedExitRewriteEnv<'_> {
    pub(crate) fn rewrite_value(&mut self, fn_ir: &mut FnIR, root: ValueId) -> ValueId {
        rewrite_value_tree_for_var_after(
            fn_ir,
            root,
            self.var,
            self.replacement,
            self.memo,
            self.visiting,
        )
    }
}

pub(crate) fn reachable_successors_after(fn_ir: &FnIR, bid: BlockId) -> Vec<BlockId> {
    let Some(block) = fn_ir.blocks.get(bid) else {
        return Vec::new();
    };
    match block.term {
        Terminator::Goto(next) => vec![next],
        Terminator::If {
            then_bb, else_bb, ..
        } => vec![then_bb, else_bb],
        Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
    }
}

pub(crate) fn rewrite_block_instrs_after(
    fn_ir: &mut FnIR,
    bid: BlockId,
    var: &str,
    replacement: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
) {
    let mut rewrite = VersionedExitRewriteEnv {
        var,
        replacement,
        memo,
        visiting,
    };
    let instrs = fn_ir.blocks[bid].instrs.clone();
    let mut new_instrs = Vec::with_capacity(instrs.len());
    for mut instr in instrs {
        rewrite_instr_value_uses_after(fn_ir, &mut instr, &mut rewrite);
        new_instrs.push(instr);
    }
    fn_ir.blocks[bid].instrs = new_instrs;
}

pub(crate) fn rewrite_instr_value_uses_after(
    fn_ir: &mut FnIR,
    instr: &mut Instr,
    rewrite: &mut VersionedExitRewriteEnv<'_>,
) {
    match instr {
        Instr::Assign { src, .. } => {
            *src = rewrite.rewrite_value(fn_ir, *src);
        }
        Instr::Eval { val, .. } => {
            *val = rewrite.rewrite_value(fn_ir, *val);
        }
        Instr::StoreIndex1D { base, idx, val, .. } => {
            rewrite_store_index1d_uses_after(fn_ir, base, idx, val, rewrite);
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => rewrite_store_index2d_uses_after(fn_ir, base, r, c, val, rewrite),
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => rewrite_store_index3d_uses_after(fn_ir, base, i, j, k, val, rewrite),
        Instr::UnsafeRBlock { .. } => {}
    }
}

pub(crate) fn rewrite_store_index1d_uses_after(
    fn_ir: &mut FnIR,
    base: &mut ValueId,
    idx: &mut ValueId,
    val: &mut ValueId,
    rewrite: &mut VersionedExitRewriteEnv<'_>,
) {
    *base = rewrite.rewrite_value(fn_ir, *base);
    *idx = rewrite.rewrite_value(fn_ir, *idx);
    *val = rewrite.rewrite_value(fn_ir, *val);
}

pub(crate) fn rewrite_store_index2d_uses_after(
    fn_ir: &mut FnIR,
    base: &mut ValueId,
    row: &mut ValueId,
    col: &mut ValueId,
    val: &mut ValueId,
    rewrite: &mut VersionedExitRewriteEnv<'_>,
) {
    *base = rewrite.rewrite_value(fn_ir, *base);
    *row = rewrite.rewrite_value(fn_ir, *row);
    *col = rewrite.rewrite_value(fn_ir, *col);
    *val = rewrite.rewrite_value(fn_ir, *val);
}

pub(crate) fn rewrite_store_index3d_uses_after(
    fn_ir: &mut FnIR,
    base: &mut ValueId,
    dim1: &mut ValueId,
    dim2: &mut ValueId,
    dim3: &mut ValueId,
    val: &mut ValueId,
    rewrite: &mut VersionedExitRewriteEnv<'_>,
) {
    *base = rewrite.rewrite_value(fn_ir, *base);
    *dim1 = rewrite.rewrite_value(fn_ir, *dim1);
    *dim2 = rewrite.rewrite_value(fn_ir, *dim2);
    *dim3 = rewrite.rewrite_value(fn_ir, *dim3);
    *val = rewrite.rewrite_value(fn_ir, *val);
}

pub(crate) fn rewrite_block_term_after(
    fn_ir: &mut FnIR,
    bid: BlockId,
    var: &str,
    replacement: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
) {
    let term = std::mem::replace(&mut fn_ir.blocks[bid].term, Terminator::Unreachable);
    fn_ir.blocks[bid].term =
        rewrite_term_value_uses_after(fn_ir, term, var, replacement, memo, visiting);
}

pub(crate) fn rewrite_term_value_uses_after(
    fn_ir: &mut FnIR,
    term: Terminator,
    var: &str,
    replacement: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
) -> Terminator {
    match term {
        Terminator::If {
            cond,
            then_bb,
            else_bb,
        } => Terminator::If {
            cond: rewrite_value_tree_for_var_after(fn_ir, cond, var, replacement, memo, visiting),
            then_bb,
            else_bb,
        },
        Terminator::Return(Some(ret)) => Terminator::Return(Some(
            rewrite_value_tree_for_var_after(fn_ir, ret, var, replacement, memo, visiting),
        )),
        other => other,
    }
}

pub(crate) fn finish_vector_assignments_versioned(
    fn_ir: &mut FnIR,
    fallback_bb: BlockId,
    site: VectorApplySite,
    assignments: Vec<PreparedVectorAssignment>,
    guard_cond: ValueId,
) -> bool {
    if assignments.is_empty()
        || assignments
            .iter()
            .any(|assignment| !assignment.shadow_vars.is_empty() || assignment.shadow_idx.is_some())
    {
        return false;
    }

    let preds = build_pred_map(fn_ir);
    let original_exit_preds = preds
        .get(&site.exit_bb)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|pred| *pred != site.preheader)
        .collect::<Vec<_>>();
    if original_exit_preds.is_empty() {
        return false;
    }

    // Reduced MIR-machine correspondence:
    // `VectorizeMirRewriteSubset` models this as
    // `preheader -> apply/fallback -> exit -> done`, with the exit preserving
    // the original scalar result once fallback and vectorized apply rejoin.
    let apply_bb = fn_ir.add_block();
    for assignment in &assignments {
        fn_ir.blocks[apply_bb].instrs.push(Instr::Assign {
            dst: assignment.dest_var.clone(),
            src: assignment.out_val,
            span: crate::utils::Span::dummy(),
        });
    }
    fn_ir.blocks[apply_bb].term = Terminator::Goto(site.exit_bb);
    fn_ir.blocks[site.preheader].term = Terminator::If {
        cond: guard_cond,
        then_bb: apply_bb,
        else_bb: fallback_bb,
    };

    // The reduced correspondence for this exit-phi merge lives in
    // `proof/{lean,coq}/.../VectorizeRewriteSubset.*`:
    // fallback edges keep the original scalar value, while result-preserving
    // apply edges may merge the vectorized result without changing the scalar
    // exit meaning.
    for assignment in assignments {
        let scalar_val = fn_ir.add_value(
            ValueKind::Load {
                var: assignment.dest_var.clone(),
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            Some(assignment.dest_var.clone()),
        );
        let mut phi_args = original_exit_preds
            .iter()
            .map(|pred| (scalar_val, *pred))
            .collect::<Vec<_>>();
        phi_args.push((assignment.out_val, apply_bb));
        let phi = fn_ir.add_value(
            ValueKind::Phi { args: phi_args },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            Some(assignment.dest_var.clone()),
        );
        fn_ir.values[phi].phi_block = Some(site.exit_bb);
        rewrite_reachable_value_uses_for_var_after(fn_ir, site.exit_bb, &assignment.dest_var, phi);
        if !has_reachable_assignment_to_var_after(fn_ir, site.exit_bb, &assignment.dest_var) {
            rewrite_returns_for_var(fn_ir, &assignment.dest_var, phi);
        }
    }
    true
}
