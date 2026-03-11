use super::*;

#[derive(Clone, Copy)]
pub(super) struct VectorApplySite {
    preheader: BlockId,
    exit_bb: BlockId,
}

#[derive(Clone, Copy)]
pub(super) struct VectorLoopRange {
    start: ValueId,
    end: ValueId,
}

#[derive(Debug)]
pub(super) struct PreparedVectorAssignment {
    dest_var: VarId,
    out_val: ValueId,
    shadow_vars: Vec<VarId>,
    shadow_idx: Option<ValueId>,
}

#[derive(Clone, Copy)]
pub(super) struct Reduce2DApplyPlan {
    acc_phi: ValueId,
    base: ValueId,
    axis: ValueId,
    range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct Reduce3DApplyPlan {
    kind: ReduceKind,
    acc_phi: ValueId,
    base: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct RecurrenceAddConstApplyPlan {
    base: ValueId,
    range: VectorLoopRange,
    delta: ValueId,
    negate_delta: bool,
}

#[derive(Clone, Copy)]
pub(super) struct RecurrenceAddConst3DApplyPlan {
    base: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    range: VectorLoopRange,
    delta: ValueId,
    negate_delta: bool,
}

#[derive(Clone, Copy)]
pub(super) struct ShiftedMapApplyPlan {
    dest: ValueId,
    src: ValueId,
    range: VectorLoopRange,
    offset: i64,
}

#[derive(Clone, Copy)]
pub(super) struct ShiftedMap3DApplyPlan {
    dest: ValueId,
    src: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    range: VectorLoopRange,
    offset: i64,
}

#[derive(Clone, Copy)]
pub(super) struct Map2DApplyPlan {
    dest: ValueId,
    axis: ValueId,
    range: VectorLoopRange,
    lhs_src: ValueId,
    rhs_src: ValueId,
    op: BinOp,
}

#[derive(Clone, Copy)]
pub(super) struct Map3DApplyPlan {
    dest: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    range: VectorLoopRange,
    lhs_src: ValueId,
    rhs_src: ValueId,
    op: BinOp,
}

pub(super) struct CallMap3DApplyPlan {
    dest: ValueId,
    callee: String,
    args: Vec<CallMapArg>,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    range: VectorLoopRange,
}

pub(super) struct CallMap3DGeneralApplyPlan {
    dest: ValueId,
    callee: String,
    args: Vec<CallMapArg>,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    iv_phi: ValueId,
    range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct ExprMap3DApplyPlan {
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct ScatterExprMap3DApplyPlan {
    dest: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    idx: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
}

#[derive(Clone, Copy)]
pub(super) struct ScatterExprMap3DGeneralApplyPlan {
    dest: ValueId,
    i: VectorAccessOperand3D,
    j: VectorAccessOperand3D,
    k: VectorAccessOperand3D,
    expr: ValueId,
    iv_phi: ValueId,
}

#[derive(Clone, Copy)]
pub(super) struct CondMap3DApplyPlan {
    dest: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    cond_lhs: ValueId,
    cond_rhs: ValueId,
    cmp_op: BinOp,
    then_src: ValueId,
    else_src: ValueId,
    range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct CondMap3DGeneralApplyPlan {
    dest: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    cond_lhs: ValueId,
    cond_rhs: ValueId,
    cmp_op: BinOp,
    then_val: ValueId,
    else_val: ValueId,
    iv_phi: ValueId,
    range: VectorLoopRange,
}

pub(super) fn vector_apply_site(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorApplySite> {
    let preds = build_pred_map(fn_ir);
    let outer_preds: Vec<BlockId> = preds
        .get(&lp.header)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|b| !lp.body.contains(b))
        .collect();

    if outer_preds.len() != 1 {
        return None;
    }
    if lp.exits.len() != 1 {
        return None;
    }

    Some(VectorApplySite {
        preheader: outer_preds[0],
        exit_bb: lp.exits[0],
    })
}

pub(super) fn finish_vector_assignment(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    dest_var: VarId,
    out_val: ValueId,
) -> bool {
    finish_vector_assignment_with_shadow_states(fn_ir, site, dest_var, out_val, &[], None)
}

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

pub(super) fn build_slice_assignment_value(
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
    for assignment in assignments {
        fn_ir.blocks[site.preheader].instrs.push(Instr::Assign {
            dst: assignment.dest_var.clone(),
            src: assignment.out_val,
            span: crate::utils::Span::dummy(),
        });
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
                rewrite_returns_for_var(fn_ir, shadow_var, shadow_val);
            }
        }
        rewrite_returns_for_var(fn_ir, &assignment.dest_var, assignment.out_val);
    }
    fn_ir.blocks[site.preheader].term = Terminator::Goto(site.exit_bb);
    true
}

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

pub(super) fn reduce_function_name(kind: ReduceKind) -> &'static str {
    match kind {
        ReduceKind::Sum => "sum",
        ReduceKind::Prod => "prod",
        ReduceKind::Min => "min",
        ReduceKind::Max => "max",
    }
}

pub(super) fn materialize_vector_expr_once(
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

pub(super) fn apply_reduce_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    kind: ReduceKind,
    acc_phi: ValueId,
    vec_expr: ValueId,
    iv_phi: ValueId,
) -> bool {
    let allow_any_base = expr_contains_index3d(fn_ir, vec_expr);
    let require_safe_index = !allow_any_base;
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

pub(super) fn apply_reduce_2d_row_sum_plan(
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

pub(super) fn apply_reduce_2d_col_sum_plan(
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

pub(super) fn apply_reduce_3d_plan(
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

pub(super) fn apply_map_plan(
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
pub(super) fn apply_cond_map_plan(
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

pub(super) fn apply_recurrence_add_const_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: RecurrenceAddConstApplyPlan,
) -> bool {
    let Some(base_var) = resolve_base_var(fn_ir, plan.base) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let delta_val = if plan.negate_delta {
        fn_ir.add_value(
            ValueKind::Unary {
                op: crate::syntax::ast::UnaryOp::Neg,
                rhs: plan.delta,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    } else {
        plan.delta
    };
    let recur_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_recur_add_const".to_string(),
            args: vec![plan.base, plan.range.start, end, delta_val],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, base_var, recur_val)
}

pub(super) fn apply_recurrence_add_const_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: RecurrenceAddConst3DApplyPlan,
) -> bool {
    let Some(base_var) = resolve_base_var(fn_ir, plan.base) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let delta_val = if plan.negate_delta {
        fn_ir.add_value(
            ValueKind::Unary {
                op: crate::syntax::ast::UnaryOp::Neg,
                rhs: plan.delta,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    } else {
        plan.delta
    };
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_recur_add_const",
        Axis3D::Dim2 => "rr_dim2_recur_add_const",
        Axis3D::Dim3 => "rr_dim3_recur_add_const",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let recur_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![
                plan.base,
                fixed_a,
                fixed_b,
                plan.range.start,
                end,
                delta_val,
            ],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, base_var, recur_val)
}

pub(super) fn apply_shifted_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ShiftedMapApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let src_start = add_int_offset(fn_ir, plan.range.start, plan.offset);
    let src_end = add_int_offset(fn_ir, end, plan.offset);
    let shifted_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_shift_assign".to_string(),
            args: vec![
                plan.dest,
                plan.src,
                plan.range.start,
                end,
                src_start,
                src_end,
            ],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, shifted_val)
}

pub(super) fn apply_shifted_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ShiftedMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let src_start = add_int_offset(fn_ir, plan.range.start, plan.offset);
    let src_end = add_int_offset(fn_ir, end, plan.offset);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_shift_assign",
        Axis3D::Dim2 => "rr_dim2_shift_assign",
        Axis3D::Dim3 => "rr_dim3_shift_assign",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let shifted_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![
                plan.dest,
                plan.src,
                fixed_a,
                fixed_b,
                plan.range.start,
                end,
                src_start,
                src_end,
            ],
            names: vec![None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, shifted_val)
}

pub(super) fn emit_call_map_argument_guards(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    dest: ValueId,
    whole_dest: bool,
    mapped_args: &[(ValueId, bool)],
    vector_args: &[ValueId],
) {
    for (arg, is_vec) in mapped_args {
        let check_val = if whole_dest && *is_vec && !same_length_proven(fn_ir, dest, *arg) {
            Some(fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_same_len".to_string(),
                    args: vec![dest, *arg],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ))
        } else if !*is_vec && !is_const_number(fn_ir, *arg) {
            Some(fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_same_or_scalar".to_string(),
                    args: vec![dest, *arg],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ))
        } else {
            None
        };
        if let Some(val) = check_val {
            fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                val,
                span: crate::utils::Span::dummy(),
            });
        }
    }

    for i in 0..vector_args.len() {
        for j in (i + 1)..vector_args.len() {
            let a = vector_args[i];
            let b = vector_args[j];
            if same_length_proven(fn_ir, a, b) {
                continue;
            }
            let check_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_same_len".to_string(),
                    args: vec![a, b],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                val: check_val,
                span: crate::utils::Span::dummy(),
            });
        }
    }
}

