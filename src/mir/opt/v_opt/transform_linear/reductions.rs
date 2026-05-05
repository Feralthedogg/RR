use super::*;

pub(crate) fn reduce_function_name(kind: ReduceKind) -> &'static str {
    match kind {
        ReduceKind::Sum => "sum",
        ReduceKind::Prod => "prod",
        ReduceKind::Min => "min",
        ReduceKind::Max => "max",
    }
}

pub(crate) fn const_float_value(fn_ir: &FnIR, vid: ValueId) -> Option<f64> {
    match fn_ir.values[canonical_value(fn_ir, vid)].kind {
        ValueKind::Const(Lit::Int(n)) => Some(n as f64),
        ValueKind::Const(Lit::Float(n)) => Some(n),
        _ => None,
    }
}

pub(crate) fn is_reduce_neutral_initial(fn_ir: &FnIR, kind: ReduceKind, init: ValueId) -> bool {
    let Some(value) = const_float_value(fn_ir, init) else {
        return false;
    };
    match kind {
        ReduceKind::Sum => value.abs() < f64::EPSILON,
        ReduceKind::Prod => (value - 1.0).abs() < f64::EPSILON,
        ReduceKind::Min | ReduceKind::Max => false,
    }
}

pub(crate) fn reduction_entry_value(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    acc_phi: ValueId,
) -> Option<ValueId> {
    let acc_phi = canonical_value(fn_ir, acc_phi);
    let ValueKind::Phi { args } = &fn_ir.values[acc_phi].kind else {
        return None;
    };

    args.iter()
        .find(|(_, pred)| *pred == site.preheader)
        .map(|(value, _)| *value)
        .or_else(|| {
            args.iter()
                .find(|(_, pred)| *pred != lp.latch && !lp.body.contains(pred))
                .map(|(value, _)| *value)
        })
        .or_else(|| {
            args.iter()
                .find(|(_, pred)| *pred != lp.latch)
                .map(|(value, _)| *value)
        })
}

pub(crate) fn combine_reduction_with_entry_value(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    kind: ReduceKind,
    acc_phi: ValueId,
    reduced: ValueId,
) -> Option<ValueId> {
    let init = reduction_entry_value(fn_ir, lp, site, acc_phi)?;
    if is_reduce_neutral_initial(fn_ir, kind, init) {
        return Some(reduced);
    }

    let span = crate::utils::Span::dummy();
    let facts = crate::mir::def::Facts::empty();
    let origin_var = fn_ir.values[canonical_value(fn_ir, acc_phi)]
        .origin_var
        .clone();
    match kind {
        ReduceKind::Sum | ReduceKind::Prod => {
            let op = if kind == ReduceKind::Sum {
                crate::syntax::ast::BinOp::Add
            } else {
                crate::syntax::ast::BinOp::Mul
            };
            Some(fn_ir.add_value(
                ValueKind::Binary {
                    op,
                    lhs: init,
                    rhs: reduced,
                },
                span,
                facts,
                origin_var,
            ))
        }
        ReduceKind::Min | ReduceKind::Max => Some(fn_ir.add_value(
            ValueKind::Call {
                callee: reduce_function_name(kind).to_string(),
                args: vec![init, reduced],
                names: vec![None, None],
            },
            span,
            facts,
            origin_var,
        )),
    }
}

pub(crate) fn materialize_vector_expr_once(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    root: ValueId,
    iv_phi: ValueId,
    policy: VectorMaterializePolicy,
) -> Option<ValueId> {
    let idx_vec = build_loop_index_vector(fn_ir, lp)?;
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root,
            iv_phi,
            idx_vec,
            lp,
            policy,
        },
        &mut memo,
        &mut interner,
    )
}

