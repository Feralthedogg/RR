use super::*;
pub(crate) fn collect_record_call_arg_specs(
    caller: &FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    shapes: &FxHashMap<ValueId, Vec<String>>,
    callee: &FnIR,
    args: &[ValueId],
) -> Vec<RecordCallArgSpec> {
    let mut specs = Vec::new();
    for (arg_index, arg) in args.iter().copied().enumerate() {
        if arg_index >= callee.params.len() || !field_maps.contains_key(&arg) {
            continue;
        }
        let Some(shape) = materialization_shape(caller, field_maps, shapes, arg) else {
            continue;
        };
        if shape.is_empty() {
            continue;
        }
        let Some(field_map) = field_maps.get(&arg) else {
            continue;
        };
        let mut fields = Vec::with_capacity(shape.len());
        for field in shape {
            let Some(value) = field_map.get(&field).copied() else {
                fields.clear();
                break;
            };
            fields.push(RecordCallFieldArg { name: field, value });
        }
        if !fields.is_empty() {
            specs.push(RecordCallArgSpec { arg_index, fields });
        }
    }
    specs
}

pub(crate) fn collect_record_return_field_uses(
    caller: &FnIR,
    all_fns: &FxHashMap<String, FnIR>,
) -> Vec<RecordReturnFieldUse> {
    let live_values = collect_live_value_ids(caller);
    let unique_assignments = unique_var_assignments(caller);
    let uses = build_use_graph(caller);
    let scalarizable_alias_vars = scalarizable_return_alias_vars(caller, all_fns, &uses);
    let direct_call_alias_vars = scalarizable_direct_return_call_alias_vars(caller, &uses);
    let mut out = Vec::new();

    for value in &caller.values {
        if !live_values.contains(&value.id) {
            continue;
        }
        let ValueKind::FieldGet { base, field } = &value.kind else {
            continue;
        };
        if let Some((callee, args, names)) = call_parts(caller, *base)
            && all_fns.contains_key(&callee)
            && callee != caller.name
        {
            let alias_var = direct_call_alias_vars.get(base).cloned();
            out.push(RecordReturnFieldUse {
                field_get: value.id,
                base_call: Some(*base),
                field: field.clone(),
                callee,
                args,
                names,
                alias_var,
            });
            continue;
        }
        if let ValueKind::Load { var } = &caller.values[*base].kind {
            if !scalarizable_alias_vars.contains(var) {
                continue;
            }
            let Some(src) = unique_assignments.get(var).copied() else {
                continue;
            };
            let Some((callee, args, names)) = call_parts(caller, src) else {
                continue;
            };
            if all_fns.contains_key(&callee) && callee != caller.name {
                out.push(RecordReturnFieldUse {
                    field_get: value.id,
                    base_call: None,
                    field: field.clone(),
                    callee,
                    args,
                    names,
                    alias_var: Some(var.clone()),
                });
            }
        }
    }

    out
}

pub(crate) fn scalarizable_direct_return_call_alias_vars(
    caller: &FnIR,
    uses: &FxHashMap<ValueId, Vec<SroaUse>>,
) -> FxHashMap<ValueId, String> {
    let unique_assignments = unique_var_assignments(caller);
    let mut out = FxHashMap::default();

    for (var, src) in unique_assignments {
        if call_parts(caller, src).is_none() {
            continue;
        }
        let Some((alias_block, alias_instr)) = unique_var_assignment_instr(caller, &var, src)
        else {
            continue;
        };
        if !direct_call_uses_are_alias_and_field_gets(caller, uses, src, &var) {
            continue;
        }
        if !alias_load_uses_are_field_gets(caller, uses, &var, alias_block, alias_instr) {
            continue;
        }
        out.insert(src, var);
    }

    out
}

