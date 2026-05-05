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
                Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. }
                | Instr::UnsafeRBlock { .. } => return true,
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
            Instr::StoreIndex2D { .. }
            | Instr::StoreIndex3D { .. }
            | Instr::UnsafeRBlock { .. } => {
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
            Instr::StoreIndex1D { .. }
            | Instr::StoreIndex2D { .. }
            | Instr::UnsafeRBlock { .. } => {
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
    let vid = resolve_load_alias_value(fn_ir, vid);
    match &fn_ir.values[vid].kind {
        ValueKind::Index1D { base: b, idx, .. } => {
            if !same_base_value(fn_ir, *b, base) {
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
    let vid = resolve_load_alias_value(fn_ir, vid);
    match &fn_ir.values[vid].kind {
        ValueKind::Index3D { base: b, i, j, k } => {
            if !same_base_value(fn_ir, *b, base) {
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
    let idx = resolve_load_alias_value(fn_ir, idx);
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
    let mut stack = vec![root];
    let mut seen = FxHashSet::default();
    while let Some(value) = stack.pop() {
        if !seen.insert(value) {
            continue;
        }
        let Some(row) = fn_ir.values.get(value) else {
            continue;
        };
        match &row.kind {
            ValueKind::Index1D { base: b, .. }
            | ValueKind::Index2D { base: b, .. }
            | ValueKind::Index3D { base: b, .. }
                if same_base_value(fn_ir, *b, base) =>
            {
                return true;
            }
            _ => {}
        }
        for dep in value_dependencies(&row.kind) {
            stack.push(dep);
        }
    }
    false
}
