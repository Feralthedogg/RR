pub(super) fn build_shadow_state_read(fn_ir: &mut FnIR, out_val: ValueId, idx: ValueId) -> ValueId {
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

pub(super) fn emit_vector_preheader_eval(fn_ir: &mut FnIR, preheader: BlockId, val: ValueId) {
    fn_ir.blocks[preheader].instrs.push(Instr::Eval {
        val,
        span: crate::utils::Span::dummy(),
    });
}

pub(super) fn emit_same_len_guard(
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

pub(super) fn emit_same_or_scalar_guard(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_cond_map_operands(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    dest_ref: ValueId,
    cond_vec: ValueId,
    then_vec: ValueId,
    else_vec: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
) -> (ValueId, ValueId, ValueId) {
    if whole_dest {
        if !same_length_proven(fn_ir, dest_ref, cond_vec) {
            emit_same_len_guard(fn_ir, preheader, dest_ref, cond_vec);
        }
        for branch_vec in [then_vec, else_vec] {
            if is_const_number(fn_ir, branch_vec) || same_length_proven(fn_ir, dest_ref, branch_vec)
            {
                continue;
            }
            emit_same_or_scalar_guard(fn_ir, preheader, dest_ref, branch_vec);
        }
        return (cond_vec, then_vec, else_vec);
    }

    (
        prepare_partial_slice_value(fn_ir, dest_ref, cond_vec, start, end),
        prepare_partial_slice_value(fn_ir, dest_ref, then_vec, start, end),
        prepare_partial_slice_value(fn_ir, dest_ref, else_vec, start, end),
    )
}

pub(super) fn emit_prepared_vector_assignments(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    assignments: Vec<PreparedVectorAssignment>,
) -> bool {
    fn has_reachable_assignment_to_var(fn_ir: &FnIR, start: BlockId, var: &str) -> bool {
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
                    Instr::Assign { .. } | Instr::Eval { .. } => {}
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

    fn rewrite_reachable_value_uses_for_var(
        fn_ir: &mut FnIR,
        start: BlockId,
        var: &str,
        replacement: ValueId,
    ) {
        fn rewrite_value_tree_for_var(
            fn_ir: &mut FnIR,
            root: ValueId,
            var: &str,
            replacement: ValueId,
            memo: &mut FxHashMap<ValueId, ValueId>,
            visiting: &mut FxHashSet<ValueId>,
        ) -> ValueId {
            let root = canonical_value(fn_ir, root);
            if let Some(mapped) = memo.get(&root) {
                return *mapped;
            }
            if !visiting.insert(root) {
                return root;
            }

            let mapped = match fn_ir.values[root].kind.clone() {
                ValueKind::Load { var: load_var } if load_var == var => root,
                kind if fn_ir.values[root].origin_var.as_deref() == Some(var) => match kind {
                    ValueKind::Load { var: load_var } if load_var == var => root,
                    _ => replacement,
                },
                ValueKind::Load { .. } => root,
                ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => root,
                ValueKind::Unary { op, rhs } => {
                    let rhs_new =
                        rewrite_value_tree_for_var(fn_ir, rhs, var, replacement, memo, visiting);
                    if rhs_new == rhs {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Unary { op, rhs: rhs_new },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Binary { op, lhs, rhs } => {
                    let lhs_new =
                        rewrite_value_tree_for_var(fn_ir, lhs, var, replacement, memo, visiting);
                    let rhs_new =
                        rewrite_value_tree_for_var(fn_ir, rhs, var, replacement, memo, visiting);
                    if lhs_new == lhs && rhs_new == rhs {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Binary {
                                op,
                                lhs: lhs_new,
                                rhs: rhs_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::RecordLit { fields } => {
                    let new_fields: Vec<(String, ValueId)> = fields
                        .iter()
                        .map(|(name, value)| {
                            (
                                name.clone(),
                                rewrite_value_tree_for_var(
                                    fn_ir,
                                    *value,
                                    var,
                                    replacement,
                                    memo,
                                    visiting,
                                ),
                            )
                        })
                        .collect();
                    if new_fields == fields {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::RecordLit { fields: new_fields },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::FieldGet { base, field } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    if base_new == base {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::FieldGet {
                                base: base_new,
                                field,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::FieldSet { base, field, value } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    let value_new =
                        rewrite_value_tree_for_var(fn_ir, value, var, replacement, memo, visiting);
                    if base_new == base && value_new == value {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::FieldSet {
                                base: base_new,
                                field,
                                value: value_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Call {
                    callee,
                    args,
                    names,
                } => {
                    let new_args: Vec<ValueId> = args
                        .iter()
                        .map(|arg| {
                            rewrite_value_tree_for_var(
                                fn_ir,
                                *arg,
                                var,
                                replacement,
                                memo,
                                visiting,
                            )
                        })
                        .collect();
                    if new_args == args {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Call {
                                callee,
                                args: new_args,
                                names,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Intrinsic { op, args } => {
                    let new_args: Vec<ValueId> = args
                        .iter()
                        .map(|arg| {
                            rewrite_value_tree_for_var(
                                fn_ir,
                                *arg,
                                var,
                                replacement,
                                memo,
                                visiting,
                            )
                        })
                        .collect();
                    if new_args == args {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Intrinsic { op, args: new_args },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Phi { args } => {
                    let new_args: Vec<(ValueId, BlockId)> = args
                        .iter()
                        .map(|(arg, bid)| {
                            (
                                rewrite_value_tree_for_var(
                                    fn_ir,
                                    *arg,
                                    var,
                                    replacement,
                                    memo,
                                    visiting,
                                ),
                                *bid,
                            )
                        })
                        .collect();
                    if new_args == args {
                        root
                    } else {
                        let phi = fn_ir.add_value(
                            ValueKind::Phi {
                                args: new_args.clone(),
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        );
                        fn_ir.values[phi].phi_block = fn_ir.values[root].phi_block;
                        phi
                    }
                }
                ValueKind::Len { base } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    if base_new == base {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Len { base: base_new },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Indices { base } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    if base_new == base {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Indices { base: base_new },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Range { start, end } => {
                    let start_new =
                        rewrite_value_tree_for_var(fn_ir, start, var, replacement, memo, visiting);
                    let end_new =
                        rewrite_value_tree_for_var(fn_ir, end, var, replacement, memo, visiting);
                    if start_new == start && end_new == end {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Range {
                                start: start_new,
                                end: end_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Index1D {
                    base,
                    idx,
                    is_safe,
                    is_na_safe,
                } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    let idx_new =
                        rewrite_value_tree_for_var(fn_ir, idx, var, replacement, memo, visiting);
                    if base_new == base && idx_new == idx {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Index1D {
                                base: base_new,
                                idx: idx_new,
                                is_safe,
                                is_na_safe,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Index2D { base, r, c } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    let r_new =
                        rewrite_value_tree_for_var(fn_ir, r, var, replacement, memo, visiting);
                    let c_new =
                        rewrite_value_tree_for_var(fn_ir, c, var, replacement, memo, visiting);
                    if base_new == base && r_new == r && c_new == c {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Index2D {
                                base: base_new,
                                r: r_new,
                                c: c_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Index3D { base, i, j, k } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    let i_new =
                        rewrite_value_tree_for_var(fn_ir, i, var, replacement, memo, visiting);
                    let j_new =
                        rewrite_value_tree_for_var(fn_ir, j, var, replacement, memo, visiting);
                    let k_new =
                        rewrite_value_tree_for_var(fn_ir, k, var, replacement, memo, visiting);
                    if base_new == base && i_new == i && j_new == j && k_new == k {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Index3D {
                                base: base_new,
                                i: i_new,
                                j: j_new,
                                k: k_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
            };

            visiting.remove(&root);
            memo.insert(root, mapped);
            mapped
        }

        let mut memo = FxHashMap::default();
        let mut visiting = FxHashSet::default();

        for bid in [start] {
            let old_instrs = std::mem::take(&mut fn_ir.blocks[bid].instrs);
            let mut new_instrs = Vec::with_capacity(old_instrs.len());
            for mut instr in old_instrs {
                match &mut instr {
                    Instr::Assign { src, .. } => {
                        *src = rewrite_value_tree_for_var(
                            fn_ir,
                            *src,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                    Instr::Eval { val, .. } => {
                        *val = rewrite_value_tree_for_var(
                            fn_ir,
                            *val,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        *base = rewrite_value_tree_for_var(
                            fn_ir,
                            *base,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *idx = rewrite_value_tree_for_var(
                            fn_ir,
                            *idx,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *val = rewrite_value_tree_for_var(
                            fn_ir,
                            *val,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        *base = rewrite_value_tree_for_var(
                            fn_ir,
                            *base,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *r = rewrite_value_tree_for_var(
                            fn_ir,
                            *r,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *c = rewrite_value_tree_for_var(
                            fn_ir,
                            *c,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *val = rewrite_value_tree_for_var(
                            fn_ir,
                            *val,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                    Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        *base = rewrite_value_tree_for_var(
                            fn_ir,
                            *base,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *i = rewrite_value_tree_for_var(
                            fn_ir,
                            *i,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *j = rewrite_value_tree_for_var(
                            fn_ir,
                            *j,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *k = rewrite_value_tree_for_var(
                            fn_ir,
                            *k,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *val = rewrite_value_tree_for_var(
                            fn_ir,
                            *val,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                }
                new_instrs.push(instr);
            }
            fn_ir.blocks[bid].instrs = new_instrs;

            let term = std::mem::replace(&mut fn_ir.blocks[bid].term, Terminator::Unreachable);
            fn_ir.blocks[bid].term = match term {
                Terminator::If {
                    cond,
                    then_bb,
                    else_bb,
                } => Terminator::If {
                    cond: rewrite_value_tree_for_var(
                        fn_ir,
                        cond,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    ),
                    then_bb,
                    else_bb,
                },
                Terminator::Return(Some(ret)) => {
                    Terminator::Return(Some(rewrite_value_tree_for_var(
                        fn_ir,
                        ret,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    )))
                }
                other => other,
            };
        }
    }

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
pub(super) fn finish_vector_assignment_with_shadow_states(
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

pub(super) fn finish_vector_phi_assignment(
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
