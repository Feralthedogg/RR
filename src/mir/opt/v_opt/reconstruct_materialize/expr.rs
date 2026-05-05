use super::*;

#[path = "invocation.rs"]
pub(crate) mod invocation;
#[path = "vector_access.rs"]
pub(crate) mod vector_access;

pub(crate) use self::invocation::{
    VectorBinaryNode, VectorCallNode, materialize_vector_binary, materialize_vector_call,
    materialize_vector_intrinsic, materialize_vector_unary,
};
pub(crate) use self::vector_access::{
    IndexAccessSafety, VectorIndex1DNode, VectorIndex3DNode, materialize_vector_index1d,
    materialize_vector_index3d,
};

#[derive(Clone, Copy)]
pub(crate) struct VectorNodeMeta {
    pub(crate) root: ValueId,
    pub(crate) span: crate::utils::Span,
    pub(crate) facts: crate::mir::flow::Facts,
}

#[derive(Clone, Copy)]
pub(crate) struct VectorMaterializeRequest<'a> {
    pub(crate) root: ValueId,
    pub(crate) iv_phi: ValueId,
    pub(crate) idx_vec: ValueId,
    pub(crate) lp: &'a LoopInfo,
    pub(crate) policy: VectorMaterializePolicy,
}

pub(crate) fn materialize_vector_len(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let next_base = ctx.recurse(fn_ir, base)?;
    if next_base == base {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Len { base: next_base },
        span,
        facts,
    ))
}

pub(crate) fn materialize_vector_range(
    fn_ir: &mut FnIR,
    root: ValueId,
    start: ValueId,
    end: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let next_start = ctx.recurse(fn_ir, start)?;
    let next_end = ctx.recurse(fn_ir, end)?;
    if next_start == start && next_end == end {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Range {
            start: next_start,
            end: next_end,
        },
        span,
        facts,
    ))
}

pub(crate) fn materialize_vector_indices(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let next_base = ctx.recurse(fn_ir, base)?;
    if next_base == base {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Indices { base: next_base },
        span,
        facts,
    ))
}

pub(crate) fn materialize_vector_leaf_or_access_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => Some(root),
        ValueKind::Load { var } => materialize_vector_load(fn_ir, root, &var, ctx),
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => materialize_vector_index1d(
            fn_ir,
            VectorIndex1DNode {
                meta: VectorNodeMeta { root, span, facts },
                base,
                idx,
                safety: IndexAccessSafety {
                    bounds_safe: is_safe,
                    na_safe: is_na_safe,
                },
            },
            ctx,
        ),
        ValueKind::Phi { args } => materialize_vector_phi(fn_ir, root, args, span, facts, ctx),
        ValueKind::Index2D { .. } => {
            trace_materialize_reject(fn_ir, root, "Index2D is not vector-materializable");
            None
        }
        ValueKind::Index3D { base, i, j, k } => materialize_vector_index3d(
            fn_ir,
            VectorIndex3DNode {
                meta: VectorNodeMeta { root, span, facts },
                base,
                i,
                j,
                k,
            },
            ctx,
        ),
        _ => None,
    }
}

pub(crate) fn materialize_vector_invocation_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Call {
            callee,
            args,
            names,
        } => materialize_vector_call(
            fn_ir,
            VectorCallNode {
                meta: VectorNodeMeta { root, span, facts },
                callee,
                args,
                names,
            },
            ctx,
        ),
        ValueKind::Intrinsic { op, args } => {
            materialize_vector_intrinsic(fn_ir, root, op, args, span, facts, ctx)
        }
        _ => None,
    }
}

pub(crate) fn materialize_vector_arithmetic_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Binary { op, lhs, rhs } => materialize_vector_binary(
            fn_ir,
            VectorBinaryNode {
                meta: VectorNodeMeta { root, span, facts },
                op,
                lhs,
                rhs,
            },
            ctx,
        ),
        ValueKind::Unary { op, rhs } => {
            materialize_vector_unary(fn_ir, root, op, rhs, span, facts, ctx)
        }
        _ => None,
    }
}

