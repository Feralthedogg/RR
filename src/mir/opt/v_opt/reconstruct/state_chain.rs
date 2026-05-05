use super::*;

pub(in crate::mir::opt::v_opt) fn phi_state_var(fn_ir: &FnIR, phi: ValueId) -> Option<String> {
    let phi = canonical_value(fn_ir, phi);
    if !matches!(&fn_ir.values[phi].kind, ValueKind::Phi { args } if !args.is_empty()) {
        return None;
    }
    fn_ir.values[phi].origin_var.clone().or_else(|| {
        let ValueKind::Phi { args } = &fn_ir.values[phi].kind else {
            return None;
        };
        infer_passthrough_origin_var_from_phi_arms(fn_ir, args)
    })
}

pub(in crate::mir::opt::v_opt) fn nearest_state_phi_value_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    use_bb: BlockId,
) -> Option<ValueId> {
    let mut best: Option<(BlockId, ValueId)> = None;
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() || phi_state_var(fn_ir, vid).as_deref() != Some(var) {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) || phi_bb > use_bb {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        match best {
            None => best = Some((phi_bb, vid)),
            Some((best_bb, _)) if phi_bb > best_bb => best = Some((phi_bb, vid)),
            Some((best_bb, best_vid))
                if phi_bb == best_bb && canonical_value(fn_ir, best_vid) != vid =>
            {
                return None;
            }
            Some(_) => {}
        }
    }
    best.map(|(_, vid)| vid)
}