pub(crate) fn apply_reduce_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    kind: ReduceKind,
    acc_phi: ValueId,
    vec_expr: ValueId,
    iv_phi: ValueId,
) -> bool {
    let policy = RELAXED_VECTOR_MATERIALIZE_POLICY;
    let reduce_val = if kind == ReduceKind::Sum {
        if let Some(v) = rewrite_sum_add_const(fn_ir, lp, vec_expr, iv_phi) {
            v
        } else {
            let Some(input_vec) = materialize_vector_expr_once(fn_ir, lp, vec_expr, iv_phi, policy)
            else {
                return false;
            };
            fn_ir.add_value(
                ValueKind::Call {
                    callee: reduce_function_name(kind).to_string(),
                    args: vec![input_vec],
                    names: vec![None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            )
        }
    } else {
        let Some(input_vec) = materialize_vector_expr_once(fn_ir, lp, vec_expr, iv_phi, policy)
        else {
            return false;
        };
        fn_ir.add_value(
            ValueKind::Call {
                callee: reduce_function_name(kind).to_string(),
                args: vec![input_vec],
                names: vec![None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    };

    let Some(out_val) =
        combine_reduction_with_entry_value(fn_ir, lp, site, kind, acc_phi, reduce_val)
    else {
        return false;
    };
    finish_vector_phi_assignment(fn_ir, site, acc_phi, out_val)
}

pub(crate) fn apply_reduce_cond_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    cond: ValueId,
    entry: ReduceCondEntry,
    iv_phi: ValueId,
) -> bool {
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let cond_vec = match materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: cond,
            iv_phi,
            idx_vec,
            lp,
            policy: SAFE_INDEX_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let then_vec = match materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: entry.then_val,
            iv_phi,
            idx_vec,
            lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let else_vec = match materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: entry.else_val,
            iv_phi,
            idx_vec,
            lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    emit_same_or_scalar_guard(fn_ir, site.preheader, cond_vec, then_vec);
    emit_same_or_scalar_guard(fn_ir, site.preheader, cond_vec, else_vec);
    let ifelse_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_vec, then_vec, else_vec],
            names: vec![None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: reduce_function_name(entry.kind).to_string(),
            args: vec![ifelse_val],
            names: vec![None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let Some(out_val) =
        combine_reduction_with_entry_value(fn_ir, lp, site, entry.kind, entry.acc_phi, reduce_val)
    else {
        return false;
    };
    finish_vector_phi_assignment(fn_ir, site, entry.acc_phi, out_val)
}

pub(crate) fn apply_multi_reduce_cond_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    cond: ValueId,
    entries: &[ReduceCondEntry],
    iv_phi: ValueId,
) -> bool {
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let cond_vec = match materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: cond,
            iv_phi,
            idx_vec,
            lp,
            policy: SAFE_INDEX_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };

    let mut staged = Vec::with_capacity(entries.len());
    for entry in entries {
        let then_vec = match materialize_vector_expr(
            fn_ir,
            VectorMaterializeRequest {
                root: entry.then_val,
                iv_phi,
                idx_vec,
                lp,
                policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
            },
            &mut memo,
            &mut interner,
        ) {
            Some(v) => v,
            None => return false,
        };
        let else_vec = match materialize_vector_expr(
            fn_ir,
            VectorMaterializeRequest {
                root: entry.else_val,
                iv_phi,
                idx_vec,
                lp,
                policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
            },
            &mut memo,
            &mut interner,
        ) {
            Some(v) => v,
            None => return false,
        };
        emit_same_or_scalar_guard(fn_ir, site.preheader, cond_vec, then_vec);
        emit_same_or_scalar_guard(fn_ir, site.preheader, cond_vec, else_vec);
        let ifelse_val = fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_ifelse_strict".to_string(),
                args: vec![cond_vec, then_vec, else_vec],
                names: vec![None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let reduce_val = fn_ir.add_value(
            ValueKind::Call {
                callee: reduce_function_name(entry.kind).to_string(),
                args: vec![ifelse_val],
                names: vec![None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let Some(reduce_val) = combine_reduction_with_entry_value(
            fn_ir,
            lp,
            site,
            entry.kind,
            entry.acc_phi,
            reduce_val,
        ) else {
            return false;
        };
        let Some(dest_var) = fn_ir.values[entry.acc_phi].origin_var.clone() else {
            return false;
        };
        staged.push(PreparedVectorAssignment {
            dest_var,
            out_val: reduce_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        });
    }

    emit_prepared_vector_assignments(fn_ir, site, staged)
}

pub(crate) fn apply_reduce_2d_row_sum_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: Reduce2DApplyPlan,
) -> bool {
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let row_val = resolve_materialized_value(fn_ir, plan.axis);
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_row_sum_range".to_string(),
            args: vec![plan.base, row_val, plan.range.start, end],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let Some(out_val) = combine_reduction_with_entry_value(
        fn_ir,
        lp,
        site,
        ReduceKind::Sum,
        plan.acc_phi,
        reduce_val,
    ) else {
        return false;
    };
    finish_vector_phi_assignment(fn_ir, site, plan.acc_phi, out_val)
}

pub(crate) fn apply_reduce_2d_col_sum_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: Reduce2DApplyPlan,
) -> bool {
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let col_val = resolve_materialized_value(fn_ir, plan.axis);
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_col_sum_range".to_string(),
            args: vec![plan.base, col_val, plan.range.start, end],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let Some(out_val) = combine_reduction_with_entry_value(
        fn_ir,
        lp,
        site,
        ReduceKind::Sum,
        plan.acc_phi,
        reduce_val,
    ) else {
        return false;
    };
    finish_vector_phi_assignment(fn_ir, site, plan.acc_phi, out_val)
}

pub(crate) fn apply_reduce_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: Reduce3DApplyPlan,
) -> bool {
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let callee = match (plan.kind, plan.axis) {
        (ReduceKind::Sum, Axis3D::Dim1) => "rr_dim1_sum_range",
        (ReduceKind::Sum, Axis3D::Dim2) => "rr_dim2_sum_range",
        (ReduceKind::Sum, Axis3D::Dim3) => "rr_dim3_sum_range",
        (_, Axis3D::Dim1) => "rr_dim1_reduce_range",
        (_, Axis3D::Dim2) => "rr_dim2_reduce_range",
        (_, Axis3D::Dim3) => "rr_dim3_reduce_range",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let reduce_val = if plan.kind == ReduceKind::Sum {
        fn_ir.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args: vec![plan.base, fixed_a, fixed_b, plan.range.start, end],
                names: vec![None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    } else {
        let op_lit = fn_ir.add_value(
            ValueKind::Const(Lit::Str(reduce_function_name(plan.kind).to_string())),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        fn_ir.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args: vec![plan.base, fixed_a, fixed_b, plan.range.start, end, op_lit],
                names: vec![None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    };
    let Some(out_val) =
        combine_reduction_with_entry_value(fn_ir, lp, site, plan.kind, plan.acc_phi, reduce_val)
    else {
        return false;
    };
    finish_vector_phi_assignment(fn_ir, site, plan.acc_phi, out_val)
}
