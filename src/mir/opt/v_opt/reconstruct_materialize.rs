use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_index_base(
    fn_ir: &mut FnIR,
    base: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    let base = canonical_value(fn_ir, base);
    if let ValueKind::Phi { args } = fn_ir.values[base].kind.clone()
        && fn_ir.values[base].origin_var.is_none()
    {
        let outside_args: Vec<ValueId> = args
            .iter()
            .filter_map(|(arg, bid)| {
                if lp.body.contains(bid) {
                    None
                } else {
                    Some(*arg)
                }
            })
            .collect();
        if outside_args.len() == 1 {
            return materialize_vector_expr_impl(
                fn_ir,
                outside_args[0],
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
            );
        }
    }
    Some(resolve_materialized_value(fn_ir, base))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_binary(
    fn_ir: &mut FnIR,
    root: ValueId,
    op: BinOp,
    lhs: ValueId,
    rhs: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let materialize_operand = |operand: ValueId,
                               fn_ir: &mut FnIR,
                               memo: &mut FxHashMap<ValueId, ValueId>,
                               interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
                               visiting: &mut FxHashSet<ValueId>| {
        if !expr_has_iv_dependency(fn_ir, operand, iv_phi)
            && let Some(scalar) =
                materialize_loop_invariant_scalar_expr(fn_ir, operand, iv_phi, lp, memo, interner)
        {
            Some(scalar)
        } else {
            recurse(
                fn_ir,
                operand,
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
            )
        }
    };
    let lhs_vec = materialize_operand(lhs, fn_ir, memo, interner, visiting)?;
    let rhs_vec = materialize_operand(rhs, fn_ir, memo, interner, visiting)?;
    if lhs_vec == lhs && rhs_vec == rhs {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Binary {
            op,
            lhs: lhs_vec,
            rhs: rhs_vec,
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_unary(
    fn_ir: &mut FnIR,
    root: ValueId,
    op: UnaryOp,
    rhs: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let rhs_vec = recurse(
        fn_ir,
        rhs,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if rhs_vec == rhs {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Unary { op, rhs: rhs_vec },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_call(
    fn_ir: &mut FnIR,
    root: ValueId,
    callee: String,
    args: Vec<ValueId>,
    names: Vec<Option<String>>,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let mut new_args = Vec::with_capacity(args.len());
    let mut changed = false;
    for arg in &args {
        let next_arg = recurse(
            fn_ir,
            *arg,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        changed |= next_arg != *arg;
        new_args.push(next_arg);
    }

    let rewrite_runtime_read = is_runtime_vector_read_call(&callee, new_args.len())
        && args
            .get(1)
            .copied()
            .is_some_and(|idx_arg| expr_has_iv_dependency(fn_ir, idx_arg, iv_phi));
    if rewrite_runtime_read
        && let Some(raw_idx) = args.get(1).copied().and_then(|idx_arg| {
            floor_like_index_source(fn_ir, idx_arg)
                .filter(|inner| expr_has_iv_dependency(fn_ir, *inner, iv_phi))
        })
    {
        let raw_idx_vec = recurse(
            fn_ir,
            raw_idx,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        if raw_idx_vec != new_args[1] {
            new_args[1] = raw_idx_vec;
            changed = true;
        }
        if !is_int_index_vector_value(fn_ir, new_args[1]) {
            let floor_idx_vec = intern_materialized_value(
                fn_ir,
                interner,
                ValueKind::Call {
                    callee: "rr_index_vec_floor".to_string(),
                    args: vec![new_args[1]],
                    names: vec![None],
                },
                span,
                facts,
            );
            if floor_idx_vec != new_args[1] {
                new_args[1] = floor_idx_vec;
                changed = true;
            }
        }
    }

    if !changed && !rewrite_runtime_read {
        return Some(root);
    }

    let (out_callee, out_names) = if rewrite_runtime_read {
        ("rr_index1_read_vec".to_string(), vec![None; new_args.len()])
    } else {
        (callee, names)
    };
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: out_callee,
            args: new_args,
            names: out_names,
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_intrinsic(
    fn_ir: &mut FnIR,
    root: ValueId,
    op: IntrinsicOp,
    args: Vec<ValueId>,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let mut new_args = Vec::with_capacity(args.len());
    let mut changed = false;
    for arg in args {
        let next_arg = recurse(
            fn_ir,
            arg,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        changed |= next_arg != arg;
        new_args.push(next_arg);
    }
    if !changed {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Intrinsic { op, args: new_args },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_index1d(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    idx: ValueId,
    is_safe: bool,
    is_na_safe: bool,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, base) {
        trace_materialize_reject(fn_ir, root, "Index1D base is not loop-compatible");
        return None;
    }

    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        let base_ref = materialize_vector_index_base(
            fn_ir,
            base,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        if is_safe
            && is_na_safe
            && let Some(iv) = lp.iv.as_ref()
            && loop_covers_whole_destination(lp, fn_ir, base, iv.init_val)
        {
            return Some(base_ref);
        }

        let mut direct_idx = idx_vec;
        if !is_int_index_vector_value(fn_ir, direct_idx) {
            direct_idx = intern_materialized_value(
                fn_ir,
                interner,
                ValueKind::Call {
                    callee: "rr_index_vec_floor".to_string(),
                    args: vec![direct_idx],
                    names: vec![None],
                },
                span,
                facts,
            );
        }
        return Some(intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Call {
                callee: "rr_index1_read_vec".to_string(),
                args: vec![base_ref, direct_idx],
                names: vec![None, None],
            },
            span,
            facts,
        ));
    }

    if !expr_has_iv_dependency(fn_ir, idx, iv_phi) {
        trace_materialize_reject(fn_ir, root, "Index1D index is not vectorizable");
        return None;
    }

    let floor_src = if is_safe && is_na_safe {
        None
    } else {
        floor_like_index_source(fn_ir, idx)
            .filter(|inner| expr_has_iv_dependency(fn_ir, *inner, iv_phi))
    };
    let idx_src = floor_src.unwrap_or(idx);
    let mut materialized_idx_vec = recurse(
        fn_ir,
        idx_src,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if floor_src.is_some() && !is_int_index_vector_value(fn_ir, materialized_idx_vec) {
        materialized_idx_vec = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Call {
                callee: "rr_index_vec_floor".to_string(),
                args: vec![materialized_idx_vec],
                names: vec![None],
            },
            span,
            facts,
        );
    }

    let base_ref = materialize_vector_index_base(
        fn_ir,
        base,
        iv_phi,
        materialized_idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if is_safe && is_na_safe {
        return Some(intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Index1D {
                base: base_ref,
                idx: materialized_idx_vec,
                is_safe: true,
                is_na_safe: true,
            },
            span,
            facts,
        ));
    }

    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_gather".to_string(),
            args: vec![base_ref, materialized_idx_vec],
            names: vec![None, None],
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_index3d(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    i: ValueId,
    j: ValueId,
    k: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    fn materialize_access_operand(
        fn_ir: &mut FnIR,
        operand: VectorAccessOperand3D,
        span: crate::utils::Span,
        facts: crate::mir::flow::Facts,
        iv_phi: ValueId,
        idx_vec: ValueId,
        lp: &LoopInfo,
        memo: &mut FxHashMap<ValueId, ValueId>,
        interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
        visiting: &mut FxHashSet<ValueId>,
        allow_any_base: bool,
        require_safe_index: bool,
        recurse: MaterializeRecurseFn,
    ) -> Option<ValueId> {
        match operand {
            VectorAccessOperand3D::Scalar(value) => Some(
                materialize_loop_invariant_scalar_expr(fn_ir, value, iv_phi, lp, memo, interner)
                    .unwrap_or_else(|| resolve_materialized_value(fn_ir, value)),
            ),
            VectorAccessOperand3D::Vector(value) => {
                let mut materialized = if is_iv_equivalent(fn_ir, value, iv_phi) {
                    idx_vec
                } else {
                    recurse(
                        fn_ir,
                        value,
                        iv_phi,
                        idx_vec,
                        lp,
                        memo,
                        interner,
                        visiting,
                        allow_any_base,
                        require_safe_index,
                    )?
                };
                if !is_int_index_vector_value(fn_ir, materialized) {
                    materialized = intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Call {
                            callee: "rr_index_vec_floor".to_string(),
                            args: vec![materialized],
                            names: vec![None],
                        },
                        span,
                        facts,
                    );
                }
                Some(materialized)
            }
        }
    }

    if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, base) {
        trace_materialize_reject(fn_ir, root, "Index3D base is not loop-compatible");
        return None;
    }
    let base_ref = materialize_vector_index_base(
        fn_ir,
        base,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if let Some((axis, dep_idx, fixed_a, fixed_b)) =
        classify_3d_vector_access_axis(fn_ir, base, i, j, k, iv_phi)
    {
        let fixed_a =
            materialize_loop_invariant_scalar_expr(fn_ir, fixed_a, iv_phi, lp, memo, interner)
                .unwrap_or_else(|| resolve_materialized_value(fn_ir, fixed_a));
        let fixed_b =
            materialize_loop_invariant_scalar_expr(fn_ir, fixed_b, iv_phi, lp, memo, interner)
                .unwrap_or_else(|| resolve_materialized_value(fn_ir, fixed_b));
        let idx_vec_arg = if is_iv_equivalent(fn_ir, dep_idx, iv_phi) {
            idx_vec
        } else {
            recurse(
                fn_ir,
                dep_idx,
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
            )?
        };
        let callee = match axis {
            Axis3D::Dim1 => "rr_dim1_read_values",
            Axis3D::Dim2 => "rr_dim2_read_values",
            Axis3D::Dim3 => "rr_dim3_read_values",
        };
        return Some(intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Call {
                callee: callee.to_string(),
                args: vec![base_ref, fixed_a, fixed_b, idx_vec_arg],
                names: vec![None, None, None, None],
            },
            span,
            facts,
        ));
    }

    let Some(pattern) = classify_3d_general_vector_access(fn_ir, base, i, j, k, iv_phi) else {
        trace_materialize_reject(fn_ir, root, "Index3D is not general vectorizable gather");
        return None;
    };
    let i_arg = materialize_access_operand(
        fn_ir,
        pattern.i,
        span,
        facts,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )?;
    let j_arg = materialize_access_operand(
        fn_ir,
        pattern.j,
        span,
        facts,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )?;
    let k_arg = materialize_access_operand(
        fn_ir,
        pattern.k,
        span,
        facts,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )?;
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_array3_gather_values".to_string(),
            args: vec![base_ref, i_arg, j_arg, k_arg],
            names: vec![None, None, None, None],
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_len(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let next_base = recurse(
        fn_ir,
        base,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if next_base == base {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Len { base: next_base },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_range(
    fn_ir: &mut FnIR,
    root: ValueId,
    start: ValueId,
    end: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let next_start = recurse(
        fn_ir,
        start,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    let next_end = recurse(
        fn_ir,
        end,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if next_start == start && next_end == end {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Range {
            start: next_start,
            end: next_end,
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_indices(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let next_base = recurse(
        fn_ir,
        base,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if next_base == base {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Indices { base: next_base },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_leaf_or_access_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => Some(root),
        ValueKind::Load { var } => materialize_vector_load(
            fn_ir,
            root,
            &var,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => materialize_vector_index1d(
            fn_ir,
            root,
            base,
            idx,
            is_safe,
            is_na_safe,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Phi { args } => materialize_vector_phi(
            fn_ir,
            root,
            args,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Index2D { .. } => {
            trace_materialize_reject(fn_ir, root, "Index2D is not vector-materializable");
            None
        }
        ValueKind::Index3D { base, i, j, k } => materialize_vector_index3d(
            fn_ir,
            root,
            base,
            i,
            j,
            k,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_invocation_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
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
            root,
            callee,
            args,
            names,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Intrinsic { op, args } => materialize_vector_intrinsic(
            fn_ir,
            root,
            op,
            args,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_arithmetic_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Binary { op, lhs, rhs } => materialize_vector_binary(
            fn_ir,
            root,
            op,
            lhs,
            rhs,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Unary { op, rhs } => materialize_vector_unary(
            fn_ir,
            root,
            op,
            rhs,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_shape_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Len { base } => materialize_vector_len(
            fn_ir,
            root,
            base,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Range { start, end } => materialize_vector_range(
            fn_ir,
            root,
            start,
            end,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Indices { base } => materialize_vector_indices(
            fn_ir,
            root,
            base,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_structural_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    materialize_vector_arithmetic_node(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )
    .or_else(|| {
        materialize_vector_shape_node(
            fn_ir,
            root,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        )
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_expr_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    materialize_vector_leaf_or_access_node(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )
    .or_else(|| {
        materialize_vector_invocation_node(
            fn_ir,
            root,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        )
    })
    .or_else(|| {
        materialize_vector_structural_node(
            fn_ir,
            root,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        )
    })
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_expr(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    materialize_vector_expr_impl(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        &mut FxHashSet::default(),
        allow_any_base,
        require_safe_index,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn materialize_vector_expr_impl(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    #[allow(clippy::only_used_in_recursion)]
    fn rec(
        fn_ir: &mut FnIR,
        root: ValueId,
        iv_phi: ValueId,
        idx_vec: ValueId,
        lp: &LoopInfo,
        memo: &mut FxHashMap<ValueId, ValueId>,
        interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
        visiting: &mut FxHashSet<ValueId>,
        allow_any_base: bool,
        require_safe_index: bool,
    ) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        let _ = require_safe_index;
        if let Some(v) = memo.get(&root) {
            return Some(*v);
        }
        // Guard against pathological phi/load cycles that stay syntactically
        // productive enough to evade the simple `visiting` back-edge check.
        // In those cases we want a clean vectorization reject, not a stack overflow.
        if visiting.len() > 256 {
            trace_materialize_reject(fn_ir, root, "materialize_vector_expr recursion depth limit");
            return None;
        }
        if !visiting.insert(root) {
            trace_materialize_reject(fn_ir, root, "cycle in materialize_vector_expr");
            return None;
        }
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            memo.insert(root, idx_vec);
            visiting.remove(&root);
            return Some(idx_vec);
        }
        if fn_ir.values[root]
            .phi_block
            .is_some_and(|phi_bb| !lp.body.contains(&phi_bb))
            && value_is_definitely_scalar_like(fn_ir, root)
            && let Some(scalar) =
                materialize_loop_invariant_scalar_expr(fn_ir, root, iv_phi, lp, memo, interner)
        {
            memo.insert(root, scalar);
            visiting.remove(&root);
            return Some(scalar);
        }
        if is_scalar_broadcast_value(fn_ir, root) && !expr_has_iv_dependency(fn_ir, root, iv_phi) {
            memo.insert(root, root);
            visiting.remove(&root);
            return Some(root);
        }

        let out = materialize_vector_expr_node(
            fn_ir,
            root,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            rec,
        )?;

        memo.insert(root, out);
        visiting.remove(&root);
        Some(out)
    }

    rec(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )
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
