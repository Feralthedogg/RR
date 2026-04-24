use super::*;

pub(crate) fn loop_vectorize_skip_reason(fn_ir: &FnIR, lp: &LoopInfo) -> VectorizeSkipReason {
    if lp.iv.is_none() {
        return VectorizeSkipReason::NoIv;
    }
    if lp.is_seq_len.is_none() {
        return VectorizeSkipReason::NonCanonicalBound;
    }
    if loop_has_unsupported_cfg_shape(fn_ir, lp) {
        return VectorizeSkipReason::UnsupportedCfgShape;
    }
    if loop_has_indirect_index_access(fn_ir, lp) {
        return VectorizeSkipReason::IndirectIndexAccess;
    }
    if loop_has_store_effect(fn_ir, lp) {
        return VectorizeSkipReason::StoreEffects;
    }
    VectorizeSkipReason::NoSupportedPattern
}

pub(crate) fn loop_has_unsupported_cfg_shape(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let preds = build_pred_map(fn_ir);
    let outer_preds = preds
        .get(&lp.header)
        .map(|ps| ps.iter().filter(|b| !lp.body.contains(b)).count())
        .unwrap_or(0);
    outer_preds != 1 || lp.exits.len() != 1
}

pub(crate) fn loop_has_indirect_index_access(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let Some(iv) = lp.iv.as_ref() else {
        return false;
    };
    let iv_phi = iv.phi_val;
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    if expr_has_non_iv_index(fn_ir, *src, iv_phi, &mut FxHashSet::default()) {
                        return true;
                    }
                }
                Instr::StoreIndex1D { idx, val, .. } => {
                    if !is_iv_equivalent(fn_ir, *idx, iv_phi) {
                        return true;
                    }
                    if expr_has_non_iv_index(fn_ir, *val, iv_phi, &mut FxHashSet::default()) {
                        return true;
                    }
                }
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => return true,
            }
        }
    }
    false
}

pub(crate) fn expr_has_non_iv_index(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Index1D { idx, base, .. } => {
            if !is_iv_equivalent(fn_ir, *idx, iv_phi) {
                return true;
            }
            expr_has_non_iv_index(fn_ir, *base, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *idx, iv_phi, seen)
        }
        ValueKind::Index2D { .. } | ValueKind::Index3D { .. } => true,
        ValueKind::RecordLit { fields } => fields
            .iter()
            .any(|(_, value)| expr_has_non_iv_index(fn_ir, *value, iv_phi, seen)),
        ValueKind::FieldGet { base, .. } => expr_has_non_iv_index(fn_ir, *base, iv_phi, seen),
        ValueKind::FieldSet { base, value, .. } => {
            expr_has_non_iv_index(fn_ir, *base, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *value, iv_phi, seen)
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_has_non_iv_index(fn_ir, *lhs, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *rhs, iv_phi, seen)
        }
        ValueKind::Unary { rhs, .. } => expr_has_non_iv_index(fn_ir, *rhs, iv_phi, seen),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|a| expr_has_non_iv_index(fn_ir, *a, iv_phi, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(a, _)| expr_has_non_iv_index(fn_ir, *a, iv_phi, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_has_non_iv_index(fn_ir, *base, iv_phi, seen)
        }
        ValueKind::Range { start, end } => {
            expr_has_non_iv_index(fn_ir, *start, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *end, iv_phi, seen)
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
    }
}

pub(crate) fn resolve_base_var(fn_ir: &FnIR, base: ValueId) -> Option<VarId> {
    if let ValueKind::Load { var } = &fn_ir.values[base].kind {
        return Some(var.clone());
    }
    fn_ir.values[base].origin_var.clone()
}

pub(crate) fn rewrite_returns_for_var(fn_ir: &mut FnIR, var: &str, new_val: ValueId) {
    for bid in 0..fn_ir.blocks.len() {
        if let Terminator::Return(Some(ret_vid)) = fn_ir.blocks[bid].term
            && fn_ir.values[ret_vid].origin_var.as_deref() == Some(var)
        {
            fn_ir.blocks[bid].term = Terminator::Return(Some(new_val));
        }
    }
}

pub(crate) fn extract_store_1d_in_block(
    fn_ir: &FnIR,
    bid: BlockId,
    iv_phi: ValueId,
) -> Option<(ValueId, ValueId)> {
    match classify_store_1d_in_block(fn_ir, bid) {
        BlockStore1DMatch::One(store)
            if !store.is_vector && is_iv_equivalent(fn_ir, store.idx, iv_phi) =>
        {
            Some((store.base, store.val))
        }
        _ => None,
    }
}

pub(crate) fn classify_store_1d_in_block(fn_ir: &FnIR, bid: BlockId) -> BlockStore1DMatch {
    let mut found: Option<BlockStore1D> = None;
    for instr in &fn_ir.blocks[bid].instrs {
        match instr {
            Instr::Assign { .. } => {}
            Instr::Eval { .. } => {
                return BlockStore1DMatch::Invalid;
            }
            Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => {
                return BlockStore1DMatch::Invalid;
            }
            Instr::StoreIndex1D {
                base,
                idx,
                val,
                is_vector,
                ..
            } => {
                if found.is_some() {
                    return BlockStore1DMatch::Invalid;
                }
                found = Some(BlockStore1D {
                    base: *base,
                    idx: *idx,
                    val: *val,
                    is_vector: *is_vector,
                });
            }
        }
    }
    match found {
        Some(store) => BlockStore1DMatch::One(store),
        None => BlockStore1DMatch::None,
    }
}

pub(crate) fn classify_store_3d_in_block(fn_ir: &FnIR, bid: BlockId) -> BlockStore3DMatch {
    let mut found: Option<BlockStore3D> = None;
    for instr in &fn_ir.blocks[bid].instrs {
        match instr {
            Instr::Assign { .. } => {}
            Instr::Eval { .. } => {
                return BlockStore3DMatch::Invalid;
            }
            Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                return BlockStore3DMatch::Invalid;
            }
            Instr::StoreIndex3D {
                base, i, j, k, val, ..
            } => {
                if found.is_some() {
                    return BlockStore3DMatch::Invalid;
                }
                found = Some(BlockStore3D {
                    base: *base,
                    i: *i,
                    j: *j,
                    k: *k,
                    val: *val,
                });
            }
        }
    }
    match found {
        Some(store) => BlockStore3DMatch::One(store),
        None => BlockStore3DMatch::None,
    }
}

