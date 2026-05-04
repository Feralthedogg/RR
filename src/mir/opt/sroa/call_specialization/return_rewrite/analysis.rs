use super::*;
pub(crate) fn create_record_return_alias_inline_state(
    caller: &FnIR,
    record_use: &RecordReturnFieldUse,
) -> Option<RecordReturnAliasInlineState> {
    let (block, instr_index) = find_record_return_alias_assignment(caller, record_use)?;
    Some(RecordReturnAliasInlineState {
        block,
        next_insert_index: instr_index + 1,
        temp_by_field: FxHashMap::default(),
    })
}

pub(crate) fn scalarizable_single_record_return_field(
    callee: &FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    field: &str,
) -> Option<(FnIR, ValueId)> {
    scalarizable_single_record_return_field_inner(callee, all_fns, field, &mut FxHashSet::default())
}

pub(crate) fn scalarizable_single_record_return_field_inner(
    callee: &FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    field: &str,
    visiting: &mut FxHashSet<String>,
) -> Option<(FnIR, ValueId)> {
    if callee.requires_conservative_optimization() || contains_store_index_instr(callee) {
        return None;
    }
    if !visiting.insert(callee.name.clone()) {
        return None;
    }

    let mut scalarized = callee.clone();
    let field_maps = infer_rewrite_field_maps(&mut scalarized);
    let ret = match single_straight_line_return_value(&scalarized) {
        Some(ret) => ret,
        None => {
            visiting.remove(&callee.name);
            return None;
        }
    };
    let replacement = match scalarized_record_field_value(
        &mut scalarized,
        all_fns,
        &field_maps,
        ret,
        field,
        visiting,
    ) {
        Some(replacement) => replacement,
        None => {
            visiting.remove(&callee.name);
            return None;
        }
    };
    visiting.remove(&callee.name);
    Some((scalarized, replacement))
}

pub(crate) fn single_straight_line_return_value(fn_ir: &FnIR) -> Option<ValueId> {
    let mut out = None;
    for block in &fn_ir.blocks {
        match block.term {
            Terminator::Return(Some(value)) => {
                if out.replace(value).is_some() {
                    return None;
                }
            }
            Terminator::Goto(_) | Terminator::Unreachable => {}
            Terminator::Return(None) | Terminator::If { .. } => return None,
        }
    }
    out
}

pub(crate) fn scalarized_record_field_value(
    fn_ir: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    value: ValueId,
    field: &str,
    visiting: &mut FxHashSet<String>,
) -> Option<ValueId> {
    if let Some(replacement) = field_maps
        .get(&value)
        .and_then(|fields| fields.get(field))
        .copied()
    {
        return Some(replacement);
    }

    scalarized_field_projection_from_value(fn_ir, all_fns, value, field, visiting)
}

pub(crate) fn scalarized_field_projection_from_value(
    fn_ir: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    value: ValueId,
    field: &str,
    visiting: &mut FxHashSet<String>,
) -> Option<ValueId> {
    let source = fn_ir.values.get(value)?.kind.clone();
    match source {
        ValueKind::RecordLit { fields } => fields
            .into_iter()
            .find_map(|(name, value)| (name == field).then_some(value)),
        ValueKind::FieldSet {
            base,
            field: updated,
            value: updated_value,
        } => {
            if updated == field {
                Some(updated_value)
            } else {
                scalarized_field_projection_from_value(fn_ir, all_fns, base, field, visiting)
            }
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let target = all_fns.get(&callee)?;
            if !call_arg_names_match_callee_order(target, &names) {
                return None;
            }
            let mut pure_cache = FxHashMap::default();
            if !function_is_effect_free(&callee, all_fns, &mut pure_cache) {
                return None;
            }
            let (scalarized, field_value) =
                scalarizable_single_record_return_field_inner(target, all_fns, field, visiting)?;
            clone_scalarizable_callee_value(
                fn_ir,
                &scalarized,
                all_fns,
                &args,
                field_value,
                &mut FxHashMap::default(),
                visiting,
            )
        }
        _ => None,
    }
}
