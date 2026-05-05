use super::*;
pub(crate) fn is_scalar_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    matches!(
        fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. }
    )
}

pub(crate) fn is_iv_equivalent(fn_ir: &FnIR, candidate: ValueId, iv_phi: ValueId) -> bool {
    let mut seen_vals = FxHashSet::default();
    let mut seen_vars = FxHashSet::default();
    is_iv_equivalent_rec(fn_ir, candidate, iv_phi, &mut seen_vals, &mut seen_vars, 0)
}

pub(crate) fn is_iv_equivalent_rec(
    fn_ir: &FnIR,
    candidate: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
    depth: usize,
) -> bool {
    if proof_budget_exhausted(depth, seen_vals, seen_vars) {
        return false;
    }
    let candidate = canonical_value(fn_ir, candidate);
    if candidate == iv_phi {
        return true;
    }
    if !seen_vals.insert(candidate) {
        return false;
    }
    match &fn_ir.values[candidate].kind {
        ValueKind::Load { var } => {
            if induction_origin_var(fn_ir, iv_phi).as_deref() == Some(var.as_str()) {
                return true;
            }
            if let Some(src) = unique_assign_source(fn_ir, var)
                && src != candidate
            {
                return is_iv_equivalent_rec(fn_ir, src, iv_phi, seen_vals, seen_vars, depth + 1);
            }
            load_var_is_floor_like_iv(fn_ir, var, iv_phi, seen_vals, seen_vars, depth + 1)
        }
        ValueKind::Phi { args } if args.is_empty() => {
            match (
                fn_ir.values[candidate].origin_var.as_deref(),
                fn_ir.values[iv_phi].origin_var.as_deref(),
            ) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == candidate {
                    return false;
                }
                return is_iv_equivalent_rec(fn_ir, first, iv_phi, seen_vals, seen_vars, depth + 1);
            }
            let mut saw_iv_progress = false;
            for (v, _) in args {
                if is_iv_equivalent_rec(fn_ir, *v, iv_phi, seen_vals, seen_vars, depth + 1) {
                    saw_iv_progress = true;
                    continue;
                }
                if is_iv_seed_expr(
                    fn_ir,
                    *v,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                    depth + 1,
                ) {
                    continue;
                }
                return false;
            }
            saw_iv_progress
        }
        ValueKind::Call { args, names, .. } => {
            let resolved = resolve_call_info(fn_ir, candidate);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            let floor_like = resolved
                .as_ref()
                .and_then(|call| call.builtin_kind)
                .is_some_and(BuiltinKind::is_floor_like)
                || matches!(callee, "floor" | "ceiling" | "trunc");
            let single_positional = call_args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_equivalent_rec(
                    fn_ir,
                    call_args[0],
                    iv_phi,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                )
        }
        _ => false,
    }
}

pub(crate) fn induction_origin_var(fn_ir: &FnIR, iv_phi: ValueId) -> Option<String> {
    let mut stack = vec![iv_phi];
    let mut seen = FxHashSet::default();
    let mut name = None;
    while let Some(value) = stack.pop() {
        let value = canonical_value(fn_ir, value);
        if !seen.insert(value) {
            continue;
        }
        let row = fn_ir.values.get(value)?;
        let next_name = match &row.kind {
            _ if row.origin_var.is_some() => row.origin_var.clone(),
            ValueKind::Load { var } => Some(var.clone()),
            ValueKind::Phi { args } if !args.is_empty() => {
                stack.extend(args.iter().map(|(arg, _)| *arg));
                continue;
            }
            _ => return None,
        };
        match (&name, next_name) {
            (None, Some(current)) => name = Some(current),
            (Some(prev), Some(current)) if prev == &current => {}
            _ => return None,
        }
    }
    name
}

pub(crate) fn is_iv_seed_expr(
    fn_ir: &FnIR,
    vid: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
    depth: usize,
) -> bool {
    if proof_budget_exhausted(depth, seen_vals, seen_vars) {
        return false;
    }
    let vid = canonical_value(fn_ir, vid);
    if vid == iv_phi {
        return false;
    }
    if !seen_vals.insert(vid) {
        return false;
    }
    match &fn_ir.values[vid].kind {
        ValueKind::Const(_) | ValueKind::Param { .. } => true,
        ValueKind::Call { args, names, .. } => {
            let resolved = resolve_call_info(fn_ir, vid);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            let floor_like = resolved
                .as_ref()
                .and_then(|call| call.builtin_kind)
                .is_some_and(BuiltinKind::is_floor_like)
                || matches!(callee, "floor" | "ceiling" | "trunc");
            let single_positional = call_args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_seed_expr(fn_ir, call_args[0], iv_phi, seen_vals, seen_vars, depth + 1)
        }
        ValueKind::Load { var } => {
            if !seen_vars.insert(var.to_string()) {
                return false;
            }
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
                    if !is_iv_seed_expr(fn_ir, *src, iv_phi, seen_vals, seen_vars, depth + 1) {
                        seen_vars.remove(var);
                        return false;
                    }
                }
            }
            seen_vars.remove(var);
            found
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == vid {
                    return false;
                }
                return is_iv_seed_expr(fn_ir, first, iv_phi, seen_vals, seen_vars, depth + 1);
            }
            args.iter()
                .all(|(v, _)| is_iv_seed_expr(fn_ir, *v, iv_phi, seen_vals, seen_vars, depth + 1))
        }
        _ => false,
    }
}