pub(crate) fn is_prev_element(fn_ir: &FnIR, vid: ValueId, base: ValueId, iv_phi: ValueId) -> bool {
    match &fn_ir.values[vid].kind {
        ValueKind::Index1D { base: b, idx, .. } => {
            if canonical_value(fn_ir, *b) != canonical_value(fn_ir, base) {
                return false;
            }
            is_iv_minus_one(fn_ir, *idx, iv_phi)
        }
        _ => false,
    }
}

pub(crate) fn is_prev_element_3d(
    fn_ir: &FnIR,
    vid: ValueId,
    base: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    iv_phi: ValueId,
) -> bool {
    match &fn_ir.values[vid].kind {
        ValueKind::Index3D { base: b, i, j, k } => {
            if canonical_value(fn_ir, *b) != canonical_value(fn_ir, base) {
                return false;
            }
            let i = resolve_load_alias_value(fn_ir, *i);
            let j = resolve_load_alias_value(fn_ir, *j);
            let k = resolve_load_alias_value(fn_ir, *k);
            let fixed_a = resolve_load_alias_value(fn_ir, fixed_a);
            let fixed_b = resolve_load_alias_value(fn_ir, fixed_b);
            match axis {
                Axis3D::Dim1 => {
                    is_iv_minus_one(fn_ir, i, iv_phi)
                        && same_loop_invariant_value(fn_ir, j, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, k, fixed_b, iv_phi)
                }
                Axis3D::Dim2 => {
                    is_iv_minus_one(fn_ir, j, iv_phi)
                        && same_loop_invariant_value(fn_ir, i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, k, fixed_b, iv_phi)
                }
                Axis3D::Dim3 => {
                    is_iv_minus_one(fn_ir, k, iv_phi)
                        && same_loop_invariant_value(fn_ir, i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, j, fixed_b, iv_phi)
                }
            }
        }
        _ => false,
    }
}

pub(crate) fn is_iv_minus_one(fn_ir: &FnIR, idx: ValueId, iv_phi: ValueId) -> bool {
    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        return false;
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } if is_iv_equivalent(fn_ir, *lhs, iv_phi) => {
            matches!(fn_ir.values[*rhs].kind, ValueKind::Const(Lit::Int(1)))
        }
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } if is_iv_equivalent(fn_ir, *lhs, iv_phi) => {
            matches!(fn_ir.values[*rhs].kind, ValueKind::Const(Lit::Int(-1)))
        }
        _ => false,
    }
}

pub(crate) fn expr_reads_base(fn_ir: &FnIR, root: ValueId, base: ValueId) -> bool {
    fn rec(fn_ir: &FnIR, root: ValueId, base: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Index1D { base: b, .. }
            | ValueKind::Index2D { base: b, .. }
            | ValueKind::Index3D { base: b, .. } => {
                if canonical_value(fn_ir, *b) == canonical_value(fn_ir, base) {
                    return true;
                }
            }
            _ => {}
        }
        match &fn_ir.values[root].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, base, seen) || rec(fn_ir, *rhs, base, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, base, seen),
            ValueKind::Call { args, .. } => args.iter().any(|a| rec(fn_ir, *a, base, seen)),
            ValueKind::Phi { args } => args.iter().any(|(a, _)| rec(fn_ir, *a, base, seen)),
            ValueKind::Len { base: b } | ValueKind::Indices { base: b } => {
                rec(fn_ir, *b, base, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, base, seen) || rec(fn_ir, *end, base, seen)
            }
            ValueKind::Index1D { base: b, idx, .. } => {
                rec(fn_ir, *b, base, seen) || rec(fn_ir, *idx, base, seen)
            }
            ValueKind::Index2D { base: b, r, c } => {
                rec(fn_ir, *b, base, seen)
                    || rec(fn_ir, *r, base, seen)
                    || rec(fn_ir, *c, base, seen)
            }
            ValueKind::Index3D { base: b, i, j, k } => {
                rec(fn_ir, *b, base, seen)
                    || rec(fn_ir, *i, base, seen)
                    || rec(fn_ir, *j, base, seen)
                    || rec(fn_ir, *k, base, seen)
            }
            _ => false,
        }
    }
    rec(fn_ir, root, base, &mut FxHashSet::default())
}

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

