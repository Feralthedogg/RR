use super::*;

pub(super) fn build_loop_index_vector(fn_ir: &mut FnIR, lp: &LoopInfo) -> Option<ValueId> {
    let iv = lp.iv.as_ref()?;
    let end = adjusted_loop_limit(fn_ir, lp.limit?, lp.limit_adjust);
    Some(fn_ir.add_value(
        ValueKind::Range {
            start: iv.init_val,
            end,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct MaterializedExprKey {
    kind: ValueKind,
}

pub(super) type MaterializeRecurseFn = fn(
    &mut FnIR,
    ValueId,
    ValueId,
    ValueId,
    &LoopInfo,
    &mut FxHashMap<ValueId, ValueId>,
    &mut FxHashMap<MaterializedExprKey, ValueId>,
    &mut FxHashSet<ValueId>,
    bool,
    bool,
) -> Option<ValueId>;

pub(super) fn intern_materialized_value(
    fn_ir: &mut FnIR,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    kind: ValueKind,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
) -> ValueId {
    let key = MaterializedExprKey { kind: kind.clone() };
    if let Some(existing) = interner.get(&key) {
        // Reuse structurally identical expressions, but keep analysis metadata
        // conservative across reuse sites.
        let merged = fn_ir.values[*existing].facts.join(&facts);
        fn_ir.values[*existing].facts = merged;
        return *existing;
    }
    let id = fn_ir.add_value(kind, span, facts, None);
    interner.insert(key, id);
    id
}

#[allow(clippy::too_many_arguments)]
pub(super) fn recurse_materialized_load_source(
    fn_ir: &mut FnIR,
    root: ValueId,
    src: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    if canonical_value(fn_ir, src) == root {
        return Some(root);
    }
    recurse(
        fn_ir,
        src,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )
}

pub(super) fn select_origin_phi_load_source(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    use_bb: Option<BlockId>,
    visiting: &FxHashSet<ValueId>,
) -> Option<(ValueId, &'static str)> {
    if let Some(use_bb) = use_bb
        && let Some(src) = unique_origin_phi_value_in_loop(fn_ir, lp, var)
            .filter(|src| {
                let src = canonical_value(fn_ir, *src);
                !visiting.contains(&src)
                    && fn_ir.values[src]
                        .phi_block
                        .is_some_and(|phi_bb| phi_bb < use_bb)
            })
            .or_else(|| {
                nearest_origin_phi_value_in_loop(fn_ir, lp, var, use_bb)
                    .filter(|src| !visiting.contains(&canonical_value(fn_ir, *src)))
            })
    {
        return Some((src, "prior-origin-phi"));
    }

    unique_origin_phi_value_in_loop(fn_ir, lp, var)
        .filter(|src| !visiting.contains(&canonical_value(fn_ir, *src)))
        .map(|src| (src, "fallback-origin-phi"))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_passthrough_origin_phi_for_load(
    fn_ir: &mut FnIR,
    var: &str,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    use_bb: BlockId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<(ValueId, ValueId)> {
    let nearest_phi = nearest_visiting_origin_phi_value_in_loop(fn_ir, lp, var, use_bb, visiting)?;
    let phi_src = materialize_passthrough_origin_phi_state(
        fn_ir,
        nearest_phi,
        var,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        allow_any_base,
        require_safe_index,
    )?;
    Some((nearest_phi, phi_src))
}

pub(super) fn phi_state_var(fn_ir: &FnIR, phi: ValueId) -> Option<String> {
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

pub(super) fn nearest_state_phi_value_in_loop(
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

pub(super) fn expr_reads_var(
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

pub(super) fn collect_independent_if_state_chain(
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

pub(super) fn same_iteration_seed_source_for_var(
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

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_independent_if_state_chain_for_load(
    fn_ir: &mut FnIR,
    root: ValueId,
    var: &str,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    use_bb: BlockId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let phi_root = nearest_state_phi_value_in_loop(fn_ir, lp, var, use_bb)?;
    materialize_independent_if_state_chain(
        fn_ir,
        root,
        phi_root,
        var,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_independent_if_state_chain(
    fn_ir: &mut FnIR,
    root: ValueId,
    phi_root: ValueId,
    var: &str,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    if vectorize_trace_enabled() {
        eprintln!(
            "   [vec-materialize] {} state-chain root={} var={} start",
            fn_ir.name, root, var
        );
    }
    let (seed_source, steps) = collect_independent_if_state_chain(fn_ir, lp, phi_root, var)?;
    let step_count = steps.len();
    let mut current = recurse(
        fn_ir,
        seed_source,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    for step in steps {
        let cond_vec = recurse(
            fn_ir,
            step.cond_root,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            true,
            true,
        )?;
        let update_vec = recurse(
            fn_ir,
            step.update_val,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        let then_vec = if step.pass_then { current } else { update_vec };
        let else_vec = if step.pass_then { update_vec } else { current };
        current = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Call {
                callee: "rr_ifelse_strict".to_string(),
                args: vec![cond_vec, then_vec, else_vec],
                names: vec![None, None, None],
            },
            fn_ir.values[step.phi].span,
            fn_ir.values[step.phi].facts,
        );
    }
    memo.insert(root, current);
    if vectorize_trace_enabled() {
        eprintln!(
            "   [vec-materialize] {} state-chain root={} var={} success steps={}",
            fn_ir.name, root, var, step_count
        );
    }
    Some(current)
}

pub(super) fn reject_unmaterialized_loop_load(
    fn_ir: &FnIR,
    root: ValueId,
    lp: &LoopInfo,
    var: &str,
    use_bb: Option<BlockId>,
    visiting: &FxHashSet<ValueId>,
) -> Option<ValueId> {
    if !has_non_passthrough_assignment_in_loop(fn_ir, lp, var) {
        return Some(root);
    }

    let detail = if let Some(use_bb) = use_bb {
        let unique_assign = unique_assign_source_in_loop(fn_ir, lp, var);
        let merged_assign = merged_assign_source_in_loop(fn_ir, lp, var);
        let unique_phi = unique_origin_phi_value_in_loop(fn_ir, lp, var);
        let nearest_phi = nearest_origin_phi_value_in_loop(fn_ir, lp, var, use_bb);
        let nearest_phi_block = nearest_phi.and_then(|src| fn_ir.values[src].phi_block);
        let nearest_phi_visiting =
            nearest_phi.is_some_and(|src| visiting.contains(&canonical_value(fn_ir, src)));
        let nearest_phi_kind = nearest_phi
            .map(|src| format!("{:?}", fn_ir.values[src].kind))
            .unwrap_or_else(|| "None".to_string());
        format!(
            "loop-local load without unique materializable source (var={}, use_bb={}, unique_assign={:?}, merged_assign={:?}, unique_phi={:?}, nearest_phi={:?}, nearest_phi_block={:?}, nearest_phi_visiting={}, nearest_phi_kind={})",
            var,
            use_bb,
            unique_assign,
            merged_assign,
            unique_phi,
            nearest_phi,
            nearest_phi_block,
            nearest_phi_visiting,
            nearest_phi_kind
        )
    } else {
        format!(
            "loop-local load without unique materializable source (var={}, use_bb=none)",
            var
        )
    };
    trace_materialize_reject(fn_ir, root, &detail);
    None
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_load(
    fn_ir: &mut FnIR,
    root: ValueId,
    var: &str,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let use_bb = value_use_block_in_loop(fn_ir, lp, root);
    if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, var) {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via unique-assign {:?}",
                fn_ir.name, var, src
            );
        }
        return recurse_materialized_load_source(
            fn_ir,
            root,
            src,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        );
    }

    if let Some(src) = merged_assign_source_in_loop(fn_ir, lp, var) {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via merged-assign {:?}",
                fn_ir.name, var, src
            );
        }
        return recurse_materialized_load_source(
            fn_ir,
            root,
            src,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        );
    }

    if let Some((src, label)) = select_origin_phi_load_source(fn_ir, lp, var, use_bb, visiting) {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via {} {:?}",
                fn_ir.name, var, label, src
            );
        }
        return recurse_materialized_load_source(
            fn_ir,
            root,
            src,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        );
    }

    if let Some(use_bb) = use_bb {
        if let Some(state_src) = materialize_independent_if_state_chain_for_load(
            fn_ir,
            root,
            var,
            iv_phi,
            idx_vec,
            lp,
            use_bb,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ) {
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-materialize] {} load {} via independent-if-state-chain",
                    fn_ir.name, var
                );
            }
            return Some(state_src);
        }
        if let Some((nearest_phi, phi_src)) = materialize_passthrough_origin_phi_for_load(
            fn_ir,
            var,
            iv_phi,
            idx_vec,
            lp,
            use_bb,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        ) {
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-materialize] {} load {} via passthrough-origin-phi {:?}",
                    fn_ir.name,
                    var,
                    Some(nearest_phi)
                );
            }
            return Some(phi_src);
        }

        let src = last_effective_assign_before_value_use_in_block(fn_ir, use_bb, var, root);
        if let Some(src) = src {
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-materialize] {} load {} via local-block-assign {:?} in bb {}",
                    fn_ir.name, var, src, use_bb
                );
            }
            return recurse_materialized_load_source(
                fn_ir,
                root,
                src,
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
                recurse,
            );
        }

        return reject_unmaterialized_loop_load(fn_ir, root, lp, var, Some(use_bb), visiting);
    }

    reject_unmaterialized_loop_load(fn_ir, root, lp, var, None, visiting)
}