pub(in crate::mir::opt::v_opt) fn expr_reads_var(
    fn_ir: &FnIR,
    root: ValueId,
    var: &str,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    if let ValueKind::Load { var: load_var } = &fn_ir.values[root].kind
        && load_var == var
    {
        return true;
    }
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return false;
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::Load { var: load_var } => load_var == var,
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_reads_var(fn_ir, *lhs, var, seen) || expr_reads_var(fn_ir, *rhs, var, seen)
        }
        ValueKind::Unary { rhs, .. } => expr_reads_var(fn_ir, *rhs, var, seen),
        ValueKind::RecordLit { fields } => fields
            .iter()
            .any(|(_, value)| expr_reads_var(fn_ir, *value, var, seen)),
        ValueKind::FieldGet { base, .. } => expr_reads_var(fn_ir, *base, var, seen),
        ValueKind::FieldSet { base, value, .. } => {
            expr_reads_var(fn_ir, *base, var, seen) || expr_reads_var(fn_ir, *value, var, seen)
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| expr_reads_var(fn_ir, *arg, var, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(arg, _)| expr_reads_var(fn_ir, *arg, var, seen)),
        ValueKind::Index1D { base, idx, .. } => {
            expr_reads_var(fn_ir, *base, var, seen) || expr_reads_var(fn_ir, *idx, var, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            expr_reads_var(fn_ir, *base, var, seen)
                || expr_reads_var(fn_ir, *r, var, seen)
                || expr_reads_var(fn_ir, *c, var, seen)
        }
        ValueKind::Index3D { base, i, j, k } => {
            expr_reads_var(fn_ir, *base, var, seen)
                || expr_reads_var(fn_ir, *i, var, seen)
                || expr_reads_var(fn_ir, *j, var, seen)
                || expr_reads_var(fn_ir, *k, var, seen)
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_reads_var(fn_ir, *base, var, seen)
        }
        ValueKind::Range { start, end } => {
            expr_reads_var(fn_ir, *start, var, seen) || expr_reads_var(fn_ir, *end, var, seen)
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
    };
    seen.remove(&root);
    out
}

pub(in crate::mir::opt::v_opt) fn collect_independent_if_state_chain(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    phi_root: ValueId,
    var: &str,
) -> Option<(ValueId, Vec<SequentialStateStep>)> {
    let mut current = canonical_value(fn_ir, phi_root);
    let mut steps_rev = Vec::new();
    let mut seen = FxHashSet::default();
    let mut header_seed_bb: Option<BlockId> = None;

    loop {
        if !seen.insert(current) {
            return None;
        }
        if fn_ir.values[current].phi_block == Some(lp.header)
            && phi_state_var(fn_ir, current).as_deref() == Some(var)
        {
            let seed_bb = header_seed_bb?;
            let seed = same_iteration_seed_source_for_var(fn_ir, lp, seed_bb, var)?;
            let seed = canonical_value(fn_ir, seed);
            if expr_reads_var(fn_ir, seed, var, &mut FxHashSet::default()) {
                return None;
            }
            steps_rev.reverse();
            return Some((seed, steps_rev));
        }
        let Some(step) = passthrough_origin_phi_step_uncanonicalized(fn_ir, current) else {
            if vectorize_trace_enabled() {
                eprintln!(
                    "      [vec-state-chain] stop: phi={} is not a passthrough step ({:?})",
                    current, fn_ir.values[current].kind
                );
            }
            return None;
        };
        let Some(arms) = classify_passthrough_origin_phi_arms(fn_ir, lp, step, var) else {
            if vectorize_trace_enabled() {
                eprintln!(
                    "      [vec-state-chain] stop: phi={} arms not classifiable for {}",
                    current, var
                );
            }
            return None;
        };
        let Some((cond_root, _, _, _)) = passthrough_origin_phi_condition_parts(fn_ir, step) else {
            if vectorize_trace_enabled() {
                eprintln!(
                    "      [vec-state-chain] stop: phi={} condition not binary compare",
                    current
                );
            }
            return None;
        };
        if expr_reads_var(fn_ir, cond_root, var, &mut FxHashSet::default()) {
            if vectorize_trace_enabled() {
                eprintln!(
                    "      [vec-state-chain] stop: phi={} condition reads {}",
                    current, var
                );
            }
            return None;
        }
        if expr_reads_var(fn_ir, arms.update_val, var, &mut FxHashSet::default()) {
            if vectorize_trace_enabled() {
                eprintln!(
                    "      [vec-state-chain] stop: phi={} update reads {}",
                    current, var
                );
            }
            return None;
        }
        let prev_arm_bb = if arms.pass_then {
            step.then_bb
        } else {
            step.else_bb
        };
        let prev_source = if let Some(prev_raw) = arms.prev_state_raw {
            if phi_state_var(fn_ir, prev_raw).as_deref() == Some(var) {
                prev_raw
            } else if is_passthrough_load_of_var(fn_ir, prev_raw, var) {
                same_iteration_seed_source_for_var(fn_ir, lp, prev_arm_bb, var)?
            } else {
                passthrough_origin_phi_prev_source(fn_ir, lp, var, step, arms.prev_state_raw)?
            }
        } else {
            passthrough_origin_phi_prev_source(fn_ir, lp, var, step, arms.prev_state_raw)?
        };
        let prev_source = if phi_state_var(fn_ir, prev_source).as_deref() == Some(var) {
            prev_source
        } else {
            resolve_non_phi_prev_source_in_loop(fn_ir, lp, var, step, prev_source)?
        };
        if vectorize_trace_enabled() {
            eprintln!(
                "      [vec-state-chain] phi={} cond={:?} update={:?} pass_then={} prev_source={:?} prev_state_var={:?}",
                step.phi,
                fn_ir.values[cond_root].kind,
                fn_ir.values[canonical_value(fn_ir, arms.update_val)].kind,
                arms.pass_then,
                fn_ir.values[prev_source].kind,
                phi_state_var(fn_ir, prev_source)
            );
        }
        steps_rev.push(SequentialStateStep {
            phi: step.phi,
            cond_root,
            update_val: arms.update_val,
            pass_then: arms.pass_then,
        });

        if phi_state_var(fn_ir, prev_source).as_deref() == Some(var) {
            header_seed_bb = Some(prev_arm_bb);
            current = prev_source;
            continue;
        }
        let prev_source = canonical_value(fn_ir, prev_source);
        if expr_reads_var(fn_ir, prev_source, var, &mut FxHashSet::default()) {
            if vectorize_trace_enabled() {
                eprintln!(
                    "      [vec-state-chain] stop: seed {:?} still reads {}",
                    fn_ir.values[prev_source].kind, var
                );
            }
            return None;
        }
        steps_rev.reverse();
        return Some((prev_source, steps_rev));
    }
}

pub(in crate::mir::opt::v_opt) fn same_iteration_seed_source_for_var(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    target_bb: BlockId,
    var: &str,
) -> Option<ValueId> {
    let preds = build_pred_map(fn_ir);
    let mut current = target_bb;
    let mut seen = FxHashSet::default();
    while seen.insert(current) {
        let mut in_loop_preds: Vec<BlockId> = preds
            .get(&current)
            .into_iter()
            .flat_map(|ps| ps.iter().copied())
            .filter(|bb| lp.body.contains(bb) && *bb != lp.latch)
            .collect();
        in_loop_preds.sort_unstable();
        in_loop_preds.dedup();
        let pred = match in_loop_preds.as_slice() {
            [only] => *only,
            _ => return None,
        };
        if let Some(src) = last_assign_to_var_in_block(fn_ir, pred, var) {
            if is_passthrough_load_of_var(fn_ir, src, var) {
                current = pred;
                continue;
            }
            return Some(src);
        }
        current = pred;
    }
    None
}

pub(in crate::mir::opt::v_opt) fn materialize_independent_if_state_chain_for_load(
    fn_ir: &mut FnIR,
    root: ValueId,
    var: &str,
    use_bb: BlockId,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let phi_root = nearest_state_phi_value_in_loop(fn_ir, ctx.lp, var, use_bb)?;
    materialize_independent_if_state_chain(fn_ir, root, phi_root, var, ctx)
}

pub(in crate::mir::opt::v_opt) fn materialize_independent_if_state_chain(
    fn_ir: &mut FnIR,
    root: ValueId,
    phi_root: ValueId,
    var: &str,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    if vectorize_trace_enabled() {
        eprintln!(
            "   [vec-materialize] {} state-chain root={} var={} start",
            fn_ir.name, root, var
        );
    }
    let (seed_source, steps) = collect_independent_if_state_chain(fn_ir, ctx.lp, phi_root, var)?;
    let step_count = steps.len();
    let mut current = ctx.recurse(fn_ir, seed_source)?;
    for step in steps {
        let cond_vec =
            ctx.recurse_with_policy(fn_ir, step.cond_root, SAFE_INDEX_VECTOR_MATERIALIZE_POLICY)?;
        let update_vec = ctx.recurse(fn_ir, step.update_val)?;
        let then_vec = if step.pass_then { current } else { update_vec };
        let else_vec = if step.pass_then { update_vec } else { current };
        current = intern_materialized_value(
            fn_ir,
            ctx.interner,
            ValueKind::Call {
                callee: "rr_ifelse_strict".to_string(),
                args: vec![cond_vec, then_vec, else_vec],
                names: vec![None, None, None],
            },
            fn_ir.values[step.phi].span,
            fn_ir.values[step.phi].facts,
        );
    }
    ctx.memo.insert(root, current);
    if vectorize_trace_enabled() {
        eprintln!(
            "   [vec-materialize] {} state-chain root={} var={} success steps={}",
            fn_ir.name, root, var, step_count
        );
    }
    Some(current)
}
