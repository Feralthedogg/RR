use super::*;
pub(crate) fn rewrite_record_return_field_use_to_value(
    caller: &mut FnIR,
    field_get: ValueId,
    replacement: ValueId,
) -> bool {
    if caller.values.get(field_get).is_none() || caller.values.get(replacement).is_none() {
        return false;
    }
    let mut replacements = FxHashMap::default();
    replacements.insert(field_get, replacement);
    let changed = apply_value_replacements(caller, &replacements);
    if changed && let Some(value) = caller.values.get_mut(field_get) {
        value.kind = ValueKind::Const(Lit::Null);
        value.origin_var = None;
    }
    changed
}

pub(crate) fn rewrite_record_return_field_use_to_temp_load(
    caller: &mut FnIR,
    field_get: ValueId,
    temp_var: &str,
) -> bool {
    let Some(value) = caller.values.get_mut(field_get) else {
        return false;
    };
    value.kind = ValueKind::Load {
        var: temp_var.to_string(),
    };
    value.origin_var = Some(temp_var.to_string());
    true
}

pub(crate) fn rewrite_record_return_alias_field_use_with_shared_temp(
    caller: &mut FnIR,
    record_use: &RecordReturnFieldUse,
    specialized_name: &str,
    alias_temp_vars: &mut FxHashMap<RecordReturnAliasTempKey, String>,
) -> bool {
    let Some(alias_var) = record_use.alias_var.as_ref() else {
        return rewrite_record_return_field_use(caller, record_use, specialized_name);
    };
    let key = RecordReturnAliasTempKey {
        alias_var: alias_var.clone(),
        field: record_use.field.clone(),
        callee: record_use.callee.clone(),
        args: record_use.args.clone(),
        names: record_use.names.clone(),
    };
    let temp_var = if let Some(temp_var) = alias_temp_vars.get(&key).cloned() {
        temp_var
    } else {
        let Some(temp_var) =
            insert_scalarized_return_alias_temp(caller, record_use, specialized_name)
        else {
            return rewrite_record_return_field_use(caller, record_use, specialized_name);
        };
        alias_temp_vars.insert(key, temp_var.clone());
        temp_var
    };

    rewrite_record_return_field_use_to_temp_load(caller, record_use.field_get, &temp_var)
}