pub(super) fn fold_phi_seed_candidate(
    fn_ir: &FnIR,
    root: ValueId,
    args: &[(ValueId, BlockId)],
    lp: &LoopInfo,
) -> Option<ValueId> {
    if phi_loads_same_var(fn_ir, args) {
        return Some(args[0].0);
    }

    let folded_non_self_args: Vec<ValueId> = args
        .iter()
        .map(|(a, _)| canonical_value(fn_ir, *a))
        .filter(|a| *a != root)
        .collect();
    if let Some(first) = folded_non_self_args.first().copied()
        && folded_non_self_args.iter().all(|a| *a == first)
    {
        return Some(first);
    }

    let outside_args: Vec<ValueId> = args
        .iter()
        .filter_map(|(a, b)| if lp.body.contains(b) { None } else { Some(*a) })
        .collect();
    if outside_args.len() == 1 {
        Some(outside_args[0])
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_conditional_phi_value(
    fn_ir: &mut FnIR,
    root: ValueId,
    args: &[(ValueId, BlockId)],
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    if fn_ir.values[root].phi_block == Some(lp.header)
        || !args.iter().all(|(_, b)| lp.body.contains(b))
    {
        return None;
    }
    let (cond, then_val, else_val) = find_conditional_phi_shape(fn_ir, root, args)?;
    for candidate in [then_val, else_val] {
        if let ValueKind::Load { var } = &fn_ir.values[canonical_value(fn_ir, candidate)].kind
            && has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
        {
            trace_materialize_reject(
                fn_ir,
                root,
                "conditional phi carries loop-local load arm with mutable state",
            );
            return None;
        }
    }
    let passthrough_prev_state = fn_ir.values[root]
        .origin_var
        .clone()
        .or_else(|| infer_passthrough_origin_var_from_phi_arms(fn_ir, args))
        .and_then(|var| {
            let step = passthrough_origin_phi_step(fn_ir, root)?;
            let arms = classify_passthrough_origin_phi_arms(fn_ir, lp, step, &var)?;
            let prev_source =
                passthrough_origin_phi_prev_source(fn_ir, lp, &var, step, arms.prev_state_raw)?;
            let prev_source =
                resolve_non_phi_prev_source_in_loop(fn_ir, lp, &var, step, prev_source)?;
            let prev_state = recurse(
                fn_ir,
                prev_source,
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
            )?;
            Some((var, prev_state))
        });
    if expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        cond,
        iv_phi,
        &FxHashSet::default(),
        &mut FxHashSet::default(),
    ) {
        trace_materialize_reject(fn_ir, root, "conditional phi has non-vector-safe condition");
        return None;
    }
    let cond_vec = recurse(
        fn_ir, cond, iv_phi, idx_vec, lp, memo, interner, visiting, true, true,
    )?;
    let then_vec = if let Some((var, prev_state)) = &passthrough_prev_state
        && is_passthrough_load_of_var(fn_ir, then_val, var)
    {
        *prev_state
    } else {
        recurse(
            fn_ir,
            then_val,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?
    };
    let else_vec = if let Some((var, prev_state)) = &passthrough_prev_state
        && is_passthrough_load_of_var(fn_ir, else_val, var)
    {
        *prev_state
    } else {
        recurse(
            fn_ir,
            else_val,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?
    };
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_vec, then_vec, else_vec],
            names: vec![None, None, None],
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_uniform_phi_value(
    fn_ir: &mut FnIR,
    root: ValueId,
    args: Vec<(ValueId, BlockId)>,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let mut picked: Option<ValueId> = None;
    for (arg, _) in args {
        let materialized = recurse(
            fn_ir,
            arg,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        match picked {
            None => picked = Some(materialized),
            Some(prev) if canonical_value(fn_ir, prev) == canonical_value(fn_ir, materialized) => {}
            Some(_) => {
                trace_materialize_reject(
                    fn_ir,
                    root,
                    "phi arguments materialize to distinct values",
                );
                return None;
            }
        }
    }
    picked
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_phi(
    fn_ir: &mut FnIR,
    root: ValueId,
    args: Vec<(ValueId, BlockId)>,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    if let Some(seed) = fold_phi_seed_candidate(fn_ir, root, &args, lp) {
        let folded = recurse(
            fn_ir,
            seed,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        memo.insert(root, folded);
        visiting.remove(&root);
        return Some(folded);
    }

    if let Some(var) = phi_state_var(fn_ir, root) {
        if let Some(phi_vec) = materialize_independent_if_state_chain(
            fn_ir,
            root,
            root,
            &var,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ) {
            memo.insert(root, phi_vec);
            visiting.remove(&root);
            return Some(phi_vec);
        }
        if let Some(phi_vec) = materialize_passthrough_origin_phi_state(
            fn_ir,
            root,
            &var,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            allow_any_base,
            require_safe_index,
        ) {
            memo.insert(root, phi_vec);
            visiting.remove(&root);
            return Some(phi_vec);
        }
    }

    if let Some(ifelse_val) = materialize_conditional_phi_value(
        fn_ir,
        root,
        &args,
        span,
        facts,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    ) {
        memo.insert(root, ifelse_val);
        visiting.remove(&root);
        return Some(ifelse_val);
    }

    materialize_uniform_phi_value(
        fn_ir,
        root,
        args,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )
}

pub(super) fn infer_passthrough_origin_var_from_phi_arms(
    fn_ir: &FnIR,
    args: &[(ValueId, BlockId)],
) -> Option<String> {
    let mut found: Option<String> = None;
    for (arg, _) in args {
        let ValueKind::Load { var } = &fn_ir.values[canonical_value(fn_ir, *arg)].kind else {
            continue;
        };
        match &found {
            None => found = Some(var.clone()),
            Some(prev) if prev == var => {}
            Some(_) => return None,
        }
    }
    found
}

pub(super) fn is_int_index_vector_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    fn rec(fn_ir: &FnIR, vid: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        let vid = canonical_value(fn_ir, vid);
        if !seen.insert(vid) {
            return false;
        }
        let v = &fn_ir.values[vid];
        if (v.value_ty.shape == ShapeTy::Vector && v.value_ty.prim == PrimTy::Int)
            || v.facts
                .has(crate::mir::flow::Facts::IS_VECTOR | crate::mir::flow::Facts::INT_SCALAR)
        {
            return true;
        }
        match &v.kind {
            ValueKind::Call { callee, args, .. } => match callee.as_str() {
                "rr_index_vec_floor" => true,
                "rr_index1_read_vec" | "rr_index1_read_vec_floor" | "rr_gather" => args
                    .first()
                    .copied()
                    .is_some_and(|base| rec(fn_ir, base, seen)),
                _ => false,
            },
            ValueKind::Index1D { base, .. } => rec(fn_ir, *base, seen),
            ValueKind::Phi { args } if !args.is_empty() => {
                args.iter().all(|(arg, _)| rec(fn_ir, *arg, seen))
            }
            _ => false,
        }
    }

    rec(fn_ir, vid, &mut FxHashSet::default())
}

pub(super) fn is_scalar_broadcast_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    let v = &fn_ir.values[canonical_value(fn_ir, vid)];
    matches!(v.kind, ValueKind::Const(_)) || v.value_ty.shape == ShapeTy::Scalar
}

pub(super) fn has_assignment_in_loop(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> bool {
    lp.body.iter().any(|bid| {
        fn_ir.blocks[*bid].instrs.iter().any(|ins| match ins {
            Instr::Assign { dst, .. } => dst == var,
            _ => false,
        })
    })
}

pub(super) fn expr_has_unstable_loop_local_load(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    root: ValueId,
) -> bool {
    fn rec(fn_ir: &FnIR, lp: &LoopInfo, root: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Load { var } => has_non_passthrough_assignment_in_loop(fn_ir, lp, var),
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, lp, *lhs, seen) || rec(fn_ir, lp, *rhs, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, lp, *rhs, seen),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                args.iter().any(|arg| rec(fn_ir, lp, *arg, seen))
            }
            ValueKind::Phi { args } => args.iter().any(|(arg, _)| rec(fn_ir, lp, *arg, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => rec(fn_ir, lp, *base, seen),
            ValueKind::Range { start, end } => {
                rec(fn_ir, lp, *start, seen) || rec(fn_ir, lp, *end, seen)
            }
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *idx, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *r, seen) || rec(fn_ir, lp, *c, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                rec(fn_ir, lp, *base, seen)
                    || rec(fn_ir, lp, *i, seen)
                    || rec(fn_ir, lp, *j, seen)
                    || rec(fn_ir, lp, *k, seen)
            }
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
        }
    }

    rec(fn_ir, lp, root, &mut FxHashSet::default())
}

pub(super) fn expr_has_ambiguous_loop_local_load(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    root: ValueId,
) -> bool {
    fn rec(fn_ir: &FnIR, lp: &LoopInfo, root: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Load { var } => {
                let can_materialize_state_chain = value_use_block_in_loop(fn_ir, lp, root)
                    .and_then(|use_bb| nearest_state_phi_value_in_loop(fn_ir, lp, var, use_bb))
                    .and_then(|phi_root| {
                        collect_independent_if_state_chain(fn_ir, lp, phi_root, var)
                    })
                    .is_some();
                has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                    && unique_assign_source_in_loop(fn_ir, lp, var).is_none()
                    && merged_assign_source_in_loop(fn_ir, lp, var).is_none()
                    && unique_origin_phi_value_in_loop(fn_ir, lp, var).is_none()
                    && !can_materialize_state_chain
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, lp, *lhs, seen) || rec(fn_ir, lp, *rhs, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, lp, *rhs, seen),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                args.iter().any(|arg| rec(fn_ir, lp, *arg, seen))
            }
            ValueKind::Phi { args } => args.iter().any(|(arg, _)| rec(fn_ir, lp, *arg, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => rec(fn_ir, lp, *base, seen),
            ValueKind::Range { start, end } => {
                rec(fn_ir, lp, *start, seen) || rec(fn_ir, lp, *end, seen)
            }
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *idx, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *r, seen) || rec(fn_ir, lp, *c, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                rec(fn_ir, lp, *base, seen)
                    || rec(fn_ir, lp, *i, seen)
                    || rec(fn_ir, lp, *j, seen)
                    || rec(fn_ir, lp, *k, seen)
            }
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
        }
    }

    rec(fn_ir, lp, root, &mut FxHashSet::default())
}

pub(super) fn has_non_passthrough_assignment_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> bool {
    lp.body.iter().any(|bid| {
        fn_ir.blocks[*bid].instrs.iter().any(|ins| {
            let Instr::Assign { dst, src, .. } = ins else {
                return false;
            };
            if dst != var {
                return false;
            }
            let src = preserve_phi_value(fn_ir, *src);
            !matches!(
                &fn_ir.values[src].kind,
                ValueKind::Load { var: load_var } if load_var == var
            ) && !matches!(&fn_ir.values[src].kind, ValueKind::Param { .. })
                && !matches!(
                    &fn_ir.values[src].kind,
                    ValueKind::Phi { args }
                        if !args.is_empty()
                            && fn_ir.values[src].origin_var.as_deref() == Some(var)
                )
        })
    })
}

pub(super) fn unique_assign_source_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> Option<ValueId> {
    let mut src: Option<ValueId> = None;
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src: s, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let s = canonical_value(fn_ir, *s);
            match src {
                None => src = Some(s),
                Some(prev) if canonical_value(fn_ir, prev) == s => {}
                Some(_) => return None,
            }
        }
    }
    src
}

