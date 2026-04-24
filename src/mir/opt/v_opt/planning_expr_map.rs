//! Expr-map and scatter-plan matchers.

use super::*;

pub(crate) fn match_expr_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut found: Option<(ValueId, ValueId, Axis3D, ValueId, ValueId)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } | Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    if found.is_some() {
                        return None;
                    }
                    let Some((axis, fixed_a, fixed_b)) =
                        classify_3d_map_axis(fn_ir, *base, *i, *j, *k, iv_phi)
                    else {
                        continue;
                    };
                    let expr = resolve_load_alias_value(fn_ir, *val);
                    if !is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false) {
                        continue;
                    }
                    if expr_has_non_vector_safe_call_in_vector_context(
                        fn_ir,
                        expr,
                        iv_phi,
                        user_call_whitelist,
                        &mut FxHashSet::default(),
                    ) {
                        continue;
                    }
                    if expr_has_unstable_loop_local_load(fn_ir, lp, expr) {
                        continue;
                    }
                    let dest = canonical_value(fn_ir, *base);
                    let allowed_dests: Vec<VarId> =
                        resolve_base_var(fn_ir, dest).into_iter().collect();
                    let Some(shadow_vars) =
                        collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)
                    else {
                        continue;
                    };
                    if !shadow_vars.is_empty() || expr_reads_base(fn_ir, expr, dest) {
                        continue;
                    }
                    found = Some((dest, expr, axis, fixed_a, fixed_b));
                }
            }
        }
    }

    let (dest, expr, axis, fixed_a, fixed_b) = found?;
    Some(VectorPlan::ExprMap3D {
        dest,
        expr,
        iv_phi,
        axis,
        fixed_a,
        fixed_b,
        start,
        end,
    })
}

pub(crate) enum ExprMapStoreCandidate {
    Standard {
        dest: ValueId,
        expr: ValueId,
    },
    Cube {
        dest: ValueId,
        expr: ValueId,
        cube: CubeSliceIndexInfo,
    },
}

pub(crate) struct ExprMapStoreSpec {
    pub(crate) base: ValueId,
    pub(crate) idx: ValueId,
    pub(crate) expr: ValueId,
    pub(crate) is_vector: bool,
}

pub(crate) fn is_canonical_expr_map_store_index(
    fn_ir: &FnIR,
    idx: ValueId,
    iv_phi: ValueId,
) -> bool {
    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        return true;
    }
    is_floor_like_iv_expr(
        fn_ir,
        idx,
        iv_phi,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
        0,
    )
}

pub(crate) fn trace_expr_map_non_canonical_store_reject(
    fn_ir: &FnIR,
    idx: ValueId,
    idx_ok: bool,
    iv_phi: ValueId,
) {
    if !vectorize_trace_enabled() {
        return;
    }
    let phi_detail = match &fn_ir.values[idx].kind {
        ValueKind::Phi { args } => {
            let parts: Vec<String> = args
                .iter()
                .map(|(v, b)| format!("b{}:{:?}", b, fn_ir.values[*v].kind))
                .collect();
            format!(" phi_args=[{}]", parts.join(" | "))
        }
        ValueKind::Load { var } => {
            let unique_src = unique_assign_source(fn_ir, var)
                .map(|src| format!("{:?}", fn_ir.values[src].kind))
                .unwrap_or_else(|| "none".to_string());
            format!(
                " load_var='{}' iv_origin={:?} unique_src={}",
                var,
                induction_origin_var(fn_ir, iv_phi),
                unique_src
            )
        }
        _ => String::new(),
    };
    eprintln!(
        "   [vec-expr-map] reject: non-canonical store index (fn={}, idx_ok={}, idx={:?}{})",
        fn_ir.name, idx_ok, fn_ir.values[idx].kind, phi_detail
    );
}

pub(crate) fn trace_expr_map_duplicate_store_reject() {
    if vectorize_trace_enabled() {
        eprintln!("   [vec-expr-map] reject: duplicate StoreIndex1D destination");
    }
}

