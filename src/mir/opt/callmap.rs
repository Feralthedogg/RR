use super::*;

pub(super) fn collect_callmap_user_whitelist(
    all_fns: &FxHashMap<String, FnIR>,
) -> FxHashSet<String> {
    let mut whitelist: FxHashSet<String> = FxHashSet::default();
    let mut changed = true;
    while changed {
        changed = false;
        let mut ordered_names: Vec<String> = all_fns.keys().cloned().collect();
        ordered_names.sort();
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            if whitelist.contains(name) {
                continue;
            }
            if is_callmap_vector_safe_user_fn(name, fn_ir, &whitelist) {
                whitelist.insert(name.clone());
                changed = true;
            }
        }
    }
    whitelist
}

pub(super) fn is_callmap_vector_safe_user_fn(
    name: &str,
    fn_ir: &FnIR,
    user_whitelist: &FxHashSet<String>,
) -> bool {
    if fn_ir.requires_conservative_optimization() {
        return false;
    }
    if name.starts_with("Sym_top_") {
        return false;
    }

    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            match ins {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } => return false,
                Instr::StoreIndex1D { .. }
                | Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. } => return false,
            }
        }
        match bb.term {
            Terminator::Goto(_) | Terminator::Return(_) | Terminator::Unreachable => {}
            Terminator::If { .. } => return false,
        }
    }

    let mut saw_return = false;
    for bb in &fn_ir.blocks {
        if let Terminator::Return(ret) = bb.term {
            let Some(ret_vid) = ret else { return false };
            saw_return = true;
            if !is_vector_safe_user_expr(fn_ir, ret_vid, user_whitelist, &mut FxHashSet::default())
            {
                return false;
            }
        }
    }
    saw_return
}

pub(super) fn is_vector_safe_user_expr(
    fn_ir: &FnIR,
    vid: ValueId,
    user_whitelist: &FxHashSet<String>,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let vid = resolve_load_alias_value(fn_ir, vid);
    if !seen.insert(vid) {
        return true;
    }
    match &fn_ir.values[vid].kind {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => true,
        ValueKind::RSymbol { .. } => false,
        ValueKind::Unary { rhs, .. } => is_vector_safe_user_expr(fn_ir, *rhs, user_whitelist, seen),
        ValueKind::Binary { lhs, rhs, .. } => {
            is_vector_safe_user_expr(fn_ir, *lhs, user_whitelist, seen)
                && is_vector_safe_user_expr(fn_ir, *rhs, user_whitelist, seen)
        }
        ValueKind::Call { callee, args, .. } => {
            (v_opt::is_builtin_vector_safe_call(callee, args.len())
                || user_whitelist.contains(callee))
                && args
                    .iter()
                    .all(|a| is_vector_safe_user_expr(fn_ir, *a, user_whitelist, seen))
        }
        ValueKind::Intrinsic { args, .. } => args
            .iter()
            .all(|a| is_vector_safe_user_expr(fn_ir, *a, user_whitelist, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .all(|(a, _)| is_vector_safe_user_expr(fn_ir, *a, user_whitelist, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            is_vector_safe_user_expr(fn_ir, *base, user_whitelist, seen)
        }
        ValueKind::Range { start, end } => {
            is_vector_safe_user_expr(fn_ir, *start, user_whitelist, seen)
                && is_vector_safe_user_expr(fn_ir, *end, user_whitelist, seen)
        }
        ValueKind::Index1D { .. } | ValueKind::Index2D { .. } | ValueKind::Index3D { .. } => false,
    }
}

pub(super) fn resolve_load_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
    fn unique_assign_source(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
        let mut src: Option<ValueId> = None;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src: s, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                match src {
                    None => src = Some(*s),
                    Some(prev) if prev == *s => {}
                    Some(_) => return None,
                }
            }
        }
        src
    }

    let mut cur = vid;
    let mut seen = FxHashSet::default();
    while seen.insert(cur) {
        if let ValueKind::Load { var } = &fn_ir.values[cur].kind
            && let Some(src) = unique_assign_source(fn_ir, var)
        {
            cur = src;
            continue;
        }
        break;
    }
    cur
}