pub(crate) fn materialize_vector_shape_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Len { base } => materialize_vector_len(fn_ir, root, base, span, facts, ctx),
        ValueKind::Range { start, end } => {
            materialize_vector_range(fn_ir, root, start, end, span, facts, ctx)
        }
        ValueKind::Indices { base } => {
            materialize_vector_indices(fn_ir, root, base, span, facts, ctx)
        }
        _ => None,
    }
}

pub(crate) fn materialize_vector_structural_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    materialize_vector_arithmetic_node(fn_ir, root, ctx)
        .or_else(|| materialize_vector_shape_node(fn_ir, root, ctx))
}

pub(crate) fn materialize_vector_expr_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    materialize_vector_leaf_or_access_node(fn_ir, root, ctx)
        .or_else(|| materialize_vector_invocation_node(fn_ir, root, ctx))
        .or_else(|| materialize_vector_structural_node(fn_ir, root, ctx))
}

pub(crate) fn materialize_vector_expr(
    fn_ir: &mut FnIR,
    request: VectorMaterializeRequest<'_>,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    materialize_vector_expr_impl(fn_ir, request, memo, interner, &mut FxHashSet::default())
}

pub(crate) fn materialize_vector_expr_impl(
    fn_ir: &mut FnIR,
    request: VectorMaterializeRequest<'_>,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
) -> Option<ValueId> {
    fn rec(fn_ir: &mut FnIR, root: ValueId, ctx: &mut VectorMaterializeCtx<'_>) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        if let Some(v) = ctx.memo.get(&root) {
            return Some(*v);
        }
        // Guard against pathological phi/load cycles that stay syntactically
        // productive enough to evade the simple `visiting` back-edge check.
        // In those cases we want a clean vectorization reject, not a stack overflow.
        if ctx.visiting.len() > 256 {
            trace_materialize_reject(fn_ir, root, "materialize_vector_expr recursion depth limit");
            return None;
        }
        if !ctx.visiting.insert(root) {
            trace_materialize_reject(fn_ir, root, "cycle in materialize_vector_expr");
            return None;
        }
        if is_iv_equivalent(fn_ir, root, ctx.iv_phi) {
            ctx.memo.insert(root, ctx.idx_vec);
            ctx.visiting.remove(&root);
            return Some(ctx.idx_vec);
        }
        if fn_ir.values[root]
            .phi_block
            .is_some_and(|phi_bb| !ctx.lp.body.contains(&phi_bb))
            && value_is_definitely_scalar_like(fn_ir, root)
            && let Some(scalar) = materialize_loop_invariant_scalar_expr(
                fn_ir,
                root,
                ctx.iv_phi,
                ctx.lp,
                ctx.memo,
                ctx.interner,
            )
        {
            ctx.memo.insert(root, scalar);
            ctx.visiting.remove(&root);
            return Some(scalar);
        }
        if is_scalar_broadcast_value(fn_ir, root)
            && !expr_has_iv_dependency(fn_ir, root, ctx.iv_phi)
        {
            ctx.memo.insert(root, root);
            ctx.visiting.remove(&root);
            return Some(root);
        }

        let out = materialize_vector_expr_node(fn_ir, root, ctx)?;

        ctx.memo.insert(root, out);
        ctx.visiting.remove(&root);
        Some(out)
    }

    let mut ctx = VectorMaterializeCtx {
        iv_phi: request.iv_phi,
        idx_vec: request.idx_vec,
        lp: request.lp,
        memo,
        interner,
        visiting,
        policy: request.policy,
        recurse: rec,
    };
    ctx.recurse(fn_ir, request.root)
}