pub(crate) fn validate_expr_map_rhs(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
) -> bool {
    let expr_iv_dependent = expr_has_iv_dependency(fn_ir, expr, iv_phi);
    let expr_ok = if expr_iv_dependent {
        is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false)
    } else {
        is_loop_invariant_scalar_expr(fn_ir, expr, iv_phi, user_call_whitelist)
    };
    if !expr_ok {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-expr-map] reject: rhs is neither vectorizable nor loop-invariant scalar"
            );
        }
        return false;
    }
    if expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        expr,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) {
        if vectorize_trace_enabled() {
            let rhs_detail = match &fn_ir.values[expr].kind {
                ValueKind::Binary { lhs, rhs, .. } => format!(
                    "lhs={:?} rhs={:?}",
                    fn_ir.values[*lhs].kind, fn_ir.values[*rhs].kind
                ),
                other => format!("{:?}", other),
            };
            eprintln!(
                "   [vec-expr-map] reject: rhs contains non-vector-safe call; rhs={:?}; detail={}",
                fn_ir.values[expr].kind, rhs_detail
            );
        }
        return false;
    }
    if expr_has_ambiguous_loop_local_load(fn_ir, lp, expr) {
        if vectorize_trace_enabled() {
            eprintln!("   [vec-expr-map] reject: rhs depends on ambiguous loop-local state");
        }
        return false;
    }
    if expr_reads_base_non_iv(fn_ir, expr, dest, iv_phi) {
        if vectorize_trace_enabled() {
            eprintln!("   [vec-expr-map] reject: loop-carried dependence on destination");
        }
        return false;
    }
    true
}

pub(crate) fn classify_expr_map_store_candidate(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
    store: ExprMapStoreSpec,
    iv_phi: ValueId,
) -> Option<ExprMapStoreCandidate> {
    let idx_ok = is_canonical_expr_map_store_index(fn_ir, store.idx, iv_phi);
    if store.is_vector {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-expr-map] reject: non-canonical store index (fn={}, idx_ok={}, idx={:?})",
                fn_ir.name, idx_ok, fn_ir.values[store.idx].kind
            );
        }
        return None;
    }

    let dest = if resolve_base_var(fn_ir, store.base).is_some() {
        store.base
    } else {
        canonical_value(fn_ir, store.base)
    };
    if !idx_ok {
        let Some(cube) =
            match_cube_slice_index_info(fn_ir, lp, user_call_whitelist, store.idx, iv_phi)
        else {
            trace_expr_map_non_canonical_store_reject(fn_ir, store.idx, idx_ok, iv_phi);
            return None;
        };
        if !validate_expr_map_rhs(fn_ir, lp, user_call_whitelist, dest, store.expr, iv_phi) {
            return None;
        }
        return Some(ExprMapStoreCandidate::Cube {
            dest,
            expr: store.expr,
            cube,
        });
    }

    if !validate_expr_map_rhs(fn_ir, lp, user_call_whitelist, dest, store.expr, iv_phi) {
        return None;
    }
    Some(ExprMapStoreCandidate::Standard {
        dest,
        expr: store.expr,
    })
}

pub(crate) fn build_expr_map_entries(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    iv_phi: ValueId,
    start: ValueId,
    pending_entries: Vec<(ValueId, ValueId)>,
) -> Option<Vec<ExprMapEntry>> {
    let allowed_dests: Vec<VarId> = pending_entries
        .iter()
        .filter_map(|(dest, _)| resolve_base_var(fn_ir, *dest))
        .collect();
    let mut entries = Vec::with_capacity(pending_entries.len());
    for (dest, expr) in pending_entries {
        let shadow_vars =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;
        entries.push(ExprMapEntry {
            dest,
            expr,
            whole_dest: loop_covers_whole_destination(lp, fn_ir, dest, start),
            shadow_vars,
        });
    }
    Some(entries)
}

pub(crate) fn finalize_expr_map_plan(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    found: Vec<(ValueId, ValueId)>,
    cube_candidate: Option<(ValueId, ValueId, CubeSliceIndexInfo)>,
) -> Option<VectorPlan> {
    if found.is_empty() {
        let (dest, expr, cube) = cube_candidate?;
        let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest).into_iter().collect();
        let shadow_vars =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;
        return Some(VectorPlan::CubeSliceExprMap {
            dest,
            expr,
            iv_phi,
            face: cube.face,
            row: cube.row,
            size: cube.size,
            ctx: cube.ctx,
            start,
            end,
            shadow_vars,
        });
    }
    if cube_candidate.is_some() {
        return None;
    }

    let mut entries = build_expr_map_entries(fn_ir, lp, iv_phi, start, found)?;
    if entries.len() == 1 {
        let entry = entries.remove(0);
        return Some(VectorPlan::ExprMap {
            dest: entry.dest,
            expr: entry.expr,
            iv_phi,
            start,
            end,
            whole_dest: entry.whole_dest,
            shadow_vars: entry.shadow_vars,
        });
    }
    Some(VectorPlan::MultiExprMap {
        entries,
        iv_phi,
        start,
        end,
    })
}