pub(super) fn merged_assign_source_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> Option<ValueId> {
    let mut assigned = Vec::new();
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst == var {
                assigned.push(canonical_value(fn_ir, *src));
            }
        }
    }
    assigned.sort_unstable();
    assigned.dedup();

    let mut phi_srcs = assigned
        .iter()
        .copied()
        .filter(
            |src| matches!(&fn_ir.values[*src].kind, ValueKind::Phi { args } if !args.is_empty()),
        )
        .filter(|src| {
            fn_ir.values[*src]
                .phi_block
                .is_some_and(|bb| lp.body.contains(&bb))
        });
    let phi_src = phi_srcs.next()?;
    if phi_srcs.next().is_some() {
        return None;
    }

    let ValueKind::Phi { args } = &fn_ir.values[phi_src].kind else {
        return None;
    };
    let phi_args: FxHashSet<ValueId> = args
        .iter()
        .map(|(arg, _)| canonical_value(fn_ir, *arg))
        .collect();
    if assigned
        .iter()
        .all(|src| *src == phi_src || phi_args.contains(src))
    {
        Some(phi_src)
    } else {
        None
    }
}

pub(super) fn unique_origin_phi_value_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> Option<ValueId> {
    let mut found: Option<ValueId> = None;
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() || value.origin_var.as_deref() != Some(var) {
            continue;
        }
        if !value.phi_block.is_some_and(|bb| lp.body.contains(&bb)) {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        match found {
            None => found = Some(vid),
            Some(prev) if canonical_value(fn_ir, prev) == vid => {}
            Some(_) => return None,
        }
    }
    found
}

pub(super) fn phi_loads_same_var(fn_ir: &FnIR, args: &[(ValueId, BlockId)]) -> bool {
    let mut found: Option<&str> = None;
    for (arg, _) in args {
        let ValueKind::Load { var } = &fn_ir.values[canonical_value(fn_ir, *arg)].kind else {
            return false;
        };
        match found {
            None => found = Some(var.as_str()),
            Some(prev) if prev == var => {}
            Some(_) => return false,
        }
    }
    found.is_some()
}

