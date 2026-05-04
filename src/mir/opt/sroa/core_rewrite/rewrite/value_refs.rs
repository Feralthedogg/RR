use super::*;
pub(crate) fn rewrite_value_kind_refs(
    kind: &mut ValueKind,
    replacements: &FxHashMap<ValueId, ValueId>,
) -> bool {
    let mut changed = false;
    match kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            changed |= rewrite_value_ref(lhs, replacements);
            changed |= rewrite_value_ref(rhs, replacements);
        }
        ValueKind::Unary { rhs, .. }
        | ValueKind::Len { base: rhs }
        | ValueKind::Indices { base: rhs }
        | ValueKind::FieldGet { base: rhs, .. } => {
            changed |= rewrite_value_ref(rhs, replacements);
        }
        ValueKind::Phi { args } => {
            for (arg, _) in args {
                changed |= rewrite_value_ref(arg, replacements);
            }
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
            for arg in args {
                changed |= rewrite_value_ref(arg, replacements);
            }
        }
        ValueKind::RecordLit { fields } => {
            for (_, value) in fields {
                changed |= rewrite_value_ref(value, replacements);
            }
        }
        ValueKind::FieldSet { base, value, .. } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(value, replacements);
        }
        ValueKind::Index1D { base, idx, .. } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(idx, replacements);
        }
        ValueKind::Index2D { base, r, c } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(r, replacements);
            changed |= rewrite_value_ref(c, replacements);
        }
        ValueKind::Index3D { base, i, j, k } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(i, replacements);
            changed |= rewrite_value_ref(j, replacements);
            changed |= rewrite_value_ref(k, replacements);
        }
        ValueKind::Range { start, end } => {
            changed |= rewrite_value_ref(start, replacements);
            changed |= rewrite_value_ref(end, replacements);
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => {}
    }
    changed
}

pub(crate) fn rewrite_instr_refs(
    instr: &mut Instr,
    replacements: &FxHashMap<ValueId, ValueId>,
) -> bool {
    let mut changed = false;
    match instr {
        Instr::Assign { src, .. } => {
            changed |= rewrite_value_ref(src, replacements);
        }
        Instr::Eval { val, .. } => {
            changed |= rewrite_value_ref(val, replacements);
        }
        Instr::StoreIndex1D { base, idx, val, .. } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(idx, replacements);
            changed |= rewrite_value_ref(val, replacements);
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(r, replacements);
            changed |= rewrite_value_ref(c, replacements);
            changed |= rewrite_value_ref(val, replacements);
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(i, replacements);
            changed |= rewrite_value_ref(j, replacements);
            changed |= rewrite_value_ref(k, replacements);
            changed |= rewrite_value_ref(val, replacements);
        }
        Instr::UnsafeRBlock { .. } => {}
    }
    changed
}

pub(crate) fn rewrite_terminator_refs(
    term: &mut Terminator,
    replacements: &FxHashMap<ValueId, ValueId>,
) -> bool {
    match term {
        Terminator::If { cond, .. } => rewrite_value_ref(cond, replacements),
        Terminator::Return(Some(value)) => rewrite_value_ref(value, replacements),
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => false,
    }
}

pub(crate) fn rewrite_value_ref(
    value: &mut ValueId,
    replacements: &FxHashMap<ValueId, ValueId>,
) -> bool {
    let replacement = resolve_replacement(*value, replacements);
    if replacement == *value {
        false
    } else {
        *value = replacement;
        true
    }
}

pub(crate) fn resolve_replacement(
    value: ValueId,
    replacements: &FxHashMap<ValueId, ValueId>,
) -> ValueId {
    let mut current = value;
    let mut seen = FxHashSet::default();
    while let Some(next) = replacements.get(&current).copied() {
        if !seen.insert(current) || next == current {
            break;
        }
        current = next;
    }
    current
}
