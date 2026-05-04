use super::*;
pub(crate) fn analyze_function(
    fn_ir: &mut FnIR,
    fn_ret: &FxHashMap<String, TypeState>,
) -> RR<TypeState> {
    seed_index_param_demands(fn_ir);
    seed_param_len_symbols(fn_ir);
    let mut changed = true;
    let mut guard = 0usize;

    while changed && guard < 32 {
        guard += 1;
        changed = false;
        let var_tys = collect_var_types(fn_ir);
        for vid in 0..fn_ir.values.len() {
            let old = fn_ir.values[vid].value_ty;
            let new = infer_value_type(fn_ir, vid, fn_ret, &var_tys);
            let joined = old.join(new);
            if joined != old {
                fn_ir.values[vid].value_ty = joined;
                changed = true;
            }
        }
    }

    let path_na = analyze_path_sensitive_na(fn_ir);
    apply_path_sensitive_na_refinements(fn_ir, &path_na);
    let ret_ty = path_refined_return_type(fn_ir, &path_na);

    Ok(ret_ty)
}

pub(crate) fn seed_index_param_demands(fn_ir: &mut FnIR) {
    let reachable = compute_reachable(fn_ir);
    let mut demanded_slots = FxHashSet::default();

    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        if !reachable.get(bid).copied().unwrap_or(false) {
            continue;
        }
        for ins in &bb.instrs {
            match ins {
                Instr::StoreIndex1D { idx, .. } => {
                    if let Some(slot) = param_slot_for_value(fn_ir, *idx) {
                        demanded_slots.insert(slot);
                    }
                }
                Instr::StoreIndex2D { r, c, .. } => {
                    for idx in [r, c] {
                        if let Some(slot) = param_slot_for_value(fn_ir, *idx) {
                            demanded_slots.insert(slot);
                        }
                    }
                }
                Instr::StoreIndex3D { i, j, k, .. } => {
                    for idx in [i, j, k] {
                        if let Some(slot) = param_slot_for_value(fn_ir, *idx) {
                            demanded_slots.insert(slot);
                        }
                    }
                }
                Instr::Assign { .. } | Instr::Eval { .. } | Instr::UnsafeRBlock { .. } => {}
            }
        }
    }

    for value in &fn_ir.values {
        match &value.kind {
            ValueKind::Index2D { r, c, .. } => {
                for idx in [r, c] {
                    if let Some(slot) = param_slot_for_value(fn_ir, *idx) {
                        demanded_slots.insert(slot);
                    }
                }
            }
            ValueKind::Index3D { i, j, k, .. } => {
                for idx in [i, j, k] {
                    if let Some(slot) = param_slot_for_value(fn_ir, *idx) {
                        demanded_slots.insert(slot);
                    }
                }
            }
            _ => {}
        }
    }

    for slot in demanded_slots {
        if param_slot_has_user_type_contract(fn_ir, slot) {
            continue;
        }
        if let Some(hint) = fn_ir.param_ty_hints.get_mut(slot) {
            *hint = coerce_index_scalar_return(*hint);
        }
        if let Some(term) = fn_ir.param_term_hints.get_mut(slot) {
            *term = TypeTerm::Int;
        }
    }
}

pub(crate) fn seed_param_len_symbols(fn_ir: &mut FnIR) {
    for (idx, hint) in fn_ir.param_ty_hints.iter_mut().enumerate() {
        if hint.len_sym.is_none() && matches!(hint.shape, ShapeTy::Vector | ShapeTy::Matrix) {
            *hint = hint.with_len(Some(LenSym((idx as u32).saturating_add(1))));
        }
    }
}

pub(crate) fn collect_var_types(fn_ir: &FnIR) -> FxHashMap<String, TypeState> {
    let mut out: FxHashMap<String, TypeState> = FxHashMap::default();
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            match ins {
                Instr::Assign { dst, src, .. } => {
                    let src_ty = fn_ir.values[*src].value_ty;
                    out.entry(dst.clone())
                        .and_modify(|acc| *acc = acc.join(src_ty))
                        .or_insert(src_ty);
                }
                Instr::StoreIndex1D { base, val, .. } => {
                    let Some(base_var) = value_base_var_name(fn_ir, *base) else {
                        continue;
                    };
                    let elem_ty = fn_ir.values[*val].value_ty;
                    let mut container_ty =
                        TypeState::vector(elem_ty.prim, elem_ty.na == NaTy::Never);
                    let len_sym = out
                        .get(&base_var)
                        .and_then(|ty| ty.len_sym)
                        .or(fn_ir.values[*base].value_ty.len_sym);
                    container_ty = container_ty.with_len(len_sym);
                    out.entry(base_var)
                        .and_modify(|acc| *acc = acc.join(container_ty))
                        .or_insert(container_ty);
                }
                Instr::StoreIndex2D { base, val, .. } => {
                    let Some(base_var) = value_base_var_name(fn_ir, *base) else {
                        continue;
                    };
                    let elem_ty = fn_ir.values[*val].value_ty;
                    let mut container_ty =
                        TypeState::matrix(elem_ty.prim, elem_ty.na == NaTy::Never);
                    let len_sym = out
                        .get(&base_var)
                        .and_then(|ty| ty.len_sym)
                        .or(fn_ir.values[*base].value_ty.len_sym);
                    container_ty = container_ty.with_len(len_sym);
                    out.entry(base_var)
                        .and_modify(|acc| *acc = acc.join(container_ty))
                        .or_insert(container_ty);
                }
                Instr::StoreIndex3D { base, val, .. } => {
                    let Some(base_var) = value_base_var_name(fn_ir, *base) else {
                        continue;
                    };
                    let elem_ty = fn_ir.values[*val].value_ty;
                    let mut container_ty =
                        TypeState::matrix(elem_ty.prim, elem_ty.na == NaTy::Never);
                    let len_sym = out
                        .get(&base_var)
                        .and_then(|ty| ty.len_sym)
                        .or(fn_ir.values[*base].value_ty.len_sym);
                    container_ty = container_ty.with_len(len_sym);
                    out.entry(base_var)
                        .and_modify(|acc| *acc = acc.join(container_ty))
                        .or_insert(container_ty);
                }
                Instr::Eval { .. } | Instr::UnsafeRBlock { .. } => {}
            }
        }
    }
    out
}
