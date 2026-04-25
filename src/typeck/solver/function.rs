fn analyze_function(fn_ir: &mut FnIR, fn_ret: &FxHashMap<String, TypeState>) -> RR<TypeState> {
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

    let mut ret_ty = TypeState::unknown();
    let reachable = compute_reachable(fn_ir);
    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        if !reachable.get(bid).copied().unwrap_or(false) {
            continue;
        }
        if let Terminator::Return(Some(v)) = bb.term {
            ret_ty = ret_ty.join(fn_ir.values[v].value_ty);
        }
    }

    // Use return hint only when no return value was observed in reachable blocks.
    if ret_ty == TypeState::unknown()
        && let Some(h) = fn_ir.ret_ty_hint
    {
        ret_ty = h;
    }

    Ok(ret_ty)
}

fn seed_param_len_symbols(fn_ir: &mut FnIR) {
    for (idx, hint) in fn_ir.param_ty_hints.iter_mut().enumerate() {
        if hint.len_sym.is_none() && matches!(hint.shape, ShapeTy::Vector | ShapeTy::Matrix) {
            *hint = hint.with_len(Some(LenSym((idx as u32).saturating_add(1))));
        }
    }
}

fn collect_var_types(fn_ir: &FnIR) -> FxHashMap<String, TypeState> {
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
                Instr::Eval { .. } => {}
            }
        }
    }
    out
}
