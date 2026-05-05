use super::*;

#[derive(Clone, Copy)]
pub(in crate::mir::opt::v_opt) struct PassthroughOriginPhiStep {
    pub(in crate::mir::opt::v_opt) phi: ValueId,
    pub(in crate::mir::opt::v_opt) phi_bb: BlockId,
    pub(in crate::mir::opt::v_opt) cond: ValueId,
    pub(in crate::mir::opt::v_opt) then_val: ValueId,
    pub(in crate::mir::opt::v_opt) then_bb: BlockId,
    pub(in crate::mir::opt::v_opt) else_val: ValueId,
    pub(in crate::mir::opt::v_opt) else_bb: BlockId,
}

#[derive(Clone, Copy)]
pub(in crate::mir::opt::v_opt) struct PassthroughOriginPhiArms {
    pub(in crate::mir::opt::v_opt) pass_then: bool,
    pub(in crate::mir::opt::v_opt) prev_state_raw: Option<ValueId>,
    pub(in crate::mir::opt::v_opt) update_val: ValueId,
}

#[derive(Clone, Copy)]
pub(in crate::mir::opt::v_opt) struct SequentialStateStep {
    pub(in crate::mir::opt::v_opt) phi: ValueId,
    pub(in crate::mir::opt::v_opt) cond_root: ValueId,
    pub(in crate::mir::opt::v_opt) update_val: ValueId,
    pub(in crate::mir::opt::v_opt) pass_then: bool,
}

pub(in crate::mir::opt::v_opt) fn passthrough_origin_phi_step(
    fn_ir: &FnIR,
    phi: ValueId,
) -> Option<PassthroughOriginPhiStep> {
    passthrough_origin_phi_step_uncanonicalized(fn_ir, canonical_value(fn_ir, phi))
}

pub(in crate::mir::opt::v_opt) fn passthrough_origin_phi_step_uncanonicalized(
    fn_ir: &FnIR,
    phi: ValueId,
) -> Option<PassthroughOriginPhiStep> {
    let ValueKind::Phi { args } = fn_ir.values[phi].kind.clone() else {
        return None;
    };
    let phi_bb = fn_ir.values[phi].phi_block?;
    let (_, cond, then_val, then_bb, else_val, else_bb) =
        find_conditional_phi_shape_with_blocks(fn_ir, phi, &args)?;
    Some(PassthroughOriginPhiStep {
        phi,
        phi_bb,
        cond,
        then_val,
        then_bb,
        else_val,
        else_bb,
    })
}

pub(in crate::mir::opt::v_opt) fn trace_passthrough_origin_phi_step(
    fn_ir: &FnIR,
    label: &str,
    var: &str,
    step: PassthroughOriginPhiStep,
) {
    if !vectorize_trace_enabled() {
        return;
    }
    eprintln!(
        "   [{}] {} phi={} var={} bb={} cond={:?} then={:?}@{} else={:?}@{}",
        label,
        fn_ir.name,
        step.phi,
        var,
        step.phi_bb,
        fn_ir.values[canonical_value(fn_ir, step.cond)].kind,
        fn_ir.values[canonical_value(fn_ir, step.then_val)].kind,
        step.then_bb,
        fn_ir.values[canonical_value(fn_ir, step.else_val)].kind,
        step.else_bb
    );
    let mut seen = FxHashSet::default();
    trace_value_tree(fn_ir, step.cond, 6, &mut seen);
    let mut seen = FxHashSet::default();
    trace_value_tree(fn_ir, step.then_val, 6, &mut seen);
    let mut seen = FxHashSet::default();
    trace_value_tree(fn_ir, step.else_val, 6, &mut seen);
    trace_block_instrs(fn_ir, step.then_bb, 6);
    trace_block_instrs(fn_ir, step.else_bb, 6);
    eprintln!(
        "      block-last-assign then={:?} else={:?}",
        last_assign_to_var_in_block(fn_ir, step.then_bb, var),
        last_assign_to_var_in_block(fn_ir, step.else_bb, var)
    );
}

