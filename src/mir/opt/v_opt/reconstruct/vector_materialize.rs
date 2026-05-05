use super::*;

pub(in crate::mir::opt::v_opt) fn reject_unmaterialized_loop_load(
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

pub(in crate::mir::opt::v_opt) fn materialize_vector_load(
    fn_ir: &mut FnIR,
    root: ValueId,
    var: &str,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let use_bb = value_use_block_in_loop(fn_ir, ctx.lp, root);
    if let Some(src) = unique_assign_source_in_loop(fn_ir, ctx.lp, var) {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via unique-assign {:?}",
                fn_ir.name, var, src
            );
        }
        return recurse_materialized_load_source(fn_ir, root, src, ctx);
    }

    if let Some(src) = merged_assign_source_in_loop(fn_ir, ctx.lp, var) {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via merged-assign {:?}",
                fn_ir.name, var, src
            );
        }
        return recurse_materialized_load_source(fn_ir, root, src, ctx);
    }

    if let Some((src, label)) =
        select_origin_phi_load_source(fn_ir, ctx.lp, var, use_bb, ctx.visiting)
    {
        if !expr_has_iv_dependency(fn_ir, src, ctx.iv_phi)
            && let Some(scalar_src) = materialize_loop_invariant_scalar_expr(
                fn_ir,
                src,
                ctx.iv_phi,
                ctx.lp,
                ctx.memo,
                ctx.interner,
            )
        {
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-materialize] {} load {} via {}-scalar {:?}",
                    fn_ir.name, var, label, scalar_src
                );
            }
            return Some(scalar_src);
        }
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via {} {:?}",
                fn_ir.name, var, label, src
            );
        }
        return recurse_materialized_load_source(fn_ir, root, src, ctx);
    }

    if let Some(use_bb) = use_bb {
        if let Some(origin_phi) = nearest_origin_phi_value_in_loop(fn_ir, ctx.lp, var, use_bb)
            .or_else(|| unique_origin_phi_value_in_loop(fn_ir, ctx.lp, var))
            && let Some(scalar_phi) = materialize_loop_invariant_scalar_expr(
                fn_ir,
                origin_phi,
                ctx.iv_phi,
                ctx.lp,
                ctx.memo,
                ctx.interner,
            )
        {
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-materialize] {} load {} via origin-phi-scalar {:?}",
                    fn_ir.name, var, scalar_phi
                );
            }
            return Some(scalar_phi);
        }
        if let Some(state_src) =
            materialize_independent_if_state_chain_for_load(fn_ir, root, var, use_bb, ctx)
        {
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-materialize] {} load {} via independent-if-state-chain",
                    fn_ir.name, var
                );
            }
            return Some(state_src);
        }
        if let Some((nearest_phi, phi_src)) =
            materialize_passthrough_origin_phi_for_load(fn_ir, var, use_bb, ctx)
        {
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
            return recurse_materialized_load_source(fn_ir, root, src, ctx);
        }

        return reject_unmaterialized_loop_load(
            fn_ir,
            root,
            ctx.lp,
            var,
            Some(use_bb),
            ctx.visiting,
        );
    }

    reject_unmaterialized_loop_load(fn_ir, root, ctx.lp, var, None, ctx.visiting)
}