pub(super) fn build_int_vector_literal(fn_ir: &mut FnIR, items: &[i64]) -> ValueId {
    let args: Vec<ValueId> = items
        .iter()
        .map(|item| {
            fn_ir.add_value(
                ValueKind::Const(Lit::Int(*item)),
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            )
        })
        .collect();
    fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args,
            names: vec![None; items.len()],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_call_map_auto_value(
    fn_ir: &mut FnIR,
    dest: ValueId,
    start: ValueId,
    end: ValueId,
    callee: &str,
    helper_cost: u32,
    mapped_args: &[(ValueId, bool)],
    whole_dest: bool,
) -> ValueId {
    let callee_val = fn_ir.add_value(
        ValueKind::Const(Lit::Str(callee.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let helper_cost_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(helper_cost as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let vector_slots: Vec<i64> = mapped_args
        .iter()
        .enumerate()
        .filter_map(|(index, (_, is_vec))| is_vec.then_some((index + 1) as i64))
        .collect();
    let vector_slots_val = build_int_vector_literal(fn_ir, &vector_slots);
    let mut args = if whole_dest {
        vec![dest, callee_val, helper_cost_val, vector_slots_val]
    } else {
        vec![
            dest,
            start,
            end,
            callee_val,
            helper_cost_val,
            vector_slots_val,
        ]
    };
    args.extend(mapped_args.iter().map(|(arg, _)| *arg));
    let callee_name = if whole_dest {
        "rr_call_map_whole_auto"
    } else {
        "rr_call_map_slice_auto"
    };
    fn_ir.add_value(
        ValueKind::Call {
            callee: callee_name.to_string(),
            args,
            names: vec![None; mapped_args.len() + if whole_dest { 4 } else { 6 }],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_call_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    callee: String,
    args: Vec<CallMapArg>,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
    shadow_vars: Vec<VarId>,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    let lowering_mode = choose_call_map_lowering(fn_ir, &callee, &args, whole_dest, &shadow_vars);
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut mapped_args = Vec::with_capacity(args.len());
    let mut vector_args = Vec::new();
    for (arg_i, arg) in args.into_iter().enumerate() {
        let out = if arg.vectorized {
            match materialize_vector_expr(
                fn_ir,
                arg.value,
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
            }
        } else {
            resolve_materialized_value(fn_ir, arg.value)
        };
        let out = if arg.vectorized {
            maybe_hoist_callmap_arg_expr(fn_ir, site.preheader, out, arg_i)
        } else {
            out
        };
        if arg.vectorized {
            vector_args.push(out);
        }
        mapped_args.push((out, arg.vectorized));
    }

    emit_call_map_argument_guards(
        fn_ir,
        site.preheader,
        dest,
        whole_dest,
        &mapped_args,
        &vector_args,
    );

    let out_val = match lowering_mode {
        CallMapLoweringMode::RuntimeAuto { helper_cost } => {
            if trace_enabled {
                eprintln!(
                    "   [vec-profit] {} call_map runtime-auto callee={} helper_cost={} whole_dest={}",
                    fn_ir.name, callee, helper_cost, whole_dest
                );
            }
            build_call_map_auto_value(
                fn_ir,
                dest,
                start,
                end,
                &callee,
                helper_cost,
                &mapped_args,
                whole_dest,
            )
        }
        CallMapLoweringMode::DirectVector => {
            let mapped_args_vals: Vec<ValueId> = mapped_args.iter().map(|(arg, _)| *arg).collect();
            let mapped_val = fn_ir.add_value(
                if let Some(op) = intrinsic_for_call(&callee, mapped_args_vals.len()) {
                    ValueKind::Intrinsic {
                        op,
                        args: mapped_args_vals,
                    }
                } else {
                    ValueKind::Call {
                        callee,
                        args: mapped_args_vals,
                        names: vec![None; mapped_args.len()],
                    }
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            if whole_dest {
                mapped_val
            } else {
                let mapped_val = prepare_partial_slice_value(fn_ir, dest, mapped_val, start, end);
                fn_ir.add_value(
                    ValueKind::Call {
                        callee: "rr_assign_slice".to_string(),
                        args: vec![dest, start, end, mapped_val],
                        names: vec![None, None, None, None],
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                )
            }
        }
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

pub(super) fn apply_call_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: CallMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let Some(iv_phi) = lp.iv.as_ref().map(|iv| iv.phi_val) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_call_assign",
        Axis3D::Dim2 => "rr_dim2_call_assign",
        Axis3D::Dim3 => "rr_dim3_call_assign",
    };
    let callee_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(plan.callee)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let mut mapped_args = vec![
        plan.dest,
        callee_lit,
        fixed_a,
        fixed_b,
        plan.range.start,
        end,
    ];
    for (arg_i, arg) in plan.args.into_iter().enumerate() {
        let out = if arg.vectorized {
            match materialize_vector_expr(
                fn_ir,
                arg.value,
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
            }
        } else {
            resolve_materialized_value(fn_ir, arg.value)
        };
        let out = if arg.vectorized {
            maybe_hoist_callmap_arg_expr(fn_ir, site.preheader, out, arg_i)
        } else {
            out
        };
        mapped_args.push(out);
    }
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: mapped_args.clone(),
            names: vec![None; mapped_args.len()],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn apply_call_map_3d_general_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: CallMap3DGeneralApplyPlan,
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
    let mut mapped_args = Vec::with_capacity(plan.args.len());
    for arg in plan.args {
        let out = match materialize_vector_or_scalar_expr(
            fn_ir,
            lp,
            arg.value,
            plan.iv_phi,
            idx_seed,
            &mut memo,
            &mut interner,
        ) {
            Some(v) => v,
            None => return false,
        };
        mapped_args.push(out);
    }
    let arg_len = mapped_args.len();
    let call_val = fn_ir.add_value(
        if let Some(op) = intrinsic_for_call(&plan.callee, arg_len) {
            ValueKind::Intrinsic {
                op,
                args: mapped_args.clone(),
            }
        } else {
            ValueKind::Call {
                callee: plan.callee,
                args: mapped_args,
                names: vec![None; arg_len],
            }
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
            args: vec![plan.dest, call_val, fixed_a, fixed_b, plan.range.start, end],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn apply_expr_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ExprMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        plan.expr,
        plan.iv_phi,
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
            args: vec![plan.dest, expr_vec, fixed_a, fixed_b, plan.range.start, end],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn same_load_leaf_var_in_phi_tree(
    fn_ir: &FnIR,
    root: ValueId,
    seen: &mut FxHashSet<ValueId>,
) -> Option<VarId> {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return None;
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::Load { var } => Some(var.clone()),
        ValueKind::Phi { args } if !args.is_empty() => {
            let mut found: Option<VarId> = None;
            for (arg, _) in args {
                let leaf_var = same_load_leaf_var_in_phi_tree(fn_ir, *arg, seen)?;
                match &found {
                    None => found = Some(leaf_var),
                    Some(prev) if prev == &leaf_var => {}
                    Some(_) => return None,
                }
            }
            found
        }
        _ => None,
    };
    seen.remove(&root);
    out
}

pub(super) fn recover_cube_slice_snapshot_scalar(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    value: ValueId,
    iv_phi: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    let root = canonical_value(fn_ir, value);
    let target_bb = fn_ir.values[root]
        .phi_block
        .or_else(|| value_use_block_in_loop(fn_ir, lp, root));
    let snapshot_var = fn_ir.values[root]
        .origin_var
        .clone()
        .or_else(|| induction_origin_var(fn_ir, root))?;
    let candidate = target_bb
        .and_then(|bb| unique_assign_source_reaching_block_in_loop(fn_ir, lp, &snapshot_var, bb))
        .or_else(|| loop_entry_seed_source_in_loop(fn_ir, lp, &snapshot_var))
        .or_else(|| unique_assign_source_in_loop(fn_ir, lp, &snapshot_var))
        .map(|src| canonical_value(fn_ir, src))
        .filter(|src| {
            *src != root && !value_depends_on(fn_ir, *src, root, &mut FxHashSet::default())
        })
        .unwrap_or(root);
    if candidate != root
        && let Some(v) =
            materialize_loop_invariant_scalar_expr(fn_ir, candidate, iv_phi, lp, memo, interner)
    {
        return Some(resolve_materialized_value(fn_ir, v));
    }
    if let Some(var) = fn_ir.values[candidate].origin_var.clone()
        && !has_non_passthrough_assignment_in_loop(fn_ir, lp, &var)
    {
        let load = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Load { var },
            fn_ir.values[candidate].span,
            fn_ir.values[candidate].facts,
        );
        return Some(resolve_materialized_value(fn_ir, load));
    }
    let leaf_var = same_load_leaf_var_in_phi_tree(fn_ir, candidate, &mut FxHashSet::default())?;
    if has_non_passthrough_assignment_in_loop(fn_ir, lp, &leaf_var) {
        return None;
    }
    let load = intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Load { var: leaf_var },
        fn_ir.values[root].span,
        fn_ir.values[root].facts,
    );
    Some(resolve_materialized_value(fn_ir, load))
}

pub(super) fn cube_slice_expr_has_complex_loop_local_axes(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    expr: ValueId,
    iv_phi: ValueId,
) -> bool {
    let expr = canonical_value(fn_ir, expr);
    let ValueKind::Call { callee, args, .. } = &fn_ir.values[expr].kind else {
        return false;
    };
    if callee != "rr_idx_cube_vec_i" || args.len() < 4 {
        return false;
    }
    for arg in [args[1], args[2]] {
        let arg = canonical_value(fn_ir, arg);
        if is_iv_equivalent(fn_ir, arg, iv_phi) {
            continue;
        }
        if let ValueKind::Load { var } = &fn_ir.values[arg].kind
            && has_assignment_in_loop(fn_ir, lp, var)
        {
            return true;
        }
        if let Some(var) = fn_ir.values[arg].origin_var.as_deref()
            && has_assignment_in_loop(fn_ir, lp, var)
            && !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, &FxHashSet::default())
        {
            return true;
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_cube_slice_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
    face: ValueId,
    row: ValueId,
    size: ValueId,
    ctx: Option<ValueId>,
    start: ValueId,
    end: ValueId,
    shadow_vars: Vec<VarId>,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();

    let materialize_scalar =
        |value: ValueId,
         label: &str,
         fn_ir: &mut FnIR,
         memo: &mut FxHashMap<ValueId, ValueId>,
         interner: &mut FxHashMap<MaterializedExprKey, ValueId>| {
            match materialize_loop_invariant_scalar_expr(fn_ir, value, iv_phi, lp, memo, interner) {
                Some(v) => Some(resolve_materialized_value(fn_ir, v)),
                None => {
                    if let Some(v) =
                        recover_cube_slice_snapshot_scalar(fn_ir, lp, value, iv_phi, memo, interner)
                    {
                        return Some(v);
                    }
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: invariant scalar {} materialization ({:?})",
                            fn_ir.name, label, fn_ir.values[value].kind
                        );
                    }
                    None
                }
            }
        };

    let Some(face) = materialize_scalar(face, "face", fn_ir, &mut memo, &mut interner) else {
        return false;
    };
    let Some(row) = materialize_scalar(row, "row", fn_ir, &mut memo, &mut interner) else {
        return false;
    };
    let Some(size) = materialize_scalar(size, "size", fn_ir, &mut memo, &mut interner) else {
        return false;
    };
    let ctx = match ctx {
        Some(ctx_val) => {
            match materialize_scalar(ctx_val, "ctx", fn_ir, &mut memo, &mut interner) {
                Some(v) => Some(v),
                None => return false,
            }
        }
        None => None,
    };

    if cube_slice_expr_has_complex_loop_local_axes(fn_ir, lp, expr, iv_phi) {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: cube-slice rhs has complex loop-local axes ({:?})",
                fn_ir.name,
                fn_ir.values[canonical_value(fn_ir, expr)].kind
            );
        }
        return false;
    }

    let expr_vec = if expr_has_iv_dependency(fn_ir, expr, iv_phi) {
        match materialize_vector_expr(
            fn_ir,
            expr,
            iv_phi,
            idx_vec,
            lp,
            &mut memo,
            &mut interner,
            true,
            false,
        ) {
            Some(v) => v,
            None => {
                if trace_enabled {
                    eprintln!(
                        "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                        fn_ir.name, fn_ir.values[expr].kind
                    );
                }
                return false;
            }
        }
    } else {
        match materialize_loop_invariant_scalar_expr(
            fn_ir,
            expr,
            iv_phi,
            lp,
            &mut memo,
            &mut interner,
        ) {
            Some(v) => v,
            None => {
                if let Some(v) = materialize_vector_expr(
                    fn_ir,
                    expr,
                    iv_phi,
                    idx_vec,
                    lp,
                    &mut memo,
                    &mut interner,
                    true,
                    false,
                ) {
                    v
                } else {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: invariant scalar expr materialization ({:?})",
                            fn_ir.name, fn_ir.values[expr].kind
                        );
                    }
                    return false;
                }
            }
        }
    };

    let expr_vec = broadcast_scalar_expr_to_slice_len(fn_ir, expr_vec, start, end);

    let has_ctx = ctx.is_some();
    let mut start_args = vec![face, row, start, size];
    let mut end_args = vec![face, row, end, size];
    if let Some(ctx) = ctx {
        start_args.push(ctx);
        end_args.push(ctx);
    }
    let names = vec![None; if has_ctx { 5 } else { 4 }];
    let slice_start = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_idx_cube_vec_i".to_string(),
            args: start_args,
            names: names.clone(),
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let slice_end = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_idx_cube_vec_i".to_string(),
            args: end_args,
            names,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_assign_slice".to_string(),
            args: vec![dest, slice_start, slice_end, expr_vec],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        &shadow_vars,
        Some(slice_end),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_apply_expr_call_map_auto(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
    shadow_vars: &[VarId],
) -> Option<bool> {
    let root = canonical_value(fn_ir, expr);
    let (callee, args) = match &fn_ir.values[root].kind {
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            if names.iter().any(Option::is_some) {
                return None;
            }
            (callee.clone(), args.clone())
        }
        _ => return None,
    };

    let call_args: Vec<CallMapArg> = args
        .iter()
        .map(|arg| CallMapArg {
            value: *arg,
            vectorized: expr_has_iv_dependency(fn_ir, *arg, iv_phi),
        })
        .collect();
    let CallMapLoweringMode::RuntimeAuto { helper_cost } =
        choose_call_map_lowering(fn_ir, &callee, &call_args, whole_dest, shadow_vars)
    else {
        return None;
    };

    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        return Some(false);
    };
    let idx_vec = build_loop_index_vector(fn_ir, lp)?;
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut mapped_args = Vec::with_capacity(call_args.len());
    let mut vector_args = Vec::new();

    for (arg_i, arg) in call_args.iter().enumerate() {
        let out = if arg.vectorized {
            materialize_vector_expr(
                fn_ir,
                arg.value,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            )?
        } else {
            materialize_loop_invariant_scalar_expr(
                fn_ir,
                arg.value,
                iv_phi,
                lp,
                &mut memo,
                &mut interner,
            )
            .unwrap_or_else(|| resolve_materialized_value(fn_ir, arg.value))
        };
        let out = if arg.vectorized {
            maybe_hoist_callmap_arg_expr(fn_ir, site.preheader, out, arg_i)
        } else {
            out
        };
        if arg.vectorized {
            vector_args.push(out);
        }
        mapped_args.push((out, arg.vectorized));
    }

    emit_call_map_argument_guards(
        fn_ir,
        site.preheader,
        dest,
        whole_dest,
        &mapped_args,
        &vector_args,
    );
    if vectorize_trace_enabled() {
        eprintln!(
            "   [vec-profit] {} expr_map runtime-auto callee={} helper_cost={} whole_dest={}",
            fn_ir.name, callee, helper_cost, whole_dest
        );
    }
    let out_val = build_call_map_auto_value(
        fn_ir,
        dest,
        start,
        end,
        &callee,
        helper_cost,
        &mapped_args,
        whole_dest,
    );
    Some(finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        shadow_vars,
        Some(end),
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
    shadow_vars: Vec<VarId>,
) -> bool {
    if let Some(applied) = try_apply_expr_call_map_auto(
        fn_ir,
        lp,
        site,
        dest,
        expr,
        iv_phi,
        start,
        end,
        whole_dest,
        &shadow_vars,
    ) {
        return applied;
    }

    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        expr,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                    fn_ir.name, fn_ir.values[expr].kind
                );
            }
            return false;
        }
    };

    let out_val = build_expr_map_output_value(
        fn_ir,
        site.preheader,
        dest,
        expr_vec,
        start,
        end,
        whole_dest,
    );
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        &shadow_vars,
        Some(end),
    )
}

