use super::*;
pub(crate) fn build_record_arg_specialized_callee(
    callee: &FnIR,
    new_name: &str,
    specs: &[RecordCallArgSpec],
) -> Option<FnIR> {
    let spec_by_param: FxHashMap<_, _> = specs.iter().map(|spec| (spec.arg_index, spec)).collect();
    let param_value_owner = specialized_param_value_owners(callee, specs);
    if !callee_record_param_uses_are_specializable(callee, specs, &param_value_owner) {
        return None;
    }

    let mut used_params = FxHashSet::default();
    let mut new_params = Vec::new();
    let mut new_param_defaults = Vec::new();
    let mut new_param_spans = Vec::new();
    let mut new_param_ty_hints = Vec::new();
    let mut new_param_term_hints = Vec::new();
    let mut new_param_hint_spans = Vec::new();
    let mut old_param_to_new = FxHashMap::default();
    let mut field_param_indices: FxHashMap<(usize, String), usize> = FxHashMap::default();

    for old_index in 0..callee.params.len() {
        if let Some(spec) = spec_by_param.get(&old_index).copied() {
            for (field_index, field) in spec.fields.iter().enumerate() {
                let param_name = unique_field_param_name(
                    callee,
                    old_index,
                    field_index,
                    &field.name,
                    &mut used_params,
                );
                let new_index = new_params.len();
                new_params.push(param_name);
                new_param_defaults.push(None);
                new_param_spans.push(param_span_at(callee, old_index));
                new_param_ty_hints.push(TypeState::unknown());
                new_param_term_hints.push(TypeTerm::Any);
                new_param_hint_spans.push(None);
                field_param_indices.insert((old_index, field.name.clone()), new_index);
            }
        } else {
            let new_index = new_params.len();
            let param_name = callee.params[old_index].clone();
            used_params.insert(param_name.clone());
            new_params.push(param_name);
            new_param_defaults.push(param_default_at(callee, old_index));
            new_param_spans.push(param_span_at(callee, old_index));
            new_param_ty_hints.push(param_ty_hint_at(callee, old_index));
            new_param_term_hints.push(param_term_hint_at(callee, old_index));
            new_param_hint_spans.push(param_hint_span_at(callee, old_index));
            old_param_to_new.insert(old_index, new_index);
        }
    }

    let mut field_get_param_indices = FxHashMap::default();
    for value in &callee.values {
        let ValueKind::FieldGet { base, field } = &value.kind else {
            continue;
        };
        let Some(param_index) = param_value_owner.get(base).copied() else {
            continue;
        };
        let new_index = field_param_indices
            .get(&(param_index, field.clone()))
            .copied()?;
        field_get_param_indices.insert(value.id, new_index);
    }

    let mut specialized = callee.clone();
    specialized.name = new_name.to_string();
    specialized.user_name = None;
    specialized.params = new_params;
    specialized.param_default_r_exprs = new_param_defaults;
    specialized.param_spans = new_param_spans;
    specialized.param_ty_hints = new_param_ty_hints;
    specialized.param_term_hints = new_param_term_hints;
    specialized.param_hint_spans = new_param_hint_spans;

    for value in &mut specialized.values {
        if let Some(param_index) = field_get_param_indices.get(&value.id).copied() {
            value.kind = ValueKind::Param { index: param_index };
            value.origin_var = specialized.params.get(param_index).cloned();
            continue;
        }
        if let ValueKind::Param { index } = value.kind {
            if spec_by_param.contains_key(&index) {
                value.kind = ValueKind::Const(Lit::Null);
                value.origin_var = None;
            } else if let Some(new_index) = old_param_to_new.get(&index).copied() {
                value.kind = ValueKind::Param { index: new_index };
                value.origin_var = specialized.params.get(new_index).cloned();
            } else {
                return None;
            }
        }
    }

    Some(specialized)
}