pub(in crate::mir::opt::v_opt) fn classify_passthrough_origin_phi_arms(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    step: PassthroughOriginPhiStep,
    var: &str,
) -> Option<PassthroughOriginPhiArms> {
    let then_load = is_passthrough_load_of_var(fn_ir, step.then_val, var);
    let else_load = is_passthrough_load_of_var(fn_ir, step.else_val, var);
    let then_local_assign = if then_load {
        last_assign_to_var_in_block(fn_ir, step.then_bb, var)
    } else {
        None
    };
    let else_local_assign = if else_load {
        last_assign_to_var_in_block(fn_ir, step.else_bb, var)
    } else {
        None
    };
    let then_reaching_assign = if then_load && then_local_assign.is_none() {
        unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, step.then_bb)
    } else {
        None
    };
    let else_reaching_assign = if else_load && else_local_assign.is_none() {
        unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, step.else_bb)
    } else {
        None
    };
    let then_prior_state = is_prior_origin_phi_state(fn_ir, step.then_val, var, step.phi_bb);
    let else_prior_state = is_prior_origin_phi_state(fn_ir, step.else_val, var, step.phi_bb);
    let then_passthrough = then_prior_state || (then_load && then_local_assign.is_none());
    let else_passthrough = else_prior_state || (else_load && else_local_assign.is_none());

    if then_passthrough && !else_passthrough {
        Some(PassthroughOriginPhiArms {
            pass_then: true,
            prev_state_raw: then_prior_state
                .then_some(canonical_value(fn_ir, step.then_val))
                .or(then_reaching_assign),
            update_val: else_local_assign.unwrap_or_else(|| canonical_value(fn_ir, step.else_val)),
        })
    } else if else_passthrough && !then_passthrough {
        Some(PassthroughOriginPhiArms {
            pass_then: false,
            prev_state_raw: else_prior_state
                .then_some(canonical_value(fn_ir, step.else_val))
                .or(else_reaching_assign),
            update_val: then_local_assign.unwrap_or_else(|| canonical_value(fn_ir, step.then_val)),
        })
    } else {
        None
    }
}

pub(in crate::mir::opt::v_opt) fn passthrough_origin_phi_prev_source(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    step: PassthroughOriginPhiStep,
    prev_state_raw: Option<ValueId>,
) -> Option<ValueId> {
    if let Some(prev_raw) = prev_state_raw {
        return Some(
            collapse_prior_origin_phi_state(
                fn_ir,
                prev_raw,
                var,
                step.phi_bb,
                &mut FxHashSet::default(),
            )
            .unwrap_or(prev_raw),
        );
    }

    if let Some(prev_phi) = nearest_origin_phi_value_in_loop(fn_ir, lp, var, step.phi_bb)
        .filter(|src| canonical_value(fn_ir, *src) != step.phi)
    {
        return Some(
            collapse_prior_origin_phi_state(
                fn_ir,
                prev_phi,
                var,
                step.phi_bb,
                &mut FxHashSet::default(),
            )
            .unwrap_or(prev_phi),
        );
    }

    unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, step.phi_bb)
}

pub(in crate::mir::opt::v_opt) fn resolve_non_phi_prev_source_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    step: PassthroughOriginPhiStep,
    source: ValueId,
) -> Option<ValueId> {
    let source = canonical_value(fn_ir, source);
    if !value_depends_on(fn_ir, source, step.phi, &mut FxHashSet::default()) {
        return Some(source);
    }
    unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, step.phi_bb).and_then(|reaching| {
        let reaching = canonical_value(fn_ir, reaching);
        (!value_depends_on(fn_ir, reaching, step.phi, &mut FxHashSet::default()))
            .then_some(reaching)
    })
}

pub(in crate::mir::opt::v_opt) fn passthrough_origin_phi_condition_parts(
    fn_ir: &FnIR,
    step: PassthroughOriginPhiStep,
) -> Option<(ValueId, BinOp, ValueId, ValueId)> {
    let cond_root = unwrap_vector_condition_value(fn_ir, step.cond);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[cond_root].kind.clone() else {
        return None;
    };
    if !is_comparison_op(op) {
        return None;
    }
    Some((cond_root, op, lhs, rhs))
}

