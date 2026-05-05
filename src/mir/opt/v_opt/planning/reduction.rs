use super::*;

pub(in crate::mir::opt::v_opt) fn expr_has_non_iv_loop_state_load(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    root: ValueId,
    iv_phi: ValueId,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        lp: &LoopInfo,
        root: ValueId,
        iv_phi: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if is_iv_equivalent(fn_ir, root, iv_phi) || !seen_vals.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Load { var } => {
                if !seen_vars.insert(var.clone()) {
                    return true;
                }
                let out = if let Some(phi) = unique_origin_phi_value_in_loop(fn_ir, lp, var) {
                    !is_iv_equivalent(fn_ir, phi, iv_phi)
                } else if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, var) {
                    rec(fn_ir, lp, src, iv_phi, seen_vals, seen_vars)
                } else if let Some(src) = merged_assign_source_in_loop(fn_ir, lp, var) {
                    rec(fn_ir, lp, src, iv_phi, seen_vals, seen_vars)
                } else {
                    false
                };
                seen_vars.remove(var);
                out
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, lp, *lhs, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *rhs, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, lp, *rhs, iv_phi, seen_vals, seen_vars),
            ValueKind::RecordLit { fields } => fields
                .iter()
                .any(|(_, value)| rec(fn_ir, lp, *value, iv_phi, seen_vals, seen_vars)),
            ValueKind::FieldGet { base, .. } => rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars),
            ValueKind::FieldSet { base, value, .. } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *value, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
                .iter()
                .any(|arg| rec(fn_ir, lp, *arg, iv_phi, seen_vals, seen_vars)),
            ValueKind::Phi { args } => args
                .iter()
                .any(|(arg, _)| rec(fn_ir, lp, *arg, iv_phi, seen_vals, seen_vars)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, lp, *start, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *end, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *idx, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index2D { base, r, c } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *r, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *c, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index3D { base, i, j, k } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *i, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *j, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *k, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
        }
    }

    rec(
        fn_ir,
        lp,
        root,
        iv_phi,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

pub(in crate::mir::opt::v_opt) fn reduction_has_non_acc_loop_state_assignments(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    iv_phi: ValueId,
) -> bool {
    let acc_var =
        phi_state_var(fn_ir, acc_phi).or_else(|| fn_ir.values[acc_phi].origin_var.clone());
    let iv_var = induction_origin_var(fn_ir, iv_phi);
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if acc_var.as_deref() == Some(dst.as_str()) || iv_var.as_deref() == Some(dst.as_str()) {
                continue;
            }
            if dst.starts_with(".arg_") {
                continue;
            }
            if expr_has_non_iv_loop_state_load(fn_ir, lp, *src, iv_phi) {
                return true;
            }
        }
    }
    false
}

pub(in crate::mir::opt::v_opt) fn reduction_has_extra_state_phi(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    iv_phi: ValueId,
) -> bool {
    let acc_phi = canonical_value(fn_ir, acc_phi);
    let mut seen = FxHashSet::default();
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        if vid == acc_phi || is_iv_equivalent(fn_ir, vid, iv_phi) || !seen.insert(vid) {
            continue;
        }
        return true;
    }
    false
}