fn is_loop_invariant_pure_scalar_call_expr(fn_ir: &FnIR, root: ValueId, iv_phi: ValueId) -> bool {
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

pub(crate) fn is_vectorizable_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    allow_any_base: bool,
    require_safe_index: bool,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        allow_any_base: bool,
        require_safe_index: bool,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        if root == iv_phi {
            return true;
        }
        if !seen.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(
                    fn_ir,
                    *lhs,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                ) && rec(
                    fn_ir,
                    *rhs,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }
            ValueKind::Unary { rhs, .. } => rec(
                fn_ir,
                *rhs,
                iv_phi,
                lp,
                allow_any_base,
                require_safe_index,
                seen,
            ),
            ValueKind::Call { args, .. } => args.iter().all(|a| {
                rec(
                    fn_ir,
                    *a,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }),
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                if require_safe_index && !(*is_safe && *is_na_safe) {
                    return false;
                }
                if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, *base) {
                    return false;
                }
                if is_iv_equivalent(fn_ir, *idx, iv_phi) {
                    return true;
                }
                !require_safe_index
                    && expr_has_iv_dependency(fn_ir, *idx, iv_phi)
                    && rec(
                        fn_ir,
                        *idx,
                        iv_phi,
                        lp,
                        allow_any_base,
                        require_safe_index,
                        seen,
                    )
            }
            ValueKind::Index3D { base, i, j, k } => {
                if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, *base) {
                    return false;
                }
                let Some(pattern) =
                    classify_3d_general_vector_access(fn_ir, *base, *i, *j, *k, iv_phi)
                else {
                    return false;
                };
                [pattern.i, pattern.j, pattern.k]
                    .into_iter()
                    .all(|operand| match operand {
                        VectorAccessOperand3D::Scalar(_) => true,
                        VectorAccessOperand3D::Vector(dep_idx) => {
                            is_iv_equivalent(fn_ir, dep_idx, iv_phi)
                                || rec(
                                    fn_ir,
                                    dep_idx,
                                    iv_phi,
                                    lp,
                                    allow_any_base,
                                    require_safe_index,
                                    seen,
                                )
                        }
                    })
            }
            ValueKind::Phi { args } => {
                if fn_ir.values[root]
                    .phi_block
                    .is_some_and(|bb| !lp.body.contains(&bb))
                    && !expr_has_iv_dependency(fn_ir, root, iv_phi)
                {
                    true
                } else {
                    args.iter().all(|(a, _)| {
                        rec(
                            fn_ir,
                            *a,
                            iv_phi,
                            lp,
                            allow_any_base,
                            require_safe_index,
                            seen,
                        )
                    })
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => rec(
                fn_ir,
                *base,
                iv_phi,
                lp,
                allow_any_base,
                require_safe_index,
                seen,
            ),
            ValueKind::Range { start, end } => {
                rec(
                    fn_ir,
                    *start,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                ) && rec(
                    fn_ir,
                    *end,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }
            _ => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        allow_any_base,
        require_safe_index,
        &mut FxHashSet::default(),
    )
}

pub(crate) fn is_condition_vectorizable(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    #[allow(clippy::too_many_arguments)]
    fn load_var_condition_vectorizable(
        fn_ir: &FnIR,
        var: &str,
        iv_phi: ValueId,
        lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
        depth: usize,
    ) -> bool {
        if proof_budget_exhausted(depth, seen_vals, seen_vars) {
            return false;
        }
        if !seen_vars.insert(var.to_string()) {
            return false;
        }
        let mut found = false;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                found = true;
                if !rec(
                    fn_ir,
                    *src,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                ) {
                    seen_vars.remove(var);
                    return false;
                }
            }
        }
        seen_vars.remove(var);
        // Params and immutable captures can appear as bare loads with no local assignment.
        // Treat them as loop-invariant condition inputs.
        if !found {
            return true;
        }
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
        depth: usize,
    ) -> bool {
        if proof_budget_exhausted(depth, seen_vals, seen_vars) {
            return false;
        }
        let root = canonical_value(fn_ir, root);
        if root == iv_phi || is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen_vals.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Param { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(
                    fn_ir,
                    *lhs,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                ) && rec(
                    fn_ir,
                    *rhs,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                )
            }
            ValueKind::Unary { rhs, .. } => rec(
                fn_ir,
                *rhs,
                iv_phi,
                lp,
                user_call_whitelist,
                seen_vals,
                seen_vars,
                depth + 1,
            ),
            // Data-dependent conditions are now allowed if the access is proven safe.
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                let iv_idx = is_iv_equivalent(fn_ir, *idx, iv_phi);
                let iv_dependent_idx = iv_idx || expr_has_iv_dependency(fn_ir, *idx, iv_phi);
                if !is_loop_compatible_base(lp, fn_ir, *base) && !iv_dependent_idx {
                    return false;
                }
                // Fast-path: proven-safe scalar read on IV-aligned index.
                if *is_safe && *is_na_safe && iv_idx {
                    return true;
                }
                // Pre-BCE loops often carry unspecialized index safety flags.
                // Allow IV-dependent index expressions and rely on runtime vector
                // read guards during materialization (`rr_index1_read_vec` path).
                iv_dependent_idx
            }
            ValueKind::Index2D { .. } => false,
            ValueKind::Index3D { base, i, j, k } => {
                is_loop_compatible_base(lp, fn_ir, *base)
                    && classify_3d_general_vector_access(fn_ir, *base, *i, *j, *k, iv_phi)
                        .is_some_and(|pattern| {
                            [pattern.i, pattern.j, pattern.k]
                                .into_iter()
                                .all(|operand| match operand {
                                    VectorAccessOperand3D::Scalar(_) => true,
                                    VectorAccessOperand3D::Vector(dep_idx) => rec(
                                        fn_ir,
                                        dep_idx,
                                        iv_phi,
                                        lp,
                                        user_call_whitelist,
                                        seen_vals,
                                        seen_vars,
                                        depth + 1,
                                    ),
                                })
                        })
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
                let runtime_read = is_runtime_vector_read_call(callee, call_args.len());
                if runtime_read
                    && let Some(base) = call_args.first().copied()
                    && !is_loop_compatible_base(lp, fn_ir, base)
                {
                    return false;
                }
                (is_vector_safe_call(callee, call_args.len(), user_call_whitelist) || runtime_read)
                    && call_args.iter().all(|a| {
                        rec(
                            fn_ir,
                            *a,
                            iv_phi,
                            lp,
                            user_call_whitelist,
                            seen_vals,
                            seen_vars,
                            depth + 1,
                        )
                    })
            }
            ValueKind::Phi { args } => args.iter().all(|(a, _)| {
                rec(
                    fn_ir,
                    *a,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                )
            }),
            ValueKind::Load { var } => load_var_condition_vectorizable(
                fn_ir,
                var,
                iv_phi,
                lp,
                user_call_whitelist,
                seen_vals,
                seen_vars,
                depth + 1,
            ),
            _ => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        user_call_whitelist,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
        0,
    )
}

