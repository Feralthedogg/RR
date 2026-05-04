use super::*;
#[derive(Debug, Clone)]
pub(crate) struct RecordFieldSnapshotPlan {
    pub(crate) block: BlockId,
    pub(crate) instr_index: usize,
    pub(crate) record: ValueId,
    pub(crate) inserted_instrs: Vec<Instr>,
    pub(crate) field_replacements: Vec<(usize, ValueId)>,
}

pub(crate) fn snapshot_record_alias_fields(fn_ir: &mut FnIR) -> bool {
    let unique_assignments = unique_var_assignments(fn_ir);
    if unique_assignments.is_empty() {
        return false;
    }

    let existing_snapshot_vars = sroa_snapshot_vars(fn_ir);
    let mut candidates = Vec::new();
    for block in &fn_ir.blocks {
        for (instr_index, instr) in block.instrs.iter().enumerate() {
            let Instr::Assign { dst, src, .. } = instr else {
                continue;
            };
            if unique_assignments.get(dst).copied() != Some(*src) {
                continue;
            }
            let Some(snapshot_fields) =
                collect_record_alias_field_snapshots(fn_ir, *src, dst, &existing_snapshot_vars)
            else {
                continue;
            };
            if !snapshot_fields.is_empty() {
                candidates.push((block.id, instr_index, *src, dst.clone(), snapshot_fields));
            }
        }
    }
    if candidates.is_empty() {
        return false;
    }

    let mut used_vars = used_var_names(fn_ir);
    let mut plans = Vec::new();
    for (block, instr_index, record, alias, snapshot_fields) in candidates {
        let mut inserted_instrs = Vec::with_capacity(snapshot_fields.len());
        let mut field_replacements = Vec::with_capacity(snapshot_fields.len());
        for (field_index, field_name, field_value) in snapshot_fields {
            let temp_var = unique_sroa_snapshot_temp_var(&alias, &field_name, &mut used_vars);
            let span = fn_ir.values[field_value].span;
            let temp_load = fn_ir.add_value(
                ValueKind::Load {
                    var: temp_var.clone(),
                },
                span,
                Facts::empty(),
                Some(temp_var.clone()),
            );
            inserted_instrs.push(Instr::Assign {
                dst: temp_var,
                src: field_value,
                span,
            });
            field_replacements.push((field_index, temp_load));
        }
        plans.push(RecordFieldSnapshotPlan {
            block,
            instr_index,
            record,
            inserted_instrs,
            field_replacements,
        });
    }

    plans.sort_by(|left, right| {
        right
            .block
            .cmp(&left.block)
            .then_with(|| right.instr_index.cmp(&left.instr_index))
    });

    let mut changed = false;
    for plan in plans {
        if let ValueKind::RecordLit { fields } = &mut fn_ir.values[plan.record].kind {
            for (field_index, replacement) in plan.field_replacements {
                if let Some((_, field_value)) = fields.get_mut(field_index) {
                    *field_value = replacement;
                    changed = true;
                }
            }
        }
        if !plan.inserted_instrs.is_empty() {
            let insert_at = plan.instr_index.min(fn_ir.blocks[plan.block].instrs.len());
            fn_ir.blocks[plan.block]
                .instrs
                .splice(insert_at..insert_at, plan.inserted_instrs);
            changed = true;
        }
    }

    changed
}

pub(crate) fn collect_record_alias_field_snapshots(
    fn_ir: &FnIR,
    record: ValueId,
    alias: &str,
    snapshot_vars: &FxHashSet<String>,
) -> Option<Vec<(usize, String, ValueId)>> {
    let ValueKind::RecordLit { fields } = &fn_ir.values[record].kind else {
        return None;
    };
    if record_shape(fields).is_err() {
        return None;
    }

    let mut snapshots = Vec::new();
    for (field_index, (field_name, field_value)) in fields.iter().enumerate() {
        if value_loads_var(fn_ir, *field_value, alias) {
            return None;
        }
        if sroa_value_is_scalarizable_field(
            fn_ir,
            *field_value,
            snapshot_vars,
            &mut FxHashSet::default(),
        ) {
            continue;
        }
        if !sroa_value_is_snapshot_safe(fn_ir, *field_value, &mut FxHashSet::default()) {
            return None;
        }
        snapshots.push((field_index, field_name.clone(), *field_value));
    }

    Some(snapshots)
}

pub(crate) fn sroa_snapshot_vars(fn_ir: &FnIR) -> FxHashSet<String> {
    let unique_assignments = unique_var_assignments(fn_ir);
    unique_assignments
        .keys()
        .filter(|var| var.contains("__rr_sroa_snap_"))
        .cloned()
        .collect()
}