pub(crate) fn specialized_param_value_owners(
    callee: &FnIR,
    specs: &[RecordCallArgSpec],
) -> FxHashMap<ValueId, usize> {
    let specialized_params: FxHashSet<_> = specs.iter().map(|spec| spec.arg_index).collect();
    callee
        .values
        .iter()
        .filter_map(|value| match value.kind {
            ValueKind::Param { index } if specialized_params.contains(&index) => {
                Some((value.id, index))
            }
            _ => None,
        })
        .collect()
}

pub(crate) fn callee_record_param_uses_are_specializable(
    callee: &FnIR,
    specs: &[RecordCallArgSpec],
    param_value_owner: &FxHashMap<ValueId, usize>,
) -> bool {
    if param_value_owner.is_empty() {
        return true;
    }

    let allowed_fields: FxHashMap<usize, FxHashSet<String>> = specs
        .iter()
        .map(|spec| {
            (
                spec.arg_index,
                spec.fields.iter().map(|field| field.name.clone()).collect(),
            )
        })
        .collect();

    for value in &callee.values {
        if let ValueKind::FieldGet { base, field } = &value.kind
            && let Some(param_index) = param_value_owner.get(base)
            && allowed_fields
                .get(param_index)
                .is_some_and(|fields| fields.contains(field))
        {
            continue;
        }
        if value_dependencies(&value.kind)
            .iter()
            .any(|dep| param_value_owner.contains_key(dep))
        {
            return false;
        }
    }

    let specialized_param_names: FxHashSet<_> = specs
        .iter()
        .filter_map(|spec| callee.params.get(spec.arg_index).cloned())
        .collect();
    for block in &callee.blocks {
        for instr in &block.instrs {
            if instr_assigns_any(instr, &specialized_param_names)
                || instr_refs_any(instr, param_value_owner)
            {
                return false;
            }
        }
        if terminator_refs_any(&block.term, param_value_owner) {
            return false;
        }
    }

    true
}

pub(crate) fn instr_assigns_any(instr: &Instr, vars: &FxHashSet<String>) -> bool {
    matches!(instr, Instr::Assign { dst, .. } if vars.contains(dst))
}

pub(crate) fn instr_refs_any(instr: &Instr, values: &FxHashMap<ValueId, usize>) -> bool {
    match instr {
        Instr::Assign { src, .. } => values.contains_key(src),
        Instr::Eval { val, .. } => values.contains_key(val),
        Instr::StoreIndex1D { base, idx, val, .. } => {
            values.contains_key(base) || values.contains_key(idx) || values.contains_key(val)
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            values.contains_key(base)
                || values.contains_key(r)
                || values.contains_key(c)
                || values.contains_key(val)
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            values.contains_key(base)
                || values.contains_key(i)
                || values.contains_key(j)
                || values.contains_key(k)
                || values.contains_key(val)
        }
        Instr::UnsafeRBlock { .. } => false,
    }
}

pub(crate) fn terminator_refs_any(term: &Terminator, values: &FxHashMap<ValueId, usize>) -> bool {
    match term {
        Terminator::If { cond, .. } => values.contains_key(cond),
        Terminator::Return(Some(value)) => values.contains_key(value),
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => false,
    }
}

pub(crate) fn apply_record_call_specialization(
    caller: &mut FnIR,
    call: ValueId,
    old_callee: &str,
    new_callee: &str,
    specs: &[RecordCallArgSpec],
) -> bool {
    let spec_by_arg: FxHashMap<_, _> = specs.iter().map(|spec| (spec.arg_index, spec)).collect();
    let Some(value) = caller.values.get_mut(call) else {
        return false;
    };
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &mut value.kind
    else {
        return false;
    };
    if callee != old_callee {
        return false;
    }

    let old_args = args.clone();
    let mut new_args = Vec::new();
    for (arg_index, arg) in old_args.iter().copied().enumerate() {
        if let Some(spec) = spec_by_arg.get(&arg_index).copied() {
            new_args.extend(spec.fields.iter().map(|field| field.value));
        } else {
            new_args.push(arg);
        }
    }
    *callee = new_callee.to_string();
    *args = new_args;
    *names = vec![None; args.len()];
    true
}