pub(crate) fn match_scatter_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let trace_enabled = vectorize_trace_enabled();
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let mut found: Option<(BlockId, ValueId, ValueId, ValueId)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-scatter] {} reject: saw Eval in loop body",
                            fn_ir.name
                        );
                    }
                    return None;
                }
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-scatter] {} reject: saw multi-dimensional store in loop body",
                            fn_ir.name
                        );
                    }
                    return None;
                }
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => {
                    if *is_vector || found.is_some() {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-scatter] {} reject: non-canonical store count/vector flag (is_vector={}, found_already={})",
                                fn_ir.name,
                                is_vector,
                                found.is_some()
                            );
                        }
                        return None;
                    }
                    found = Some((bid, canonical_value(fn_ir, *base), *idx, *val));
                }
            }
        }
    }

    let (store_bid, dest, idx, expr) = found?;
    let idx_root = resolve_match_alias_value(fn_ir, idx);
    let is_cube_scatter = matches!(
        &fn_ir.values[idx_root].kind,
        ValueKind::Call { callee, args, names }
            if callee == "rr_idx_cube_vec_i"
                && (args.len() == 4 || args.len() == 5)
                && names.iter().all(|n| n.is_none())
    );
    if store_bid != lp.latch && !is_cube_scatter {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: store not in latch (bb={} latch={})",
                fn_ir.name, store_bid, lp.latch
            );
        }
        return None;
    }
    if lp.body.len() > 2 && !is_cube_scatter {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: loop body too large for non-cube scatter (body_len={})",
                fn_ir.name,
                lp.body.len()
            );
        }
        return None;
    }
    if is_iv_equivalent(fn_ir, idx, iv_phi)
        || is_floor_like_iv_expr(
            fn_ir,
            idx,
            iv_phi,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
            0,
        )
    {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: index is canonical IV-equivalent",
                fn_ir.name
            );
        }
        return None;
    }
    if !expr_has_iv_dependency(fn_ir, idx, iv_phi) {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: index has no IV dependency ({:?})",
                fn_ir.name, fn_ir.values[idx_root].kind
            );
        }
        return None;
    }
    if !is_vectorizable_expr(fn_ir, idx, iv_phi, lp, true, false)
        || !is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false)
    {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: idx/expr not vectorizable (idx={:?} expr={:?})",
                fn_ir.name,
                fn_ir.values[idx_root].kind,
                fn_ir.values[canonical_value(fn_ir, expr)].kind
            );
        }
        return None;
    }
    if expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        idx,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) || expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        expr,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: idx/expr contains non-vector-safe call",
                fn_ir.name
            );
        }
        return None;
    }
    if expr_has_unstable_loop_local_load(fn_ir, lp, idx)
        || expr_has_unstable_loop_local_load(fn_ir, lp, expr)
    {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: idx/expr depends on unstable loop-local state",
                fn_ir.name
            );
        }
        return None;
    }
    if expr_reads_base_non_iv(fn_ir, expr, dest, iv_phi) {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: expr reads destination via non-IV access",
                fn_ir.name
            );
        }
        return None;
    }
    if expr_reads_base(fn_ir, expr, dest) {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: expr reads destination base",
                fn_ir.name
            );
        }
        return None;
    }
    Some(VectorPlan::ScatterExprMap {
        dest,
        idx,
        expr,
        iv_phi,
    })
}

