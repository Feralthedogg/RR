use super::*;

pub(in crate::mir::opt::v_opt) fn same_loop_invariant_value(
    fn_ir: &FnIR,
    a: ValueId,
    b: ValueId,
    iv_phi: ValueId,
) -> bool {
    if is_value_equivalent(fn_ir, a, b) {
        return true;
    }
    if expr_has_iv_dependency(fn_ir, a, iv_phi) || expr_has_iv_dependency(fn_ir, b, iv_phi) {
        return false;
    }
    if let (ValueKind::Const(ca), ValueKind::Const(cb)) =
        (&fn_ir.values[a].kind, &fn_ir.values[b].kind)
    {
        return ca == cb;
    }
    match (
        fn_ir.values[a].origin_var.as_deref(),
        fn_ir.values[b].origin_var.as_deref(),
    ) {
        (Some(va), Some(vb)) => va == vb,
        _ => false,
    }
}

pub(in crate::mir::opt::v_opt) fn is_origin_var_iv_alias_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    candidate: ValueId,
    iv_phi: ValueId,
) -> bool {
    let candidate = canonical_value(fn_ir, candidate);
    let Some(origin_var) = fn_ir.values[candidate].origin_var.as_deref() else {
        return false;
    };
    let iv_origin = induction_origin_var(fn_ir, iv_phi);
    let matches_iv = |src: ValueId| {
        is_iv_equivalent(fn_ir, src, iv_phi)
            || is_floor_like_iv_expr(
                fn_ir,
                src,
                iv_phi,
                &mut FxHashSet::default(),
                &mut FxHashSet::default(),
                0,
            )
    };
    let matches_seed = |src: ValueId| {
        let src = canonical_value(fn_ir, src);
        matches_iv(src)
            || matches!(
                (&fn_ir.values[src].kind, iv_origin.as_deref()),
                (ValueKind::Load { var }, Some(iv_var)) if var == iv_var
            )
    };
    if let Some(src) = loop_entry_seed_source_in_loop(fn_ir, lp, origin_var)
        && matches_seed(src)
    {
        return true;
    }
    if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, origin_var)
        && matches_seed(src)
    {
        return true;
    }
    let target_bb =
        value_use_block_in_loop(fn_ir, lp, candidate).or(fn_ir.values[candidate].phi_block);
    let Some(target_bb) = target_bb else {
        return false;
    };
    let Some(src) = unique_assign_source_reaching_block_in_loop(fn_ir, lp, origin_var, target_bb)
    else {
        return false;
    };
    !matches!(
        fn_ir.values[canonical_value(fn_ir, src)].kind,
        ValueKind::Phi { .. }
    ) && matches_seed(src)
}

pub(in crate::mir::opt::v_opt) fn collapse_same_var_passthrough_phi_to_load(
    fn_ir: &FnIR,
    root: ValueId,
    var: &str,
    seen: &mut FxHashSet<ValueId>,
) -> Option<ValueId> {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return None;
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::Load { var: load_var } if load_var == var => Some(root),
        ValueKind::Phi { args }
            if !args.is_empty() && fn_ir.values[root].origin_var.as_deref() == Some(var) =>
        {
            let mut found: Option<ValueId> = None;
            for (arg, _) in args {
                let arg = canonical_value(fn_ir, *arg);
                let load = collapse_same_var_passthrough_phi_to_load(fn_ir, arg, var, seen)?;
                match found {
                    None => found = Some(load),
                    Some(prev) if canonical_value(fn_ir, prev) == canonical_value(fn_ir, load) => {}
                    Some(_) => return None,
                }
            }
            found
        }
        _ => None,
    };
    seen.remove(&root);
    out
}

pub(in crate::mir::opt::v_opt) fn loop_entry_seed_source_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> Option<ValueId> {
    let mut seed: Option<ValueId> = None;
    let mut passthrough_seen = FxHashSet::default();
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let src = canonical_value(fn_ir, *src);
            passthrough_seen.clear();
            if is_passthrough_load_of_var(fn_ir, src, var)
                || collapse_same_var_passthrough_phi_to_load(fn_ir, src, var, &mut passthrough_seen)
                    .is_some()
            {
                continue;
            }
            match seed {
                None => seed = Some(src),
                Some(prev) if canonical_value(fn_ir, prev) == src => {}
                Some(_) => return None,
            }
        }
    }
    seed
}