pub(crate) fn unique_sroa_snapshot_temp_var(
    alias: &str,
    field: &str,
    used_vars: &mut FxHashSet<String>,
) -> String {
    let alias = sanitize_symbol_segment(alias);
    let field = sanitize_symbol_segment(field);
    let seed = format!("{alias}__rr_sroa_snap_{field}");
    if used_vars.insert(seed.clone()) {
        return seed;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{seed}_{suffix}");
        if used_vars.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

pub(crate) fn value_loads_var(fn_ir: &FnIR, value: ValueId, target: &str) -> bool {
    loaded_vars_in_values(fn_ir, [value]).contains(target)
}

pub(crate) fn scalarizable_record_field_map(
    fn_ir: &FnIR,
    fields: &[(String, ValueId)],
    snapshot_vars: &FxHashSet<String>,
) -> Option<SroaFieldMap> {
    if record_shape(fields).is_err() {
        return None;
    }
    let mut field_map = FxHashMap::default();
    for (field, value) in fields {
        if !sroa_value_is_scalarizable_field(
            fn_ir,
            *value,
            snapshot_vars,
            &mut FxHashSet::default(),
        ) {
            return None;
        }
        field_map.insert(field.clone(), *value);
    }
    Some(field_map)
}

pub(crate) fn sroa_value_is_pure(
    fn_ir: &FnIR,
    value: ValueId,
    visiting: &mut FxHashSet<ValueId>,
) -> bool {
    sroa_value_is_scalarizable_field(fn_ir, value, &FxHashSet::default(), visiting)
}

pub(crate) fn sroa_value_is_scalarizable_field(
    fn_ir: &FnIR,
    value: ValueId,
    snapshot_vars: &FxHashSet<String>,
    visiting: &mut FxHashSet<ValueId>,
) -> bool {
    if !visiting.insert(value) {
        return true;
    }

    let pure = match &fn_ir.values[value].kind {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => true,
        ValueKind::Load { var } => snapshot_vars.contains(var),
        ValueKind::Binary { lhs, rhs, .. } => {
            sroa_value_is_scalarizable_field(fn_ir, *lhs, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *rhs, snapshot_vars, visiting)
        }
        ValueKind::Unary { rhs, .. }
        | ValueKind::Len { base: rhs }
        | ValueKind::Indices { base: rhs }
        | ValueKind::FieldGet { base: rhs, .. } => {
            sroa_value_is_scalarizable_field(fn_ir, *rhs, snapshot_vars, visiting)
        }
        ValueKind::Range { start, end } => {
            sroa_value_is_scalarizable_field(fn_ir, *start, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *end, snapshot_vars, visiting)
        }
        ValueKind::Call { callee, args, .. } => {
            effects::call_is_pure(callee)
                && args.iter().all(|arg| {
                    sroa_value_is_scalarizable_field(fn_ir, *arg, snapshot_vars, visiting)
                })
        }
        ValueKind::RecordLit { fields } => fields.iter().all(|(_, value)| {
            sroa_value_is_scalarizable_field(fn_ir, *value, snapshot_vars, visiting)
        }),
        ValueKind::FieldSet { base, value, .. } => {
            sroa_value_is_scalarizable_field(fn_ir, *base, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *value, snapshot_vars, visiting)
        }
        ValueKind::Intrinsic { args, .. } => args
            .iter()
            .all(|arg| sroa_value_is_scalarizable_field(fn_ir, *arg, snapshot_vars, visiting)),
        ValueKind::Index1D { base, idx, .. } => {
            sroa_value_is_scalarizable_field(fn_ir, *base, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *idx, snapshot_vars, visiting)
        }
        ValueKind::Index2D { base, r, c } => {
            sroa_value_is_scalarizable_field(fn_ir, *base, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *r, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *c, snapshot_vars, visiting)
        }
        ValueKind::Index3D { base, i, j, k } => {
            sroa_value_is_scalarizable_field(fn_ir, *base, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *i, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *j, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *k, snapshot_vars, visiting)
        }
        ValueKind::Phi { args } => args
            .iter()
            .all(|(arg, _)| sroa_value_is_scalarizable_field(fn_ir, *arg, snapshot_vars, visiting)),
    };

    visiting.remove(&value);
    pure
}

pub(crate) fn sroa_value_is_snapshot_safe(
    fn_ir: &FnIR,
    value: ValueId,
    visiting: &mut FxHashSet<ValueId>,
) -> bool {
    if !visiting.insert(value) {
        return true;
    }

    let safe = match &fn_ir.values[value].kind {
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => true,
        ValueKind::Binary { lhs, rhs, .. } => {
            sroa_value_is_snapshot_safe(fn_ir, *lhs, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *rhs, visiting)
        }
        ValueKind::Unary { rhs, .. }
        | ValueKind::Len { base: rhs }
        | ValueKind::Indices { base: rhs }
        | ValueKind::FieldGet { base: rhs, .. } => {
            sroa_value_is_snapshot_safe(fn_ir, *rhs, visiting)
        }
        ValueKind::Range { start, end } => {
            sroa_value_is_snapshot_safe(fn_ir, *start, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *end, visiting)
        }
        ValueKind::Call { callee, args, .. } => {
            effects::call_is_pure(callee)
                && args
                    .iter()
                    .all(|arg| sroa_value_is_snapshot_safe(fn_ir, *arg, visiting))
        }
        ValueKind::RecordLit { fields } => fields
            .iter()
            .all(|(_, value)| sroa_value_is_snapshot_safe(fn_ir, *value, visiting)),
        ValueKind::FieldSet { base, value, .. } => {
            sroa_value_is_snapshot_safe(fn_ir, *base, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *value, visiting)
        }
        ValueKind::Intrinsic { args, .. } => args
            .iter()
            .all(|arg| sroa_value_is_snapshot_safe(fn_ir, *arg, visiting)),
        ValueKind::Index1D { base, idx, .. } => {
            sroa_value_is_snapshot_safe(fn_ir, *base, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *idx, visiting)
        }
        ValueKind::Index2D { base, r, c } => {
            sroa_value_is_snapshot_safe(fn_ir, *base, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *r, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *c, visiting)
        }
        ValueKind::Index3D { base, i, j, k } => {
            sroa_value_is_snapshot_safe(fn_ir, *base, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *i, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *j, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *k, visiting)
        }
        ValueKind::Phi { .. } => false,
    };

    visiting.remove(&value);
    safe
}