pub(crate) fn is_loop_compatible_base(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    if loop_matches_vec(lp, fn_ir, base) {
        return true;
    }
    let Some(loop_key) = loop_length_key(lp, fn_ir) else {
        return false;
    };
    match vector_length_key(fn_ir, base) {
        Some(k) => canonical_value(fn_ir, k) == canonical_value(fn_ir, loop_key),
        None => false,
    }
}

pub(crate) fn loop_covers_whole_destination(
    lp: &LoopInfo,
    fn_ir: &FnIR,
    base: ValueId,
    start: ValueId,
) -> bool {
    is_const_one(fn_ir, start) && loop_matches_full_base(lp, fn_ir, base)
}

pub(crate) fn loop_length_key(lp: &LoopInfo, fn_ir: &FnIR) -> Option<ValueId> {
    if lp.limit_adjust != 0 {
        return None;
    }
    let limit = lp.limit?;
    match &fn_ir.values[limit].kind {
        ValueKind::Len { base } => vector_length_key(fn_ir, *base),
        _ => Some(resolve_load_alias_value(fn_ir, limit)),
    }
}

pub(crate) fn affine_iv_offset(fn_ir: &FnIR, idx: ValueId, iv_phi: ValueId) -> Option<i64> {
    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        return Some(0);
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(k);
            }
            if is_iv_equivalent(fn_ir, *rhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*lhs].kind
            {
                return Some(k);
            }
            None
        }
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(-k);
            }
            None
        }
        _ => None,
    }
}

pub(crate) fn loop_matches_full_base(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    if lp.limit_adjust != 0 {
        return false;
    }

    let base = canonical_value(fn_ir, base);
    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) == Some(base) {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        )
        && a == b
    {
        return true;
    }

    if let Some(limit) = lp
        .is_seq_len
        .map(|limit| resolve_load_alias_value(fn_ir, limit))
    {
        if let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind {
            if canonical_value(fn_ir, len_base) == base {
                return true;
            }
            if let (Some(a), Some(b)) = (
                resolve_base_var(fn_ir, base),
                resolve_base_var(fn_ir, len_base),
            ) && a == b
            {
                return true;
            }
        }

        if base_length_key(fn_ir, base).is_some_and(|base_key| {
            canonical_value(fn_ir, base_key) == canonical_value(fn_ir, limit)
        }) {
            return true;
        }
    }

    if let (Some(base_key), Some(loop_key)) =
        (base_length_key(fn_ir, base), loop_length_key(lp, fn_ir))
    {
        return canonical_value(fn_ir, base_key) == canonical_value(fn_ir, loop_key);
    }

    false
}

pub(crate) fn loop_matches_vec(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    let base = canonical_value(fn_ir, base);
    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) == Some(base) {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        )
        && a == b
    {
        return true;
    }
    if let Some(limit) = lp.is_seq_len
        && let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind
    {
        if canonical_value(fn_ir, len_base) == base {
            return true;
        }
        if let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, len_base),
        ) && a == b
        {
            return true;
        }
    }
    false
}