pub(super) fn value_use_block_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    vid: ValueId,
) -> Option<BlockId> {
    let vid = canonical_value(fn_ir, vid);
    let mut use_blocks: Vec<Option<BlockId>> = vec![None; fn_ir.values.len()];
    let mut worklist: Vec<(ValueId, BlockId)> = Vec::new();
    let mut body: Vec<BlockId> = lp.body.iter().copied().collect();
    body.sort_unstable();
    for bid in body {
        for ins in &fn_ir.blocks[bid].instrs {
            match ins {
                Instr::Assign { src, .. } => worklist.push((canonical_value(fn_ir, *src), bid)),
                Instr::Eval { val, .. } => worklist.push((canonical_value(fn_ir, *val), bid)),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    worklist.push((canonical_value(fn_ir, *base), bid));
                    worklist.push((canonical_value(fn_ir, *idx), bid));
                    worklist.push((canonical_value(fn_ir, *val), bid));
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    worklist.push((canonical_value(fn_ir, *base), bid));
                    worklist.push((canonical_value(fn_ir, *r), bid));
                    worklist.push((canonical_value(fn_ir, *c), bid));
                    worklist.push((canonical_value(fn_ir, *val), bid));
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    worklist.push((canonical_value(fn_ir, *base), bid));
                    worklist.push((canonical_value(fn_ir, *i), bid));
                    worklist.push((canonical_value(fn_ir, *j), bid));
                    worklist.push((canonical_value(fn_ir, *k), bid));
                    worklist.push((canonical_value(fn_ir, *val), bid));
                }
            }
        }
        match &fn_ir.blocks[bid].term {
            Terminator::If { cond, .. } => worklist.push((canonical_value(fn_ir, *cond), bid)),
            Terminator::Return(Some(ret)) => worklist.push((canonical_value(fn_ir, *ret), bid)),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    while let Some((curr, bid)) = worklist.pop() {
        if let Some(prev) = use_blocks[curr]
            && bid >= prev
        {
            continue;
        }
        use_blocks[curr] = Some(bid);
        match &fn_ir.values[curr].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                worklist.push((canonical_value(fn_ir, *lhs), bid));
                worklist.push((canonical_value(fn_ir, *rhs), bid));
            }
            ValueKind::Unary { rhs, .. } => {
                worklist.push((canonical_value(fn_ir, *rhs), bid));
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    worklist.push((canonical_value(fn_ir, *arg), bid));
                }
            }
            ValueKind::Phi { args } => {
                for (arg, _) in args {
                    worklist.push((canonical_value(fn_ir, *arg), bid));
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
                worklist.push((canonical_value(fn_ir, *idx), bid));
            }
            ValueKind::Index2D { base, r, c } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
                worklist.push((canonical_value(fn_ir, *r), bid));
                worklist.push((canonical_value(fn_ir, *c), bid));
            }
            ValueKind::Index3D { base, i, j, k } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
                worklist.push((canonical_value(fn_ir, *i), bid));
                worklist.push((canonical_value(fn_ir, *j), bid));
                worklist.push((canonical_value(fn_ir, *k), bid));
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
            }
            ValueKind::Range { start, end } => {
                worklist.push((canonical_value(fn_ir, *start), bid));
                worklist.push((canonical_value(fn_ir, *end), bid));
            }
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => {}
        }
    }
    use_blocks[vid]
}

pub(super) fn nearest_origin_phi_value_in_loop(
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
        if args.is_empty() || value.origin_var.as_deref() != Some(var) {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) || phi_bb >= use_bb {
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

pub(super) fn nearest_visiting_origin_phi_value_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    use_bb: BlockId,
    visiting: &FxHashSet<ValueId>,
) -> Option<ValueId> {
    let mut best: Option<(BlockId, ValueId)> = None;
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() || value.origin_var.as_deref() != Some(var) {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) || phi_bb > use_bb {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        if !visiting.contains(&vid) {
            continue;
        }
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

pub(super) fn unique_assign_source_reaching_block_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    target_bb: BlockId,
) -> Option<ValueId> {
    let preds = build_pred_map(fn_ir);
    let mut seen = FxHashSet::default();
    let mut stack: Vec<BlockId> = preds
        .get(&target_bb)
        .into_iter()
        .flat_map(|ps| ps.iter().copied())
        .filter(|bb| lp.body.contains(bb))
        .collect();
    let mut src: Option<ValueId> = None;
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        for ins in &fn_ir.blocks[bid].instrs {
            let Instr::Assign { dst, src: s, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let s = canonical_value(fn_ir, *s);
            match src {
                None => src = Some(s),
                Some(prev) if canonical_value(fn_ir, prev) == s => {}
                Some(_) => return None,
            }
        }
        if let Some(ps) = preds.get(&bid) {
            for pred in ps {
                if lp.body.contains(pred) {
                    stack.push(*pred);
                }
            }
        }
    }
    src
}

pub(super) fn unwrap_vector_condition_value(fn_ir: &FnIR, root: ValueId) -> ValueId {
    let root = canonical_value(fn_ir, root);
    match &fn_ir.values[root].kind {
        ValueKind::Call { callee, args, .. }
            if matches!(callee.as_str(), "rr_truthy1" | "rr_bool") && !args.is_empty() =>
        {
            canonical_value(fn_ir, args[0])
        }
        _ => root,
    }
}

pub(super) fn is_comparison_op(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    )
}

#[derive(Clone, Copy)]
pub(super) struct PassthroughOriginPhiStep {
    phi: ValueId,
    phi_bb: BlockId,
    cond: ValueId,
    then_val: ValueId,
    then_bb: BlockId,
    else_val: ValueId,
    else_bb: BlockId,
}

#[derive(Clone, Copy)]
pub(super) struct PassthroughOriginPhiArms {
    pass_then: bool,
    prev_state_raw: Option<ValueId>,
    update_val: ValueId,
}

#[derive(Clone, Copy)]
pub(super) struct SequentialStateStep {
    phi: ValueId,
    cond_root: ValueId,
    update_val: ValueId,
    pass_then: bool,
}

pub(super) fn passthrough_origin_phi_step(
    fn_ir: &FnIR,
    phi: ValueId,
) -> Option<PassthroughOriginPhiStep> {
    passthrough_origin_phi_step_uncanonicalized(fn_ir, canonical_value(fn_ir, phi))
}

pub(super) fn passthrough_origin_phi_step_uncanonicalized(
    fn_ir: &FnIR,
    phi: ValueId,
) -> Option<PassthroughOriginPhiStep> {
    let ValueKind::Phi { args } = fn_ir.values[phi].kind.clone() else {
        return None;
    };
    let phi_bb = fn_ir.values[phi].phi_block?;
    let (_, cond, then_val, then_bb, else_val, else_bb) =
        find_conditional_phi_shape_with_blocks(fn_ir, phi, &args)?;
    Some(PassthroughOriginPhiStep {
        phi,
        phi_bb,
        cond,
        then_val,
        then_bb,
        else_val,
        else_bb,
    })
}

pub(super) fn trace_passthrough_origin_phi_step(
    fn_ir: &FnIR,
    label: &str,
    var: &str,
    step: PassthroughOriginPhiStep,
) {
    if !vectorize_trace_enabled() {
        return;
    }
    eprintln!(
        "   [{}] {} phi={} var={} bb={} cond={:?} then={:?}@{} else={:?}@{}",
        label,
        fn_ir.name,
        step.phi,
        var,
        step.phi_bb,
        fn_ir.values[canonical_value(fn_ir, step.cond)].kind,
        fn_ir.values[canonical_value(fn_ir, step.then_val)].kind,
        step.then_bb,
        fn_ir.values[canonical_value(fn_ir, step.else_val)].kind,
        step.else_bb
    );
    let mut seen = FxHashSet::default();
    trace_value_tree(fn_ir, step.cond, 6, &mut seen);
    let mut seen = FxHashSet::default();
    trace_value_tree(fn_ir, step.then_val, 6, &mut seen);
    let mut seen = FxHashSet::default();
    trace_value_tree(fn_ir, step.else_val, 6, &mut seen);
    trace_block_instrs(fn_ir, step.then_bb, 6);
    trace_block_instrs(fn_ir, step.else_bb, 6);
    eprintln!(
        "      block-last-assign then={:?} else={:?}",
        last_assign_to_var_in_block(fn_ir, step.then_bb, var),
        last_assign_to_var_in_block(fn_ir, step.else_bb, var)
    );
}

pub(super) fn classify_passthrough_origin_phi_arms(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    step: PassthroughOriginPhiStep,
    var: &str,
) -> Option<PassthroughOriginPhiArms> {
    let then_load = is_passthrough_load_of_var(fn_ir, step.then_val, var);
    let else_load = is_passthrough_load_of_var(fn_ir, step.else_val, var);
    let then_local_assign = if then_load {
        last_assign_to_var_in_block(fn_ir, step.then_bb, var)
    } else {
        None
    };
    let else_local_assign = if else_load {
        last_assign_to_var_in_block(fn_ir, step.else_bb, var)
    } else {
        None
    };
    let then_reaching_assign = if then_load && then_local_assign.is_none() {
        unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, step.then_bb)
    } else {
        None
    };
    let else_reaching_assign = if else_load && else_local_assign.is_none() {
        unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, step.else_bb)
    } else {
        None
    };
    let then_prior_state = is_prior_origin_phi_state(fn_ir, step.then_val, var, step.phi_bb);
    let else_prior_state = is_prior_origin_phi_state(fn_ir, step.else_val, var, step.phi_bb);
    let then_passthrough = then_prior_state || (then_load && then_local_assign.is_none());
    let else_passthrough = else_prior_state || (else_load && else_local_assign.is_none());

    if then_passthrough && !else_passthrough {
        Some(PassthroughOriginPhiArms {
            pass_then: true,
            prev_state_raw: then_prior_state
                .then_some(canonical_value(fn_ir, step.then_val))
                .or(then_reaching_assign),
            update_val: else_local_assign.unwrap_or_else(|| canonical_value(fn_ir, step.else_val)),
        })
    } else if else_passthrough && !then_passthrough {
        Some(PassthroughOriginPhiArms {
            pass_then: false,
            prev_state_raw: else_prior_state
                .then_some(canonical_value(fn_ir, step.else_val))
                .or(else_reaching_assign),
            update_val: then_local_assign.unwrap_or_else(|| canonical_value(fn_ir, step.then_val)),
        })
    } else {
        None
    }
}

