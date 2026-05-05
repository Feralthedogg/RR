use super::*;
pub(crate) fn expr_has_non_vector_safe_call(
    fn_ir: &FnIR,
    root: ValueId,
    user_call_whitelist: &FxHashSet<String>,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Call { args, .. } => {
            let resolved = resolve_call_info(fn_ir, root);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            if !is_vector_safe_call(callee, call_args.len(), user_call_whitelist)
                && !is_runtime_vector_read_call(callee, call_args.len())
            {
                if vectorize_trace_enabled() {
                    eprintln!(
                        "   [vec-expr-map] non-vector-safe call: {} / arity {}",
                        callee,
                        call_args.len()
                    );
                }
                return true;
            }
            call_args
                .iter()
                .any(|a| expr_has_non_vector_safe_call(fn_ir, *a, user_call_whitelist, seen))
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *lhs, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *rhs, user_call_whitelist, seen)
        }
        ValueKind::Unary { rhs, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *rhs, user_call_whitelist, seen)
        }
        ValueKind::RecordLit { fields } => fields.iter().any(|(_, value)| {
            expr_has_non_vector_safe_call(fn_ir, *value, user_call_whitelist, seen)
        }),
        ValueKind::FieldGet { base, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
        }
        ValueKind::FieldSet { base, value, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *value, user_call_whitelist, seen)
        }
        ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|a| expr_has_non_vector_safe_call(fn_ir, *a, user_call_whitelist, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(a, _)| expr_has_non_vector_safe_call(fn_ir, *a, user_call_whitelist, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
        }
        ValueKind::Range { start, end } => {
            expr_has_non_vector_safe_call(fn_ir, *start, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *end, user_call_whitelist, seen)
        }
        ValueKind::Index1D { base, idx, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *idx, user_call_whitelist, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *r, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *c, user_call_whitelist, seen)
        }
        ValueKind::Index3D { base, i, j, k } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *i, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *j, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *k, user_call_whitelist, seen)
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
    }
}

pub(crate) fn expr_has_non_vector_safe_call_in_vector_context(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    user_call_whitelist: &FxHashSet<String>,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    if is_loop_invariant_pure_scalar_call_expr(fn_ir, root, iv_phi) {
        return false;
    }
    if !seen.insert(root) {
        return false;
    }
    if is_scalar_broadcast_value(fn_ir, root) && !expr_has_iv_dependency(fn_ir, root, iv_phi) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Call { args, .. } => {
            let resolved = resolve_call_info(fn_ir, root);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            if !is_vector_safe_call(callee, call_args.len(), user_call_whitelist)
                && !is_runtime_vector_read_call(callee, call_args.len())
            {
                if vectorize_trace_enabled() {
                    eprintln!(
                        "   [vec-expr-map] non-vector-safe call: {} / arity {}",
                        callee,
                        call_args.len()
                    );
                }
                return true;
            }
            call_args.iter().any(|a| {
                expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *a,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
            })
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *lhs,
                iv_phi,
                user_call_whitelist,
                seen,
            ) || expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *rhs,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }
        ValueKind::Unary { rhs, .. } => expr_has_non_vector_safe_call_in_vector_context(
            fn_ir,
            *rhs,
            iv_phi,
            user_call_whitelist,
            seen,
        ),
        ValueKind::RecordLit { fields } => fields.iter().any(|(_, value)| {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *value,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }),
        ValueKind::FieldGet { base, .. } => {
            expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::FieldSet { base, value, .. } => {
            let base_blocking = expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                );
            base_blocking
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *value,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Intrinsic { args, .. } => args.iter().any(|a| {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *a,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }),
        ValueKind::Phi { args } => args.iter().any(|(a, _)| {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *a,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Range { start, end } => {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *start,
                iv_phi,
                user_call_whitelist,
                seen,
            ) || expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *end,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }
        ValueKind::Index1D { base, idx, .. } => {
            let base_blocking = expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                );
            base_blocking
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *idx,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Index2D { base, r, c } => {
            let base_blocking = expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                );
            base_blocking
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *r,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *c,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Index3D { base, i, j, k } => {
            let base_blocking = expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                );
            base_blocking
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *i,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *j,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *k,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
    }
}

pub(crate) fn is_loop_invariant_pure_scalar_call_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
) -> bool {
    fn scalar_reducer_call(callee: &str, arity: usize) -> bool {
        let callee = normalize_callee_name(callee);
        matches!(
            (callee, arity),
            ("sum", 1)
                | ("mean", 1)
                | ("var", 1)
                | ("sd", 1)
                | ("min", 1)
                | ("max", 1)
                | ("prod", 1)
                | ("length", 1)
                | ("nrow", 1)
                | ("ncol", 1)
        )
    }

    fn invariant_pure_arg_expr(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return true;
        }
        if expr_has_iv_dependency(fn_ir, root, iv_phi) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_)
            | ValueKind::Load { .. }
            | ValueKind::Param { .. }
            | ValueKind::RSymbol { .. } => true,
            ValueKind::Unary { rhs, .. } => invariant_pure_arg_expr(fn_ir, *rhs, iv_phi, seen),
            ValueKind::Binary { lhs, rhs, .. } => {
                invariant_pure_arg_expr(fn_ir, *lhs, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *rhs, iv_phi, seen)
            }
            ValueKind::RecordLit { fields } => fields
                .iter()
                .all(|(_, value)| invariant_pure_arg_expr(fn_ir, *value, iv_phi, seen)),
            ValueKind::FieldGet { base, .. } => invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen),
            ValueKind::FieldSet { base, value, .. } => {
                invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *value, iv_phi, seen)
            }
            ValueKind::Call { args, .. } => {
                let resolved = resolve_call_info(fn_ir, root);
                let callee = resolved
                    .as_ref()
                    .map(|call| call.callee.as_str())
                    .unwrap_or("rr_call_closure");
                let call_args = resolved
                    .as_ref()
                    .map(|call| call.args.as_slice())
                    .unwrap_or(args.as_slice());
                call_is_semantically_pure(callee)
                    && call_args
                        .iter()
                        .all(|arg| invariant_pure_arg_expr(fn_ir, *arg, iv_phi, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|arg| invariant_pure_arg_expr(fn_ir, *arg, iv_phi, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen)
            }
            ValueKind::Range { start, end } => {
                invariant_pure_arg_expr(fn_ir, *start, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *end, iv_phi, seen)
            }
            ValueKind::Index1D { base, idx, .. } => {
                invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *idx, iv_phi, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *r, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *c, iv_phi, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *i, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *j, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *k, iv_phi, seen)
            }
            ValueKind::Phi { .. } => false,
        }
    }

    if expr_has_iv_dependency(fn_ir, root, iv_phi) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Call { args, .. } => {
            let resolved = resolve_call_info(fn_ir, root);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            scalar_reducer_call(callee, call_args.len())
                && call_args.iter().all(|arg| {
                    invariant_pure_arg_expr(fn_ir, *arg, iv_phi, &mut FxHashSet::default())
                })
        }
        ValueKind::Intrinsic { op, args } => {
            matches!(op, IntrinsicOp::VecSumF64 | IntrinsicOp::VecMeanF64)
                && args.iter().all(|arg| {
                    invariant_pure_arg_expr(fn_ir, *arg, iv_phi, &mut FxHashSet::default())
                })
        }
        _ => false,
    }
}