pub(super) fn build_expr_map_output_value(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    dest: ValueId,
    expr_vec: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
) -> ValueId {
    let expr_is_scalar = is_scalar_broadcast_value(fn_ir, expr_vec);
    let expr_vec = if expr_is_scalar {
        broadcast_scalar_expr_to_slice_len(fn_ir, expr_vec, start, end)
    } else if whole_dest {
        expr_vec
    } else {
        prepare_partial_slice_value(fn_ir, dest, expr_vec, start, end)
    };

    if whole_dest {
        if !expr_is_scalar && !same_length_proven(fn_ir, dest, expr_vec) {
            emit_same_len_guard(fn_ir, preheader, dest, expr_vec);
        }
        expr_vec
    } else {
        build_slice_assignment_value(fn_ir, dest, start, end, expr_vec)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn stage_multi_expr_map_entry(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entry_index: usize,
    entry: &ExprMapEntry,
    iv_phi: ValueId,
    idx_vec: ValueId,
    start: ValueId,
    end: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    trace_enabled: bool,
) -> Option<PreparedVectorAssignment> {
    let dest_var = resolve_base_var(fn_ir, entry.dest).or_else(|| {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        None
    })?;
    let expr_vec = materialize_vector_expr(
        fn_ir, entry.expr, iv_phi, idx_vec, lp, memo, interner, true, false,
    )
    .or_else(|| {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                fn_ir.name, fn_ir.values[entry.expr].kind
            );
        }
        None
    })?;
    let expr_vec = hoist_vector_expr_temp(
        fn_ir,
        site.preheader,
        expr_vec,
        &format!("exprmap{}", entry_index),
    );
    let out_val = build_expr_map_output_value(
        fn_ir,
        site.preheader,
        entry.dest,
        expr_vec,
        start,
        end,
        entry.whole_dest,
    );
    let shadow_idx = if entry.whole_dest {
        Some(fn_ir.add_value(
            ValueKind::Len { base: out_val },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ))
    } else {
        Some(end)
    };

    Some(PreparedVectorAssignment {
        dest_var,
        out_val,
        shadow_vars: entry.shadow_vars.clone(),
        shadow_idx,
    })
}