pub(crate) fn vector_length_key(fn_ir: &FnIR, root: ValueId) -> Option<ValueId> {
    fn unify_length_keys(
        fn_ir: &FnIR,
        keys: impl IntoIterator<Item = Option<ValueId>>,
    ) -> Option<ValueId> {
        let mut out: Option<ValueId> = None;
        for key in keys {
            let key = key.map(|k| resolve_load_alias_value(fn_ir, k))?;
            match out {
                None => out = Some(key),
                Some(prev) if resolve_load_alias_value(fn_ir, prev) == key => {}
                Some(_) => return None,
            }
        }
        out
    }

    fn rec_var(
        fn_ir: &FnIR,
        var: &str,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }
        if let Some(param_index) = fn_ir.params.iter().position(|p| p == var) {
            let param_vid = fn_ir
                .values
                .iter()
                .position(|value| matches!(value.kind, ValueKind::Param { index } if index == param_index))
                .map(|vid| vid as ValueId);
            if let Some(param_vid) = param_vid {
                return Some(resolve_load_alias_value(fn_ir, param_vid));
            }
        }

        let mut saw_assign = false;
        let mut keys = Vec::new();
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                saw_assign = true;
                keys.push(rec(fn_ir, *src, seen_vals, seen_vars));
            }
        }
        if !saw_assign {
            return None;
        }
        unify_length_keys(fn_ir, keys)
    }

    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        if !seen_vals.insert(root) {
            return None;
        }
        if matches!(fn_ir.values[root].kind, ValueKind::Phi { .. })
            && let Some(var) = fn_ir.values[root].origin_var.as_deref()
            && let Some(key) = rec_var(fn_ir, var, seen_vals, seen_vars)
        {
            seen_vals.remove(&root);
            return Some(key);
        }
        let out = match &fn_ir.values[root].kind {
            ValueKind::Load { var } => rec_var(fn_ir, var, seen_vals, seen_vars),
            ValueKind::Phi { args } => {
                let keys = args
                    .iter()
                    .filter_map(|(arg, _)| {
                        let arg = canonical_value(fn_ir, *arg);
                        if arg == root {
                            None
                        } else {
                            rec(fn_ir, arg, seen_vals, seen_vars)
                        }
                    })
                    .collect::<Vec<_>>();
                if keys.is_empty() {
                    None
                } else {
                    unify_length_keys(fn_ir, keys.into_iter().map(Some))
                }
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
                if callee == "seq_len" && call_args.len() == 1 {
                    Some(resolve_load_alias_value(fn_ir, call_args[0]))
                } else if matches!(callee, "rep.int" | "numeric") && !call_args.is_empty() {
                    let len_arg = if callee == "rep.int" {
                        call_args
                            .get(1)
                            .copied()
                            .or_else(|| call_args.first().copied())
                    } else {
                        call_args.first().copied()
                    }?;
                    Some(resolve_load_alias_value(fn_ir, len_arg))
                } else if is_builtin_vector_safe_call(callee, call_args.len())
                    || matches!(callee, "ifelse" | "rr_ifelse_strict")
                {
                    let mut vec_keys = Vec::new();
                    let mut saw_vectorish = false;
                    for arg in call_args {
                        let key = rec(fn_ir, *arg, seen_vals, seen_vars);
                        if key.is_some() {
                            saw_vectorish = true;
                            vec_keys.push(key);
                        } else if !is_scalar_value(fn_ir, *arg) {
                            return None;
                        }
                    }
                    if !saw_vectorish {
                        None
                    } else {
                        unify_length_keys(fn_ir, vec_keys)
                    }
                } else {
                    None
                }
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                let lk = rec(fn_ir, *lhs, seen_vals, seen_vars);
                let rk = rec(fn_ir, *rhs, seen_vals, seen_vars);
                match (lk, rk) {
                    (Some(a), Some(b))
                        if canonical_value(fn_ir, a) == canonical_value(fn_ir, b) =>
                    {
                        Some(canonical_value(fn_ir, a))
                    }
                    (Some(k), None) if is_scalar_value(fn_ir, *rhs) => Some(k),
                    (None, Some(k)) if is_scalar_value(fn_ir, *lhs) => Some(k),
                    _ => None,
                }
            }
            _ => None,
        };
        seen_vals.remove(&root);
        out
    }
    rec(
        fn_ir,
        root,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

pub(crate) fn variable_length_key(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
    let mut keys = Vec::new();
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            keys.push(vector_length_key(fn_ir, *src));
        }
    }
    if keys.is_empty() {
        return None;
    }
    let mut out: Option<ValueId> = None;
    for key in keys {
        let key = key.map(|k| resolve_load_alias_value(fn_ir, k))?;
        match out {
            None => out = Some(key),
            Some(prev) if resolve_load_alias_value(fn_ir, prev) == key => {}
            Some(_) => return None,
        }
    }
    out
}

pub(crate) fn base_length_key(fn_ir: &FnIR, base: ValueId) -> Option<ValueId> {
    if let Some(var) = resolve_base_var(fn_ir, base)
        && let Some(key) = variable_length_key(fn_ir, &var)
    {
        return Some(key);
    }
    vector_length_key(fn_ir, base)
}

pub(crate) fn same_length_proven(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    let a = canonical_value(fn_ir, a);
    let b = canonical_value(fn_ir, b);
    if a == b {
        return true;
    }
    if fn_ir.values[a].value_ty.len_sym.is_some()
        && fn_ir.values[a].value_ty.len_sym == fn_ir.values[b].value_ty.len_sym
    {
        return true;
    }
    match (vector_length_key(fn_ir, a), vector_length_key(fn_ir, b)) {
        (Some(ka), Some(kb)) => canonical_value(fn_ir, ka) == canonical_value(fn_ir, kb),
        _ => false,
    }
}

pub(crate) fn is_scalar_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    matches!(
        fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. }
    )
}

pub(crate) fn is_iv_equivalent(fn_ir: &FnIR, candidate: ValueId, iv_phi: ValueId) -> bool {
    let mut seen_vals = FxHashSet::default();
    let mut seen_vars = FxHashSet::default();
    is_iv_equivalent_rec(fn_ir, candidate, iv_phi, &mut seen_vals, &mut seen_vars, 0)
}

pub(crate) fn is_iv_equivalent_rec(
    fn_ir: &FnIR,
    candidate: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
    depth: usize,
) -> bool {
    if proof_budget_exhausted(depth, seen_vals, seen_vars) {
        return false;
    }
    let candidate = canonical_value(fn_ir, candidate);
    if candidate == iv_phi {
        return true;
    }
    if !seen_vals.insert(candidate) {
        return false;
    }
    match &fn_ir.values[candidate].kind {
        ValueKind::Load { var } => {
            if induction_origin_var(fn_ir, iv_phi).as_deref() == Some(var.as_str()) {
                return true;
            }
            if let Some(src) = unique_assign_source(fn_ir, var)
                && src != candidate
            {
                return is_iv_equivalent_rec(fn_ir, src, iv_phi, seen_vals, seen_vars, depth + 1);
            }
            load_var_is_floor_like_iv(fn_ir, var, iv_phi, seen_vals, seen_vars, depth + 1)
        }
        ValueKind::Phi { args } if args.is_empty() => {
            match (
                fn_ir.values[candidate].origin_var.as_deref(),
                fn_ir.values[iv_phi].origin_var.as_deref(),
            ) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == candidate {
                    return false;
                }
                return is_iv_equivalent_rec(fn_ir, first, iv_phi, seen_vals, seen_vars, depth + 1);
            }
            let mut saw_iv_progress = false;
            for (v, _) in args {
                if is_iv_equivalent_rec(fn_ir, *v, iv_phi, seen_vals, seen_vars, depth + 1) {
                    saw_iv_progress = true;
                    continue;
                }
                if is_iv_seed_expr(
                    fn_ir,
                    *v,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                    depth + 1,
                ) {
                    continue;
                }
                return false;
            }
            saw_iv_progress
        }
        ValueKind::Call { args, names, .. } => {
            let resolved = resolve_call_info(fn_ir, candidate);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            let floor_like = resolved
                .as_ref()
                .and_then(|call| call.builtin_kind)
                .is_some_and(BuiltinKind::is_floor_like)
                || matches!(callee, "floor" | "ceiling" | "trunc");
            let single_positional = call_args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_equivalent_rec(
                    fn_ir,
                    call_args[0],
                    iv_phi,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                )
        }
        _ => false,
    }
}