pub(in crate::mir::opt::v_opt) fn has_any_var_binding(fn_ir: &FnIR, var: &str) -> bool {
    if fn_ir.params.iter().any(|p| p == var) {
        return true;
    }
    fn_ir.blocks.iter().any(|bb| {
        bb.instrs.iter().any(|ins| match ins {
            Instr::Assign { dst, .. } => dst == var,
            _ => false,
        })
    })
}

pub(in crate::mir::opt::v_opt) fn is_loop_invariant_axis(
    fn_ir: &FnIR,
    axis: ValueId,
    iv_phi: ValueId,
    dest: ValueId,
) -> bool {
    if expr_has_iv_dependency(fn_ir, axis, iv_phi) {
        return false;
    }
    match &fn_ir.values[canonical_value(fn_ir, axis)].kind {
        ValueKind::Const(Lit::Int(_)) => true,
        ValueKind::Param { .. } => true,
        ValueKind::Load { var } => {
            if let Some(dest_var) = resolve_base_var(fn_ir, dest) {
                var != &dest_var && has_any_var_binding(fn_ir, var)
            } else {
                has_any_var_binding(fn_ir, var)
            }
        }
        _ => false,
    }
}

pub(in crate::mir::opt::v_opt) fn as_safe_loop_index(
    fn_ir: &FnIR,
    vid: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    if let ValueKind::Index1D {
        base,
        idx,
        is_safe,
        is_na_safe,
    } = &fn_ir.values[vid].kind
        && is_iv_equivalent(fn_ir, *idx, iv_phi)
        && *is_safe
        && *is_na_safe
    {
        return Some(canonical_value(fn_ir, *base));
    }
    None
}
pub(in crate::mir::opt::v_opt) fn loop_has_store_effect(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            if matches!(
                instr,
                Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
            ) {
                return true;
            }
        }
    }
    false
}