pub(crate) fn direct_call_uses_are_alias_and_field_gets(
    caller: &FnIR,
    uses: &FxHashMap<ValueId, Vec<SroaUse>>,
    call: ValueId,
    alias_var: &str,
) -> bool {
    let Some(call_uses) = uses.get(&call) else {
        return false;
    };
    if call_uses.is_empty() {
        return false;
    }

    let mut alias_assignment = None;
    let mut direct_field_gets = Vec::new();
    for call_use in call_uses {
        match call_use.user {
            SroaUser::Value(user)
                if matches!(
                    &caller.values[user].kind,
                    ValueKind::FieldGet { base, .. } if *base == call
                ) =>
            {
                direct_field_gets.push(user);
            }
            SroaUser::Instr { block, instr }
                if caller
                    .blocks
                    .get(block)
                    .and_then(|block| block.instrs.get(instr))
                    .is_some_and(|instr| {
                        matches!(
                            instr,
                            Instr::Assign { dst, src, .. }
                                if dst == alias_var && *src == call
                        )
                    }) =>
            {
                if alias_assignment.replace((block, instr)).is_some() {
                    return false;
                }
            }
            _ => return false,
        }
    }

    let Some((alias_block, alias_instr)) = alias_assignment else {
        return false;
    };
    direct_field_gets.into_iter().all(|field_get| {
        value_uses_occur_after_instr(
            uses,
            field_get,
            alias_block,
            alias_instr,
            &mut FxHashSet::default(),
        )
    })
}

pub(crate) fn value_uses_occur_after_instr(
    uses: &FxHashMap<ValueId, Vec<SroaUse>>,
    value: ValueId,
    anchor_block: BlockId,
    anchor_instr: usize,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    if !seen.insert(value) {
        return false;
    }
    let ok = uses.get(&value).is_some_and(|value_uses| {
        !value_uses.is_empty()
            && value_uses.iter().all(|value_use| match value_use.user {
                SroaUser::Value(next) => {
                    value_uses_occur_after_instr(uses, next, anchor_block, anchor_instr, seen)
                }
                SroaUser::Instr { block, instr } => block == anchor_block && instr > anchor_instr,
                SroaUser::Terminator { block } => block == anchor_block,
            })
    });
    seen.remove(&value);
    ok
}

pub(crate) fn alias_load_uses_are_field_gets(
    caller: &FnIR,
    uses: &FxHashMap<ValueId, Vec<SroaUse>>,
    alias_var: &str,
    alias_block: BlockId,
    alias_instr: usize,
) -> bool {
    caller
        .values
        .iter()
        .filter_map(|value| match &value.kind {
            ValueKind::Load { var } if var == alias_var => Some(value.id),
            _ => None,
        })
        .all(|load| {
            uses.get(&load).is_some_and(|load_uses| {
                !load_uses.is_empty()
                    && load_uses.iter().all(|load_use| match load_use.user {
                        SroaUser::Value(user)
                            if matches!(
                                &caller.values[user].kind,
                                ValueKind::FieldGet { base, .. } if base == &load
                            ) =>
                        {
                            value_uses_occur_after_instr(
                                uses,
                                user,
                                alias_block,
                                alias_instr,
                                &mut FxHashSet::default(),
                            )
                        }
                        SroaUser::Instr { .. } | SroaUser::Terminator { .. } => false,
                        SroaUser::Value(_) => false,
                    })
            })
        })
}

pub(crate) fn call_arg_names_match_callee_order(callee: &FnIR, names: &[Option<String>]) -> bool {
    names.len() == callee.params.len()
        && names.iter().enumerate().all(|(index, name)| {
            name.as_ref()
                .is_none_or(|name| callee.params.get(index) == Some(name))
        })
}

pub(crate) fn scalarizable_return_alias_vars(
    caller: &FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    uses: &FxHashMap<ValueId, Vec<SroaUse>>,
) -> FxHashSet<String> {
    let unique_assignments = unique_var_assignments(caller);
    let mut out = FxHashSet::default();

    for (var, src) in unique_assignments {
        let Some((callee, _, _)) = call_parts(caller, src) else {
            continue;
        };
        if !all_fns.contains_key(&callee) || callee == caller.name {
            continue;
        }
        let Some((alias_block, alias_instr)) = unique_var_assignment_instr(caller, &var, src)
        else {
            continue;
        };
        let load_ids: Vec<_> = caller
            .values
            .iter()
            .filter_map(|value| match &value.kind {
                ValueKind::Load { var: load_var } if load_var == &var => Some(value.id),
                _ => None,
            })
            .collect();
        if load_ids.is_empty() {
            continue;
        }
        let all_load_uses_are_field_gets =
            alias_load_uses_are_field_gets(caller, uses, &var, alias_block, alias_instr);
        if all_load_uses_are_field_gets {
            out.insert(var);
        }
    }

    out
}

pub(crate) fn call_parts(
    fn_ir: &FnIR,
    value: ValueId,
) -> Option<(String, Vec<ValueId>, Vec<Option<String>>)> {
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values.get(value)?.kind
    else {
        return None;
    };
    Some((callee.clone(), args.clone(), names.clone()))
}
