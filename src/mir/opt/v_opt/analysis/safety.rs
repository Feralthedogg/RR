use super::*;

pub(in crate::mir::opt::v_opt) fn is_vector_safe_call(
    callee: &str,
    arity: usize,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    let callee = normalize_callee_name(callee);
    is_builtin_vector_safe_call(callee, arity)
        || user_call_whitelist.contains(callee)
        || user_call_whitelist.contains(&format!("base::{callee}"))
}

pub(in crate::mir::opt::v_opt) fn is_const_number(fn_ir: &FnIR, vid: ValueId) -> bool {
    matches!(
        fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(Lit::Int(_)) | ValueKind::Const(Lit::Float(_))
    )
}

pub(in crate::mir::opt::v_opt) fn is_const_one(fn_ir: &FnIR, vid: ValueId) -> bool {
    match fn_ir.values[canonical_value(fn_ir, vid)].kind {
        ValueKind::Const(Lit::Int(n)) => n == 1,
        ValueKind::Const(Lit::Float(f)) => (f - 1.0).abs() < f64::EPSILON,
        _ => false,
    }
}

pub(in crate::mir::opt::v_opt) fn is_invariant_reduce_scalar(
    fn_ir: &FnIR,
    scalar: ValueId,
    iv_phi: ValueId,
    base: ValueId,
) -> bool {
    if expr_has_iv_dependency(fn_ir, scalar, iv_phi) || expr_reads_base(fn_ir, scalar, base) {
        return false;
    }
    match &fn_ir.values[canonical_value(fn_ir, scalar)].kind {
        ValueKind::Const(Lit::Int(_))
        | ValueKind::Const(Lit::Float(_))
        | ValueKind::Param { .. } => true,
        ValueKind::Load { var } => {
            if let Some(base_var) = resolve_base_var(fn_ir, base) {
                var != &base_var
            } else {
                true
            }
        }
        _ => false,
    }
}

pub(in crate::mir::opt::v_opt) fn is_loop_invariant_scalar_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        user_call_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return true;
        }
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) => true,
            ValueKind::Load { .. } | ValueKind::Param { .. } => {
                vector_length_key(fn_ir, root).is_none()
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, iv_phi, user_call_whitelist, seen),
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, user_call_whitelist, seen)
                    && rec(fn_ir, *rhs, iv_phi, user_call_whitelist, seen)
            }
            ValueKind::Len { base } => rec(fn_ir, *base, iv_phi, user_call_whitelist, seen),
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
                is_vector_safe_call(callee, call_args.len(), user_call_whitelist)
                    && call_args
                        .iter()
                        .all(|a| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen)),
            ValueKind::RecordLit { fields } => fields
                .iter()
                .all(|(_, value)| rec(fn_ir, *value, iv_phi, user_call_whitelist, seen)),
            ValueKind::FieldGet { .. } | ValueKind::FieldSet { .. } => false,
            ValueKind::Phi { args } => args
                .iter()
                .all(|(a, _)| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen)),
            ValueKind::Index1D { .. }
            | ValueKind::Index2D { .. }
            | ValueKind::Index3D { .. }
            | ValueKind::Range { .. }
            | ValueKind::Indices { .. }
            | ValueKind::RSymbol { .. } => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    )
}

pub(in crate::mir::opt::v_opt) fn is_vector_safe_call_chain_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        _lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. } => true,
            ValueKind::Unary { rhs, .. } => {
                rec(fn_ir, *rhs, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, _lp, user_call_whitelist, seen)
                    && rec(fn_ir, *rhs, iv_phi, _lp, user_call_whitelist, seen)
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
                is_vector_safe_call(callee, call_args.len(), user_call_whitelist)
                    && call_args
                        .iter()
                        .all(|a| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen)),
            ValueKind::RecordLit { fields } => fields
                .iter()
                .all(|(_, value)| rec(fn_ir, *value, iv_phi, _lp, user_call_whitelist, seen)),
            ValueKind::FieldGet { .. } | ValueKind::FieldSet { .. } => false,
            ValueKind::Index1D {
                base: _base,
                idx,
                is_safe: _is_safe,
                is_na_safe: _is_na_safe,
            } => is_iv_equivalent(fn_ir, *idx, iv_phi),
            ValueKind::Phi { args } => args
                .iter()
                .all(|(a, _)| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, *base, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, iv_phi, _lp, user_call_whitelist, seen)
                    && rec(fn_ir, *end, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Index2D { .. } | ValueKind::Index3D { .. } | ValueKind::RSymbol { .. } => {
                false
            }
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        user_call_whitelist,
        &mut FxHashSet::default(),
    )
}
