use super::*;
#[derive(Debug, Clone)]
pub(crate) struct RecordCallArgSpec {
    pub(crate) arg_index: usize,
    pub(crate) fields: Vec<RecordCallFieldArg>,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordCallFieldArg {
    pub(crate) name: String,
    pub(crate) value: ValueId,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordReturnFieldUse {
    pub(crate) field_get: ValueId,
    pub(crate) base_call: Option<ValueId>,
    pub(crate) field: String,
    pub(crate) callee: String,
    pub(crate) args: Vec<ValueId>,
    pub(crate) names: Vec<Option<String>>,
    pub(crate) alias_var: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RecordReturnAliasTempKey {
    pub(crate) alias_var: String,
    pub(crate) field: String,
    pub(crate) callee: String,
    pub(crate) args: Vec<ValueId>,
    pub(crate) names: Vec<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RecordReturnAliasCallKey {
    pub(crate) alias_var: String,
    pub(crate) callee: String,
    pub(crate) args: Vec<ValueId>,
    pub(crate) names: Vec<Option<String>>,
}

#[derive(Debug, Clone)]
pub(crate) struct RecordReturnAliasInlineState {
    pub(crate) block: BlockId,
    pub(crate) next_insert_index: usize,
    pub(crate) temp_by_field: FxHashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct RecordReturnDirectCallKey {
    pub(crate) base_call: ValueId,
    pub(crate) callee: String,
    pub(crate) args: Vec<ValueId>,
    pub(crate) names: Vec<Option<String>>,
}

#[derive(Debug, Default, Clone)]
pub(crate) struct RecordReturnDirectInlineState {
    pub(crate) replacement_by_field: FxHashMap<String, ValueId>,
}

pub(crate) fn specialize_record_field_calls(all_fns: &mut FxHashMap<String, FnIR>) -> bool {
    let mut ordered_names: Vec<_> = all_fns.keys().cloned().collect();
    ordered_names.sort();

    let mut reserved_names: FxHashSet<_> = all_fns.keys().cloned().collect();
    let mut generated_fns = Vec::new();
    let mut changed = false;

    for caller_name in ordered_names {
        let Some(mut caller) = all_fns.remove(&caller_name) else {
            continue;
        };
        let caller_changed = specialize_record_field_calls_in_caller(
            &mut caller,
            all_fns,
            &mut reserved_names,
            &mut generated_fns,
        );
        if caller_changed {
            changed = true;
            let _ = optimize(&mut caller);
        }
        all_fns.insert(caller_name, caller);
    }

    for (name, fn_ir) in generated_fns {
        all_fns.insert(name, fn_ir);
    }

    changed
}

pub(crate) fn specialize_record_return_field_calls(all_fns: &mut FxHashMap<String, FnIR>) -> bool {
    let mut ordered_names: Vec<_> = all_fns.keys().cloned().collect();
    ordered_names.sort();

    let mut reserved_names: FxHashSet<_> = all_fns.keys().cloned().collect();
    let mut generated_fns = Vec::new();
    let mut specialized_names = FxHashMap::default();
    let mut pure_cache = FxHashMap::default();
    let mut changed = false;

    for caller_name in ordered_names {
        let Some(mut caller) = all_fns.remove(&caller_name) else {
            continue;
        };
        let caller_changed = specialize_record_return_fields_in_caller(
            &mut caller,
            all_fns,
            &mut reserved_names,
            &mut generated_fns,
            &mut specialized_names,
            &mut pure_cache,
        );
        if caller_changed {
            changed = true;
            let _ = optimize(&mut caller);
        }
        all_fns.insert(caller_name, caller);
    }

    for (name, fn_ir) in generated_fns {
        all_fns.insert(name, fn_ir);
    }

    changed
}

pub(crate) fn specialize_record_field_calls_in_caller(
    caller: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    reserved_names: &mut FxHashSet<String>,
    generated_fns: &mut Vec<(String, FnIR)>,
) -> bool {
    if caller.requires_conservative_optimization() {
        return false;
    }

    let before_value_count = caller.values.len();
    let field_maps = infer_rewrite_field_maps(caller);
    let analysis_changed = caller.values.len() != before_value_count;
    if field_maps.is_empty() {
        return analysis_changed;
    }
    let (shapes, _) = infer_candidate_shapes(caller);
    let live_values = collect_live_value_ids(caller);
    let call_sites: Vec<_> = caller
        .values
        .iter()
        .filter(|value| live_values.contains(&value.id))
        .filter_map(|value| {
            let ValueKind::Call {
                callee,
                args,
                names,
            } = &value.kind
            else {
                return None;
            };
            Some((value.id, callee.clone(), args.clone(), names.clone()))
        })
        .collect();

    let mut changed = analysis_changed;
    for (call, callee_name, args, names) in call_sites {
        let Some(callee) = all_fns.get(&callee_name) else {
            continue;
        };
        if callee.requires_conservative_optimization()
            || args.len() != callee.params.len()
            || callee.name == caller.name
            || !call_arg_names_match_callee_order(callee, &names)
        {
            continue;
        }
        let arg_specs = collect_record_call_arg_specs(caller, &field_maps, &shapes, callee, &args);
        if arg_specs.is_empty() {
            continue;
        }

        let new_name =
            unique_sroa_specialized_name(&caller.name, &callee_name, call, reserved_names);
        let Some(specialized) = build_record_arg_specialized_callee(callee, &new_name, &arg_specs)
        else {
            continue;
        };
        if apply_record_call_specialization(caller, call, &callee_name, &new_name, &arg_specs) {
            reserved_names.insert(new_name.clone());
            generated_fns.push((new_name, specialized));
            changed = true;
        }
    }

    changed
}

pub(crate) fn specialize_record_return_fields_in_caller(
    caller: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    reserved_names: &mut FxHashSet<String>,
    generated_fns: &mut Vec<(String, FnIR)>,
    specialized_names: &mut FxHashMap<(String, String), String>,
    pure_cache: &mut FxHashMap<String, bool>,
) -> bool {
    if caller.requires_conservative_optimization() {
        return false;
    }

    let uses = collect_record_return_field_uses(caller, all_fns);
    if uses.is_empty() {
        return false;
    }

    let mut changed = false;
    let mut scalarized_alias_vars = FxHashSet::default();
    let mut alias_temp_vars = FxHashMap::default();
    let mut alias_inline_states = FxHashMap::default();
    let mut direct_inline_states = FxHashMap::default();
    for record_use in uses {
        if !function_is_effect_free(&record_use.callee, all_fns, pure_cache) {
            continue;
        }
        let Some(callee) = all_fns.get(&record_use.callee) else {
            continue;
        };
        if !call_arg_names_match_callee_order(callee, &record_use.names) {
            continue;
        }
        if record_use.alias_var.is_none()
            && rewrite_direct_record_return_field_use_with_inlined_value(
                caller,
                all_fns,
                &record_use,
                &mut direct_inline_states,
            )
        {
            changed = true;
            continue;
        }
        if record_use.alias_var.is_some()
            && rewrite_record_return_alias_field_use_with_inlined_temp(
                caller,
                all_fns,
                &record_use,
                &mut alias_inline_states,
            )
        {
            if let Some(alias_var) = record_use.alias_var.as_ref() {
                scalarized_alias_vars.insert(alias_var.clone());
            }
            changed = true;
            continue;
        }
        let cache_key = (record_use.callee.clone(), record_use.field.clone());
        let specialized_name = if let Some(name) = specialized_names.get(&cache_key).cloned() {
            name
        } else {
            let new_name = unique_sroa_return_specialized_name(
                &record_use.callee,
                &record_use.field,
                reserved_names,
            );
            let Some(specialized) =
                build_record_return_field_specialized_callee(callee, &new_name, &record_use.field)
            else {
                continue;
            };
            reserved_names.insert(new_name.clone());
            specialized_names.insert(cache_key, new_name.clone());
            generated_fns.push((new_name.clone(), specialized));
            new_name
        };

        let rewritten = if record_use.alias_var.is_some() {
            rewrite_record_return_alias_field_use_with_shared_temp(
                caller,
                &record_use,
                &specialized_name,
                &mut alias_temp_vars,
            )
        } else {
            rewrite_record_return_field_use(caller, &record_use, &specialized_name)
        };
        if rewritten {
            if let Some(alias_var) = record_use.alias_var.as_ref() {
                scalarized_alias_vars.insert(alias_var.clone());
            }
            changed = true;
        }
    }

    if !scalarized_alias_vars.is_empty() {
        changed |= remove_scalarized_return_aliases(caller, &scalarized_alias_vars);
    }

    changed
}
