//! Reduce/map vectorization rewrites and dispatch helpers.

use super::*;

pub(crate) fn reduce_function_name(kind: ReduceKind) -> &'static str {
    match kind {
        ReduceKind::Sum => "sum",
        ReduceKind::Prod => "prod",
        ReduceKind::Min => "min",
        ReduceKind::Max => "max",
    }
}

pub(crate) fn materialize_vector_expr_once(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    root: ValueId,
    iv_phi: ValueId,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    let idx_vec = build_loop_index_vector(fn_ir, lp)?;
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    materialize_vector_expr(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        allow_any_base,
        require_safe_index,
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
    let allow_any_base = true;
    let require_safe_index = false;
    let reduce_val = if kind == ReduceKind::Sum {
        if let Some(v) = rewrite_sum_add_const(fn_ir, lp, vec_expr, iv_phi) {
            v
        } else {
            let Some(input_vec) = materialize_vector_expr_once(
                fn_ir,
                lp,
                vec_expr,
                iv_phi,
                allow_any_base,
                require_safe_index,
            ) else {
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
        let Some(input_vec) = materialize_vector_expr_once(
            fn_ir,
            lp,
            vec_expr,
            iv_phi,
            allow_any_base,
            require_safe_index,
        ) else {
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

    finish_vector_phi_assignment(fn_ir, site, acc_phi, reduce_val)
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
        cond,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        true,
    ) {
        Some(v) => v,
        None => return false,
    };
    let then_vec = match materialize_vector_expr(
        fn_ir,
        entry.then_val,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => v,
        None => return false,
    };
    let else_vec = match materialize_vector_expr(
        fn_ir,
        entry.else_val,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
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
    finish_vector_phi_assignment(fn_ir, site, entry.acc_phi, reduce_val)
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
        cond,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        true,
    ) {
        Some(v) => v,
        None => return false,
    };

    let mut staged = Vec::with_capacity(entries.len());
    for entry in entries {
        let then_vec = match materialize_vector_expr(
            fn_ir,
            entry.then_val,
            iv_phi,
            idx_vec,
            lp,
            &mut memo,
            &mut interner,
            true,
            false,
        ) {
            Some(v) => v,
            None => return false,
        };
        let else_vec = match materialize_vector_expr(
            fn_ir,
            entry.else_val,
            iv_phi,
            idx_vec,
            lp,
            &mut memo,
            &mut interner,
            true,
            false,
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
    finish_vector_phi_assignment(fn_ir, site, plan.acc_phi, reduce_val)
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
    finish_vector_phi_assignment(fn_ir, site, plan.acc_phi, reduce_val)
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
    finish_vector_phi_assignment(fn_ir, site, plan.acc_phi, reduce_val)
}

pub(crate) fn apply_map_plan(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    dest: ValueId,
    src: ValueId,
    op: BinOp,
    other: ValueId,
    shadow_vars: Vec<VarId>,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        return false;
    };
    let map_val = fn_ir.add_value(
        ValueKind::Binary {
            op,
            lhs: src,
            rhs: other,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let shadow_idx = fn_ir.add_value(
        ValueKind::Len { base: map_val },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        map_val,
        &shadow_vars,
        Some(shadow_idx),
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_cond_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    cond: ValueId,
    then_val: ValueId,
    else_val: ValueId,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
    shadow_vars: Vec<VarId>,
) -> bool {
    let whole_dest = whole_dest && lp.limit_adjust == 0;
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let dest_ref = resolve_materialized_value(fn_ir, dest);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        return false;
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let cond_vec = match materialize_vector_expr(
        fn_ir,
        cond,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        true,
    ) {
        Some(v) => v,
        None => return false,
    };
    let then_vec = match materialize_vector_expr(
        fn_ir,
        then_val,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => v,
        None => return false,
    };
    let else_vec = match materialize_vector_expr(
        fn_ir,
        else_val,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => v,
        None => return false,
    };

    let (cond_vec, then_vec, else_vec) = prepare_cond_map_operands(
        fn_ir,
        site.preheader,
        dest_ref,
        cond_vec,
        then_vec,
        else_vec,
        start,
        end,
        whole_dest,
    );

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
    let out_val = if whole_dest {
        ifelse_val
    } else {
        build_slice_assignment_value(fn_ir, dest, start, end, ifelse_val)
    };
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        &shadow_vars,
        Some(end),
    )
}

pub(crate) fn apply_map_2d_row_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: Map2DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let op_sym = match plan.op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        _ => return false,
    };
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(op_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let row_val = resolve_materialized_value(fn_ir, plan.axis);
    let row_map_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_row_binop_assign".to_string(),
            args: vec![
                plan.dest,
                plan.lhs_src,
                plan.rhs_src,
                row_val,
                plan.range.start,
                end,
                op_lit,
            ],
            names: vec![None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, row_map_val)
}

pub(crate) fn apply_map_2d_col_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: Map2DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let op_sym = match plan.op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        _ => return false,
    };
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(op_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let col_val = resolve_materialized_value(fn_ir, plan.axis);
    let col_map_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_col_binop_assign".to_string(),
            args: vec![
                plan.dest,
                plan.lhs_src,
                plan.rhs_src,
                col_val,
                plan.range.start,
                end,
                op_lit,
            ],
            names: vec![None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, col_map_val)
}

pub(crate) fn apply_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: Map3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let op_sym = match plan.op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        _ => return false,
    };
    let callee = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_binop_assign",
        Axis3D::Dim2 => "rr_dim2_binop_assign",
        Axis3D::Dim3 => "rr_dim3_binop_assign",
    };
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(op_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let map_val = fn_ir.add_value(
        ValueKind::Call {
            callee: callee.to_string(),
            args: vec![
                plan.dest,
                plan.lhs_src,
                plan.rhs_src,
                fixed_a,
                fixed_b,
                plan.range.start,
                end,
                op_lit,
            ],
            names: vec![None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, map_val)
}

pub(crate) fn apply_cond_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: CondMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let callee = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_ifelse_assign",
        Axis3D::Dim2 => "rr_dim2_ifelse_assign",
        Axis3D::Dim3 => "rr_dim3_ifelse_assign",
    };
    let cmp_sym = match plan.cmp_op {
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        _ => return false,
    };
    let cmp_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(cmp_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: callee.to_string(),
            args: vec![
                plan.dest,
                plan.cond_lhs,
                plan.cond_rhs,
                cmp_lit,
                plan.then_src,
                plan.else_src,
                fixed_a,
                fixed_b,
                plan.range.start,
                end,
            ],
            names: vec![None, None, None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(crate) fn materialize_vector_or_scalar_expr(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    root: ValueId,
    iv_phi: ValueId,
    idx_seed: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    if expr_has_iv_dependency(fn_ir, root, iv_phi) {
        materialize_vector_expr(
            fn_ir, root, iv_phi, idx_seed, lp, memo, interner, true, false,
        )
    } else {
        Some(
            materialize_loop_invariant_scalar_expr(fn_ir, root, iv_phi, lp, memo, interner)
                .unwrap_or_else(|| resolve_materialized_value(fn_ir, root)),
        )
    }
}

pub(crate) fn apply_cond_map_3d_general_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: CondMap3DGeneralApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let idx_seed = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let lhs_vec = match materialize_vector_or_scalar_expr(
        fn_ir,
        lp,
        plan.cond_lhs,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let rhs_vec = match materialize_vector_or_scalar_expr(
        fn_ir,
        lp,
        plan.cond_rhs,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let then_vec = match materialize_vector_or_scalar_expr(
        fn_ir,
        lp,
        plan.then_val,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let else_vec = match materialize_vector_or_scalar_expr(
        fn_ir,
        lp,
        plan.else_val,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let cond_vec = fn_ir.add_value(
        ValueKind::Binary {
            op: plan.cmp_op,
            lhs: lhs_vec,
            rhs: rhs_vec,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let ifelse_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_vec, then_vec, else_vec],
            names: vec![None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_assign_values",
        Axis3D::Dim2 => "rr_dim2_assign_values",
        Axis3D::Dim3 => "rr_dim3_assign_values",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![
                plan.dest,
                ifelse_vec,
                fixed_a,
                fixed_b,
                plan.range.start,
                end,
            ],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(crate) fn apply_reduce_vector_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: VectorPlan,
) -> bool {
    match plan {
        VectorPlan::Reduce {
            kind,
            acc_phi,
            vec_expr,
            iv_phi,
        } => apply_reduce_plan(fn_ir, lp, site, kind, acc_phi, vec_expr, iv_phi),
        VectorPlan::ReduceCond {
            kind,
            acc_phi,
            cond,
            then_val,
            else_val,
            iv_phi,
        } => apply_reduce_cond_plan(
            fn_ir,
            lp,
            site,
            cond,
            ReduceCondEntry {
                kind,
                acc_phi,
                then_val,
                else_val,
            },
            iv_phi,
        ),
        VectorPlan::MultiReduceCond {
            cond,
            entries,
            iv_phi,
        } => apply_multi_reduce_cond_plan(fn_ir, lp, site, cond, &entries, iv_phi),
        VectorPlan::Reduce2DRowSum {
            acc_phi,
            base,
            row,
            start,
            end,
        } => apply_reduce_2d_row_sum_plan(
            fn_ir,
            lp,
            site,
            Reduce2DApplyPlan {
                acc_phi,
                base,
                axis: row,
                range: VectorLoopRange { start, end },
            },
        ),
        VectorPlan::Reduce2DColSum {
            acc_phi,
            base,
            col,
            start,
            end,
        } => apply_reduce_2d_col_sum_plan(
            fn_ir,
            lp,
            site,
            Reduce2DApplyPlan {
                acc_phi,
                base,
                axis: col,
                range: VectorLoopRange { start, end },
            },
        ),
        VectorPlan::Reduce3D {
            kind,
            acc_phi,
            base,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
        } => apply_reduce_3d_plan(
            fn_ir,
            lp,
            site,
            Reduce3DApplyPlan {
                kind,
                acc_phi,
                base,
                axis,
                fixed_a,
                fixed_b,
                range: VectorLoopRange { start, end },
            },
        ),
        _ => false,
    }
}

pub(crate) fn apply_linear_vector_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: VectorPlan,
) -> bool {
    match plan {
        VectorPlan::Map {
            dest,
            src,
            op,
            other,
            shadow_vars,
        } => apply_map_plan(fn_ir, site, dest, src, op, other, shadow_vars),
        VectorPlan::CondMap {
            dest,
            cond,
            then_val,
            else_val,
            iv_phi,
            start,
            end,
            whole_dest,
            shadow_vars,
        } => apply_cond_map_plan(
            fn_ir,
            lp,
            site,
            dest,
            cond,
            then_val,
            else_val,
            iv_phi,
            start,
            end,
            whole_dest,
            shadow_vars,
        ),
        VectorPlan::CondMap3D {
            dest,
            axis,
            fixed_a,
            fixed_b,
            cond_lhs,
            cond_rhs,
            cmp_op,
            then_src,
            else_src,
            start,
            end,
        } => apply_cond_map_3d_plan(
            fn_ir,
            lp,
            site,
            CondMap3DApplyPlan {
                dest,
                axis,
                fixed_a,
                fixed_b,
                cond_lhs,
                cond_rhs,
                cmp_op,
                then_src,
                else_src,
                range: VectorLoopRange { start, end },
            },
        ),
        VectorPlan::CondMap3DGeneral {
            dest,
            axis,
            fixed_a,
            fixed_b,
            cond_lhs,
            cond_rhs,
            cmp_op,
            then_val,
            else_val,
            iv_phi,
            start,
            end,
        } => apply_cond_map_3d_general_plan(
            fn_ir,
            lp,
            site,
            CondMap3DGeneralApplyPlan {
                dest,
                axis,
                fixed_a,
                fixed_b,
                cond_lhs,
                cond_rhs,
                cmp_op,
                then_val,
                else_val,
                iv_phi,
                range: VectorLoopRange { start, end },
            },
        ),
        VectorPlan::RecurrenceAddConst {
            base,
            start,
            end,
            delta,
            negate_delta,
        } => apply_recurrence_add_const_plan(
            fn_ir,
            lp,
            site,
            RecurrenceAddConstApplyPlan {
                base,
                range: VectorLoopRange { start, end },
                delta,
                negate_delta,
            },
        ),
        VectorPlan::RecurrenceAddConst3D {
            base,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
            delta,
            negate_delta,
        } => apply_recurrence_add_const_3d_plan(
            fn_ir,
            lp,
            site,
            RecurrenceAddConst3DApplyPlan {
                base,
                axis,
                fixed_a,
                fixed_b,
                range: VectorLoopRange { start, end },
                delta,
                negate_delta,
            },
        ),
        VectorPlan::ShiftedMap {
            dest,
            src,
            start,
            end,
            offset,
        } => apply_shifted_map_plan(
            fn_ir,
            lp,
            site,
            ShiftedMapApplyPlan {
                dest,
                src,
                range: VectorLoopRange { start, end },
                offset,
            },
        ),
        VectorPlan::ShiftedMap3D {
            dest,
            src,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
            offset,
        } => apply_shifted_map_3d_plan(
            fn_ir,
            lp,
            site,
            ShiftedMap3DApplyPlan {
                dest,
                src,
                axis,
                fixed_a,
                fixed_b,
                range: VectorLoopRange { start, end },
                offset,
            },
        ),
        VectorPlan::CallMap {
            dest,
            callee,
            args,
            iv_phi,
            start,
            end,
            whole_dest,
            shadow_vars,
        } => apply_call_map_plan(
            fn_ir,
            lp,
            site,
            dest,
            callee,
            args,
            iv_phi,
            start,
            end,
            whole_dest,
            shadow_vars,
        ),
        VectorPlan::CallMap3D {
            dest,
            callee,
            args,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
        } => apply_call_map_3d_plan(
            fn_ir,
            lp,
            site,
            CallMap3DApplyPlan {
                dest,
                callee,
                args,
                axis,
                fixed_a,
                fixed_b,
                range: VectorLoopRange { start, end },
            },
        ),
        VectorPlan::CallMap3DGeneral {
            dest,
            callee,
            args,
            axis,
            fixed_a,
            fixed_b,
            iv_phi,
            start,
            end,
        } => apply_call_map_3d_general_plan(
            fn_ir,
            lp,
            site,
            CallMap3DGeneralApplyPlan {
                dest,
                callee,
                args,
                axis,
                fixed_a,
                fixed_b,
                iv_phi,
                range: VectorLoopRange { start, end },
            },
        ),
        VectorPlan::ExprMap3D {
            dest,
            expr,
            iv_phi,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
        } => apply_expr_map_3d_plan(
            fn_ir,
            lp,
            site,
            ExprMap3DApplyPlan {
                dest,
                expr,
                iv_phi,
                axis,
                fixed_a,
                fixed_b,
                range: VectorLoopRange { start, end },
            },
        ),
        _ => false,
    }
}
