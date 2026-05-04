use super::*;
pub(crate) fn value_base_var_name(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
    fn rec(fn_ir: &FnIR, vid: ValueId, seen: &mut FxHashSet<ValueId>) -> Option<String> {
        if !seen.insert(vid) {
            return None;
        }
        match &fn_ir.values.get(vid)?.kind {
            ValueKind::Load { var } => Some(var.clone()),
            ValueKind::Param { index } => fn_ir.params.get(*index).cloned(),
            ValueKind::Phi { args } => {
                let mut out: Option<String> = None;
                let mut saw = false;
                for (a, _) in args {
                    if *a == vid {
                        continue;
                    }
                    let name = rec(fn_ir, *a, seen)?;
                    saw = true;
                    match &out {
                        None => out = Some(name),
                        Some(prev) if prev == &name => {}
                        Some(_) => return None,
                    }
                }
                if saw { out } else { None }
            }
            _ => None,
        }
    }
    rec(fn_ir, vid, &mut FxHashSet::default())
}

pub(crate) fn is_floor_like_single_positional_call(
    callee: &str,
    args: &[ValueId],
    names: &[Option<String>],
) -> bool {
    matches!(callee, "floor" | "ceiling" | "trunc" | "round")
        && args.len() == 1
        && names.first().map(|name| name.is_none()).unwrap_or(true)
}

pub(crate) fn param_slot_for_value(fn_ir: &FnIR, vid: ValueId) -> Option<usize> {
    fn resolve_var_alias_slot(
        fn_ir: &FnIR,
        var: &str,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<usize> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }
        let mut slot: Option<usize> = None;
        let mut found = false;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                found = true;
                let src_slot = resolve_value_slot(fn_ir, *src, seen_vals, seen_vars)?;
                match slot {
                    None => slot = Some(src_slot),
                    Some(prev) if prev == src_slot => {}
                    Some(_) => return None,
                }
            }
        }
        if found { slot } else { None }
    }

    fn resolve_value_slot(
        fn_ir: &FnIR,
        vid: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<usize> {
        if !seen_vals.insert(vid) {
            return None;
        }
        match &fn_ir.values.get(vid)?.kind {
            ValueKind::Param { index } => Some(*index),
            ValueKind::Load { var } => fn_ir
                .params
                .iter()
                .position(|p| p == var)
                .or_else(|| resolve_var_alias_slot(fn_ir, var, seen_vals, seen_vars)),
            ValueKind::Phi { args } => {
                let mut out: Option<usize> = None;
                let mut saw = false;
                for (a, _) in args {
                    if *a == vid {
                        continue;
                    }
                    let slot = resolve_value_slot(fn_ir, *a, seen_vals, seen_vars)?;
                    saw = true;
                    match out {
                        None => out = Some(slot),
                        Some(prev) if prev == slot => {}
                        Some(_) => return None,
                    }
                }
                if saw { out } else { None }
            }
            _ => None,
        }
    }

    resolve_value_slot(
        fn_ir,
        vid,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

pub(crate) fn collect_index_vector_param_slots(fn_ir: &FnIR) -> FxHashSet<usize> {
    let mut slots = FxHashSet::default();
    for v in &fn_ir.values {
        let ValueKind::Call {
            callee,
            args,
            names,
        } = &v.kind
        else {
            continue;
        };
        if callee == "rr_index1_read_idx" && !args.is_empty() {
            if let Some(slot) = param_slot_for_value(fn_ir, args[0]) {
                slots.insert(slot);
            }
            continue;
        }
        if !is_floor_like_single_positional_call(callee, args, names) {
            continue;
        }
        let Some(inner) = args.first().copied() else {
            continue;
        };
        match &fn_ir.values[inner].kind {
            ValueKind::Index1D { base, .. } => {
                if let Some(slot) = param_slot_for_value(fn_ir, *base) {
                    slots.insert(slot);
                }
            }
            ValueKind::Call {
                callee: inner_callee,
                args: inner_args,
                names: inner_names,
            } if matches!(
                inner_callee.as_str(),
                "rr_index1_read" | "rr_index1_read_strict" | "rr_index1_read_floor"
            ) && (inner_args.len() == 2 || inner_args.len() == 3)
                && inner_names.iter().take(2).all(std::option::Option::is_none) =>
            {
                if let Some(slot) = param_slot_for_value(fn_ir, inner_args[0]) {
                    slots.insert(slot);
                }
            }
            _ => {}
        }
    }
    slots
}

pub(crate) fn collect_index_vector_param_slots_by_function(
    all_fns: &FxHashMap<String, FnIR>,
) -> FxHashMap<String, FxHashSet<usize>> {
    index_demands::collect_index_vector_param_slots_by_function(all_fns)
}

pub(crate) fn collect_scalar_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
) -> FxHashSet<String> {
    index_demands::collect_scalar_index_return_demands(all_fns)
}

pub(crate) fn collect_vector_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
    index_param_slots: &FxHashMap<String, FxHashSet<usize>>,
) -> FxHashSet<String> {
    index_demands::collect_vector_index_return_demands(all_fns, index_param_slots)
}

pub(crate) fn can_apply_index_return_override(
    all_fns: &FxHashMap<String, FnIR>,
    fname: &str,
    demanded_shape: ShapeTy,
    demanded_term: &TypeTerm,
) -> bool {
    index_demands::can_apply_index_return_override(all_fns, fname, demanded_shape, demanded_term)
}

pub(crate) fn coerce_index_scalar_return(ty: TypeState) -> TypeState {
    index_demands::coerce_index_scalar_return(ty)
}

pub(crate) fn coerce_index_vector_return(ty: TypeState) -> TypeState {
    index_demands::coerce_index_vector_return(ty)
}

pub(crate) fn apply_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
    fn_ret: &mut FxHashMap<String, TypeState>,
    fn_ret_term: &mut FxHashMap<String, TypeTerm>,
    scalar_demands: &FxHashSet<String>,
    vector_demands: &FxHashSet<String>,
) -> bool {
    index_demands::apply_index_return_demands(
        all_fns,
        fn_ret,
        fn_ret_term,
        scalar_demands,
        vector_demands,
    )
}
