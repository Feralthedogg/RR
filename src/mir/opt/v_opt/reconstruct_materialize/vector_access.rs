use super::*;

#[derive(Clone, Copy, Debug)]
pub(crate) struct IndexAccessSafety {
    pub(crate) bounds_safe: bool,
    pub(crate) na_safe: bool,
}

impl IndexAccessSafety {
    pub(crate) fn direct_index_is_safe(self) -> bool {
        self.bounds_safe && self.na_safe
    }
}

pub(crate) struct VectorIndex1DNode {
    pub(crate) meta: VectorNodeMeta,
    pub(crate) base: ValueId,
    pub(crate) idx: ValueId,
    pub(crate) safety: IndexAccessSafety,
}

pub(crate) struct VectorIndex3DNode {
    pub(crate) meta: VectorNodeMeta,
    pub(crate) base: ValueId,
    pub(crate) i: ValueId,
    pub(crate) j: ValueId,
    pub(crate) k: ValueId,
}

pub(crate) fn materialize_vector_index_base(
    fn_ir: &mut FnIR,
    base: ValueId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let base = canonical_value(fn_ir, base);
    if let ValueKind::Phi { args } = fn_ir.values[base].kind.clone()
        && fn_ir.values[base].origin_var.is_none()
    {
        let outside_args: Vec<ValueId> = args
            .iter()
            .filter_map(|(arg, bid)| {
                if ctx.lp.body.contains(bid) {
                    None
                } else {
                    Some(*arg)
                }
            })
            .collect();
        if outside_args.len() == 1 {
            return ctx.recurse(fn_ir, outside_args[0]);
        }
    }
    Some(resolve_materialized_value(fn_ir, base))
}

pub(crate) fn materialize_vector_index1d(
    fn_ir: &mut FnIR,
    node: VectorIndex1DNode,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let root = node.meta.root;
    let base = node.base;
    let idx = node.idx;
    let safety = node.safety;
    if !ctx.policy.allow_any_base && !is_loop_compatible_base(ctx.lp, fn_ir, base) {
        trace_materialize_reject(fn_ir, root, "Index1D base is not loop-compatible");
        return None;
    }

    if is_iv_equivalent(fn_ir, idx, ctx.iv_phi) {
        let base_ref = materialize_vector_index_base(fn_ir, base, ctx)?;
        if safety.direct_index_is_safe()
            && let Some(iv) = ctx.lp.iv.as_ref()
            && loop_covers_whole_destination(ctx.lp, fn_ir, base, iv.init_val)
        {
            return Some(base_ref);
        }

        let mut direct_idx = ctx.idx_vec;
        if !is_int_index_vector_value(fn_ir, direct_idx) {
            direct_idx = intern_materialized_value(
                fn_ir,
                ctx.interner,
                ValueKind::Call {
                    callee: "rr_index_vec_floor".to_string(),
                    args: vec![direct_idx],
                    names: vec![None],
                },
                node.meta.span,
                node.meta.facts,
            );
        }
        return Some(intern_materialized_value(
            fn_ir,
            ctx.interner,
            ValueKind::Call {
                callee: "rr_index1_read_vec".to_string(),
                args: vec![base_ref, direct_idx],
                names: vec![None, None],
            },
            node.meta.span,
            node.meta.facts,
        ));
    }

    if !expr_has_iv_dependency(fn_ir, idx, ctx.iv_phi) {
        trace_materialize_reject(fn_ir, root, "Index1D index is not vectorizable");
        return None;
    }

    let floor_src = if safety.direct_index_is_safe() {
        None
    } else {
        floor_like_index_source(fn_ir, idx)
            .filter(|inner| expr_has_iv_dependency(fn_ir, *inner, ctx.iv_phi))
    };
    let idx_src = floor_src.unwrap_or(idx);
    let mut materialized_idx_vec = ctx.recurse(fn_ir, idx_src)?;
    if floor_src.is_some() && !is_int_index_vector_value(fn_ir, materialized_idx_vec) {
        materialized_idx_vec = intern_materialized_value(
            fn_ir,
            ctx.interner,
            ValueKind::Call {
                callee: "rr_index_vec_floor".to_string(),
                args: vec![materialized_idx_vec],
                names: vec![None],
            },
            node.meta.span,
            node.meta.facts,
        );
    }

    let saved_idx_vec = ctx.idx_vec;
    ctx.idx_vec = materialized_idx_vec;
    let base_ref = materialize_vector_index_base(fn_ir, base, ctx);
    ctx.idx_vec = saved_idx_vec;
    let base_ref = base_ref?;
    if safety.direct_index_is_safe() {
        return Some(intern_materialized_value(
            fn_ir,
            ctx.interner,
            ValueKind::Index1D {
                base: base_ref,
                idx: materialized_idx_vec,
                is_safe: true,
                is_na_safe: true,
            },
            node.meta.span,
            node.meta.facts,
        ));
    }

    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Call {
            callee: "rr_gather".to_string(),
            args: vec![base_ref, materialized_idx_vec],
            names: vec![None, None],
        },
        node.meta.span,
        node.meta.facts,
    ))
}