pub(crate) fn load_var_is_floor_like_iv(
    fn_ir: &FnIR,
    var: &str,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
    depth: usize,
) -> bool {
    fn is_seed_expr(fn_ir: &FnIR, src: ValueId) -> bool {
        matches!(
            fn_ir.values[canonical_value(fn_ir, src)].kind,
            ValueKind::Const(_) | ValueKind::Param { .. }
        )
    }

    if proof_budget_exhausted(depth, seen_vals, seen_vars) {
        return false;
    }
    if !seen_vars.insert(var.to_string()) {
        return false;
    }
    let mut found = false;
    let mut all_match = true;
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            // Ignore non-recursive seeds/reinitializations like `ii <- 0`.
            if is_seed_expr(fn_ir, *src) {
                continue;
            }
            found = true;
            if !is_floor_like_iv_expr(fn_ir, *src, iv_phi, seen_vals, seen_vars, depth + 1) {
                all_match = false;
                break;
            }
        }
        if !all_match {
            break;
        }
    }
    seen_vars.remove(var);
    found && all_match
}

pub(crate) fn is_floor_like_iv_expr(
    fn_ir: &FnIR,
    src: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
    depth: usize,
) -> bool {
    if proof_budget_exhausted(depth, seen_vals, seen_vars) {
        return false;
    }
    let src = canonical_value(fn_ir, src);
    match &fn_ir.values[src].kind {
        ValueKind::Call { args, names, .. } => {
            let resolved = resolve_call_info(fn_ir, src);
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            let floor_like = resolved
                .as_ref()
                .and_then(|call| call.builtin_kind)
                .is_some_and(BuiltinKind::is_floor_like)
                || matches!(callee, "floor" | "ceiling" | "trunc");
            let single_positional = call_args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_equivalent_rec(
                    fn_ir,
                    call_args[0],
                    iv_phi,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                )
        }
        ValueKind::Load { var } => {
            load_var_is_floor_like_iv(fn_ir, var, iv_phi, seen_vals, seen_vars, depth + 1)
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == src {
                    return false;
                }
                return is_floor_like_iv_expr(
                    fn_ir,
                    first,
                    iv_phi,
                    seen_vals,
                    seen_vars,
                    depth + 1,
                );
            }
            let mut saw_progress = false;
            for (v, _) in args {
                if is_iv_seed_expr(
                    fn_ir,
                    *v,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                    depth + 1,
                ) {
                    continue;
                }
                saw_progress = true;
                if !is_floor_like_iv_expr(fn_ir, *v, iv_phi, seen_vals, seen_vars, depth + 1) {
                    return false;
                }
            }
            saw_progress
        }
        _ => false,
    }
}

pub(crate) fn is_value_equivalent(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    if a == b {
        return true;
    }
    if canonical_value(fn_ir, a) == canonical_value(fn_ir, b) {
        return true;
    }
    match (&fn_ir.values[a].kind, &fn_ir.values[b].kind) {
        (ValueKind::Load { var: va }, ValueKind::Load { var: vb }) => va == vb,
        (ValueKind::Load { var }, ValueKind::Phi { args }) if args.is_empty() => {
            fn_ir.values[b].origin_var.as_deref() == Some(var.as_str())
        }
        (ValueKind::Phi { args }, ValueKind::Load { var }) if args.is_empty() => {
            fn_ir.values[a].origin_var.as_deref() == Some(var.as_str())
        }
        _ => false,
    }
}

pub(crate) fn canonical_value(fn_ir: &FnIR, mut vid: ValueId) -> ValueId {
    let mut seen = FxHashSet::default();
    loop {
        if !seen.insert(vid) {
            return vid;
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Phi { args } if !args.is_empty() => {
                let first = args[0].0;
                if args.iter().all(|(v, _)| *v == first) {
                    vid = first;
                    continue;
                }
                let mut unique_non_self = FxHashSet::default();
                for (v, _) in args {
                    if *v != vid {
                        unique_non_self.insert(*v);
                    }
                }
                if let Some(unique_non_self_vid) = (unique_non_self.len() == 1)
                    .then(|| unique_non_self.iter().next().copied())
                    .flatten()
                {
                    // loop-invariant self-phi: v = phi(seed, v) -> seed
                    vid = unique_non_self_vid;
                    continue;
                }
            }
            _ => {}
        }
        return vid;
    }
}