pub(super) fn passthrough_origin_phi_prev_source(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    step: PassthroughOriginPhiStep,
    prev_state_raw: Option<ValueId>,
) -> Option<ValueId> {
    if let Some(prev_raw) = prev_state_raw {
        return Some(
            collapse_prior_origin_phi_state(
                fn_ir,
                prev_raw,
                var,
                step.phi_bb,
                &mut FxHashSet::default(),
            )
            .unwrap_or(prev_raw),
        );
    }

    if let Some(prev_phi) = nearest_origin_phi_value_in_loop(fn_ir, lp, var, step.phi_bb)
        .filter(|src| canonical_value(fn_ir, *src) != step.phi)
    {
        return Some(
            collapse_prior_origin_phi_state(
                fn_ir,
                prev_phi,
                var,
                step.phi_bb,
                &mut FxHashSet::default(),
            )
            .unwrap_or(prev_phi),
        );
    }

    unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, step.phi_bb)
}

pub(super) fn resolve_non_phi_prev_source_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    step: PassthroughOriginPhiStep,
    source: ValueId,
) -> Option<ValueId> {
    let source = canonical_value(fn_ir, source);
    if !value_depends_on(fn_ir, source, step.phi, &mut FxHashSet::default()) {
        return Some(source);
    }
    unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, step.phi_bb).and_then(|reaching| {
        let reaching = canonical_value(fn_ir, reaching);
        (!value_depends_on(fn_ir, reaching, step.phi, &mut FxHashSet::default()))
            .then_some(reaching)
    })
}