pub(super) fn apply_multi_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entries: Vec<ExprMapEntry>,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut staged = Vec::with_capacity(entries.len());

    for (entry_index, entry) in entries.iter().enumerate() {
        let Some(assignment) = stage_multi_expr_map_entry(
            fn_ir,
            lp,
            site,
            entry_index,
            entry,
            iv_phi,
            idx_vec,
            start,
            end,
            &mut memo,
            &mut interner,
            trace_enabled,
        ) else {
            return false;
        };
        staged.push(assignment);
    }
    emit_prepared_vector_assignments(fn_ir, site, staged)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn stage_multi_expr_map_3d_entry(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entry_index: usize,
    entry: &ExprMapEntry3D,
    iv_phi: ValueId,
    idx_vec: ValueId,
    start: ValueId,
    end: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    trace_enabled: bool,
) -> Option<PreparedVectorAssignment> {
    let dest_var = resolve_base_var(fn_ir, entry.dest).or_else(|| {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr3d] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        None
    })?;
    let expr_vec = materialize_vector_expr(
        fn_ir, entry.expr, iv_phi, idx_vec, lp, memo, interner, true, false,
    )
    .or_else(|| {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr3d] {} fail: materialize_vector_expr({:?})",
                fn_ir.name, fn_ir.values[entry.expr].kind
            );
        }
        None
    })?;
    let expr_vec = hoist_vector_expr_temp(
        fn_ir,
        site.preheader,
        expr_vec,
        &format!("exprmap3d{}", entry_index),
    );
    let helper = match entry.axis {
        Axis3D::Dim1 => "rr_dim1_assign_values",
        Axis3D::Dim2 => "rr_dim2_assign_values",
        Axis3D::Dim3 => "rr_dim3_assign_values",
    };
    let fixed_a = resolve_materialized_value(fn_ir, entry.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, entry.fixed_b);
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![entry.dest, expr_vec, fixed_a, fixed_b, start, end],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );

    Some(PreparedVectorAssignment {
        dest_var,
        out_val,
        shadow_vars: entry.shadow_vars.clone(),
        shadow_idx: Some(end),
    })
}