pub(in crate::mir::opt::v_opt) fn fold_phi_seed_candidate(
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

pub(in crate::mir::opt::v_opt) fn materialize_conditional_phi_value(
    fn_ir: &mut FnIR,
    root: ValueId,
    args: &[(ValueId, BlockId)],
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    if fn_ir.values[root].phi_block == Some(ctx.lp.header)
        || !args.iter().all(|(_, b)| ctx.lp.body.contains(b))
    {
        return None;
    }
    let (cond, then_val, else_val) = find_conditional_phi_shape(fn_ir, root, args)?;
    for candidate in [then_val, else_val] {
        if let ValueKind::Load { var } = &fn_ir.values[canonical_value(fn_ir, candidate)].kind
            && has_non_passthrough_assignment_in_loop(fn_ir, ctx.lp, var)
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
            let arms = classify_passthrough_origin_phi_arms(fn_ir, ctx.lp, step, &var)?;
            let prev_source =
                passthrough_origin_phi_prev_source(fn_ir, ctx.lp, &var, step, arms.prev_state_raw)?;
            let prev_source =
                resolve_non_phi_prev_source_in_loop(fn_ir, ctx.lp, &var, step, prev_source)?;
            let prev_state = ctx.recurse(fn_ir, prev_source)?;
            Some((var, prev_state))
        });
    if expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        cond,
        ctx.iv_phi,
        &FxHashSet::default(),
        &mut FxHashSet::default(),
    ) {
        trace_materialize_reject(fn_ir, root, "conditional phi has non-vector-safe condition");
        return None;
    }
    let cond_vec = ctx.recurse_with_policy(fn_ir, cond, SAFE_INDEX_VECTOR_MATERIALIZE_POLICY)?;
    let then_vec = if let Some((var, prev_state)) = &passthrough_prev_state
        && is_passthrough_load_of_var(fn_ir, then_val, var)
    {
        *prev_state
    } else {
        ctx.recurse(fn_ir, then_val)?
    };
    let else_vec = if let Some((var, prev_state)) = &passthrough_prev_state
        && is_passthrough_load_of_var(fn_ir, else_val, var)
    {
        *prev_state
    } else {
        ctx.recurse(fn_ir, else_val)?
    };
    Some(intern_materialized_value(
        fn_ir,
        ctx.interner,
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_vec, then_vec, else_vec],
            names: vec![None, None, None],
        },
        span,
        facts,
    ))
}

pub(in crate::mir::opt::v_opt) fn materialize_uniform_phi_value(
    fn_ir: &mut FnIR,
    root: ValueId,
    args: Vec<(ValueId, BlockId)>,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    let mut picked: Option<ValueId> = None;
    for (arg, _) in args {
        let materialized = ctx.recurse(fn_ir, arg)?;
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

pub(in crate::mir::opt::v_opt) fn materialize_vector_phi(
    fn_ir: &mut FnIR,
    root: ValueId,
    args: Vec<(ValueId, BlockId)>,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    ctx: &mut VectorMaterializeCtx<'_>,
) -> Option<ValueId> {
    if args.is_empty()
        && let Some(var) = fn_ir.values[root].origin_var.clone()
        && !has_non_passthrough_assignment_in_loop(fn_ir, ctx.lp, &var)
    {
        let load =
            intern_materialized_value(fn_ir, ctx.interner, ValueKind::Load { var }, span, facts);
        ctx.memo.insert(root, load);
        ctx.visiting.remove(&root);
        return Some(load);
    }

    if let Some(seed) = fold_phi_seed_candidate(fn_ir, root, &args, ctx.lp) {
        let folded = ctx.recurse(fn_ir, seed)?;
        ctx.memo.insert(root, folded);
        ctx.visiting.remove(&root);
        return Some(folded);
    }

    if let Some(var) = phi_state_var(fn_ir, root) {
        if let Some(phi_vec) = materialize_independent_if_state_chain(fn_ir, root, root, &var, ctx)
        {
            ctx.memo.insert(root, phi_vec);
            ctx.visiting.remove(&root);
            return Some(phi_vec);
        }
        if let Some(phi_vec) = materialize_passthrough_origin_phi_state(fn_ir, root, &var, ctx) {
            ctx.memo.insert(root, phi_vec);
            ctx.visiting.remove(&root);
            return Some(phi_vec);
        }
    }

    if let Some(ifelse_val) =
        materialize_conditional_phi_value(fn_ir, root, &args, span, facts, ctx)
    {
        ctx.memo.insert(root, ifelse_val);
        ctx.visiting.remove(&root);
        return Some(ifelse_val);
    }

    materialize_uniform_phi_value(fn_ir, root, args, ctx)
}