pub(super) fn passthrough_origin_phi_condition_parts(
    fn_ir: &FnIR,
    step: PassthroughOriginPhiStep,
) -> Option<(ValueId, BinOp, ValueId, ValueId)> {
    let cond_root = unwrap_vector_condition_value(fn_ir, step.cond);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[cond_root].kind.clone() else {
        return None;
    };
    if !is_comparison_op(op) {
        return None;
    }
    Some((cond_root, op, lhs, rhs))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_passthrough_origin_phi_state(
    fn_ir: &mut FnIR,
    phi: ValueId,
    var: &str,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    if var.starts_with(".arg_") && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var) {
        let load = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Load {
                var: var.to_string(),
            },
            fn_ir.values[phi].span,
            fn_ir.values[phi].facts,
        );
        return Some(load);
    }
    let Some(step) = passthrough_origin_phi_step(fn_ir, phi) else {
        trace_materialize_reject(fn_ir, phi, "passthrough-origin-phi: root is not phi");
        return None;
    };
    trace_passthrough_origin_phi_step(fn_ir, "vec-materialize", var, step);

    let Some(arms) = classify_passthrough_origin_phi_arms(fn_ir, lp, step, var) else {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            "passthrough-origin-phi: could not classify pass/update arms",
        );
        return None;
    };

    let Some(prev_source) =
        passthrough_origin_phi_prev_source(fn_ir, lp, var, step, arms.prev_state_raw)
    else {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            "passthrough-origin-phi: no reaching seed assign",
        );
        return None;
    };

    let Some(prev_source) = resolve_non_phi_prev_source_in_loop(fn_ir, lp, var, step, prev_source)
    else {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            &format!(
                "passthrough-origin-phi: prev_source still depends on phi ({:?})",
                fn_ir.values[canonical_value(fn_ir, prev_source)].kind
            ),
        );
        return None;
    };

    let prev_state = materialize_vector_expr(
        fn_ir,
        prev_source,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        allow_any_base,
        require_safe_index,
    )?;

    let Some((cond_root, op, lhs, rhs)) = passthrough_origin_phi_condition_parts(fn_ir, step)
    else {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            "passthrough-origin-phi: condition is not a binary compare",
        );
        return None;
    };
    let prev_cmp_raw = arms.prev_state_raw.map(|src| canonical_value(fn_ir, src));
    let materialize_cmp_side =
        |operand: ValueId,
         fn_ir: &mut FnIR,
         memo: &mut FxHashMap<ValueId, ValueId>,
         interner: &mut FxHashMap<MaterializedExprKey, ValueId>| {
            let operand = canonical_value(fn_ir, operand);
            if is_passthrough_load_of_var(fn_ir, operand, var)
                || prev_cmp_raw.is_some_and(|raw| raw == operand)
            {
                Some(prev_state)
            } else {
                materialize_vector_expr(
                    fn_ir,
                    operand,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    allow_any_base,
                    require_safe_index,
                )
            }
        };
    let cmp_lhs = materialize_cmp_side(lhs, fn_ir, memo, interner)?;
    let cmp_rhs = materialize_cmp_side(rhs, fn_ir, memo, interner)?;
    if cmp_lhs == prev_state && cmp_rhs == prev_state {
        trace_materialize_reject(
            fn_ir,
            step.phi,
            "passthrough-origin-phi: comparison collapsed to same prev state on both sides",
        );
        return None;
    }
    let cond_vec = intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Binary {
            op,
            lhs: cmp_lhs,
            rhs: cmp_rhs,
        },
        fn_ir.values[cond_root].span,
        fn_ir.values[cond_root].facts,
    );
    let update_vec = materialize_vector_expr(
        fn_ir,
        arms.update_val,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        allow_any_base,
        require_safe_index,
    )?;
    let then_vec = if arms.pass_then {
        prev_state
    } else {
        update_vec
    };
    let else_vec = if arms.pass_then {
        update_vec
    } else {
        prev_state
    };
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_vec, then_vec, else_vec],
            names: vec![None, None, None],
        },
        fn_ir.values[step.phi].span,
        fn_ir.values[step.phi].facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_index_base(
    fn_ir: &mut FnIR,
    base: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    let base = canonical_value(fn_ir, base);
    if let ValueKind::Phi { args } = fn_ir.values[base].kind.clone()
        && fn_ir.values[base].origin_var.is_none()
    {
        let outside_args: Vec<ValueId> = args
            .iter()
            .filter_map(|(arg, bid)| {
                if lp.body.contains(bid) {
                    None
                } else {
                    Some(*arg)
                }
            })
            .collect();
        if outside_args.len() == 1 {
            return materialize_vector_expr_impl(
                fn_ir,
                outside_args[0],
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
            );
        }
    }
    Some(resolve_materialized_value(fn_ir, base))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_binary(
    fn_ir: &mut FnIR,
    root: ValueId,
    op: BinOp,
    lhs: ValueId,
    rhs: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let lhs_vec = recurse(
        fn_ir,
        lhs,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    let rhs_vec = recurse(
        fn_ir,
        rhs,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if lhs_vec == lhs && rhs_vec == rhs {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Binary {
            op,
            lhs: lhs_vec,
            rhs: rhs_vec,
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_unary(
    fn_ir: &mut FnIR,
    root: ValueId,
    op: UnaryOp,
    rhs: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let rhs_vec = recurse(
        fn_ir,
        rhs,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if rhs_vec == rhs {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Unary { op, rhs: rhs_vec },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_call(
    fn_ir: &mut FnIR,
    root: ValueId,
    callee: String,
    args: Vec<ValueId>,
    names: Vec<Option<String>>,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let mut new_args = Vec::with_capacity(args.len());
    let mut changed = false;
    for arg in &args {
        let next_arg = recurse(
            fn_ir,
            *arg,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        changed |= next_arg != *arg;
        new_args.push(next_arg);
    }

    let rewrite_runtime_read = is_runtime_vector_read_call(&callee, new_args.len())
        && args
            .get(1)
            .copied()
            .is_some_and(|idx_arg| expr_has_iv_dependency(fn_ir, idx_arg, iv_phi));
    if rewrite_runtime_read
        && let Some(raw_idx) = args.get(1).copied().and_then(|idx_arg| {
            floor_like_index_source(fn_ir, idx_arg)
                .filter(|inner| expr_has_iv_dependency(fn_ir, *inner, iv_phi))
        })
    {
        let raw_idx_vec = recurse(
            fn_ir,
            raw_idx,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        if raw_idx_vec != new_args[1] {
            new_args[1] = raw_idx_vec;
            changed = true;
        }
        if !is_int_index_vector_value(fn_ir, new_args[1]) {
            let floor_idx_vec = intern_materialized_value(
                fn_ir,
                interner,
                ValueKind::Call {
                    callee: "rr_index_vec_floor".to_string(),
                    args: vec![new_args[1]],
                    names: vec![None],
                },
                span,
                facts,
            );
            if floor_idx_vec != new_args[1] {
                new_args[1] = floor_idx_vec;
                changed = true;
            }
        }
    }

    if !changed && !rewrite_runtime_read {
        return Some(root);
    }

    let (out_callee, out_names) = if rewrite_runtime_read {
        ("rr_index1_read_vec".to_string(), vec![None; new_args.len()])
    } else {
        (callee, names)
    };
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: out_callee,
            args: new_args,
            names: out_names,
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_intrinsic(
    fn_ir: &mut FnIR,
    root: ValueId,
    op: IntrinsicOp,
    args: Vec<ValueId>,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let mut new_args = Vec::with_capacity(args.len());
    let mut changed = false;
    for arg in args {
        let next_arg = recurse(
            fn_ir,
            arg,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        changed |= next_arg != arg;
        new_args.push(next_arg);
    }
    if !changed {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Intrinsic { op, args: new_args },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_index1d(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    idx: ValueId,
    is_safe: bool,
    is_na_safe: bool,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, base) {
        trace_materialize_reject(fn_ir, root, "Index1D base is not loop-compatible");
        return None;
    }

    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        let base_ref = materialize_vector_index_base(
            fn_ir,
            base,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        if is_safe
            && is_na_safe
            && let Some(iv) = lp.iv.as_ref()
            && loop_covers_whole_destination(lp, fn_ir, base, iv.init_val)
        {
            return Some(base_ref);
        }

        let mut direct_idx = idx_vec;
        if !is_int_index_vector_value(fn_ir, direct_idx) {
            direct_idx = intern_materialized_value(
                fn_ir,
                interner,
                ValueKind::Call {
                    callee: "rr_index_vec_floor".to_string(),
                    args: vec![direct_idx],
                    names: vec![None],
                },
                span,
                facts,
            );
        }
        return Some(intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Call {
                callee: "rr_index1_read_vec".to_string(),
                args: vec![base_ref, direct_idx],
                names: vec![None, None],
            },
            span,
            facts,
        ));
    }

    if !expr_has_iv_dependency(fn_ir, idx, iv_phi) {
        trace_materialize_reject(fn_ir, root, "Index1D index is not vectorizable");
        return None;
    }

    let floor_src = if is_safe && is_na_safe {
        None
    } else {
        floor_like_index_source(fn_ir, idx)
            .filter(|inner| expr_has_iv_dependency(fn_ir, *inner, iv_phi))
    };
    let idx_src = floor_src.unwrap_or(idx);
    let mut materialized_idx_vec = recurse(
        fn_ir,
        idx_src,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if floor_src.is_some() && !is_int_index_vector_value(fn_ir, materialized_idx_vec) {
        materialized_idx_vec = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Call {
                callee: "rr_index_vec_floor".to_string(),
                args: vec![materialized_idx_vec],
                names: vec![None],
            },
            span,
            facts,
        );
    }

    let base_ref = materialize_vector_index_base(
        fn_ir,
        base,
        iv_phi,
        materialized_idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if is_safe && is_na_safe {
        return Some(intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Index1D {
                base: base_ref,
                idx: materialized_idx_vec,
                is_safe: true,
                is_na_safe: true,
            },
            span,
            facts,
        ));
    }

    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_gather".to_string(),
            args: vec![base_ref, materialized_idx_vec],
            names: vec![None, None],
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_index3d(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    i: ValueId,
    j: ValueId,
    k: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    fn materialize_access_operand(
        fn_ir: &mut FnIR,
        operand: VectorAccessOperand3D,
        span: crate::utils::Span,
        facts: crate::mir::flow::Facts,
        iv_phi: ValueId,
        idx_vec: ValueId,
        lp: &LoopInfo,
        memo: &mut FxHashMap<ValueId, ValueId>,
        interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
        visiting: &mut FxHashSet<ValueId>,
        allow_any_base: bool,
        require_safe_index: bool,
        recurse: MaterializeRecurseFn,
    ) -> Option<ValueId> {
        match operand {
            VectorAccessOperand3D::Scalar(value) => Some(
                materialize_loop_invariant_scalar_expr(fn_ir, value, iv_phi, lp, memo, interner)
                    .unwrap_or_else(|| resolve_materialized_value(fn_ir, value)),
            ),
            VectorAccessOperand3D::Vector(value) => {
                let mut materialized = if is_iv_equivalent(fn_ir, value, iv_phi) {
                    idx_vec
                } else {
                    recurse(
                        fn_ir,
                        value,
                        iv_phi,
                        idx_vec,
                        lp,
                        memo,
                        interner,
                        visiting,
                        allow_any_base,
                        require_safe_index,
                    )?
                };
                if !is_int_index_vector_value(fn_ir, materialized) {
                    materialized = intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Call {
                            callee: "rr_index_vec_floor".to_string(),
                            args: vec![materialized],
                            names: vec![None],
                        },
                        span,
                        facts,
                    );
                }
                Some(materialized)
            }
        }
    }

    if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, base) {
        trace_materialize_reject(fn_ir, root, "Index3D base is not loop-compatible");
        return None;
    }
    let base_ref = materialize_vector_index_base(
        fn_ir,
        base,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if let Some((axis, dep_idx, fixed_a, fixed_b)) =
        classify_3d_vector_access_axis(fn_ir, base, i, j, k, iv_phi)
    {
        let fixed_a =
            materialize_loop_invariant_scalar_expr(fn_ir, fixed_a, iv_phi, lp, memo, interner)
                .unwrap_or_else(|| resolve_materialized_value(fn_ir, fixed_a));
        let fixed_b =
            materialize_loop_invariant_scalar_expr(fn_ir, fixed_b, iv_phi, lp, memo, interner)
                .unwrap_or_else(|| resolve_materialized_value(fn_ir, fixed_b));
        let idx_vec_arg = if is_iv_equivalent(fn_ir, dep_idx, iv_phi) {
            idx_vec
        } else {
            recurse(
                fn_ir,
                dep_idx,
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
            )?
        };
        let callee = match axis {
            Axis3D::Dim1 => "rr_dim1_read_values",
            Axis3D::Dim2 => "rr_dim2_read_values",
            Axis3D::Dim3 => "rr_dim3_read_values",
        };
        return Some(intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Call {
                callee: callee.to_string(),
                args: vec![base_ref, fixed_a, fixed_b, idx_vec_arg],
                names: vec![None, None, None, None],
            },
            span,
            facts,
        ));
    }

    let Some(pattern) = classify_3d_general_vector_access(fn_ir, base, i, j, k, iv_phi) else {
        trace_materialize_reject(fn_ir, root, "Index3D is not general vectorizable gather");
        return None;
    };
    let i_arg = materialize_access_operand(
        fn_ir,
        pattern.i,
        span,
        facts,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )?;
    let j_arg = materialize_access_operand(
        fn_ir,
        pattern.j,
        span,
        facts,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )?;
    let k_arg = materialize_access_operand(
        fn_ir,
        pattern.k,
        span,
        facts,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )?;
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_array3_gather_values".to_string(),
            args: vec![base_ref, i_arg, j_arg, k_arg],
            names: vec![None, None, None, None],
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_len(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let next_base = recurse(
        fn_ir,
        base,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if next_base == base {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Len { base: next_base },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_range(
    fn_ir: &mut FnIR,
    root: ValueId,
    start: ValueId,
    end: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let next_start = recurse(
        fn_ir,
        start,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    let next_end = recurse(
        fn_ir,
        end,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if next_start == start && next_end == end {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Range {
            start: next_start,
            end: next_end,
        },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_indices(
    fn_ir: &mut FnIR,
    root: ValueId,
    base: ValueId,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let next_base = recurse(
        fn_ir,
        base,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )?;
    if next_base == base {
        return Some(root);
    }
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Indices { base: next_base },
        span,
        facts,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_leaf_or_access_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => Some(root),
        ValueKind::Load { var } => materialize_vector_load(
            fn_ir,
            root,
            &var,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => materialize_vector_index1d(
            fn_ir,
            root,
            base,
            idx,
            is_safe,
            is_na_safe,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Phi { args } => materialize_vector_phi(
            fn_ir,
            root,
            args,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Index2D { .. } => {
            trace_materialize_reject(fn_ir, root, "Index2D is not vector-materializable");
            None
        }
        ValueKind::Index3D { base, i, j, k } => materialize_vector_index3d(
            fn_ir,
            root,
            base,
            i,
            j,
            k,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_invocation_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Call {
            callee,
            args,
            names,
        } => materialize_vector_call(
            fn_ir,
            root,
            callee,
            args,
            names,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Intrinsic { op, args } => materialize_vector_intrinsic(
            fn_ir,
            root,
            op,
            args,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_arithmetic_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Binary { op, lhs, rhs } => materialize_vector_binary(
            fn_ir,
            root,
            op,
            lhs,
            rhs,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Unary { op, rhs } => materialize_vector_unary(
            fn_ir,
            root,
            op,
            rhs,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_shape_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    match fn_ir.values[root].kind.clone() {
        ValueKind::Len { base } => materialize_vector_len(
            fn_ir,
            root,
            base,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Range { start, end } => materialize_vector_range(
            fn_ir,
            root,
            start,
            end,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        ValueKind::Indices { base } => materialize_vector_indices(
            fn_ir,
            root,
            base,
            span,
            facts,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        ),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_structural_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    materialize_vector_arithmetic_node(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )
    .or_else(|| {
        materialize_vector_shape_node(
            fn_ir,
            root,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        )
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_expr_node(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    materialize_vector_leaf_or_access_node(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
        recurse,
    )
    .or_else(|| {
        materialize_vector_invocation_node(
            fn_ir,
            root,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        )
    })
    .or_else(|| {
        materialize_vector_structural_node(
            fn_ir,
            root,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            recurse,
        )
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_expr(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    materialize_vector_expr_impl(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        &mut FxHashSet::default(),
        allow_any_base,
        require_safe_index,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_vector_expr_impl(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    #[allow(clippy::only_used_in_recursion)]
    fn rec(
        fn_ir: &mut FnIR,
        root: ValueId,
        iv_phi: ValueId,
        idx_vec: ValueId,
        lp: &LoopInfo,
        memo: &mut FxHashMap<ValueId, ValueId>,
        interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
        visiting: &mut FxHashSet<ValueId>,
        allow_any_base: bool,
        require_safe_index: bool,
    ) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        let _ = require_safe_index;
        if let Some(v) = memo.get(&root) {
            return Some(*v);
        }
        // Guard against pathological phi/load cycles that stay syntactically
        // productive enough to evade the simple `visiting` back-edge check.
        // In those cases we want a clean vectorization reject, not a stack overflow.
        if visiting.len() > 256 {
            trace_materialize_reject(fn_ir, root, "materialize_vector_expr recursion depth limit");
            return None;
        }
        if !visiting.insert(root) {
            trace_materialize_reject(fn_ir, root, "cycle in materialize_vector_expr");
            return None;
        }
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            memo.insert(root, idx_vec);
            visiting.remove(&root);
            return Some(idx_vec);
        }
        if is_scalar_broadcast_value(fn_ir, root) && !expr_has_iv_dependency(fn_ir, root, iv_phi) {
            memo.insert(root, root);
            visiting.remove(&root);
            return Some(root);
        }

        let out = materialize_vector_expr_node(
            fn_ir,
            root,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
            rec,
        )?;

        memo.insert(root, out);
        visiting.remove(&root);
        Some(out)
    }

    rec(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        visiting,
        allow_any_base,
        require_safe_index,
    )
}

pub(super) fn materialize_passthrough_origin_phi_state_scalar(
    fn_ir: &mut FnIR,
    phi: ValueId,
    var: &str,
    iv_phi: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    if var.starts_with(".arg_") && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var) {
        let load = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Load {
                var: var.to_string(),
            },
            fn_ir.values[phi].span,
            fn_ir.values[phi].facts,
        );
        return Some(load);
    }
    let trace_enabled = vectorize_trace_enabled();
    let step = passthrough_origin_phi_step(fn_ir, phi)?;
    let depends_on_phi = |vid: ValueId, fn_ir: &FnIR| {
        value_depends_on(fn_ir, vid, step.phi, &mut FxHashSet::default())
    };
    trace_passthrough_origin_phi_step(fn_ir, "vec-scalar-phi", var, step);

    let Some(arms) = classify_passthrough_origin_phi_arms(fn_ir, lp, step, var) else {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: could not classify pass/update (then_passthrough=false else_passthrough=false then_prior=false else_prior=false)",
                fn_ir.name, step.phi, var
            );
        }
        return None;
    };
    if trace_enabled {
        eprintln!(
            "      classified pass_then={} prev_raw={:?} update={:?}",
            arms.pass_then,
            arms.prev_state_raw,
            fn_ir.values[canonical_value(fn_ir, arms.update_val)].kind
        );
    }

    let prev_source =
        passthrough_origin_phi_prev_source(fn_ir, lp, var, step, arms.prev_state_raw)?;

    let prev_state = {
        let Some(prev_raw) = resolve_non_phi_prev_source_in_loop(fn_ir, lp, var, step, prev_source)
        else {
            if trace_enabled {
                eprintln!(
                    "   [vec-scalar-phi] {} phi={} var={} reject: prev_raw still depends on phi ({:?})",
                    fn_ir.name,
                    step.phi,
                    var,
                    fn_ir.values[canonical_value(fn_ir, prev_source)].kind
                );
            }
            return None;
        };
        materialize_loop_invariant_scalar_expr(fn_ir, prev_raw, iv_phi, lp, memo, interner)?
    };

    let Some((cond_root, op, lhs, rhs)) = passthrough_origin_phi_condition_parts(fn_ir, step)
    else {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: condition not binary ({:?})",
                fn_ir.name,
                step.phi,
                var,
                fn_ir.values[unwrap_vector_condition_value(fn_ir, step.cond)].kind
            );
        }
        return None;
    };
    let prev_cmp_raw = arms.prev_state_raw.map(|src| canonical_value(fn_ir, src));
    let materialize_cmp_side =
        |operand: ValueId,
         fn_ir: &mut FnIR,
         memo: &mut FxHashMap<ValueId, ValueId>,
         interner: &mut FxHashMap<MaterializedExprKey, ValueId>| {
            let operand = canonical_value(fn_ir, operand);
            if is_passthrough_load_of_var(fn_ir, operand, var)
                || prev_cmp_raw.is_some_and(|raw| raw == operand)
            {
                Some(prev_state)
            } else {
                if depends_on_phi(operand, fn_ir) {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-scalar-phi] {} phi={} var={} reject: cmp operand depends on phi ({:?})",
                            fn_ir.name, step.phi, var, fn_ir.values[operand].kind
                        );
                    }
                    return None;
                }
                materialize_loop_invariant_scalar_expr(fn_ir, operand, iv_phi, lp, memo, interner)
            }
        };
    let cmp_lhs = materialize_cmp_side(lhs, fn_ir, memo, interner)?;
    let cmp_rhs = materialize_cmp_side(rhs, fn_ir, memo, interner)?;
    if cmp_lhs == prev_state && cmp_rhs == prev_state {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: comparison collapsed to prev_state on both sides",
                fn_ir.name, step.phi, var
            );
        }
        return None;
    }
    let cond_scalar = intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Binary {
            op,
            lhs: cmp_lhs,
            rhs: cmp_rhs,
        },
        fn_ir.values[cond_root].span,
        fn_ir.values[cond_root].facts,
    );
    if depends_on_phi(arms.update_val, fn_ir) {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: update depends on phi ({:?})",
                fn_ir.name,
                step.phi,
                var,
                fn_ir.values[canonical_value(fn_ir, arms.update_val)].kind
            );
        }
        return None;
    }
    let update_scalar =
        materialize_loop_invariant_scalar_expr(fn_ir, arms.update_val, iv_phi, lp, memo, interner)?;
    let then_scalar = if arms.pass_then {
        prev_state
    } else {
        update_scalar
    };
    let else_scalar = if arms.pass_then {
        update_scalar
    } else {
        prev_state
    };
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_scalar, then_scalar, else_scalar],
            names: vec![None, None, None],
        },
        fn_ir.values[step.phi].span,
        fn_ir.values[step.phi].facts,
    ))
}

pub(super) fn materialize_loop_invariant_scalar_expr(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    fn rec(
        fn_ir: &mut FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        memo: &mut FxHashMap<ValueId, ValueId>,
        interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
        visiting: &mut FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        if let Some(v) = memo.get(&root) {
            return Some(*v);
        }
        if expr_has_iv_dependency(fn_ir, root, iv_phi) {
            return None;
        }
        if !visiting.insert(root) {
            return None;
        }

        let span = fn_ir.values[root].span;
        let facts = fn_ir.values[root].facts;
        let out = match fn_ir.values[root].kind.clone() {
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => root,
            ValueKind::Load { var } => {
                if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, &var) {
                    rec(fn_ir, src, iv_phi, lp, memo, interner, visiting)?
                } else if let Some(src) = merged_assign_source_in_loop(fn_ir, lp, &var) {
                    rec(fn_ir, src, iv_phi, lp, memo, interner, visiting)?
                } else if let Some(src) =
                    unique_origin_phi_value_in_loop(fn_ir, lp, &var).or_else(|| {
                        nearest_origin_phi_value_in_loop(fn_ir, lp, &var, fn_ir.blocks.len())
                    })
                {
                    rec(fn_ir, src, iv_phi, lp, memo, interner, visiting)?
                } else if has_non_passthrough_assignment_in_loop(fn_ir, lp, &var) {
                    return None;
                } else {
                    root
                }
            }
            ValueKind::Unary { op, rhs } => {
                let rhs_v = rec(fn_ir, rhs, iv_phi, lp, memo, interner, visiting)?;
                if rhs_v == rhs {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Unary { op, rhs: rhs_v },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Binary { op, lhs, rhs } => {
                let lhs_v = rec(fn_ir, lhs, iv_phi, lp, memo, interner, visiting)?;
                let rhs_v = rec(fn_ir, rhs, iv_phi, lp, memo, interner, visiting)?;
                if lhs_v == lhs && rhs_v == rhs {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Binary {
                            op,
                            lhs: lhs_v,
                            rhs: rhs_v,
                        },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                let mut new_args = Vec::with_capacity(args.len());
                let mut changed = false;
                for arg in &args {
                    let mapped = rec(fn_ir, *arg, iv_phi, lp, memo, interner, visiting)?;
                    changed |= mapped != *arg;
                    new_args.push(mapped);
                }
                if !changed {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Call {
                            callee,
                            args: new_args,
                            names,
                        },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Intrinsic { op, args } => {
                let mut new_args = Vec::with_capacity(args.len());
                let mut changed = false;
                for arg in args {
                    let mapped = rec(fn_ir, arg, iv_phi, lp, memo, interner, visiting)?;
                    changed |= mapped != arg;
                    new_args.push(mapped);
                }
                if !changed {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Intrinsic { op, args: new_args },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Phi { args } => {
                if let Some(var) = fn_ir.values[root].origin_var.clone()
                    && let Some(v) = materialize_passthrough_origin_phi_state_scalar(
                        fn_ir, root, &var, iv_phi, lp, memo, interner,
                    )
                {
                    v
                } else if phi_loads_same_var(fn_ir, &args) {
                    rec(fn_ir, args[0].0, iv_phi, lp, memo, interner, visiting)?
                } else if let Some((cond, then_val, else_val)) =
                    find_conditional_phi_shape(fn_ir, root, &args)
                {
                    let cond_v = rec(fn_ir, cond, iv_phi, lp, memo, interner, visiting)?;
                    let then_v = rec(fn_ir, then_val, iv_phi, lp, memo, interner, visiting)?;
                    let else_v = rec(fn_ir, else_val, iv_phi, lp, memo, interner, visiting)?;
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Call {
                            callee: "rr_ifelse_strict".to_string(),
                            args: vec![cond_v, then_v, else_v],
                            names: vec![None, None, None],
                        },
                        span,
                        facts,
                    )
                } else {
                    let mut picked: Option<ValueId> = None;
                    for (arg, _) in args {
                        let mapped = rec(fn_ir, arg, iv_phi, lp, memo, interner, visiting)?;
                        match picked {
                            None => picked = Some(mapped),
                            Some(prev)
                                if canonical_value(fn_ir, prev)
                                    == canonical_value(fn_ir, mapped) => {}
                            Some(_) => return None,
                        }
                    }
                    picked?
                }
            }
            ValueKind::Len { base } => {
                let base_v = rec(fn_ir, base, iv_phi, lp, memo, interner, visiting)?;
                if base_v == base {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Len { base: base_v },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Range { .. }
            | ValueKind::Indices { .. }
            | ValueKind::Index1D { .. }
            | ValueKind::Index2D { .. }
            | ValueKind::Index3D { .. } => return None,
        };

        memo.insert(root, out);
        visiting.remove(&root);
        Some(out)
    }

    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        memo,
        interner,
        &mut FxHashSet::default(),
    )
}
