#[allow(clippy::too_many_arguments)]
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
        | plan @ VectorPlan::ReduceCond { .. }
        | plan @ VectorPlan::MultiReduceCond { .. }
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

// Transactional vectorization apply has a reduced proof model in
// `proof/optimizer_correspondence.md` and
// `proof/{lean,coq}/.../VectorizeApplySubset.*`:
// rejected plans must roll back to the scalar original, and certified
// result-preserving plans may commit without changing the scalar result.
pub(crate) fn try_apply_vectorization_transactionally(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    plan: VectorPlan,
) -> bool {
    let mut cloned = fn_ir.clone();
    if !apply_vectorization(&mut cloned, lp, plan) {
        return false;
    }
    if let Err(err) = crate::mir::verify::verify_ir(&cloned) {
        if super::debug::proof_trace_enabled() {
            eprintln!(
                "   [vec-transform] {} reject transactional apply: {}",
                cloned.name, err
            );
        }
        return false;
    }
    *fn_ir = cloned;
    true
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
