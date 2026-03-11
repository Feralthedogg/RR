use super::*;

pub(super) fn collect_index_vector_param_slots_by_function(
    all_fns: &FxHashMap<String, FnIR>,
) -> FxHashMap<String, FxHashSet<usize>> {
    let mut out: FxHashMap<String, FxHashSet<usize>> = FxHashMap::default();
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        let slots = collect_index_vector_param_slots(fn_ir);
        if !slots.is_empty() {
            out.insert(name, slots);
        }
    }
    out
}

fn symbol_callee_for_value(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
    fn resolve_var(
        fn_ir: &FnIR,
        var: &str,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<String> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }
        let mut out: Option<String> = None;
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
                let sym = resolve_value(fn_ir, *src, seen_vals, seen_vars)?;
                match &out {
                    None => out = Some(sym),
                    Some(prev) if prev == &sym => {}
                    Some(_) => return None,
                }
            }
        }
        if found { out } else { None }
    }

    fn resolve_value(
        fn_ir: &FnIR,
        vid: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<String> {
        if !seen_vals.insert(vid) {
            return None;
        }
        match &fn_ir.values.get(vid)?.kind {
            ValueKind::Call { callee, .. } if callee.starts_with("Sym_") => Some(callee.clone()),
            ValueKind::Load { var } => resolve_var(fn_ir, var, seen_vals, seen_vars),
            ValueKind::Phi { args } => {
                let mut out: Option<String> = None;
                let mut saw = false;
                for (a, _) in args {
                    if *a == vid {
                        continue;
                    }
                    let sym = resolve_value(fn_ir, *a, seen_vals, seen_vars)?;
                    saw = true;
                    match &out {
                        None => out = Some(sym),
                        Some(prev) if prev == &sym => {}
                        Some(_) => return None,
                    }
                }
                if saw { out } else { None }
            }
            _ => None,
        }
    }

    resolve_value(
        fn_ir,
        vid,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

pub(super) fn collect_scalar_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        let mut scalar_indices = FxHashSet::default();
        for v in &fn_ir.values {
            match &v.kind {
                ValueKind::Index1D { idx, .. } => {
                    scalar_indices.insert(*idx);
                }
                ValueKind::Call { callee, args, .. } if callee == "rr_index1_read_idx" => {
                    if args.len() >= 2 {
                        scalar_indices.insert(args[1]);
                    }
                }
                _ => {}
            }
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                match ins {
                    Instr::StoreIndex1D { idx, .. } => {
                        scalar_indices.insert(*idx);
                    }
                    Instr::StoreIndex2D { r, c, .. } => {
                        scalar_indices.insert(*r);
                        scalar_indices.insert(*c);
                    }
                    Instr::StoreIndex3D { i, j, k, .. } => {
                        scalar_indices.insert(*i);
                        scalar_indices.insert(*j);
                        scalar_indices.insert(*k);
                    }
                    _ => {}
                }
            }
        }
        for idx in scalar_indices {
            if let Some(sym) = symbol_callee_for_value(fn_ir, idx) {
                out.insert(sym);
            }
        }
    }
    out
}

pub(super) fn collect_vector_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
    index_param_slots: &FxHashMap<String, FxHashSet<usize>>,
) -> FxHashSet<String> {
    let mut out = FxHashSet::default();
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        for v in &fn_ir.values {
            let ValueKind::Call { callee, args, .. } = &v.kind else {
                continue;
            };
            let Some(slots) = index_param_slots.get(callee) else {
                continue;
            };
            let mut ordered_slots: Vec<usize> = slots.iter().copied().collect();
            ordered_slots.sort_unstable();
            for slot in ordered_slots {
                let Some(arg) = args.get(slot).copied() else {
                    continue;
                };
                if let Some(sym) = symbol_callee_for_value(fn_ir, arg) {
                    out.insert(sym);
                }
            }
        }
    }
    out
}

pub(super) fn can_apply_index_return_override(
    all_fns: &FxHashMap<String, FnIR>,
    fname: &str,
    demanded_shape: ShapeTy,
    demanded_term: &TypeTerm,
) -> bool {
    let Some(fn_ir) = all_fns.get(fname) else {
        return false;
    };
    if let Some(hint) = fn_ir.ret_ty_hint
        && hint != TypeState::unknown()
    {
        if hint.shape != ShapeTy::Unknown && hint.shape != demanded_shape {
            return false;
        }
        if hint.prim != PrimTy::Any && hint.prim != PrimTy::Int {
            return false;
        }
    }
    if let Some(term_hint) = &fn_ir.ret_term_hint
        && !term_hint.is_any()
        && !term_hint.compatible_with(demanded_term)
    {
        return false;
    }
    true
}

pub(super) fn coerce_index_scalar_return(ty: TypeState) -> TypeState {
    let mut out = if ty == TypeState::unknown() {
        TypeState::scalar(PrimTy::Int, false)
    } else {
        ty
    };
    out.prim = PrimTy::Int;
    out.shape = ShapeTy::Scalar;
    out.len_sym = None;
    out
}

pub(super) fn coerce_index_vector_return(ty: TypeState) -> TypeState {
    let mut out = if ty == TypeState::unknown() {
        TypeState::vector(PrimTy::Int, false)
    } else {
        ty
    };
    out.prim = PrimTy::Int;
    out.shape = ShapeTy::Vector;
    out
}

pub(super) fn apply_index_return_demands(
    all_fns: &FxHashMap<String, FnIR>,
    fn_ret: &mut FxHashMap<String, TypeState>,
    fn_ret_term: &mut FxHashMap<String, TypeTerm>,
    scalar_demands: &FxHashSet<String>,
    vector_demands: &FxHashSet<String>,
) -> bool {
    let mut changed = false;

    let mut scalar_names: Vec<String> = scalar_demands.iter().cloned().collect();
    scalar_names.sort();
    for name in scalar_names {
        if !can_apply_index_return_override(all_fns, &name, ShapeTy::Scalar, &TypeTerm::Int) {
            continue;
        }
        if let Some(slot) = fn_ret.get_mut(&name) {
            let next = coerce_index_scalar_return(*slot);
            if *slot != next {
                *slot = next;
                changed = true;
            }
        }
        if let Some(slot) = fn_ret_term.get_mut(&name)
            && *slot != TypeTerm::Int
        {
            *slot = TypeTerm::Int;
            changed = true;
        }
    }

    let vec_term = TypeTerm::Vector(Box::new(TypeTerm::Int));
    let mut vector_names: Vec<String> = vector_demands.iter().cloned().collect();
    vector_names.sort();
    for name in vector_names {
        if !can_apply_index_return_override(all_fns, &name, ShapeTy::Vector, &vec_term) {
            continue;
        }
        if let Some(slot) = fn_ret.get_mut(&name) {
            let next = coerce_index_vector_return(*slot);
            if *slot != next {
                *slot = next;
                changed = true;
            }
        }
        if let Some(slot) = fn_ret_term.get_mut(&name)
            && *slot != vec_term
        {
            *slot = vec_term.clone();
            changed = true;
        }
    }

    changed
}