pub(crate) fn match_scatter_expr_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let trace_enabled = vectorize_trace_enabled();
    enum Scatter3DMatch {
        SingleAxis {
            store_bid: BlockId,
            dest: ValueId,
            axis: Axis3D,
            fixed_a: ValueId,
            fixed_b: ValueId,
            idx: ValueId,
            expr: ValueId,
        },
        General {
            store_bid: BlockId,
            dest: ValueId,
            pattern: VectorAccessPattern3D,
            expr: ValueId,
        },
    }
    let mut found: Option<Scatter3DMatch> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } | Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    if found.is_some() {
                        return None;
                    }
                    let Some(pattern) =
                        classify_3d_general_vector_access(fn_ir, *base, *i, *j, *k, iv_phi)
                    else {
                        continue;
                    };
                    if let Some((axis, dep_idx, fixed_a, fixed_b)) =
                        classify_3d_vector_access_axis(fn_ir, *base, *i, *j, *k, iv_phi)
                    {
                        if is_iv_equivalent(fn_ir, dep_idx, iv_phi)
                            || is_floor_like_iv_expr(
                                fn_ir,
                                dep_idx,
                                iv_phi,
                                &mut FxHashSet::default(),
                                &mut FxHashSet::default(),
                                0,
                            )
                        {
                            continue;
                        }
                        found = Some(Scatter3DMatch::SingleAxis {
                            store_bid: bid,
                            dest: canonical_value(fn_ir, *base),
                            axis,
                            fixed_a,
                            fixed_b,
                            idx: dep_idx,
                            expr: *val,
                        });
                    } else {
                        found = Some(Scatter3DMatch::General {
                            store_bid: bid,
                            dest: canonical_value(fn_ir, *base),
                            pattern,
                            expr: *val,
                        });
                    }
                }
            }
        }
    }

    let (store_bid, dest, idx_reads, expr, single_axis, general_pattern) = match found? {
        Scatter3DMatch::SingleAxis {
            store_bid,
            dest,
            axis,
            fixed_a,
            fixed_b,
            idx,
            expr,
        } => (
            store_bid,
            dest,
            vec![idx],
            expr,
            Some((axis, fixed_a, fixed_b, idx)),
            None,
        ),
        Scatter3DMatch::General {
            store_bid,
            dest,
            pattern,
            expr,
        } => {
            let idx_reads = [pattern.i, pattern.j, pattern.k]
                .into_iter()
                .filter_map(|operand| match operand {
                    VectorAccessOperand3D::Scalar(_) => None,
                    VectorAccessOperand3D::Vector(value) => Some(value),
                })
                .collect();
            (store_bid, dest, idx_reads, expr, None, Some(pattern))
        }
    };
    if store_bid != lp.latch && lp.body.len() > 2 {
        return None;
    }
    if idx_reads.is_empty()
        || !idx_reads
            .iter()
            .all(|idx| expr_has_iv_dependency(fn_ir, *idx, iv_phi))
        || !idx_reads
            .iter()
            .all(|idx| is_vectorizable_expr(fn_ir, *idx, iv_phi, lp, true, false))
        || !is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false)
    {
        return None;
    }
    if idx_reads.iter().any(|idx| {
        expr_has_non_vector_safe_call_in_vector_context(
            fn_ir,
            *idx,
            iv_phi,
            user_call_whitelist,
            &mut FxHashSet::default(),
        )
    }) || expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        expr,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) {
        return None;
    }
    if idx_reads
        .iter()
        .any(|idx| expr_has_unstable_loop_local_load(fn_ir, lp, *idx))
        || expr_has_unstable_loop_local_load(fn_ir, lp, expr)
    {
        return None;
    }
    let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest).into_iter().collect();
    let shadow_vars = collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;
    if !shadow_vars.is_empty() || expr_reads_base(fn_ir, expr, dest) {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter3d] {} reject: expr reads destination or loop carries non-destination state",
                fn_ir.name
            );
        }
        return None;
    }

    if let Some((axis, fixed_a, fixed_b, idx)) = single_axis {
        Some(VectorPlan::ScatterExprMap3D {
            dest,
            axis,
            fixed_a,
            fixed_b,
            idx,
            expr,
            iv_phi,
        })
    } else {
        general_pattern.map(|pattern| VectorPlan::ScatterExprMap3DGeneral {
            dest,
            i: pattern.i,
            j: pattern.j,
            k: pattern.k,
            expr,
            iv_phi,
        })
    }
}

