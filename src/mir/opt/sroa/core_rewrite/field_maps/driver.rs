use super::*;
pub(crate) fn optimize_once(fn_ir: &mut FnIR) -> bool {
    let snapshot_changed = snapshot_record_alias_fields(fn_ir);
    let field_maps = infer_rewrite_field_maps(fn_ir);
    if field_maps.is_empty() {
        return snapshot_changed;
    }

    let mut replacements = FxHashMap::default();
    for value in &fn_ir.values {
        let ValueKind::FieldGet { base, field } = &value.kind else {
            continue;
        };
        let Some(replacement) = field_maps.get(base).and_then(|fields| fields.get(field)) else {
            continue;
        };
        if *replacement != value.id {
            replacements.insert(value.id, *replacement);
        }
    }

    let mut changed = snapshot_changed;
    if !replacements.is_empty() {
        changed |= apply_value_replacements(fn_ir, &replacements);
    }

    changed |= rematerialize_aggregate_boundaries(fn_ir, &field_maps);
    changed |= remove_dead_scalarized_aggregate_assigns(fn_ir, &field_maps);
    changed
}

pub(crate) fn contains_store_index_instr(fn_ir: &FnIR) -> bool {
    fn_ir.blocks.iter().any(|block| {
        block.instrs.iter().any(|instr| {
            matches!(
                instr,
                Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
            )
        })
    })
}

pub(crate) fn infer_rewrite_field_maps(fn_ir: &mut FnIR) -> FxHashMap<ValueId, SroaFieldMap> {
    let snapshot_vars = sroa_snapshot_vars(fn_ir);
    let mut field_maps = FxHashMap::default();
    for value in &fn_ir.values {
        if let ValueKind::RecordLit { fields } = &value.kind
            && let Some(field_map) = scalarizable_record_field_map(fn_ir, fields, &snapshot_vars)
        {
            field_maps.insert(value.id, field_map);
        }
    }

    propagate_rewrite_field_maps(fn_ir, &mut field_maps, &snapshot_vars);
    split_demanded_record_phis(fn_ir, &mut field_maps);
    propagate_rewrite_field_maps(fn_ir, &mut field_maps, &snapshot_vars);

    field_maps
}

pub(crate) fn propagate_rewrite_field_maps(
    fn_ir: &FnIR,
    field_maps: &mut FxHashMap<ValueId, SroaFieldMap>,
    snapshot_vars: &FxHashSet<String>,
) {
    let unique_assignments = unique_var_assignments(fn_ir);
    let mut var_maps: FxHashMap<String, SroaFieldMap> = FxHashMap::default();
    let mut changed = true;
    while changed {
        changed = false;

        for (var, src) in &unique_assignments {
            let Some(field_map) = field_maps.get(src) else {
                continue;
            };
            if var_maps.get(var) != Some(field_map) {
                var_maps.insert(var.clone(), field_map.clone());
                changed = true;
            }
        }

        for value in &fn_ir.values {
            let inferred = match &value.kind {
                ValueKind::Load { var } => var_maps.get(var).cloned(),
                ValueKind::FieldSet { base, field, value } => {
                    if let Some(base_map) = field_maps.get(base) {
                        if !base_map.contains_key(field)
                            || !sroa_value_is_scalarizable_field(
                                fn_ir,
                                *value,
                                snapshot_vars,
                                &mut FxHashSet::default(),
                            )
                        {
                            None
                        } else {
                            let mut updated_map = base_map.clone();
                            updated_map.insert(field.clone(), *value);
                            Some(updated_map)
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let Some(field_map) = inferred else {
                continue;
            };
            if field_maps.get(&value.id) != Some(&field_map) {
                field_maps.insert(value.id, field_map);
                changed = true;
            }
        }
    }
}
