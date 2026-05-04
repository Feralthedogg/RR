use super::*;

pub(in crate::mir::opt::v_opt) fn value_depends_on(
    fn_ir: &FnIR,
    root: ValueId,
    target: ValueId,
    visiting: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    let target = canonical_value(fn_ir, target);
    if root == target {
        return true;
    }
    if !visiting.insert(root) {
        return false;
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::RecordLit { fields } => fields
            .iter()
            .any(|(_, value)| value_depends_on(fn_ir, *value, target, visiting)),
        ValueKind::FieldGet { base, .. } => value_depends_on(fn_ir, *base, target, visiting),
        ValueKind::FieldSet { base, value, .. } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *value, target, visiting)
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            value_depends_on(fn_ir, *lhs, target, visiting)
                || value_depends_on(fn_ir, *rhs, target, visiting)
        }
        ValueKind::Unary { rhs, .. } => value_depends_on(fn_ir, *rhs, target, visiting),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| value_depends_on(fn_ir, *arg, target, visiting)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(arg, _)| value_depends_on(fn_ir, *arg, target, visiting)),
        ValueKind::Index1D { base, idx, .. } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *idx, target, visiting)
        }
        ValueKind::Index2D { base, r, c } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *r, target, visiting)
                || value_depends_on(fn_ir, *c, target, visiting)
        }
        ValueKind::Index3D { base, i, j, k } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *i, target, visiting)
                || value_depends_on(fn_ir, *j, target, visiting)
                || value_depends_on(fn_ir, *k, target, visiting)
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            value_depends_on(fn_ir, *base, target, visiting)
        }
        ValueKind::Range { start, end } => {
            value_depends_on(fn_ir, *start, target, visiting)
                || value_depends_on(fn_ir, *end, target, visiting)
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
    };
    visiting.remove(&root);
    out
}

pub(in crate::mir::opt::v_opt) fn block_instr_uses_value(
    fn_ir: &FnIR,
    ins: &Instr,
    vid: ValueId,
) -> bool {
    let uses = |value: ValueId| value_depends_on(fn_ir, value, vid, &mut FxHashSet::default());
    match ins {
        Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => uses(*src),
        Instr::StoreIndex1D { base, idx, val, .. } => uses(*base) || uses(*idx) || uses(*val),
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => uses(*base) || uses(*r) || uses(*c) || uses(*val),
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => uses(*base) || uses(*i) || uses(*j) || uses(*k) || uses(*val),
        Instr::UnsafeRBlock { .. } => false,
    }
}

pub(in crate::mir::opt::v_opt) fn block_term_uses_value(
    fn_ir: &FnIR,
    term: &Terminator,
    vid: ValueId,
) -> bool {
    let uses = |value: ValueId| value_depends_on(fn_ir, value, vid, &mut FxHashSet::default());
    match term {
        Terminator::If { cond, .. } => uses(*cond),
        Terminator::Return(Some(ret)) => uses(*ret),
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => false,
    }
}

pub(in crate::mir::opt::v_opt) fn last_effective_assign_before_value_use_in_block(
    fn_ir: &FnIR,
    bid: BlockId,
    var: &str,
    vid: ValueId,
) -> Option<ValueId> {
    let vid = canonical_value(fn_ir, vid);
    let block = fn_ir.blocks.get(bid)?;
    for (idx, ins) in block.instrs.iter().enumerate() {
        if !block_instr_uses_value(fn_ir, ins, vid) {
            continue;
        }
        for prev in block.instrs[..idx].iter().rev() {
            let Instr::Assign { dst, src, .. } = prev else {
                continue;
            };
            if dst != var {
                continue;
            }
            if is_passthrough_load_of_var(fn_ir, *src, var) {
                continue;
            }
            return Some(canonical_value(fn_ir, *src));
        }
        return None;
    }
    if !block_term_uses_value(fn_ir, &block.term, vid) {
        return None;
    }
    for prev in block.instrs.iter().rev() {
        let Instr::Assign { dst, src, .. } = prev else {
            continue;
        };
        if dst != var {
            continue;
        }
        if is_passthrough_load_of_var(fn_ir, *src, var) {
            continue;
        }
        return Some(canonical_value(fn_ir, *src));
    }
    None
}
