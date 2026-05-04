use super::*;
pub(crate) fn build_record_return_field_specialized_callee(
    callee: &FnIR,
    new_name: &str,
    field: &str,
) -> Option<FnIR> {
    if callee.requires_conservative_optimization() || contains_store_index_instr(callee) {
        return None;
    }

    let mut specialized = callee.clone();
    specialized.name = new_name.to_string();
    specialized.user_name = None;
    let field_maps = infer_rewrite_field_maps(&mut specialized);
    let mut rewrites = Vec::new();
    for block in &specialized.blocks {
        match block.term {
            Terminator::Return(Some(ret)) => {
                let replacement = field_maps.get(&ret)?.get(field).copied()?;
                rewrites.push((block.id, replacement));
            }
            Terminator::Return(None) => return None,
            Terminator::Goto(_) | Terminator::If { .. } | Terminator::Unreachable => {}
        }
    }
    if rewrites.is_empty() {
        return None;
    }

    for (block, replacement) in rewrites {
        specialized.blocks[block].term = Terminator::Return(Some(replacement));
    }
    let _ = optimize(&mut specialized);
    Some(specialized)
}

pub(crate) fn rewrite_record_return_field_use(
    caller: &mut FnIR,
    record_use: &RecordReturnFieldUse,
    specialized_name: &str,
) -> bool {
    let Some(value) = caller.values.get_mut(record_use.field_get) else {
        return false;
    };
    value.kind = ValueKind::Call {
        callee: specialized_name.to_string(),
        args: record_use.args.clone(),
        names: record_use.names.clone(),
    };
    caller.set_call_semantics(record_use.field_get, CallSemantics::UserDefined);
    true
}

pub(crate) fn rewrite_direct_record_return_field_use_with_inlined_value(
    caller: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    record_use: &RecordReturnFieldUse,
    direct_inline_states: &mut FxHashMap<RecordReturnDirectCallKey, RecordReturnDirectInlineState>,
) -> bool {
    let Some(base_call) = record_use.base_call else {
        return false;
    };
    if caller.values.get(record_use.field_get).is_none() {
        return false;
    }

    let key = RecordReturnDirectCallKey {
        base_call,
        callee: record_use.callee.clone(),
        args: record_use.args.clone(),
        names: record_use.names.clone(),
    };
    let mut state = direct_inline_states.remove(&key).unwrap_or_default();

    let replacement =
        if let Some(replacement) = state.replacement_by_field.get(&record_use.field).copied() {
            replacement
        } else {
            let Some(callee) = all_fns.get(&record_use.callee) else {
                direct_inline_states.insert(key, state);
                return false;
            };
            let Some((scalarized, field_value)) =
                scalarizable_single_record_return_field(callee, all_fns, &record_use.field)
            else {
                direct_inline_states.insert(key, state);
                return false;
            };
            let mut value_map = FxHashMap::default();
            let Some(cloned_value) = clone_scalarizable_callee_value(
                caller,
                &scalarized,
                all_fns,
                &record_use.args,
                field_value,
                &mut value_map,
                &mut FxHashSet::default(),
            ) else {
                direct_inline_states.insert(key, state);
                return false;
            };
            state
                .replacement_by_field
                .insert(record_use.field.clone(), cloned_value);
            cloned_value
        };

    let rewritten =
        rewrite_record_return_field_use_to_value(caller, record_use.field_get, replacement);
    direct_inline_states.insert(key, state);
    rewritten
}

pub(crate) fn rewrite_record_return_alias_field_use_with_inlined_temp(
    caller: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    record_use: &RecordReturnFieldUse,
    alias_inline_states: &mut FxHashMap<RecordReturnAliasCallKey, RecordReturnAliasInlineState>,
) -> bool {
    let Some(alias_var) = record_use.alias_var.as_ref() else {
        return false;
    };
    if caller.values.get(record_use.field_get).is_none() {
        return false;
    }
    let key = RecordReturnAliasCallKey {
        alias_var: alias_var.clone(),
        callee: record_use.callee.clone(),
        args: record_use.args.clone(),
        names: record_use.names.clone(),
    };
    let Some(mut state) = alias_inline_states
        .remove(&key)
        .or_else(|| create_record_return_alias_inline_state(caller, record_use))
    else {
        return false;
    };

    if let Some(temp_var) = state.temp_by_field.get(&record_use.field).cloned() {
        let rewritten =
            rewrite_record_return_field_use_to_temp_load(caller, record_use.field_get, &temp_var);
        alias_inline_states.insert(key, state);
        return rewritten;
    }

    let Some(callee) = all_fns.get(&record_use.callee) else {
        alias_inline_states.insert(key, state);
        return false;
    };
    let Some((scalarized, field_value)) =
        scalarizable_single_record_return_field(callee, all_fns, &record_use.field)
    else {
        alias_inline_states.insert(key, state);
        return false;
    };
    let mut value_map = FxHashMap::default();
    let Some(cloned_value) = clone_scalarizable_callee_value(
        caller,
        &scalarized,
        all_fns,
        &record_use.args,
        field_value,
        &mut value_map,
        &mut FxHashSet::default(),
    ) else {
        alias_inline_states.insert(key, state);
        return false;
    };

    let span = caller.values[record_use.field_get].span;
    let temp_var = unique_sroa_return_temp_var(caller, alias_var, &record_use.field);
    let insert_at = state
        .next_insert_index
        .min(caller.blocks[state.block].instrs.len());
    caller.blocks[state.block].instrs.insert(
        insert_at,
        Instr::Assign {
            dst: temp_var.clone(),
            src: cloned_value,
            span,
        },
    );
    state.next_insert_index = insert_at + 1;
    state
        .temp_by_field
        .insert(record_use.field.clone(), temp_var.clone());

    let rewritten =
        rewrite_record_return_field_use_to_temp_load(caller, record_use.field_get, &temp_var);
    alias_inline_states.insert(key, state);
    rewritten
}
