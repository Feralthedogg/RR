use super::*;
pub(crate) fn store_index_operands(instr: &Instr) -> Vec<(StoreIndexOperand, ValueId)> {
    match instr {
        Instr::StoreIndex1D { base, idx, val, .. } => vec![
            (StoreIndexOperand::Base, *base),
            (StoreIndexOperand::Index, *idx),
            (StoreIndexOperand::Value, *val),
        ],
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => vec![
            (StoreIndexOperand::Base, *base),
            (StoreIndexOperand::Row, *r),
            (StoreIndexOperand::Column, *c),
            (StoreIndexOperand::Value, *val),
        ],
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => vec![
            (StoreIndexOperand::Base, *base),
            (StoreIndexOperand::Plane, *i),
            (StoreIndexOperand::Row, *j),
            (StoreIndexOperand::Column, *k),
            (StoreIndexOperand::Value, *val),
        ],
        Instr::Assign { .. } | Instr::Eval { .. } | Instr::UnsafeRBlock { .. } => Vec::new(),
    }
}

pub(crate) fn rewrite_store_index_operand(
    instr: &mut Instr,
    operand: StoreIndexOperand,
    old: ValueId,
    replacement: ValueId,
) -> bool {
    let target = match (instr, operand) {
        (Instr::StoreIndex1D { base, .. }, StoreIndexOperand::Base)
        | (Instr::StoreIndex2D { base, .. }, StoreIndexOperand::Base)
        | (Instr::StoreIndex3D { base, .. }, StoreIndexOperand::Base) => base,
        (Instr::StoreIndex1D { idx, .. }, StoreIndexOperand::Index) => idx,
        (Instr::StoreIndex2D { r, .. }, StoreIndexOperand::Row) => r,
        (Instr::StoreIndex2D { c, .. }, StoreIndexOperand::Column) => c,
        (Instr::StoreIndex3D { i, .. }, StoreIndexOperand::Plane) => i,
        (Instr::StoreIndex3D { j, .. }, StoreIndexOperand::Row) => j,
        (Instr::StoreIndex3D { k, .. }, StoreIndexOperand::Column) => k,
        (Instr::StoreIndex1D { val, .. }, StoreIndexOperand::Value)
        | (Instr::StoreIndex2D { val, .. }, StoreIndexOperand::Value)
        | (Instr::StoreIndex3D { val, .. }, StoreIndexOperand::Value) => val,
        _ => return false,
    };
    if *target != old {
        return false;
    }
    *target = replacement;
    true
}

pub(crate) fn rewrite_concrete_base_consumer(
    kind: &mut ValueKind,
    old_base: ValueId,
    replacement: ValueId,
) -> bool {
    let base = match kind {
        ValueKind::Len { base }
        | ValueKind::Indices { base }
        | ValueKind::Index1D { base, .. }
        | ValueKind::Index2D { base, .. }
        | ValueKind::Index3D { base, .. } => base,
        _ => return false,
    };
    if *base != old_base {
        return false;
    }
    *base = replacement;
    true
}

pub(crate) fn should_rematerialize_boundary_value(
    fn_ir: &FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    value: ValueId,
) -> bool {
    field_maps.contains_key(&value)
        && !matches!(fn_ir.values[value].kind, ValueKind::RecordLit { .. })
}

pub(crate) fn rematerialize_value(
    fn_ir: &mut FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    shapes: &FxHashMap<ValueId, Vec<String>>,
    materialized: &mut FxHashMap<ValueId, ValueId>,
    value: ValueId,
) -> Option<ValueId> {
    if let Some(existing) = materialized.get(&value).copied() {
        return Some(existing);
    }

    if matches!(fn_ir.values[value].kind, ValueKind::RecordLit { .. }) {
        return Some(value);
    }

    let field_map = field_maps.get(&value)?;
    let shape = materialization_shape(fn_ir, field_maps, shapes, value)?;
    let mut fields = Vec::with_capacity(shape.len());
    for field in shape {
        let mut field_value = *field_map.get(&field)?;
        if should_rematerialize_boundary_value(fn_ir, field_maps, field_value) {
            field_value =
                rematerialize_value(fn_ir, field_maps, shapes, materialized, field_value)?;
        }
        fields.push((field, field_value));
    }

    let rematerialized = fn_ir.add_value(
        ValueKind::RecordLit { fields },
        fn_ir.values[value].span,
        Facts::empty(),
        None,
    );
    materialized.insert(value, rematerialized);
    Some(rematerialized)
}

pub(crate) fn materialization_shape(
    fn_ir: &FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    shapes: &FxHashMap<ValueId, Vec<String>>,
    value: ValueId,
) -> Option<Vec<String>> {
    if let Some(shape) = shapes.get(&value) {
        return Some(shape.clone());
    }
    if let ValueKind::RecordLit { fields } = &fn_ir.values[value].kind {
        return record_shape(fields).ok();
    }
    let mut fields: Vec<_> = field_maps.get(&value)?.keys().cloned().collect();
    fields.sort();
    Some(fields)
}