pub(crate) fn induction_origin_var(fn_ir: &FnIR, iv_phi: ValueId) -> Option<String> {
    fn rec(fn_ir: &FnIR, vid: ValueId, seen: &mut FxHashSet<ValueId>) -> Option<String> {
        let vid = canonical_value(fn_ir, vid);
        if !seen.insert(vid) {
            return None;
        }
        if let Some(origin) = fn_ir.values[vid].origin_var.clone() {
            return Some(origin);
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Load { var } => Some(var.clone()),
            ValueKind::Phi { args } if !args.is_empty() => {
                let mut name: Option<String> = None;
                for (arg, _) in args {
                    let arg_name = rec(fn_ir, *arg, seen)?;
                    match &name {
                        None => name = Some(arg_name),
                        Some(prev) if prev == &arg_name => {}
                        Some(_) => return None,
                    }
                }
                name
            }
            _ => None,
        }
    }

    rec(fn_ir, iv_phi, &mut FxHashSet::default())
}

pub(crate) fn is_iv_seed_expr(
    fn_ir: &FnIR,
    vid: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
    depth: usize,
) -> bool {
    if proof_budget_exhausted(depth, seen_vals, seen_vars) {
        return false;
    }
    let vid = canonical_value(fn_ir, vid);
    if vid == iv_phi {
        return false;
    }
    if !seen_vals.insert(vid) {
        return false;
    }
    match &fn_ir.values[vid].kind {
        ValueKind::Const(_) | ValueKind::Param { .. } => true,
        ValueKind::Call { args, names, .. } => {
            let resolved = resolve_call_info(fn_ir, vid);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            let floor_like = resolved
                .as_ref()
                .and_then(|call| call.builtin_kind)
                .is_some_and(BuiltinKind::is_floor_like)
                || matches!(callee, "floor" | "ceiling" | "trunc");
            let single_positional = call_args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_seed_expr(fn_ir, call_args[0], iv_phi, seen_vals, seen_vars, depth + 1)
        }
        ValueKind::Load { var } => {
            if !seen_vars.insert(var.to_string()) {
                return false;
            }
            let mut found = false;
            for bb in &fn_ir.blocks {
                for ins in &bb.instrs {
                    let Instr::Assign { dst, src, .. } = ins else {
                        continue;
                    };
                    if dst != var {
                        continue;
                    }
                    found = true;
                    if !is_iv_seed_expr(fn_ir, *src, iv_phi, seen_vals, seen_vars, depth + 1) {
                        seen_vars.remove(var);
                        return false;
                    }
                }
            }
            seen_vars.remove(var);
            found
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == vid {
                    return false;
                }
                return is_iv_seed_expr(fn_ir, first, iv_phi, seen_vals, seen_vars, depth + 1);
            }
            args.iter()
                .all(|(v, _)| is_iv_seed_expr(fn_ir, *v, iv_phi, seen_vals, seen_vars, depth + 1))
        }
        _ => false,
    }
}

pub(crate) fn load_var_is_floor_like_iv(
    fn_ir: &FnIR,
    var: &str,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
    depth: usize,
) -> bool {
    fn is_seed_expr(fn_ir: &FnIR, src: ValueId) -> bool {
        matches!(
            fn_ir.values[canonical_value(fn_ir, src)].kind,
            ValueKind::Const(_) | ValueKind::Param { .. }
        )
    }

    if proof_budget_exhausted(depth, seen_vals, seen_vars) {
        return false;
    }
    if !seen_vars.insert(var.to_string()) {
        return false;
    }
    let mut found = false;
    let mut all_match = true;
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            // Ignore non-recursive seeds/reinitializations like `ii <- 0`.
            if is_seed_expr(fn_ir, *src) {
                continue;
            }
            found = true;
            if !is_floor_like_iv_expr(fn_ir, *src, iv_phi, seen_vals, seen_vars, depth + 1) {
                all_match = false;
                break;
            }
        }
        if !all_match {
            break;
        }
    }
    seen_vars.remove(var);
    found && all_match
}

pub(crate) fn is_floor_like_iv_expr(
    fn_ir: &FnIR,
    src: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
    depth: usize,
) -> bool {
    if proof_budget_exhausted(depth, seen_vals, seen_vars) {
        return false;
    }
    let src = canonical_value(fn_ir, src);
    match &fn_ir.values[src].kind {
        ValueKind::Call { args, names, .. } => {
            let resolved = resolve_call_info(fn_ir, src);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            let floor_like = resolved
                .as_ref()
                .and_then(|call| call.builtin_kind)
                .is_some_and(BuiltinKind::is_floor_like)
                || matches!(callee, "floor" | "ceiling" | "trunc");
            let single_positional = call_args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_equivalent_rec(
                    fn_ir,
                    call_args[0],
                    iv_phi,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                )
        }
        ValueKind::Load { var } => {
            load_var_is_floor_like_iv(fn_ir, var, iv_phi, seen_vals, seen_vars, depth + 1)
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == src {
                    return false;
                }
                return is_floor_like_iv_expr(
                    fn_ir,
                    first,
                    iv_phi,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                );
            }
            let mut saw_progress = false;
            for (v, _) in args {
                if is_iv_seed_expr(
                    fn_ir,
                    *v,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                    depth + 1,
                ) {
                    continue;
                }
                saw_progress = true;
                if !is_floor_like_iv_expr(fn_ir, *v, iv_phi, seen_vals, seen_vars, depth + 1) {
                    return false;
                }
            }
            saw_progress
        }
        _ => false,
    }
}

