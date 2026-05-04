use super::*;
pub(crate) fn remove_dead_scalarized_aggregate_assigns(
    fn_ir: &mut FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
) -> bool {
    let unique_assignments = unique_var_assignments(fn_ir);
    let required_loaded_vars = scalarized_loaded_vars_required_by_non_alias_uses(fn_ir, field_maps);
    let mut changed = false;

    for value in &mut fn_ir.values {
        let ValueKind::Load { var } = &value.kind else {
            continue;
        };
        let Some(src) = unique_assignments.get(var) else {
            continue;
        };
        if field_maps.contains_key(src) && !required_loaded_vars.contains(var) {
            value.kind = ValueKind::Const(Lit::Null);
            value.origin_var = None;
            changed = true;
        }
    }

    for block in &mut fn_ir.blocks {
        let old_len = block.instrs.len();
        block.instrs.retain(|instr| {
            let Instr::Assign { dst, src, .. } = instr else {
                return true;
            };
            !field_maps.contains_key(src) || required_loaded_vars.contains(dst)
        });
        changed |= block.instrs.len() != old_len;
    }
    changed
}

pub(crate) fn scalarized_loaded_vars_required_by_non_alias_uses(
    fn_ir: &FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
) -> FxHashSet<String> {
    let mut roots = Vec::new();
    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { val, .. } => roots.push(*val),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    roots.extend([*base, *idx, *val]);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    roots.extend([*base, *r, *c, *val]);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    roots.extend([*base, *i, *j, *k, *val]);
                }
                Instr::UnsafeRBlock { .. } => {}
            }
        }
        match &block.term {
            Terminator::If { cond, .. } => roots.push(*cond),
            Terminator::Return(Some(value)) => roots.push(*value),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    let unique_assignments = unique_var_assignments(fn_ir);
    let mut required = loaded_vars_in_values(fn_ir, roots);
    let mut changed = true;
    while changed {
        changed = false;
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                let Instr::Assign { dst, src, .. } = instr else {
                    continue;
                };
                if !required.contains(dst) {
                    continue;
                }
                for var in loaded_vars_in_values(fn_ir, [*src]) {
                    if let Some(src) = unique_assignments.get(&var)
                        && field_maps.contains_key(src)
                        && required.insert(var)
                    {
                        changed = true;
                    }
                }
            }
        }
    }

    required
}

pub(crate) fn loaded_vars_in_values(
    fn_ir: &FnIR,
    roots: impl IntoIterator<Item = ValueId>,
) -> FxHashSet<String> {
    let mut vars = FxHashSet::default();
    let mut seen = FxHashSet::default();
    let mut stack: Vec<_> = roots.into_iter().collect();

    while let Some(value) = stack.pop() {
        if !seen.insert(value) {
            continue;
        }
        if let ValueKind::Load { var } = &fn_ir.values[value].kind {
            vars.insert(var.clone());
        }
        stack.extend(value_dependencies(&fn_ir.values[value].kind));
    }

    vars
}

pub(crate) fn collect_non_alias_live_value_ids(fn_ir: &FnIR) -> FxHashSet<ValueId> {
    let mut live = FxHashSet::default();
    let mut stack = Vec::new();

    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { val, .. } => stack.push(*val),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    stack.extend([*base, *idx, *val]);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    stack.extend([*base, *r, *c, *val]);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    stack.extend([*base, *i, *j, *k, *val]);
                }
                Instr::UnsafeRBlock { .. } => {}
            }
        }
        match &block.term {
            Terminator::If { cond, .. } => stack.push(*cond),
            Terminator::Return(Some(value)) => stack.push(*value),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    while let Some(value) = stack.pop() {
        if !live.insert(value) {
            continue;
        }
        stack.extend(value_dependencies(&fn_ir.values[value].kind));
    }

    live
}

pub(crate) fn collect_live_value_ids(fn_ir: &FnIR) -> FxHashSet<ValueId> {
    let mut live = FxHashSet::default();
    let mut stack = Vec::new();

    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Assign { src, .. } => stack.push(*src),
                Instr::Eval { val, .. } => stack.push(*val),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    stack.extend([*base, *idx, *val]);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    stack.extend([*base, *r, *c, *val]);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    stack.extend([*base, *i, *j, *k, *val]);
                }
                Instr::UnsafeRBlock { .. } => {}
            }
        }
        match &block.term {
            Terminator::If { cond, .. } => stack.push(*cond),
            Terminator::Return(Some(value)) => stack.push(*value),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    while let Some(value) = stack.pop() {
        if !live.insert(value) {
            continue;
        }
        stack.extend(value_dependencies(&fn_ir.values[value].kind));
    }

    live
}
