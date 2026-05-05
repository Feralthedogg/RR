use super::*;
use crate::mir::opt::v_opt::analysis::is_runtime_vector_read_call;

pub(crate) struct VectorBinaryNode {
    pub(crate) meta: VectorNodeMeta,
    pub(crate) op: BinOp,
    pub(crate) lhs: ValueId,
    pub(crate) rhs: ValueId,
}

pub(crate) struct VectorCallNode {
    pub(crate) meta: VectorNodeMeta,
    pub(crate) callee: String,
    pub(crate) args: Vec<ValueId>,
    pub(crate) names: Vec<Option<String>>,
}

pub(crate) fn materialize_vector_binary(
    fn_ir: &mut FnIR,
    node: VectorBinaryNode,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    fn materialize_operand(
        fn_ir: &mut FnIR,
        operand: ValueId,
        ctx: &mut VectorMaterializeCtx<'_>,
    ) -> Option<ValueId> {
        if !expr_has_iv_dependency(fn_ir, operand, ctx.iv_phi)
            && let Some(scalar) = materialize_loop_invariant_scalar_expr(
                fn_ir,
                operand,
                ctx.iv_phi,
                ctx.lp,
                ctx.memo,
                ctx.interner,
            )
        {
            Some(scalar)
        } else {
            ctx.recurse(fn_ir, operand)
        }
    }

    let lhs_vec = materialize_operand(fn_ir, node.lhs, ctx)?;
    let rhs_vec = materialize_operand(fn_ir, node.rhs, ctx)?;
    if lhs_vec == node.lhs && rhs_vec == node.rhs {
        return Some(node.meta.root);
    }
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Binary {
            op: node.op,
            lhs: lhs_vec,
            rhs: rhs_vec,
        },
        node.meta.span,
        node.meta.facts,
    ))
}

pub(crate) fn materialize_vector_unary(
    fn_ir: &mut FnIR,
    root: ValueId,
    op: UnaryOp,
    rhs: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let rhs_vec = ctx.recurse(fn_ir, rhs)?;
    if rhs_vec == rhs {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Unary { op, rhs: rhs_vec },
        span,
        facts,
    ))
}

pub(crate) fn materialize_vector_call(
    fn_ir: &mut FnIR,
    node: VectorCallNode,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let mut new_args = Vec::with_capacity(node.args.len());
    let mut changed = false;
    for arg in &node.args {
        let next_arg = ctx.recurse(fn_ir, *arg)?;
        changed |= next_arg != *arg;
        new_args.push(next_arg);
    }

    let rewrite_runtime_read = is_runtime_vector_read_call(&node.callee, new_args.len())
        && node
            .args
            .get(1)
            .copied()
            .is_some_and(|idx_arg| expr_has_iv_dependency(fn_ir, idx_arg, ctx.iv_phi));
    if rewrite_runtime_read
        && let Some(raw_idx) = node.args.get(1).copied().and_then(|idx_arg| {
            floor_like_index_source(fn_ir, idx_arg)
                .filter(|inner| expr_has_iv_dependency(fn_ir, *inner, ctx.iv_phi))
        })
    {
        let raw_idx_vec = ctx.recurse(fn_ir, raw_idx)?;
        if raw_idx_vec != new_args[1] {
            new_args[1] = raw_idx_vec;
            changed = true;
        }
        if !is_int_index_vector_value(fn_ir, new_args[1]) {
            let floor_idx_vec = intern_materialized_value(
                fn_ir,
                ctx.interner,
                ValueKind::Call {
                    callee: "rr_index_vec_floor".to_string(),
                    args: vec![new_args[1]],
                    names: vec![None],
                },
                node.meta.span,
                node.meta.facts,
            );
            if floor_idx_vec != new_args[1] {
                new_args[1] = floor_idx_vec;
                changed = true;
            }
        }
    }

    if !changed && !rewrite_runtime_read {
        return Some(node.meta.root);
    }

    let (out_callee, out_names) = if rewrite_runtime_read {
        ("rr_index1_read_vec".to_string(), vec![None; new_args.len()])
    } else {
        (node.callee, node.names)
    };
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Call {
            callee: out_callee,
            args: new_args,
            names: out_names,
        },
        node.meta.span,
        node.meta.facts,
    ))
}

pub(crate) fn materialize_vector_intrinsic(
    fn_ir: &mut FnIR,
    root: ValueId,
    op: IntrinsicOp,
    args: Vec<ValueId>,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let mut new_args = Vec::with_capacity(args.len());
    let mut changed = false;
    for arg in args {
        let next_arg = ctx.recurse(fn_ir, arg)?;
        changed |= next_arg != arg;
        new_args.push(next_arg);
    }
    if !changed {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Intrinsic { op, args: new_args },
        span,
        facts,
    ))
}