pub(crate) fn is_value_equivalent(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    if a == b {
        return true;
    }
    if canonical_value(fn_ir, a) == canonical_value(fn_ir, b) {
        return true;
    }
    match (&fn_ir.values[a].kind, &fn_ir.values[b].kind) {
        (ValueKind::Load { var: va }, ValueKind::Load { var: vb }) => va == vb,
        (ValueKind::Load { var }, ValueKind::Phi { args }) if args.is_empty() => {
            fn_ir.values[b].origin_var.as_deref() == Some(var.as_str())
        }
        (ValueKind::Phi { args }, ValueKind::Load { var }) if args.is_empty() => {
            fn_ir.values[a].origin_var.as_deref() == Some(var.as_str())
        }
        _ => false,
    }
}

pub(crate) fn canonical_value(fn_ir: &FnIR, mut vid: ValueId) -> ValueId {
    let mut seen = FxHashSet::default();
    loop {
        if !seen.insert(vid) {
            return vid;
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Phi { args } if !args.is_empty() => {
                let first = args[0].0;
                if args.iter().all(|(v, _)| *v == first) {
                    vid = first;
                    continue;
                }
                let mut unique_non_self = FxHashSet::default();
                for (v, _) in args {
                    if *v != vid {
                        unique_non_self.insert(*v);
                    }
                }
                if let Some(unique_non_self_vid) = (unique_non_self.len() == 1)
                    .then(|| unique_non_self.iter().next().copied())
                    .flatten()
                {
                    // loop-invariant self-phi: v = phi(seed, v) -> seed
                    vid = unique_non_self_vid;
                    continue;
                }
            }
            _ => {}
        }
        return vid;
    }
}

pub(crate) fn should_hoist_callmap_arg_expr(fn_ir: &FnIR, vid: ValueId) -> bool {
    !matches!(
        &fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. }
    )
}

pub(crate) fn next_callmap_tmp_var(fn_ir: &FnIR, prefix: &str) -> VarId {
    let mut idx = 0usize;
    loop {
        let candidate = format!(".tachyon_{}_{}", prefix, idx);
        if fn_ir.params.iter().all(|p| p != &candidate) && !has_any_var_binding(fn_ir, &candidate) {
            return candidate;
        }
        idx += 1;
    }
}

pub(crate) fn maybe_hoist_callmap_arg_expr(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    val: ValueId,
    arg_index: usize,
) -> ValueId {
    let val = resolve_materialized_value(fn_ir, val);
    if !should_hoist_callmap_arg_expr(fn_ir, val) {
        return val;
    }
    let var = next_callmap_tmp_var(fn_ir, &format!("callmap_arg{}", arg_index));
    let span = fn_ir.values[val].span;
    let facts = fn_ir.values[val].facts;
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: var.clone(),
        src: val,
        span: crate::utils::Span::dummy(),
    });
    fn_ir.add_value(ValueKind::Load { var: var.clone() }, span, facts, Some(var))
}

pub(crate) fn hoist_vector_expr_temp(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    val: ValueId,
    prefix: &str,
) -> ValueId {
    let val = resolve_materialized_value(fn_ir, val);
    if !should_hoist_callmap_arg_expr(fn_ir, val) {
        return val;
    }
    let var = next_callmap_tmp_var(fn_ir, prefix);
    let span = fn_ir.values[val].span;
    let facts = fn_ir.values[val].facts;
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: var.clone(),
        src: val,
        span: crate::utils::Span::dummy(),
    });
    fn_ir.add_value(ValueKind::Load { var: var.clone() }, span, facts, Some(var))
}

pub(crate) fn resolve_materialized_value(fn_ir: &mut FnIR, vid: ValueId) -> ValueId {
    let c = canonical_value(fn_ir, vid);
    if let ValueKind::Phi { args } = &fn_ir.values[c].kind
        && args.is_empty()
        && let Some(var) = fn_ir.values[c].origin_var.clone()
    {
        return fn_ir.add_value(
            ValueKind::Load { var: var.clone() },
            fn_ir.values[c].span,
            fn_ir.values[c].facts,
            Some(var),
        );
    }
    c
}

pub(crate) fn resolve_load_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
    let mut cur = canonical_value(fn_ir, vid);
    let mut seen = FxHashSet::default();
    while seen.insert(cur) {
        if let ValueKind::Load { var } = &fn_ir.values[cur].kind
            && let Some(src) = unique_assign_source(fn_ir, var)
        {
            cur = canonical_value(fn_ir, src);
            continue;
        }
        break;
    }
    cur
}

pub(crate) fn resolve_match_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
    let mut cur = canonical_value(fn_ir, vid);
    let mut seen = FxHashSet::default();
    while seen.insert(cur) {
        match &fn_ir.values[cur].kind {
            ValueKind::Load { var } => {
                let Some(src) = unique_assign_source(fn_ir, var) else {
                    break;
                };
                let next = canonical_value(fn_ir, src);
                if next == cur {
                    break;
                }
                cur = next;
            }
            ValueKind::Phi { args } => {
                let folded_non_self_args: Vec<ValueId> = args
                    .iter()
                    .map(|(arg, _)| canonical_value(fn_ir, *arg))
                    .filter(|arg| *arg != cur)
                    .collect();
                if let Some(first) = folded_non_self_args.first().copied()
                    && folded_non_self_args.iter().all(|arg| *arg == first)
                {
                    cur = first;
                    continue;
                }
                if let Some(first) = folded_non_self_args.first().copied()
                    && folded_non_self_args
                        .iter()
                        .all(|arg| fn_ir.values[*arg].kind == fn_ir.values[first].kind)
                {
                    cur = first;
                    continue;
                }
                let Some(var) = fn_ir.values[cur].origin_var.as_deref() else {
                    break;
                };
                let Some(src) = unique_assign_source(fn_ir, var) else {
                    break;
                };
                let next = canonical_value(fn_ir, src);
                if next == cur {
                    break;
                }
                cur = next;
            }
            _ => break,
        }
    }
    cur
}