pub(in crate::mir::opt::v_opt) fn collect_loop_shadow_vars_for_dest(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    allowed_vars: &[VarId],
    dest_base: ValueId,
    iv_phi: ValueId,
) -> Option<Vec<VarId>> {
    let Some(_) = lp.iv.as_ref() else {
        return Some(Vec::new());
    };
    let mut shadow_vars = Vec::new();
    for value in &fn_ir.values {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) {
            continue;
        }
        let has_loop_pred = args.iter().any(|(_, pred)| lp.body.contains(pred));
        let has_outer_pred = args.iter().any(|(_, pred)| !lp.body.contains(pred));
        if !has_loop_pred || !has_outer_pred {
            continue;
        }
        if is_iv_equivalent(fn_ir, value.id, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, value.id, iv_phi)
        {
            continue;
        }
        if loop_carried_phi_is_unmodified(fn_ir, value.id) {
            continue;
        }
        if loop_carried_phi_is_invariant_passthrough(fn_ir, lp, value.id, iv_phi) {
            continue;
        }
        let Some(origin_var) = value.origin_var.as_deref() else {
            continue;
        };
        if !has_non_passthrough_assignment_in_loop(fn_ir, lp, origin_var) {
            continue;
        }
        if allowed_vars
            .iter()
            .any(|allowed| origin_var == allowed.as_str())
        {
            continue;
        }
        if let Some(shadow_base) =
            loop_carried_phi_last_value_shadow_base(fn_ir, lp, value.id, iv_phi)
        {
            if same_base_value(
                fn_ir,
                canonical_value(fn_ir, shadow_base),
                canonical_value(fn_ir, dest_base),
            ) {
                shadow_vars.push(origin_var.to_string());
                continue;
            }
            if resolve_base_var(fn_ir, shadow_base)
                .is_some_and(|var| allowed_vars.iter().any(|allowed| allowed == &var))
            {
                continue;
            }
        }
        if vectorize_trace_enabled() {
            let phi_args = match &value.kind {
                ValueKind::Phi { args } => args
                    .iter()
                    .map(|(arg, pred)| {
                        let arg = canonical_value(fn_ir, *arg);
                        format!(
                            "{}@{} kind={:?} origin={:?}",
                            arg, pred, fn_ir.values[arg].kind, fn_ir.values[arg].origin_var
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | "),
                _ => String::new(),
            };
            eprintln!(
                "   [vec-loop-state] {} reject: phi={} var={} bb={} non-destination-loop-state kind={:?} args=[{}]",
                fn_ir.name, value.id, origin_var, phi_bb, value.kind, phi_args
            );
        }
        return None;
    }
    shadow_vars.sort_unstable();
    shadow_vars.dedup();
    Some(shadow_vars)
}

pub(in crate::mir::opt::v_opt) fn loop_carried_phi_is_dest_last_value_shadow(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    phi_vid: ValueId,
    dest_base: ValueId,
    iv_phi: ValueId,
) -> bool {
    loop_carried_phi_last_value_shadow_base(fn_ir, lp, phi_vid, iv_phi).is_some_and(|shadow_base| {
        same_base_value(
            fn_ir,
            canonical_value(fn_ir, shadow_base),
            canonical_value(fn_ir, dest_base),
        )
    })
}

pub(in crate::mir::opt::v_opt) fn loop_carried_phi_last_value_shadow_base(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    phi_vid: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    let phi_vid = preserve_phi_value(fn_ir, phi_vid);
    let ValueKind::Phi { args } = &fn_ir.values[phi_vid].kind else {
        return None;
    };
    let mut loop_arg: Option<ValueId> = None;
    for (arg, pred) in args {
        if !lp.body.contains(pred) {
            continue;
        }
        let arg = resolve_load_alias_value(fn_ir, *arg);
        match loop_arg {
            None => loop_arg = Some(arg),
            Some(prev) if canonical_value(fn_ir, prev) == canonical_value(fn_ir, arg) => {}
            Some(_) => return None,
        }
    }
    let loop_arg = loop_arg?;
    match &fn_ir.values[canonical_value(fn_ir, loop_arg)].kind {
        ValueKind::Index1D { base, idx, .. } => {
            if is_iv_equivalent(fn_ir, *idx, iv_phi)
                || is_floor_like_iv_expr(
                    fn_ir,
                    *idx,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                    0,
                )
            {
                Some(canonical_value(fn_ir, *base))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(in crate::mir::opt::v_opt) fn loop_has_non_iv_loop_carried_state_except(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    allowed_vars: &[VarId],
) -> bool {
    let Some(iv) = lp.iv.as_ref() else {
        return false;
    };
    let iv_phi = iv.phi_val;
    for value in &fn_ir.values {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) {
            continue;
        }
        let has_loop_pred = args.iter().any(|(_, pred)| lp.body.contains(pred));
        let has_outer_pred = args.iter().any(|(_, pred)| !lp.body.contains(pred));
        if !has_loop_pred || !has_outer_pred {
            continue;
        }
        if is_iv_equivalent(fn_ir, value.id, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, value.id, iv_phi)
        {
            continue;
        }
        if loop_carried_phi_is_unmodified(fn_ir, value.id) {
            continue;
        }
        if loop_carried_phi_is_invariant_passthrough(fn_ir, lp, value.id, iv_phi) {
            continue;
        }
        let Some(origin_var) = value.origin_var.as_deref() else {
            continue;
        };
        if !has_non_passthrough_assignment_in_loop(fn_ir, lp, origin_var) {
            continue;
        }
        if allowed_vars
            .iter()
            .any(|allowed| origin_var == allowed.as_str())
        {
            continue;
        }
        if vectorize_trace_enabled() {
            let phi_args = match &value.kind {
                ValueKind::Phi { args } => args
                    .iter()
                    .map(|(arg, pred)| {
                        let arg = canonical_value(fn_ir, *arg);
                        format!(
                            "{}@{} kind={:?} origin={:?}",
                            arg, pred, fn_ir.values[arg].kind, fn_ir.values[arg].origin_var
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | "),
                _ => String::new(),
            };
            eprintln!(
                "   [vec-loop-state] {} reject: phi={} var={} bb={} non-destination-loop-state kind={:?} args=[{}]",
                fn_ir.name, value.id, origin_var, phi_bb, value.kind, phi_args
            );
        }
        return true;
    }
    false
}

pub(in crate::mir::opt::v_opt) fn preserve_phi_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
    if matches!(&fn_ir.values[vid].kind, ValueKind::Phi { .. }) {
        vid
    } else {
        canonical_value(fn_ir, vid)
    }
}

pub(in crate::mir::opt::v_opt) fn loop_carried_phi_is_unmodified(
    fn_ir: &FnIR,
    phi_vid: ValueId,
) -> bool {
    let phi_vid = preserve_phi_value(fn_ir, phi_vid);
    let ValueKind::Phi { args } = &fn_ir.values[phi_vid].kind else {
        return false;
    };
    let mut found: Option<ValueId> = None;
    for (arg, _) in args {
        let arg = canonical_value(fn_ir, *arg);
        if arg == phi_vid {
            continue;
        }
        match found {
            None => found = Some(arg),
            Some(prev) if canonical_value(fn_ir, prev) == arg => {}
            Some(_) => return false,
        }
    }
    found.is_some()
}

pub(in crate::mir::opt::v_opt) fn loop_carried_phi_is_invariant_passthrough(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    phi_vid: ValueId,
    iv_phi: ValueId,
) -> bool {
    let phi_vid = preserve_phi_value(fn_ir, phi_vid);
    if let Some(var) = fn_ir.values[phi_vid].origin_var.as_deref()
        && var.starts_with(".arg_")
        && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
    {
        return true;
    }

    fn invariant_passthrough_key(
        fn_ir: &FnIR,
        lp: &LoopInfo,
        root: ValueId,
        iv_phi: ValueId,
        seen: &mut FxHashSet<ValueId>,
    ) -> Option<String> {
        let root = preserve_phi_value(fn_ir, root);
        if let Some(var) = fn_ir.values[root].origin_var.as_deref()
            && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
        {
            return Some(format!("origin:{var}"));
        }
        if !seen.insert(root) {
            if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
            {
                return Some(format!("origin:{var}"));
            }
            return None;
        }
        let out = match &fn_ir.values[root].kind {
            ValueKind::Phi { args }
                if fn_ir.values[root]
                    .phi_block
                    .is_some_and(|bb| lp.body.contains(&bb)) =>
            {
                if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                    && has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                {
                    return None;
                }
                let mut found: Option<String> = None;
                let mut saw_non_self = false;
                for (arg, _) in args {
                    let arg = preserve_phi_value(fn_ir, *arg);
                    if arg == root {
                        continue;
                    }
                    saw_non_self = true;
                    let key = invariant_passthrough_key(fn_ir, lp, arg, iv_phi, seen)?;
                    match &found {
                        None => found = Some(key),
                        Some(prev) if prev == &key => {}
                        Some(_) => return None,
                    }
                }
                if saw_non_self {
                    found
                } else if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                    && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                {
                    Some(format!("origin:{var}"))
                } else {
                    None
                }
            }
            ValueKind::Phi { args } if args.is_empty() => {
                if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                    && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                {
                    Some(format!("origin:{var}"))
                } else {
                    None
                }
            }
            ValueKind::Const(lit) => Some(format!("const:{lit:?}")),
            ValueKind::Param { index } => {
                if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                    && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                {
                    Some(format!("origin:{var}"))
                } else {
                    Some(format!("param:{index}"))
                }
            }
            ValueKind::Load { var } => {
                if !has_non_passthrough_assignment_in_loop(fn_ir, lp, var) {
                    Some(format!("origin:{var}"))
                } else {
                    Some(format!("load:{var}"))
                }
            }
            ValueKind::Call { callee, args, .. }
                if matches!(callee.as_str(), "seq_len" | "rep.int" | "numeric") =>
            {
                let mut arg_keys = Vec::with_capacity(args.len());
                for arg in args {
                    let arg = preserve_phi_value(fn_ir, *arg);
                    let key = invariant_passthrough_key(fn_ir, lp, arg, iv_phi, seen)?;
                    arg_keys.push(key);
                }
                Some(format!("call:{}({})", callee, arg_keys.join(",")))
            }
            _ if is_iv_equivalent(fn_ir, root, iv_phi)
                || expr_has_iv_dependency(fn_ir, root, iv_phi) =>
            {
                None
            }
            _ => None,
        };
        seen.remove(&root);
        out
    }

    let phi_vid = preserve_phi_value(fn_ir, phi_vid);
    let ValueKind::Phi { args } = &fn_ir.values[phi_vid].kind else {
        return false;
    };
    if !fn_ir.values[phi_vid]
        .phi_block
        .is_some_and(|bb| lp.body.contains(&bb))
    {
        return false;
    }

    let mut found: Option<String> = None;
    for (arg, _) in args {
        let key = invariant_passthrough_key(fn_ir, lp, *arg, iv_phi, &mut FxHashSet::default());
        let Some(key) = key else {
            return false;
        };
        match &found {
            None => found = Some(key),
            Some(prev) if prev == &key => {}
            Some(_) => return false,
        }
    }
    found.is_some()
}