pub(super) fn apply_multi_expr_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entries: Vec<ExprMapEntry3D>,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr3d] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut staged = Vec::with_capacity(entries.len());

    for (entry_index, entry) in entries.iter().enumerate() {
        let Some(assignment) = stage_multi_expr_map_3d_entry(
            fn_ir,
            lp,
            site,
            entry_index,
            entry,
            iv_phi,
            idx_vec,
            start,
            end,
            &mut memo,
            &mut interner,
            trace_enabled,
        ) else {
            return false;
        };
        staged.push(assignment);
    }
    emit_prepared_vector_assignments(fn_ir, site, staged)
}

pub(super) fn apply_scatter_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    idx: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
) -> bool {
    let idx_seed = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        return false;
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let idx_vec = match materialize_vector_expr(
        fn_ir,
        idx,
        iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter_idx"),
        None => return false,
    };
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        expr,
        iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter_val"),
        None => return false,
    };
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_assign_index_vec".to_string(),
            args: vec![dest, idx_vec, expr_vec],
            names: vec![None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn apply_scatter_expr_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ScatterExprMap3DApplyPlan,
) -> bool {
    let idx_seed = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let idx_vec = match materialize_vector_expr(
        fn_ir,
        plan.idx,
        plan.iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter3d_idx"),
        None => return false,
    };
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        plan.expr,
        plan.iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter3d_val"),
        None => return false,
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_assign_index_values",
        Axis3D::Dim2 => "rr_dim2_assign_index_values",
        Axis3D::Dim3 => "rr_dim3_assign_index_values",
    };
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![plan.dest, expr_vec, fixed_a, fixed_b, idx_vec],
            names: vec![None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_3d_index_operand_for_scatter(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    operand: VectorAccessOperand3D,
    iv_phi: ValueId,
    idx_seed: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    match operand {
        VectorAccessOperand3D::Scalar(value) => Some(
            materialize_loop_invariant_scalar_expr(fn_ir, value, iv_phi, lp, memo, interner)
                .unwrap_or_else(|| resolve_materialized_value(fn_ir, value)),
        ),
        VectorAccessOperand3D::Vector(value) => {
            let mut materialized = if is_iv_equivalent(fn_ir, value, iv_phi) {
                idx_seed
            } else {
                materialize_vector_expr(
                    fn_ir, value, iv_phi, idx_seed, lp, memo, interner, true, false,
                )?
            };
            if !is_int_index_vector_value(fn_ir, materialized) {
                materialized = fn_ir.add_value(
                    ValueKind::Call {
                        callee: "rr_index_vec_floor".to_string(),
                        args: vec![materialized],
                        names: vec![None],
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                );
            }
            Some(hoist_vector_expr_temp(
                fn_ir,
                site.preheader,
                materialized,
                "scatter3d_axis",
            ))
        }
    }
}

pub(super) fn apply_scatter_expr_map_3d_general_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ScatterExprMap3DGeneralApplyPlan,
) -> bool {
    let idx_seed = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let i_vec = match materialize_3d_index_operand_for_scatter(
        fn_ir,
        lp,
        site,
        plan.i,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let j_vec = match materialize_3d_index_operand_for_scatter(
        fn_ir,
        lp,
        site,
        plan.j,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let k_vec = match materialize_3d_index_operand_for_scatter(
        fn_ir,
        lp,
        site,
        plan.k,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        plan.expr,
        plan.iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter3d_val"),
        None => return false,
    };
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_array3_assign_gather_values".to_string(),
            args: vec![plan.dest, expr_vec, i_vec, j_vec, k_vec],
            names: vec![None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn broadcast_scalar_expr_to_slice_len(
    fn_ir: &mut FnIR,
    expr_vec: ValueId,
    start: ValueId,
    end: ValueId,
) -> ValueId {
    if !is_scalar_broadcast_value(fn_ir, expr_vec) {
        return expr_vec;
    }

    let slice_len = if is_const_one(fn_ir, start) {
        end
    } else {
        let span = crate::utils::Span::dummy();
        let facts = crate::mir::def::Facts::empty();
        let span_delta = fn_ir.add_value(
            ValueKind::Binary {
                op: crate::syntax::ast::BinOp::Sub,
                lhs: end,
                rhs: start,
            },
            span,
            facts,
            None,
        );
        let one_val = fn_ir.add_value(ValueKind::Const(Lit::Float(1.0)), span, facts, None);
        fn_ir.add_value(
            ValueKind::Binary {
                op: crate::syntax::ast::BinOp::Add,
                lhs: span_delta,
                rhs: one_val,
            },
            span,
            facts,
            None,
        )
    };

    fn_ir.add_value(
        ValueKind::Call {
            callee: "rep.int".to_string(),
            args: vec![expr_vec, slice_len],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

pub(super) fn narrow_vector_expr_to_slice_range(
    fn_ir: &mut FnIR,
    expr_vec: ValueId,
    start: ValueId,
    end: ValueId,
) -> ValueId {
    let idx_range = fn_ir.add_value(
        ValueKind::Range { start, end },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_index1_read_vec".to_string(),
            args: vec![expr_vec, idx_range],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

pub(super) fn prepare_partial_slice_value(
    fn_ir: &mut FnIR,
    dest: ValueId,
    expr_vec: ValueId,
    start: ValueId,
    end: ValueId,
) -> ValueId {
    if is_scalar_broadcast_value(fn_ir, expr_vec) {
        return broadcast_scalar_expr_to_slice_len(fn_ir, expr_vec, start, end);
    }
    if same_length_proven(fn_ir, dest, expr_vec) {
        return narrow_vector_expr_to_slice_range(fn_ir, expr_vec, start, end);
    }
    expr_vec
}

pub(super) fn apply_map_2d_row_plan(
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

pub(super) fn apply_map_2d_col_plan(
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

pub(super) fn apply_map_3d_plan(
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

pub(super) fn apply_cond_map_3d_plan(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_or_scalar_expr(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_cond_map_3d_general_plan(
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

pub(super) fn apply_reduce_vector_plan(
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

pub(super) fn apply_linear_vector_plan(
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

pub(super) fn apply_expr_vector_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: VectorPlan,
) -> bool {
    match plan {
        VectorPlan::CubeSliceExprMap {
            dest,
            expr,
            iv_phi,
            face,
            row,
            size,
            ctx,
            start,
            end,
            shadow_vars,
        } => apply_cube_slice_expr_map_plan(
            fn_ir,
            lp,
            site,
            dest,
            expr,
            iv_phi,
            face,
            row,
            size,
            ctx,
            start,
            end,
            shadow_vars,
        ),
        VectorPlan::ExprMap {
            dest,
            expr,
            iv_phi,
            start,
            end,
            whole_dest,
            shadow_vars,
        } => apply_expr_map_plan(
            fn_ir,
            lp,
            site,
            dest,
            expr,
            iv_phi,
            start,
            end,
            whole_dest,
            shadow_vars,
        ),
        VectorPlan::MultiExprMap {
            entries,
            iv_phi,
            start,
            end,
        } => apply_multi_expr_map_plan(fn_ir, lp, site, entries, iv_phi, start, end),
        VectorPlan::MultiExprMap3D {
            entries,
            iv_phi,
            start,
            end,
        } => apply_multi_expr_map_3d_plan(fn_ir, lp, site, entries, iv_phi, start, end),
        VectorPlan::ScatterExprMap {
            dest,
            idx,
            expr,
            iv_phi,
        } => apply_scatter_expr_map_plan(fn_ir, lp, site, dest, idx, expr, iv_phi),
        VectorPlan::ScatterExprMap3D {
            dest,
            axis,
            fixed_a,
            fixed_b,
            idx,
            expr,
            iv_phi,
        } => apply_scatter_expr_map_3d_plan(
            fn_ir,
            lp,
            site,
            ScatterExprMap3DApplyPlan {
                dest,
                axis,
                fixed_a,
                fixed_b,
                idx,
                expr,
                iv_phi,
            },
        ),
        VectorPlan::ScatterExprMap3DGeneral {
            dest,
            i,
            j,
            k,
            expr,
            iv_phi,
        } => apply_scatter_expr_map_3d_general_plan(
            fn_ir,
            lp,
            site,
            ScatterExprMap3DGeneralApplyPlan {
                dest,
                i,
                j,
                k,
                expr,
                iv_phi,
            },
        ),
        _ => false,
    }
}

pub(super) fn apply_structured_vector_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: VectorPlan,
) -> bool {
    match plan {
        VectorPlan::Map2DRow {
            dest,
            row,
            start,
            end,
            lhs_src,
            rhs_src,
            op,
        } => apply_map_2d_row_plan(
            fn_ir,
            lp,
            site,
            Map2DApplyPlan {
                dest,
                axis: row,
                range: VectorLoopRange { start, end },
                lhs_src,
                rhs_src,
                op,
            },
        ),
        VectorPlan::Map2DCol {
            dest,
            col,
            start,
            end,
            lhs_src,
            rhs_src,
            op,
        } => apply_map_2d_col_plan(
            fn_ir,
            lp,
            site,
            Map2DApplyPlan {
                dest,
                axis: col,
                range: VectorLoopRange { start, end },
                lhs_src,
                rhs_src,
                op,
            },
        ),
        VectorPlan::Map3D {
            dest,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
            lhs_src,
            rhs_src,
            op,
        } => apply_map_3d_plan(
            fn_ir,
            lp,
            site,
            Map3DApplyPlan {
                dest,
                axis,
                fixed_a,
                fixed_b,
                range: VectorLoopRange { start, end },
                lhs_src,
                rhs_src,
                op,
            },
        ),
        _ => false,
    }
}

pub(super) fn apply_vectorization(fn_ir: &mut FnIR, lp: &LoopInfo, plan: VectorPlan) -> bool {
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return false;
    };

    match plan {
        plan @ VectorPlan::Reduce { .. }
        | plan @ VectorPlan::Reduce2DRowSum { .. }
        | plan @ VectorPlan::Reduce2DColSum { .. }
        | plan @ VectorPlan::Reduce3D { .. } => apply_reduce_vector_plan(fn_ir, lp, site, plan),
        plan @ VectorPlan::Map { .. }
        | plan @ VectorPlan::CondMap { .. }
        | plan @ VectorPlan::CondMap3D { .. }
        | plan @ VectorPlan::CondMap3DGeneral { .. }
        | plan @ VectorPlan::RecurrenceAddConst { .. }
        | plan @ VectorPlan::RecurrenceAddConst3D { .. }
        | plan @ VectorPlan::ShiftedMap { .. }
        | plan @ VectorPlan::ShiftedMap3D { .. }
        | plan @ VectorPlan::CallMap { .. }
        | plan @ VectorPlan::CallMap3D { .. }
        | plan @ VectorPlan::CallMap3DGeneral { .. }
        | plan @ VectorPlan::ExprMap3D { .. } => apply_linear_vector_plan(fn_ir, lp, site, plan),
        plan @ VectorPlan::CubeSliceExprMap { .. }
        | plan @ VectorPlan::ExprMap { .. }
        | plan @ VectorPlan::MultiExprMap3D { .. }
        | plan @ VectorPlan::MultiExprMap { .. }
        | plan @ VectorPlan::ScatterExprMap { .. }
        | plan @ VectorPlan::ScatterExprMap3D { .. }
        | plan @ VectorPlan::ScatterExprMap3DGeneral { .. } => {
            apply_expr_vector_plan(fn_ir, lp, site, plan)
        }
        plan @ VectorPlan::Map2DRow { .. }
        | plan @ VectorPlan::Map2DCol { .. }
        | plan @ VectorPlan::Map3D { .. } => apply_structured_vector_plan(fn_ir, lp, site, plan),
    }
}

pub(super) fn rewrite_sum_add_const(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    vec_expr: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    let root = canonical_value(fn_ir, vec_expr);
    let ValueKind::Binary {
        op: BinOp::Add,
        lhs,
        rhs,
    } = fn_ir.values[root].kind
    else {
        return None;
    };

    let (base, cst) = if let Some(base) = as_safe_loop_index(fn_ir, lhs, iv_phi) {
        if is_invariant_reduce_scalar(fn_ir, rhs, iv_phi, base) {
            (base, rhs)
        } else {
            return None;
        }
    } else if let Some(base) = as_safe_loop_index(fn_ir, rhs, iv_phi) {
        if is_invariant_reduce_scalar(fn_ir, lhs, iv_phi, base) {
            (base, lhs)
        } else {
            return None;
        }
    } else {
        return None;
    };

    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) != Some(canonical_value(fn_ir, base)) {
        return None;
    }

    let sum_base = fn_ir.add_value(
        ValueKind::Call {
            callee: "sum".to_string(),
            args: vec![base],
            names: vec![None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let len_base = fn_ir.add_value(
        ValueKind::Len { base },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let cst = resolve_materialized_value(fn_ir, cst);
    let c_times_n = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Mul,
            lhs: cst,
            rhs: len_base,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some(fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: sum_base,
            rhs: c_times_n,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    ))
}

pub(super) fn affine_iv_offset(fn_ir: &FnIR, idx: ValueId, iv_phi: ValueId) -> Option<i64> {
    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        return Some(0);
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(k);
            }
            if is_iv_equivalent(fn_ir, *rhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*lhs].kind
            {
                return Some(k);
            }
            None
        }
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(-k);
            }
            None
        }
        _ => None,
    }
}

pub(super) fn add_int_offset(fn_ir: &mut FnIR, base: ValueId, offset: i64) -> ValueId {
    if offset == 0 {
        return base;
    }
    if let ValueKind::Const(Lit::Int(n)) = fn_ir.values[base].kind {
        return fn_ir.add_value(
            ValueKind::Const(Lit::Int(n + offset)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
    }
    let k = fn_ir.add_value(
        ValueKind::Const(Lit::Int(offset)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: base,
            rhs: k,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

pub(super) fn adjusted_loop_limit(fn_ir: &mut FnIR, limit: ValueId, adjust: i64) -> ValueId {
    if adjust == 0 {
        limit
    } else {
        add_int_offset(fn_ir, limit, adjust)
    }
}

pub(super) fn loop_matches_full_base(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    if lp.limit_adjust != 0 {
        return false;
    }

    let base = canonical_value(fn_ir, base);
    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) == Some(base) {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        )
        && a == b
    {
        return true;
    }

    if let Some(limit) = lp
        .is_seq_len
        .map(|limit| resolve_load_alias_value(fn_ir, limit))
    {
        if let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind {
            if canonical_value(fn_ir, len_base) == base {
                return true;
            }
            if let (Some(a), Some(b)) = (
                resolve_base_var(fn_ir, base),
                resolve_base_var(fn_ir, len_base),
            ) && a == b
            {
                return true;
            }
        }

        if base_length_key(fn_ir, base).is_some_and(|base_key| {
            canonical_value(fn_ir, base_key) == canonical_value(fn_ir, limit)
        }) {
            return true;
        }
    }

    if let (Some(base_key), Some(loop_key)) =
        (base_length_key(fn_ir, base), loop_length_key(lp, fn_ir))
    {
        return canonical_value(fn_ir, base_key) == canonical_value(fn_ir, loop_key);
    }

    false
}

pub(super) fn loop_matches_vec(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    let base = canonical_value(fn_ir, base);
    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) == Some(base) {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        )
        && a == b
    {
        return true;
    }
    if let Some(limit) = lp.is_seq_len
        && let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind
    {
        if canonical_value(fn_ir, len_base) == base {
            return true;
        }
        if let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, len_base),
        ) && a == b
        {
            return true;
        }
    }
    false
}