pub(in crate::mir::opt::v_opt) fn materialize_passthrough_origin_phi_state(
    fn_ir: &mut FnIR,
    phi: ValueId,
    var: &str,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    if var.starts_with(".arg_") && !has_non_passthrough_assignment_in_loop(fn_ir, ctx.lp, var) {
        let load = intern_materialized_value(
            fn_ir,
            ctx.interner,
            ValueKind::Load {
                var: var.to_string(),
            },
            fn_ir.values[phi].span,
            fn_ir.values[phi].facts,
        );
        return Some(load);
    }
    let Some(step) = passthrough_origin_phi_step(fn_ir, phi) else {
        trace_materialize_reject(fn_ir, phi, "passthrough-origin-phi: root is not phi");
        return None;
    };
    trace_passthrough_origin_phi_step(fn_ir, "vec-materialize", var, step);

    let Some(arms) = classify_passthrough_origin_phi_arms(fn_ir, ctx.lp, step, var) else {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            "passthrough-origin-phi: could not classify pass/update arms",
        );
        return None;
    };

    let Some(prev_source) =
        passthrough_origin_phi_prev_source(fn_ir, ctx.lp, var, step, arms.prev_state_raw)
    else {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            "passthrough-origin-phi: no reaching seed assign",
        );
        return None;
    };

    let Some(prev_source) =
        resolve_non_phi_prev_source_in_loop(fn_ir, ctx.lp, var, step, prev_source)
    else {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            &format!(
                "passthrough-origin-phi: prev_source still depends on phi ({:?})",
                fn_ir.values[canonical_value(fn_ir, prev_source)].kind
            ),
        );
        return None;
    };

    let prev_state = materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: prev_source,
            iv_phi: ctx.iv_phi,
            idx_vec: ctx.idx_vec,
            lp: ctx.lp,
            policy: ctx.policy,
        },
        ctx.memo,
        ctx.interner,
    )?;

    let Some((cond_root, op, lhs, rhs)) = passthrough_origin_phi_condition_parts(fn_ir, step)
    else {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            "passthrough-origin-phi: condition is not a binary compare",
        );
        return None;
    };
    let prev_cmp_raw = arms.prev_state_raw.map(|src| canonical_value(fn_ir, src));
    let materialize_cmp_side =
        |operand: ValueId,
         fn_ir: &mut FnIR,
         memo: &mut FxHashMap<ValueId, ValueId>,
         interner: &mut FxHashMap<MaterializedExprKey, ValueId>| {
            let operand = canonical_value(fn_ir, operand);
            if is_passthrough_load_of_var(fn_ir, operand, var)
                || prev_cmp_raw.is_some_and(|raw| raw == operand)
            {
                Some(prev_state)
            } else {
                materialize_vector_expr(
                    fn_ir,
                    VectorMaterializeRequest {
                        root: operand,
                        iv_phi: ctx.iv_phi,
                        idx_vec: ctx.idx_vec,
                        lp: ctx.lp,
                        policy: ctx.policy,
                    },
                    memo,
                    interner,
                )
            }
        };
    let cmp_lhs = materialize_cmp_side(lhs, fn_ir, ctx.memo, ctx.interner)?;
    let cmp_rhs = materialize_cmp_side(rhs, fn_ir, ctx.memo, ctx.interner)?;
    if cmp_lhs == prev_state && cmp_rhs == prev_state {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            "passthrough-origin-phi: comparison collapsed to same prev state on both sides",
        );
        return None;
    }
    let cond_vec = intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Binary {
            op,
            lhs: cmp_lhs,
            rhs: cmp_rhs,
        },
        fn_ir.values[cond_root].span,
        fn_ir.values[cond_root].facts,
    );
    let update_vec = materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: arms.update_val,
            iv_phi: ctx.iv_phi,
            idx_vec: ctx.idx_vec,
            lp: ctx.lp,
            policy: ctx.policy,
        },
        ctx.memo,
        ctx.interner,
    )?;
    let then_vec = if arms.pass_then {
        prev_state
    } else {
        update_vec
    };
    let else_vec = if arms.pass_then {
        update_vec
    } else {
        prev_state
    };
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_vec, then_vec, else_vec],
            names: vec![None, None, None],
        },
        fn_ir.values[step.phi].span,
        fn_ir.values[step.phi].facts,
    ))
}