pub(crate) fn match_cube_slice_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let trace_enabled = vectorize_trace_enabled();
    let mut found: Option<(ValueId, ValueId, ValueId)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-cube-slice] {} reject: saw Eval in loop body",
                            fn_ir.name
                        );
                    }
                    return None;
                }
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-cube-slice] {} reject: saw multi-dimensional store in loop body",
                            fn_ir.name
                        );
                    }
                    return None;
                }
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => {
                    if *is_vector || found.is_some() {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-cube-slice] {} reject: non-canonical store count/vector flag (is_vector={}, found_already={})",
                                fn_ir.name,
                                is_vector,
                                found.is_some()
                            );
                        }
                        return None;
                    }
                    found = Some((canonical_value(fn_ir, *base), *idx, *val));
                }
            }
        }
    }

    let (dest, idx, expr) = found?;
    let cube = match_cube_slice_index_info(fn_ir, lp, user_call_whitelist, idx, iv_phi)?;

    let expr_iv_dependent = expr_has_iv_dependency(fn_ir, expr, iv_phi);
    let expr_ok = if expr_iv_dependent {
        is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false)
    } else {
        is_loop_invariant_scalar_expr(fn_ir, expr, iv_phi, user_call_whitelist)
    };
    if !expr_ok {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: expr is neither vectorizable nor loop-invariant scalar ({:?})",
                fn_ir.name, fn_ir.values[expr].kind
            );
        }
        return None;
    }
    if expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        expr,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: expr contains non-vector-safe call ({:?})",
                fn_ir.name, fn_ir.values[expr].kind
            );
        }
        return None;
    }
    if expr_reads_base_non_iv(fn_ir, expr, dest, iv_phi) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: expr reads destination via non-IV access",
                fn_ir.name
            );
        }
        return None;
    }
    let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest).into_iter().collect();
    let shadow_vars = collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;

    Some(VectorPlan::CubeSliceExprMap {
        dest,
        expr,
        iv_phi,
        face: cube.face,
        row: cube.row,
        size: cube.size,
        ctx: cube.ctx,
        start,
        end,
        shadow_vars,
    })
}

pub(crate) struct CubeSliceIndexInfo {
    face: ValueId,
    row: ValueId,
    size: ValueId,
    ctx: Option<ValueId>,
}

pub(crate) fn match_cube_slice_index_info(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
    idx: ValueId,
    iv_phi: ValueId,
) -> Option<CubeSliceIndexInfo> {
    let trace_enabled = vectorize_trace_enabled();
    let idx_root = resolve_match_alias_value(fn_ir, idx);
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values[idx_root].kind
    else {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: store index is not call ({:?})",
                fn_ir.name, fn_ir.values[idx_root].kind
            );
        }
        return None;
    };
    if callee != "rr_idx_cube_vec_i" || (args.len() != 4 && args.len() != 5) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: index call shape mismatch (callee={}, arity={})",
                fn_ir.name,
                callee,
                args.len()
            );
        }
        return None;
    }
    if names.iter().any(|n| n.is_some()) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: named args in rr_idx_cube_vec_i",
                fn_ir.name
            );
        }
        return None;
    }

    let y_arg = args[2];
    let y_ok = is_iv_equivalent(fn_ir, y_arg, iv_phi)
        || is_origin_var_iv_alias_in_loop(fn_ir, lp, y_arg, iv_phi)
        || is_floor_like_iv_expr(
            fn_ir,
            y_arg,
            iv_phi,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
            0,
        );
    if !y_ok {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: y arg is not IV-equivalent (iv={:?}, iv_origin={:?}, y={:?}, y_origin={:?})",
                fn_ir.name,
                fn_ir.values[iv_phi].kind,
                induction_origin_var(fn_ir, iv_phi),
                fn_ir.values[y_arg].kind,
                fn_ir.values[canonical_value(fn_ir, y_arg)].origin_var
            );
        }
        return None;
    }

    for arg in [args[0], args[1], args[3]] {
        if expr_has_iv_dependency(fn_ir, arg, iv_phi)
            || !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, user_call_whitelist)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cube-slice] {} reject: non-invariant cube arg ({:?}) dep={}",
                    fn_ir.name,
                    fn_ir.values[arg].kind,
                    expr_has_iv_dependency(fn_ir, arg, iv_phi)
                );
            }
            return None;
        }
    }
    let ctx = if args.len() == 5 {
        let arg = args[4];
        if expr_has_iv_dependency(fn_ir, arg, iv_phi)
            || !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, user_call_whitelist)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cube-slice] {} reject: non-invariant cube ctx arg ({:?}) dep={}",
                    fn_ir.name,
                    fn_ir.values[arg].kind,
                    expr_has_iv_dependency(fn_ir, arg, iv_phi)
                );
            }
            return None;
        }
        Some(arg)
    } else {
        None
    };

    Some(CubeSliceIndexInfo {
        face: args[0],
        row: args[1],
        size: args[3],
        ctx,
    })
}
