use super::*;
pub(crate) fn apply_scatter_expr_map_plan(
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
        VectorMaterializeRequest {
            root: idx,
            iv_phi,
            idx_vec: idx_seed,
            lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter_idx"),
        None => return false,
    };
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: expr,
            iv_phi,
            idx_vec: idx_seed,
            lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
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

pub(crate) fn apply_scatter_expr_map_3d_plan(
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
        VectorMaterializeRequest {
            root: plan.idx,
            iv_phi: plan.iv_phi,
            idx_vec: idx_seed,
            lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter3d_idx"),
        None => return false,
    };
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: plan.expr,
            iv_phi: plan.iv_phi,
            idx_vec: idx_seed,
            lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
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

pub(crate) struct ScatterIndexMaterialize<'a> {
    pub(crate) lp: &'a LoopInfo,
    pub(crate) site: VectorApplySite,
    pub(crate) iv_phi: ValueId,
    pub(crate) idx_seed: ValueId,
    pub(crate) memo: &'a mut FxHashMap<ValueId, ValueId>,
    pub(crate) interner: &'a mut FxHashMap<MaterializedExprKey, ValueId>,
}

pub(crate) fn materialize_3d_index_operand_for_scatter(
    fn_ir: &mut FnIR,
    operand: VectorAccessOperand3D,
    request: &mut ScatterIndexMaterialize<'_>,
) -> Option<ValueId> {
    match operand {
        VectorAccessOperand3D::Scalar(value) => Some(
            materialize_loop_invariant_scalar_expr(
                fn_ir,
                value,
                request.iv_phi,
                request.lp,
                request.memo,
                request.interner,
            )
            .unwrap_or_else(|| resolve_materialized_value(fn_ir, value)),
        ),
        VectorAccessOperand3D::Vector(value) => {
            let mut materialized = if is_iv_equivalent(fn_ir, value, request.iv_phi) {
                request.idx_seed
            } else {
                materialize_vector_expr(
                    fn_ir,
                    VectorMaterializeRequest {
                        root: value,
                        iv_phi: request.iv_phi,
                        idx_vec: request.idx_seed,
                        lp: request.lp,
                        policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
                    },
                    request.memo,
                    request.interner,
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
                request.site.preheader,
                materialized,
                "scatter3d_axis",
            ))
        }
    }
}

pub(crate) fn apply_scatter_expr_map_3d_general_plan(
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
    let (i_vec, j_vec, k_vec) = {
        let mut index_materialize = ScatterIndexMaterialize {
            lp,
            site,
            iv_phi: plan.iv_phi,
            idx_seed,
            memo: &mut memo,
            interner: &mut interner,
        };
        let i_vec =
            match materialize_3d_index_operand_for_scatter(fn_ir, plan.i, &mut index_materialize) {
                Some(v) => v,
                None => return false,
            };
        let j_vec =
            match materialize_3d_index_operand_for_scatter(fn_ir, plan.j, &mut index_materialize) {
                Some(v) => v,
                None => return false,
            };
        let k_vec =
            match materialize_3d_index_operand_for_scatter(fn_ir, plan.k, &mut index_materialize) {
                Some(v) => v,
                None => return false,
            };
        (i_vec, j_vec, k_vec)
    };
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: plan.expr,
            iv_phi: plan.iv_phi,
            idx_vec: idx_seed,
            lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
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

pub(crate) fn broadcast_scalar_expr_to_slice_len(
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

pub(crate) fn narrow_vector_expr_to_slice_range(
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

pub(crate) fn prepare_partial_slice_value(
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