pub(crate) fn materialize_vector_index3d(
    fn_ir: &mut FnIR,
    node: VectorIndex3DNode,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let root = node.meta.root;
    let base = node.base;
    let i = node.i;
    let j = node.j;
    let k = node.k;
    if !ctx.policy.allow_any_base && !is_loop_compatible_base(ctx.lp, fn_ir, base) {
        trace_materialize_reject(fn_ir, root, "Index3D base is not loop-compatible");
        return None;
    }
    let base_ref = materialize_vector_index_base(fn_ir, base, ctx)?;
    if let Some((axis, dep_idx, fixed_a, fixed_b)) =
        classify_3d_vector_access_axis(fn_ir, base, i, j, k, ctx.iv_phi)
    {
        let fixed_a = materialize_loop_invariant_scalar_expr(
            fn_ir,
            fixed_a,
            ctx.iv_phi,
            ctx.lp,
            ctx.memo,
            ctx.interner,
        )
        .unwrap_or_else(|| resolve_materialized_value(fn_ir, fixed_a));
        let fixed_b = materialize_loop_invariant_scalar_expr(
            fn_ir,
            fixed_b,
            ctx.iv_phi,
            ctx.lp,
            ctx.memo,
            ctx.interner,
        )
        .unwrap_or_else(|| resolve_materialized_value(fn_ir, fixed_b));
        let idx_vec_arg = if is_iv_equivalent(fn_ir, dep_idx, ctx.iv_phi) {
            ctx.idx_vec
        } else {
            ctx.recurse(fn_ir, dep_idx)?
        };
        let callee = match axis {
            Axis3D::Dim1 => "rr_dim1_read_values",
            Axis3D::Dim2 => "rr_dim2_read_values",
            Axis3D::Dim3 => "rr_dim3_read_values",
        };
        return Some(intern_materialized_value(
            fn_ir,
            ctx.interner,
            ValueKind::Call {
                callee: callee.to_string(),
                args: vec![base_ref, fixed_a, fixed_b, idx_vec_arg],
                names: vec![None, None, None, None],
            },
            node.meta.span,
            node.meta.facts,
        ));
    }

    let Some(pattern) = classify_3d_general_vector_access(fn_ir, base, i, j, k, ctx.iv_phi) else {
        trace_materialize_reject(fn_ir, root, "Index3D is not general vectorizable gather");
        return None;
    };
    let i_arg = materialize_access_operand(fn_ir, pattern.i, node.meta, ctx)?;
    let j_arg = materialize_access_operand(fn_ir, pattern.j, node.meta, ctx)?;
    let k_arg = materialize_access_operand(fn_ir, pattern.k, node.meta, ctx)?;
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Call {
            callee: "rr_array3_gather_values".to_string(),
            args: vec![base_ref, i_arg, j_arg, k_arg],
            names: vec![None, None, None, None],
        },
        node.meta.span,
        node.meta.facts,
    ))
}

pub(crate) fn materialize_access_operand(
    fn_ir: &mut FnIR,
    operand: VectorAccessOperand3D,
    meta: VectorNodeMeta,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    match operand {
        VectorAccessOperand3D::Scalar(value) => Some(
            materialize_loop_invariant_scalar_expr(
                fn_ir,
                value,
                ctx.iv_phi,
                ctx.lp,
                ctx.memo,
                ctx.interner,
            )
            .unwrap_or_else(|| resolve_materialized_value(fn_ir, value)),
        ),
        VectorAccessOperand3D::Vector(value) => {
            let mut materialized = if is_iv_equivalent(fn_ir, value, ctx.iv_phi) {
                ctx.idx_vec
            } else {
                ctx.recurse(fn_ir, value)?
            };
            if !is_int_index_vector_value(fn_ir, materialized) {
                materialized = intern_materialized_value(
                    fn_ir,
                    ctx.interner,
                    ValueKind::Call {
                        callee: "rr_index_vec_floor".to_string(),
                        args: vec![materialized],
                        names: vec![None],
                    },
                    meta.span,
                    meta.facts,
                );
            }
            Some(materialized)
        }
    }
}