pub(crate) fn materialize_passthrough_origin_phi_state_scalar(
    fn_ir: &mut FnIR,
    phi: ValueId,
    var: &str,
    iv_phi: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    if var.starts_with(".arg_") && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var) {
        let load = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Load {
                var: var.to_string(),
            },
            fn_ir.values[phi].span,
            fn_ir.values[phi].facts,
        );
        return Some(load);
    }
    let trace_enabled = vectorize_trace_enabled();
    let step = passthrough_origin_phi_step(fn_ir, phi)?;
    let depends_on_phi = |vid: ValueId, fn_ir: &FnIR| {
        value_depends_on(fn_ir, vid, step.phi, &mut FxHashSet::default())
    };
    trace_passthrough_origin_phi_step(fn_ir, "vec-scalar-phi", var, step);

    let Some(arms) = classify_passthrough_origin_phi_arms(fn_ir, lp, step, var) else {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: could not classify pass/update (then_passthrough=false else_passthrough=false then_prior=false else_prior=false)",
                fn_ir.name, step.phi, var
            );
        }
        return None;
    };
    if trace_enabled {
        eprintln!(
            "      classified pass_then={} prev_raw={:?} update={:?}",
            arms.pass_then,
            arms.prev_state_raw,
            fn_ir.values[canonical_value(fn_ir, arms.update_val)].kind
        );
    }

    let prev_source =
        passthrough_origin_phi_prev_source(fn_ir, lp, var, step, arms.prev_state_raw)?;

    let prev_state = {
        let Some(prev_raw) = resolve_non_phi_prev_source_in_loop(fn_ir, lp, var, step, prev_source)
        else {
            if trace_enabled {
                eprintln!(
                    "   [vec-scalar-phi] {} phi={} var={} reject: prev_raw still depends on phi ({:?})",
                    fn_ir.name,
                    step.phi,
                    var,
                    fn_ir.values[canonical_value(fn_ir, prev_source)].kind
                );
            }
            return None;
        };
        materialize_loop_invariant_scalar_expr(fn_ir, prev_raw, iv_phi, lp, memo, interner)?
    };

    let Some((cond_root, op, lhs, rhs)) = passthrough_origin_phi_condition_parts(fn_ir, step)
    else {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: condition not binary ({:?})",
                fn_ir.name,
                step.phi,
                var,
                fn_ir.values[unwrap_vector_condition_value(fn_ir, step.cond)].kind
            );
        }
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
                if depends_on_phi(operand, fn_ir) {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-scalar-phi] {} phi={} var={} reject: cmp operand depends on phi ({:?})",
                            fn_ir.name, step.phi, var, fn_ir.values[operand].kind
                        );
                    }
                    return None;
                }
                materialize_loop_invariant_scalar_expr(fn_ir, operand, iv_phi, lp, memo, interner)
            }
        };
    let cmp_lhs = materialize_cmp_side(lhs, fn_ir, memo, interner)?;
    let cmp_rhs = materialize_cmp_side(rhs, fn_ir, memo, interner)?;
    if cmp_lhs == prev_state && cmp_rhs == prev_state {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: comparison collapsed to prev_state on both sides",
                fn_ir.name, step.phi, var
            );
        }
        return None;
    }
    let cond_scalar = intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Binary {
            op,
            lhs: cmp_lhs,
            rhs: cmp_rhs,
        },
        fn_ir.values[cond_root].span,
        fn_ir.values[cond_root].facts,
    );
    if depends_on_phi(arms.update_val, fn_ir) {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: update depends on phi ({:?})",
                fn_ir.name,
                step.phi,
                var,
                fn_ir.values[canonical_value(fn_ir, arms.update_val)].kind
            );
        }
        return None;
    }
    let update_scalar =
        materialize_loop_invariant_scalar_expr(fn_ir, arms.update_val, iv_phi, lp, memo, interner)?;
    let then_scalar = if arms.pass_then {
        prev_state
    } else {
        update_scalar
    };
    let else_scalar = if arms.pass_then {
        update_scalar
    } else {
        prev_state
    };
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_scalar, then_scalar, else_scalar],
            names: vec![None, None, None],
        },
        fn_ir.values[step.phi].span,
        fn_ir.values[step.phi].facts,
    ))
}
