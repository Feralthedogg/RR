use super::*;
pub(crate) fn is_runtime_vector_read_call(callee: &str, arity: usize) -> bool {
    (matches!(callee, "rr_index1_read" | "rr_index1_read_strict") && (arity == 2 || arity == 3))
        || (callee == "rr_array3_gather_values" && arity == 4)
}

pub(crate) fn floor_like_index_source(fn_ir: &FnIR, idx: ValueId) -> Option<ValueId> {
    let idx = canonical_value(fn_ir, idx);
    let ValueKind::Call { names, .. } = &fn_ir.values[idx].kind else {
        return None;
    };
    let resolved = resolve_call_info(fn_ir, idx)?;
    if !matches!(resolved.callee.as_str(), "floor" | "ceiling" | "trunc") {
        return None;
    }
    if resolved.args.len() != 1 {
        return None;
    }
    if !names.is_empty() && names.first().and_then(|n| n.as_ref()).is_some() {
        return None;
    }
    Some(resolved.args[0])
}

pub(crate) fn same_base_value(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    let a = canonical_value(fn_ir, a);
    let b = canonical_value(fn_ir, b);
    if a == b {
        return true;
    }
    match (resolve_base_var(fn_ir, a), resolve_base_var(fn_ir, b)) {
        (Some(va), Some(vb)) => va == vb,
        _ => false,
    }
}

pub(crate) fn expr_reads_base_non_iv(
    fn_ir: &FnIR,
    root: ValueId,
    base: ValueId,
    iv_phi: ValueId,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        base: ValueId,
        iv_phi: ValueId,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Index1D {
                base: read_base,
                idx,
                ..
            } => {
                if same_base_value(fn_ir, *read_base, base)
                    && !is_iv_equivalent(fn_ir, *idx, iv_phi)
                {
                    return true;
                }
                rec(fn_ir, *read_base, base, iv_phi, seen) || rec(fn_ir, *idx, base, iv_phi, seen)
            }
            ValueKind::Index2D {
                base: read_base,
                r,
                c,
            } => {
                if same_base_value(fn_ir, *read_base, base) {
                    return true;
                }
                rec(fn_ir, *read_base, base, iv_phi, seen)
                    || rec(fn_ir, *r, base, iv_phi, seen)
                    || rec(fn_ir, *c, base, iv_phi, seen)
            }
            ValueKind::Index3D {
                base: read_base,
                i,
                j,
                k,
            } => {
                if same_base_value(fn_ir, *read_base, base) {
                    return true;
                }
                rec(fn_ir, *read_base, base, iv_phi, seen)
                    || rec(fn_ir, *i, base, iv_phi, seen)
                    || rec(fn_ir, *j, base, iv_phi, seen)
                    || rec(fn_ir, *k, base, iv_phi, seen)
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, base, iv_phi, seen) || rec(fn_ir, *rhs, base, iv_phi, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, base, iv_phi, seen),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                args.iter().any(|a| rec(fn_ir, *a, base, iv_phi, seen))
            }
            ValueKind::RecordLit { fields } => fields
                .iter()
                .any(|(_, value)| rec(fn_ir, *value, base, iv_phi, seen)),
            ValueKind::FieldGet {
                base: read_base, ..
            } => rec(fn_ir, *read_base, base, iv_phi, seen),
            ValueKind::FieldSet {
                base: read_base,
                value,
                ..
            } => {
                rec(fn_ir, *read_base, base, iv_phi, seen) || rec(fn_ir, *value, base, iv_phi, seen)
            }
            ValueKind::Phi { args } => args.iter().any(|(a, _)| rec(fn_ir, *a, base, iv_phi, seen)),
            ValueKind::Len { base: b } | ValueKind::Indices { base: b } => {
                rec(fn_ir, *b, base, iv_phi, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, base, iv_phi, seen) || rec(fn_ir, *end, base, iv_phi, seen)
            }
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => false,
        }
    }
    rec(fn_ir, root, base, iv_phi, &mut FxHashSet::default())
}

pub(crate) fn expr_has_iv_dependency(fn_ir: &FnIR, root: ValueId, iv_phi: ValueId) -> bool {
    fn load_var_depends_on_iv(
        fn_ir: &FnIR,
        var: &str,
        iv_phi: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        if !seen_vars.insert(var.to_string()) {
            return false;
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if rec(fn_ir, *src, iv_phi, seen_vals, seen_vars) {
                    seen_vars.remove(var);
                    return true;
                }
            }
        }
        seen_vars.remove(var);
        false
    }

    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen_vals.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *rhs, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, iv_phi, seen_vals, seen_vars),
            ValueKind::Call { args, .. } => args
                .iter()
                .any(|a| rec(fn_ir, *a, iv_phi, seen_vals, seen_vars)),
            ValueKind::Phi { args } => args
                .iter()
                .any(|(a, _)| rec(fn_ir, *a, iv_phi, seen_vals, seen_vars)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, *base, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *end, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index1D { idx, .. } => rec(fn_ir, *idx, iv_phi, seen_vals, seen_vars),
            ValueKind::Index2D { r, c, .. } => {
                rec(fn_ir, *r, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *c, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index3D { i, j, k, .. } => {
                rec(fn_ir, *i, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *j, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *k, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Load { var } => {
                load_var_depends_on_iv(fn_ir, var, iv_phi, seen_vals, seen_vars)
            }
            _ => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}