pub(in crate::mir::opt::v_opt) fn match_reduction(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let reduction_rhs_vectorizable = |root: ValueId| {
        is_vectorizable_expr(
            fn_ir,
            root,
            iv_phi,
            lp,
            crate::mir::opt::v_opt::analysis::RELAXED_VECTOR_EXPR_POLICY,
        )
    };
    if loop_has_store_effect(fn_ir, lp) {
        // Conservative: do not fold reductions if loop writes memory.
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        if is_iv_equivalent(fn_ir, id, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, id, iv_phi)
        {
            continue;
        }
        if let ValueKind::Phi { args } = &val.kind
            && args.len() == 2
            && args.iter().any(|(_, b)| *b == lp.latch)
        {
            let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
                continue;
            };
            let next_v = &fn_ir.values[*next_val];

            match &next_v.kind {
                ValueKind::Binary {
                    op: crate::syntax::ast::BinOp::Add,
                    lhs,
                    rhs,
                } if (*lhs == id || *rhs == id) => {
                    let other = if *lhs == id { *rhs } else { *lhs };
                    if reduction_has_non_acc_loop_state_assignments(fn_ir, lp, id, iv_phi)
                        || reduction_has_extra_state_phi(fn_ir, lp, id, iv_phi)
                    {
                        continue;
                    }
                    let acc_reads_self = phi_state_var(fn_ir, id)
                        .or_else(|| fn_ir.values[id].origin_var.clone())
                        .is_some_and(|acc_var| {
                            expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default())
                        });
                    if expr_has_iv_dependency(fn_ir, other, iv_phi)
                        && !acc_reads_self
                        && !expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
                        && !expr_has_unstable_loop_local_load(fn_ir, lp, other)
                        && !expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
                        && !expr_has_non_vector_safe_call_in_vector_context(
                            fn_ir,
                            other,
                            iv_phi,
                            user_call_whitelist,
                            &mut FxHashSet::default(),
                        )
                        && reduction_rhs_vectorizable(other)
                    {
                        if vectorize_trace_enabled() {
                            let lhs_detail = match &fn_ir.values[other].kind {
                                ValueKind::Binary { lhs, .. } => {
                                    format!("{:?}", fn_ir.values[*lhs].kind)
                                }
                                _ => "-".to_string(),
                            };
                            let rhs_detail = match &fn_ir.values[other].kind {
                                ValueKind::Binary { rhs, .. } => {
                                    format!("{:?}", fn_ir.values[*rhs].kind)
                                }
                                _ => "-".to_string(),
                            };
                            eprintln!(
                                "   [vec-reduce] {} sum acc={:?} other={:?} lhs={} rhs={}",
                                fn_ir.name,
                                fn_ir.values[id].origin_var,
                                fn_ir.values[other].kind,
                                lhs_detail,
                                rhs_detail
                            );
                        }
                        return Some(VectorPlan::Reduce {
                            kind: ReduceKind::Sum,
                            acc_phi: id,
                            vec_expr: other,
                            iv_phi,
                        });
                    }
                }
                ValueKind::Binary {
                    op: crate::syntax::ast::BinOp::Mul,
                    lhs,
                    rhs,
                } if (*lhs == id || *rhs == id) => {
                    let other = if *lhs == id { *rhs } else { *lhs };
                    if reduction_has_non_acc_loop_state_assignments(fn_ir, lp, id, iv_phi)
                        || reduction_has_extra_state_phi(fn_ir, lp, id, iv_phi)
                    {
                        continue;
                    }
                    let acc_reads_self = phi_state_var(fn_ir, id)
                        .or_else(|| fn_ir.values[id].origin_var.clone())
                        .is_some_and(|acc_var| {
                            expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default())
                        });
                    if expr_has_iv_dependency(fn_ir, other, iv_phi)
                        && !acc_reads_self
                        && !expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
                        && !expr_has_unstable_loop_local_load(fn_ir, lp, other)
                        && !expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
                        && !expr_has_non_vector_safe_call_in_vector_context(
                            fn_ir,
                            other,
                            iv_phi,
                            user_call_whitelist,
                            &mut FxHashSet::default(),
                        )
                        && reduction_rhs_vectorizable(other)
                    {
                        if vectorize_trace_enabled() {
                            eprintln!(
                                "   [vec-reduce] {} prod acc={:?} other={:?}",
                                fn_ir.name, fn_ir.values[id].origin_var, fn_ir.values[other].kind
                            );
                        }
                        return Some(VectorPlan::Reduce {
                            kind: ReduceKind::Prod,
                            acc_phi: id,
                            vec_expr: other,
                            iv_phi,
                        });
                    }
                }
                ValueKind::Call { .. } => {
                    let Some(call) = resolve_call_info(fn_ir, *next_val) else {
                        continue;
                    };
                    if !call.args.len().eq(&2)
                        || !call.builtin_kind.is_some_and(BuiltinKind::is_minmax)
                            && !matches!(call.callee.as_str(), "min" | "max")
                    {
                        continue;
                    }
                    let (a, b) = (call.args[0], call.args[1]);
                    let acc_side = if a == id {
                        Some(b)
                    } else if b == id {
                        Some(a)
                    } else {
                        None
                    };
                    if let Some(other) = acc_side
                        && !reduction_has_non_acc_loop_state_assignments(fn_ir, lp, id, iv_phi)
                        && !reduction_has_extra_state_phi(fn_ir, lp, id, iv_phi)
                        && !phi_state_var(fn_ir, id)
                            .or_else(|| fn_ir.values[id].origin_var.clone())
                            .is_some_and(|acc_var| {
                                expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default())
                            })
                        && expr_has_iv_dependency(fn_ir, other, iv_phi)
                        && !expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
                        && !expr_has_unstable_loop_local_load(fn_ir, lp, other)
                        && !expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
                        && !expr_has_non_vector_safe_call_in_vector_context(
                            fn_ir,
                            other,
                            iv_phi,
                            user_call_whitelist,
                            &mut FxHashSet::default(),
                        )
                        && reduction_rhs_vectorizable(other)
                    {
                        if vectorize_trace_enabled() {
                            eprintln!(
                                "   [vec-reduce] {} minmax acc={:?} other={:?} callee={}",
                                fn_ir.name,
                                fn_ir.values[id].origin_var,
                                fn_ir.values[other].kind,
                                call.callee
                            );
                        }
                        let kind = if call.builtin_kind == Some(BuiltinKind::Min)
                            || call.callee == "min"
                        {
                            ReduceKind::Min
                        } else {
                            ReduceKind::Max
                        };
                        return Some(VectorPlan::Reduce {
                            kind,
                            acc_phi: id,
                            vec_expr: other,
                            iv_phi,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

pub(in crate::mir::opt::v_opt) fn match_2d_row_reduction_sum(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    if loop_has_store_effect(fn_ir, lp) {
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, b)| *b == lp.latch) {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
            continue;
        };
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = fn_ir.values[*next_val].kind
        else {
            continue;
        };
        let other = if lhs == id {
            rhs
        } else if rhs == id {
            lhs
        } else {
            continue;
        };
        let ValueKind::Index2D { base, r, c } = fn_ir.values[other].kind else {
            continue;
        };
        if !is_iv_equivalent(fn_ir, c, iv_phi) || expr_has_iv_dependency(fn_ir, r, iv_phi) {
            continue;
        }
        if !is_loop_invariant_axis(fn_ir, r, iv_phi, base) {
            continue;
        }
        if !structured_reduction_stride_allowed(fn_ir, lp, matrix_access_stride(true)) {
            continue;
        }
        return Some(VectorPlan::Reduce2DRowSum {
            acc_phi: id,
            base: canonical_value(fn_ir, base),
            row: r,
            start,
            end,
        });
    }
    None
}

pub(in crate::mir::opt::v_opt) fn match_2d_col_reduction_sum(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    if loop_has_store_effect(fn_ir, lp) {
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, b)| *b == lp.latch) {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
            continue;
        };
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = fn_ir.values[*next_val].kind
        else {
            continue;
        };
        let other = if lhs == id {
            rhs
        } else if rhs == id {
            lhs
        } else {
            continue;
        };
        let ValueKind::Index2D { base, r, c } = fn_ir.values[other].kind else {
            continue;
        };
        if !is_iv_equivalent(fn_ir, r, iv_phi) || expr_has_iv_dependency(fn_ir, c, iv_phi) {
            continue;
        }
        if !is_loop_invariant_axis(fn_ir, c, iv_phi, base) {
            continue;
        }
        if !structured_reduction_stride_allowed(fn_ir, lp, matrix_access_stride(false)) {
            continue;
        }
        return Some(VectorPlan::Reduce2DColSum {
            acc_phi: id,
            base: canonical_value(fn_ir, base),
            col: c,
            start,
            end,
        });
    }
    None
}