pub(crate) fn unique_assign_source(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
    let mut src: Option<ValueId> = None;
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src: s, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let s = canonical_value(fn_ir, *s);
            match src {
                None => src = Some(s),
                Some(prev) if canonical_value(fn_ir, prev) == s => {}
                Some(_) => return None,
            }
        }
    }
    src
}

#[cfg(test)]
mod tests {
    use super::super::super::api::{rank_vector_plans, vector_plan_label};
    use super::super::super::planning::ExprMapEntry;
    use super::{
        BlockStore3DMatch, ResolvedCallInfo, VectorPlan, classify_store_3d_in_block,
        is_iv_equivalent, resolve_call_info,
    };
    use crate::mir::{BuiltinKind, FnIR, Instr, ValueKind};
    use crate::utils::Span;

    #[test]
    fn rank_prefers_specific_vector_plan_over_generic_map() {
        let mut plans = vec![
            VectorPlan::Map {
                dest: 1,
                src: 2,
                op: crate::syntax::ast::BinOp::Add,
                other: 3,
                shadow_vars: Vec::new(),
            },
            VectorPlan::CondMap {
                dest: 1,
                cond: 4,
                then_val: 5,
                else_val: 6,
                iv_phi: 7,
                start: 8,
                end: 9,
                whole_dest: true,
                shadow_vars: Vec::new(),
            },
        ];
        rank_vector_plans(&mut plans);
        assert_eq!(vector_plan_label(&plans[0]), "cond_map");
    }

    #[test]
    fn rank_prefers_multi_output_expr_map_over_single_expr_map() {
        let mut plans = vec![
            VectorPlan::ExprMap {
                dest: 1,
                expr: 2,
                iv_phi: 3,
                start: 4,
                end: 5,
                whole_dest: true,
                shadow_vars: Vec::new(),
            },
            VectorPlan::MultiExprMap {
                entries: vec![
                    ExprMapEntry {
                        dest: 1,
                        expr: 2,
                        whole_dest: true,
                        shadow_vars: Vec::new(),
                    },
                    ExprMapEntry {
                        dest: 6,
                        expr: 7,
                        whole_dest: true,
                        shadow_vars: Vec::new(),
                    },
                ],
                iv_phi: 3,
                start: 4,
                end: 5,
            },
        ];
        rank_vector_plans(&mut plans);
        assert_eq!(vector_plan_label(&plans[0]), "multi_expr_map");
    }

    #[test]
    fn resolve_call_info_canonicalizes_simple_builtin_alias() {
        let mut fn_ir = FnIR::new("alias".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let floor_load = fn_ir.add_value(
            ValueKind::Load {
                var: "floor".to_string(),
            },
            Span::default(),
            crate::mir::Facts::empty(),
            Some("floor".to_string()),
        );
        fn_ir.blocks[entry].instrs.push(crate::mir::Instr::Assign {
            dst: "floor_fn".to_string(),
            src: floor_load,
            span: Span::default(),
        });

        let alias_load = fn_ir.add_value(
            ValueKind::Load {
                var: "floor_fn".to_string(),
            },
            Span::default(),
            crate::mir::Facts::empty(),
            Some("floor_fn".to_string()),
        );
        let arg = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            crate::mir::Facts::empty(),
            None,
        );
        let call = fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_call_closure".to_string(),
                args: vec![alias_load, arg],
                names: vec![None, None],
            },
            Span::default(),
            crate::mir::Facts::empty(),
            None,
        );

        let resolved = resolve_call_info(&fn_ir, call).expect("expected call alias to resolve");
        assert_eq!(
            resolved,
            ResolvedCallInfo {
                callee: "floor".to_string(),
                builtin_kind: Some(BuiltinKind::Floor),
                args: vec![arg]
            }
        );
    }

    #[test]
    fn iv_equivalence_budget_bails_out_on_deep_floor_chain() {
        let mut fn_ir = FnIR::new("deep_iv".to_string(), vec!["x".to_string()]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let iv_phi = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            crate::mir::Facts::empty(),
            Some("x".to_string()),
        );

        let mut current = iv_phi;
        for _ in 0..300 {
            current = fn_ir.add_value(
                ValueKind::Call {
                    callee: "floor".to_string(),
                    args: vec![current],
                    names: vec![None],
                },
                Span::default(),
                crate::mir::Facts::empty(),
                None,
            );
        }

        assert!(
            !is_iv_equivalent(&fn_ir, current, iv_phi),
            "expected deep recursive proof to bail out conservatively"
        );
    }

    #[test]
    fn classify_store_3d_rejects_block_with_eval_side_effect() {
        let mut fn_ir = FnIR::new("store3d_eval".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let zero = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(0)),
            Span::default(),
            crate::mir::Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            crate::mir::Facts::empty(),
            None,
        );
        let base = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            crate::mir::Facts::empty(),
            Some("x".to_string()),
        );
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            crate::mir::Facts::empty(),
            None,
        );

        fn_ir.blocks[entry].instrs.push(Instr::Eval {
            val: impure,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::StoreIndex3D {
            base,
            i: one,
            j: one,
            k: one,
            val: zero,
            span: Span::default(),
        });

        assert!(matches!(
            classify_store_3d_in_block(&fn_ir, entry),
            BlockStore3DMatch::Invalid
        ));
    }

}
