use super::*;
pub(crate) fn materialize_loop_invariant_scalar_expr(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    fn rec(
        fn_ir: &mut FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        memo: &mut FxHashMap<ValueId, ValueId>,
        interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
        visiting: &mut FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        if let Some(v) = memo.get(&root) {
            return Some(*v);
        }
        if fn_ir.values[root]
            .phi_block
            .is_some_and(|phi_bb| !lp.body.contains(&phi_bb))
            && value_is_definitely_scalar_like(fn_ir, root)
        {
            memo.insert(root, root);
            return Some(root);
        }
        if expr_has_iv_dependency(fn_ir, root, iv_phi) {
            return None;
        }
        if !visiting.insert(root) {
            return None;
        }

        let span = fn_ir.values[root].span;
        let facts = fn_ir.values[root].facts;
        let out = match fn_ir.values[root].kind.clone() {
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => root,
            ValueKind::Load { var } => {
                if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, &var) {
                    rec(fn_ir, src, iv_phi, lp, memo, interner, visiting)?
                } else if let Some(src) = merged_assign_source_in_loop(fn_ir, lp, &var) {
                    rec(fn_ir, src, iv_phi, lp, memo, interner, visiting)?
                } else if let Some(src) =
                    unique_origin_phi_value_in_loop(fn_ir, lp, &var).or_else(|| {
                        nearest_origin_phi_value_in_loop(fn_ir, lp, &var, fn_ir.blocks.len())
                    })
                {
                    rec(fn_ir, src, iv_phi, lp, memo, interner, visiting)?
                } else if has_non_passthrough_assignment_in_loop(fn_ir, lp, &var) {
                    return None;
                } else {
                    root
                }
            }
            ValueKind::Unary { op, rhs } => {
                let rhs_v = rec(fn_ir, rhs, iv_phi, lp, memo, interner, visiting)?;
                if rhs_v == rhs {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Unary { op, rhs: rhs_v },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Binary { op, lhs, rhs } => {
                let lhs_v = rec(fn_ir, lhs, iv_phi, lp, memo, interner, visiting)?;
                let rhs_v = rec(fn_ir, rhs, iv_phi, lp, memo, interner, visiting)?;
                if lhs_v == lhs && rhs_v == rhs {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Binary {
                            op,
                            lhs: lhs_v,
                            rhs: rhs_v,
                        },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::RecordLit { fields } => {
                let mut new_fields = Vec::with_capacity(fields.len());
                let mut changed = false;
                for (field, value) in fields {
                    let mapped = rec(fn_ir, value, iv_phi, lp, memo, interner, visiting)?;
                    changed |= mapped != value;
                    new_fields.push((field, mapped));
                }
                if !changed {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::RecordLit { fields: new_fields },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::FieldGet { base, field } => {
                let base_v = rec(fn_ir, base, iv_phi, lp, memo, interner, visiting)?;
                if base_v == base {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::FieldGet {
                            base: base_v,
                            field,
                        },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::FieldSet { base, field, value } => {
                let base_v = rec(fn_ir, base, iv_phi, lp, memo, interner, visiting)?;
                let value_v = rec(fn_ir, value, iv_phi, lp, memo, interner, visiting)?;
                if base_v == base && value_v == value {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::FieldSet {
                            base: base_v,
                            field,
                            value: value_v,
                        },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                let mut new_args = Vec::with_capacity(args.len());
                let mut changed = false;
                for arg in &args {
                    let mapped = rec(fn_ir, *arg, iv_phi, lp, memo, interner, visiting)?;
                    changed |= mapped != *arg;
                    new_args.push(mapped);
                }
                if !changed {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Call {
                            callee,
                            args: new_args,
                            names,
                        },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Intrinsic { op, args } => {
                let mut new_args = Vec::with_capacity(args.len());
                let mut changed = false;
                for arg in args {
                    let mapped = rec(fn_ir, arg, iv_phi, lp, memo, interner, visiting)?;
                    changed |= mapped != arg;
                    new_args.push(mapped);
                }
                if !changed {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Intrinsic { op, args: new_args },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Phi { args } => {
                if fn_ir.values[root]
                    .phi_block
                    .is_some_and(|phi_bb| !lp.body.contains(&phi_bb))
                    && value_is_definitely_scalar_like(fn_ir, root)
                {
                    root
                } else if let Some(var) = fn_ir.values[root].origin_var.clone()
                    && let Some(v) = materialize_passthrough_origin_phi_state_scalar(
                        fn_ir, root, &var, iv_phi, lp, memo, interner,
                    )
                {
                    v
                } else if phi_loads_same_var(fn_ir, &args) {
                    rec(fn_ir, args[0].0, iv_phi, lp, memo, interner, visiting)?
                } else if let Some((cond, then_val, else_val)) =
                    find_conditional_phi_shape(fn_ir, root, &args)
                {
                    let cond_v = rec(fn_ir, cond, iv_phi, lp, memo, interner, visiting)?;
                    let then_v = rec(fn_ir, then_val, iv_phi, lp, memo, interner, visiting)?;
                    let else_v = rec(fn_ir, else_val, iv_phi, lp, memo, interner, visiting)?;
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Call {
                            callee: "rr_ifelse_strict".to_string(),
                            args: vec![cond_v, then_v, else_v],
                            names: vec![None, None, None],
                        },
                        span,
                        facts,
                    )
                } else {
                    let mut picked: Option<ValueId> = None;
                    for (arg, _) in args {
                        let mapped = rec(fn_ir, arg, iv_phi, lp, memo, interner, visiting)?;
                        match picked {
                            None => picked = Some(mapped),
                            Some(prev)
                                if canonical_value(fn_ir, prev)
                                    == canonical_value(fn_ir, mapped) => {}
                            Some(_) => return None,
                        }
                    }
                    picked?
                }
            }
            ValueKind::Len { base } => {
                let base_v = rec(fn_ir, base, iv_phi, lp, memo, interner, visiting)?;
                if base_v == base {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Len { base: base_v },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Range { .. }
            | ValueKind::Indices { .. }
            | ValueKind::Index1D { .. }
            | ValueKind::Index2D { .. }
            | ValueKind::Index3D { .. } => return None,
        };

        memo.insert(root, out);
        visiting.remove(&root);
        Some(out)
    }

    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        memo,
        interner,
        &mut FxHashSet::default(),
    )
}