pub(in crate::mir::opt::v_opt) fn match_3d_axis_reduction(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    if loop_has_store_effect(fn_ir, lp) {
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, b)| *b == lp.latch) {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
            continue;
        };
        match &fn_ir.values[*next_val].kind {
            ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
                let kind = match op {
                    BinOp::Add => ReduceKind::Sum,
                    BinOp::Mul => ReduceKind::Prod,
                    _ => continue,
                };
                let other = if *lhs == id {
                    *rhs
                } else if *rhs == id {
                    *lhs
                } else {
                    continue;
                };
                let ValueKind::Index3D { base, i, j, k } = fn_ir.values[other].kind else {
                    continue;
                };
                let Some((axis, fixed_a, fixed_b)) =
                    classify_3d_map_axis(fn_ir, base, i, j, k, iv_phi)
                else {
                    continue;
                };
                if !structured_reduction_stride_allowed(fn_ir, lp, array3_access_stride(axis)) {
                    continue;
                }
                return Some(VectorPlan::Reduce3D {
                    kind,
                    acc_phi: id,
                    base: canonical_value(fn_ir, base),
                    axis,
                    fixed_a,
                    fixed_b,
                    start,
                    end,
                });
            }
            ValueKind::Call { .. } => {
                let Some(call) = resolve_call_info(fn_ir, *next_val) else {
                    continue;
                };
                if !call.args.len().eq(&2)
                    || !call.builtin_kind.is_some_and(BuiltinKind::is_minmax)
                        && !matches!(call.callee.as_str(), "min" | "max")
                {
                    continue;
                }
                let (a, b) = (call.args[0], call.args[1]);
                let other = if a == id {
                    b
                } else if b == id {
                    a
                } else {
                    continue;
                };
                let ValueKind::Index3D { base, i, j, k } = fn_ir.values[other].kind else {
                    continue;
                };
                let Some((axis, fixed_a, fixed_b)) =
                    classify_3d_map_axis(fn_ir, base, i, j, k, iv_phi)
                else {
                    continue;
                };
                if !structured_reduction_stride_allowed(fn_ir, lp, array3_access_stride(axis)) {
                    continue;
                }
                let kind = if call.builtin_kind == Some(BuiltinKind::Min) || call.callee == "min" {
                    ReduceKind::Min
                } else {
                    ReduceKind::Max
                };
                return Some(VectorPlan::Reduce3D {
                    kind,
                    acc_phi: id,
                    base: canonical_value(fn_ir, base),
                    axis,
                    fixed_a,
                    fixed_b,
                    start,
                    end,
                });
            }
            _ => {}
        }
    }
    None
}
