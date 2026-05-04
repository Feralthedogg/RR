use super::*;
pub(crate) fn build_shadow_state_read(fn_ir: &mut FnIR, out_val: ValueId, idx: ValueId) -> ValueId {
    let out_val = resolve_materialized_value(fn_ir, out_val);
    let idx = resolve_materialized_value(fn_ir, idx);
    let ctx = fn_ir.add_value(
        ValueKind::Const(Lit::Str("index".to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_index1_read".to_string(),
            args: vec![out_val, idx, ctx],
            names: vec![None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

pub(crate) fn emit_vector_preheader_eval(fn_ir: &mut FnIR, preheader: BlockId, val: ValueId) {
    fn_ir.blocks[preheader].instrs.push(Instr::Eval {
        val,
        span: crate::utils::Span::dummy(),
    });
}

pub(crate) fn emit_same_len_guard(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    lhs: ValueId,
    rhs: ValueId,
) {
    let check_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_same_len".to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    emit_vector_preheader_eval(fn_ir, preheader, check_val);
}

pub(crate) fn emit_same_or_scalar_guard(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    lhs: ValueId,
    rhs: ValueId,
) {
    let check_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_same_or_scalar".to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    emit_vector_preheader_eval(fn_ir, preheader, check_val);
}

pub(crate) fn emit_same_matrix_shape_or_scalar_guard(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    lhs: ValueId,
    rhs: ValueId,
) {
    let check_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_same_matrix_shape_or_scalar".to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    emit_vector_preheader_eval(fn_ir, preheader, check_val);
}

pub(crate) fn emit_same_array3_shape_or_scalar_guard(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    lhs: ValueId,
    rhs: ValueId,
) {
    let check_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_same_array3_shape_or_scalar".to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    emit_vector_preheader_eval(fn_ir, preheader, check_val);
}

pub(crate) fn build_slice_assignment_value(
    fn_ir: &mut FnIR,
    dest: ValueId,
    start: ValueId,
    end: ValueId,
    value: ValueId,
) -> ValueId {
    fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_assign_slice".to_string(),
            args: vec![dest, start, end, value],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

pub(crate) struct CondMapOperands {
    pub(crate) preheader: BlockId,
    pub(crate) dest_ref: ValueId,
    pub(crate) cond_vec: ValueId,
    pub(crate) then_vec: ValueId,
    pub(crate) else_vec: ValueId,
    pub(crate) start: ValueId,
    pub(crate) end: ValueId,
    pub(crate) whole_dest: bool,
}

pub(crate) fn prepare_cond_map_operands(
    fn_ir: &mut FnIR,
    operands: CondMapOperands,
) -> (ValueId, ValueId, ValueId) {
    if operands.whole_dest {
        if !same_length_proven(fn_ir, operands.dest_ref, operands.cond_vec) {
            emit_same_len_guard(
                fn_ir,
                operands.preheader,
                operands.dest_ref,
                operands.cond_vec,
            );
        }
        for branch_vec in [operands.then_vec, operands.else_vec] {
            if is_const_number(fn_ir, branch_vec)
                || same_length_proven(fn_ir, operands.dest_ref, branch_vec)
            {
                continue;
            }
            emit_same_or_scalar_guard(fn_ir, operands.preheader, operands.dest_ref, branch_vec);
        }
        return (operands.cond_vec, operands.then_vec, operands.else_vec);
    }

    (
        prepare_partial_slice_value(
            fn_ir,
            operands.dest_ref,
            operands.cond_vec,
            operands.start,
            operands.end,
        ),
        prepare_partial_slice_value(
            fn_ir,
            operands.dest_ref,
            operands.then_vec,
            operands.start,
            operands.end,
        ),
        prepare_partial_slice_value(
            fn_ir,
            operands.dest_ref,
            operands.else_vec,
            operands.start,
            operands.end,
        ),
    )
}

pub(crate) fn has_reachable_assignment_to_var(fn_ir: &FnIR, start: BlockId, var: &str) -> bool {
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

pub(crate) struct ValueTreeRewriter<'a> {
    pub(crate) fn_ir: &'a mut FnIR,
    pub(crate) var: &'a str,
    pub(crate) replacement: ValueId,
    pub(crate) memo: &'a mut FxHashMap<ValueId, ValueId>,
    pub(crate) visiting: &'a mut FxHashSet<ValueId>,
}

impl ValueTreeRewriter<'_> {
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
        let new_fields: Vec<(String, ValueId)> = fields
            .iter()
            .map(|(name, value)| (name.clone(), self.rewrite(*value)))
            .collect();
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
        let new_args: Vec<(ValueId, BlockId)> = args
            .iter()
            .map(|(arg, bid)| (self.rewrite(*arg), *bid))
            .collect();
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

pub(crate) fn rewrite_value_tree_for_var(
    fn_ir: &mut FnIR,
    root: ValueId,
    var: &str,
    replacement: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
) -> ValueId {
    ValueTreeRewriter {
        fn_ir,
        var,
        replacement,
        memo,
        visiting,
    }
    .rewrite(root)
}

pub(crate) struct AssignmentValueRewriteEnv<'a> {
    pub(crate) var: &'a str,
    pub(crate) replacement: ValueId,
    pub(crate) memo: &'a mut FxHashMap<ValueId, ValueId>,
    pub(crate) visiting: &'a mut FxHashSet<ValueId>,
}

impl AssignmentValueRewriteEnv<'_> {
    pub(crate) fn rewrite_value(&mut self, fn_ir: &mut FnIR, root: ValueId) -> ValueId {
        rewrite_value_tree_for_var(
            fn_ir,
            root,
            self.var,
            self.replacement,
            self.memo,
            self.visiting,
        )
    }
}

pub(crate) fn rewrite_reachable_value_uses_for_var(
    fn_ir: &mut FnIR,
    start: BlockId,
    var: &str,
    replacement: ValueId,
) {
    let mut memo = FxHashMap::default();
    let mut visiting = FxHashSet::default();

    rewrite_block_instr_value_uses_for_var(
        fn_ir,
        start,
        var,
        replacement,
        &mut memo,
        &mut visiting,
    );
    rewrite_block_terminator_value_uses_for_var(
        fn_ir,
        start,
        var,
        replacement,
        &mut memo,
        &mut visiting,
    );
}

pub(crate) fn rewrite_block_instr_value_uses_for_var(
    fn_ir: &mut FnIR,
    bid: BlockId,
    var: &str,
    replacement: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
) {
    let mut rewrite = AssignmentValueRewriteEnv {
        var,
        replacement,
        memo,
        visiting,
    };
    let old_instrs = std::mem::take(&mut fn_ir.blocks[bid].instrs);
    let mut new_instrs = Vec::with_capacity(old_instrs.len());
    for mut instr in old_instrs {
        rewrite_instr_value_uses_for_var(fn_ir, &mut instr, &mut rewrite);
        new_instrs.push(instr);
    }
    fn_ir.blocks[bid].instrs = new_instrs;
}

pub(crate) fn rewrite_instr_value_uses_for_var(
    fn_ir: &mut FnIR,
    instr: &mut Instr,
    rewrite: &mut AssignmentValueRewriteEnv<'_>,
) {
    match instr {
        Instr::Assign { src, .. } => {
            *src = rewrite.rewrite_value(fn_ir, *src);
        }
        Instr::Eval { val, .. } => {
            *val = rewrite.rewrite_value(fn_ir, *val);
        }
        Instr::StoreIndex1D { base, idx, val, .. } => {
            rewrite_store_index1d_value_uses(fn_ir, base, idx, val, rewrite);
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => rewrite_store_index2d_value_uses(fn_ir, base, r, c, val, rewrite),
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => rewrite_store_index3d_value_uses(fn_ir, base, i, j, k, val, rewrite),
        Instr::UnsafeRBlock { .. } => {}
    }
}

pub(crate) fn rewrite_store_index1d_value_uses(
    fn_ir: &mut FnIR,
    base: &mut ValueId,
    idx: &mut ValueId,
    val: &mut ValueId,
    rewrite: &mut AssignmentValueRewriteEnv<'_>,
) {
    *base = rewrite.rewrite_value(fn_ir, *base);
    *idx = rewrite.rewrite_value(fn_ir, *idx);
    *val = rewrite.rewrite_value(fn_ir, *val);
}

pub(crate) fn rewrite_store_index2d_value_uses(
    fn_ir: &mut FnIR,
    base: &mut ValueId,
    r: &mut ValueId,
    c: &mut ValueId,
    val: &mut ValueId,
    rewrite: &mut AssignmentValueRewriteEnv<'_>,
) {
    *base = rewrite.rewrite_value(fn_ir, *base);
    *r = rewrite.rewrite_value(fn_ir, *r);
    *c = rewrite.rewrite_value(fn_ir, *c);
    *val = rewrite.rewrite_value(fn_ir, *val);
}

pub(crate) fn rewrite_store_index3d_value_uses(
    fn_ir: &mut FnIR,
    base: &mut ValueId,
    i: &mut ValueId,
    j: &mut ValueId,
    k: &mut ValueId,
    val: &mut ValueId,
    rewrite: &mut AssignmentValueRewriteEnv<'_>,
) {
    *base = rewrite.rewrite_value(fn_ir, *base);
    *i = rewrite.rewrite_value(fn_ir, *i);
    *j = rewrite.rewrite_value(fn_ir, *j);
    *k = rewrite.rewrite_value(fn_ir, *k);
    *val = rewrite.rewrite_value(fn_ir, *val);
}

pub(crate) fn rewrite_block_terminator_value_uses_for_var(
    fn_ir: &mut FnIR,
    bid: BlockId,
    var: &str,
    replacement: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
) {
    let term = std::mem::replace(&mut fn_ir.blocks[bid].term, Terminator::Unreachable);
    fn_ir.blocks[bid].term =
        rewrite_terminator_value_uses_for_var(fn_ir, term, var, replacement, memo, visiting);
}

pub(crate) fn rewrite_terminator_value_uses_for_var(
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
            cond: rewrite_value_tree_for_var(fn_ir, cond, var, replacement, memo, visiting),
            then_bb,
            else_bb,
        },
        Terminator::Return(Some(ret)) => Terminator::Return(Some(rewrite_value_tree_for_var(
            fn_ir,
            ret,
            var,
            replacement,
            memo,
            visiting,
        ))),
        other => other,
    }
}

pub(crate) fn emit_prepared_vector_assignments(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    assignments: Vec<PreparedVectorAssignment>,
) -> bool {
    for assignment in assignments {
        fn_ir.blocks[site.preheader].instrs.push(Instr::Assign {
            dst: assignment.dest_var.clone(),
            src: assignment.out_val,
            span: crate::utils::Span::dummy(),
        });
        let rewrite_dest_returns =
            !has_reachable_assignment_to_var(fn_ir, site.exit_bb, &assignment.dest_var);
        if let Some(shadow_idx) = assignment.shadow_idx
            && !assignment.shadow_vars.is_empty()
        {
            let shadow_val = build_shadow_state_read(fn_ir, assignment.out_val, shadow_idx);
            for shadow_var in &assignment.shadow_vars {
                fn_ir.blocks[site.preheader].instrs.push(Instr::Assign {
                    dst: shadow_var.clone(),
                    src: shadow_val,
                    span: crate::utils::Span::dummy(),
                });
                rewrite_reachable_value_uses_for_var(fn_ir, site.exit_bb, shadow_var, shadow_val);
                if !has_reachable_assignment_to_var(fn_ir, site.exit_bb, shadow_var) {
                    rewrite_returns_for_var(fn_ir, shadow_var, shadow_val);
                }
            }
        }
        rewrite_reachable_value_uses_for_var(
            fn_ir,
            site.exit_bb,
            &assignment.dest_var,
            assignment.out_val,
        );
        if rewrite_dest_returns {
            rewrite_returns_for_var(fn_ir, &assignment.dest_var, assignment.out_val);
        }
    }
    fn_ir.blocks[site.preheader].term = Terminator::Goto(site.exit_bb);
    true
}

/// Commit a vectorized assignment in the loop preheader and rewrite reachable
/// exit-path uses/returns so loop-carried shadow state still observes the
/// vectorized result at the correct index.
pub(crate) fn finish_vector_assignment_with_shadow_states(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    dest_var: VarId,
    out_val: ValueId,
    shadow_vars: &[VarId],
    shadow_idx: Option<ValueId>,
) -> bool {
    emit_prepared_vector_assignments(
        fn_ir,
        site,
        vec![PreparedVectorAssignment {
            dest_var,
            out_val,
            shadow_vars: shadow_vars.to_vec(),
            shadow_idx,
        }],
    )
}

pub(crate) fn finish_vector_phi_assignment(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    acc_phi: ValueId,
    out_val: ValueId,
) -> bool {
    let Some(acc_var) = fn_ir.values[acc_phi].origin_var.clone() else {
        return false;
    };
    finish_vector_assignment(fn_ir, site, acc_var, out_val)
}
