pub(super) fn apply_expr_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ExprMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        plan.expr,
        plan.iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => v,
        None => return false,
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_assign_values",
        Axis3D::Dim2 => "rr_dim2_assign_values",
        Axis3D::Dim3 => "rr_dim3_assign_values",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![plan.dest, expr_vec, fixed_a, fixed_b, plan.range.start, end],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn same_load_leaf_var_in_phi_tree(
    fn_ir: &FnIR,
    root: ValueId,
    seen: &mut FxHashSet<ValueId>,
) -> Option<VarId> {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return None;
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::Load { var } => Some(var.clone()),
        ValueKind::Phi { args } if !args.is_empty() => {
            let mut found: Option<VarId> = None;
            for (arg, _) in args {
                let leaf_var = same_load_leaf_var_in_phi_tree(fn_ir, *arg, seen)?;
                match &found {
                    None => found = Some(leaf_var),
                    Some(prev) if prev == &leaf_var => {}
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

pub(super) fn recover_cube_slice_snapshot_scalar(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    value: ValueId,
    iv_phi: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    let root = canonical_value(fn_ir, value);
    let target_bb = fn_ir.values[root]
        .phi_block
        .or_else(|| value_use_block_in_loop(fn_ir, lp, root));
    let snapshot_var = fn_ir.values[root]
        .origin_var
        .clone()
        .or_else(|| induction_origin_var(fn_ir, root))?;
    let candidate = target_bb
        .and_then(|bb| unique_assign_source_reaching_block_in_loop(fn_ir, lp, &snapshot_var, bb))
        .or_else(|| loop_entry_seed_source_in_loop(fn_ir, lp, &snapshot_var))
        .or_else(|| unique_assign_source_in_loop(fn_ir, lp, &snapshot_var))
        .map(|src| canonical_value(fn_ir, src))
        .filter(|src| {
            *src != root && !value_depends_on(fn_ir, *src, root, &mut FxHashSet::default())
        })
        .unwrap_or(root);
    if candidate != root
        && let Some(v) =
            materialize_loop_invariant_scalar_expr(fn_ir, candidate, iv_phi, lp, memo, interner)
    {
        return Some(resolve_materialized_value(fn_ir, v));
    }
    if let Some(var) = fn_ir.values[candidate].origin_var.clone()
        && !has_non_passthrough_assignment_in_loop(fn_ir, lp, &var)
    {
        let load = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Load { var },
            fn_ir.values[candidate].span,
            fn_ir.values[candidate].facts,
        );
        return Some(resolve_materialized_value(fn_ir, load));
    }
    let leaf_var = same_load_leaf_var_in_phi_tree(fn_ir, candidate, &mut FxHashSet::default())?;
    if has_non_passthrough_assignment_in_loop(fn_ir, lp, &leaf_var) {
        return None;
    }
    let load = intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Load { var: leaf_var },
        fn_ir.values[root].span,
        fn_ir.values[root].facts,
    );
    Some(resolve_materialized_value(fn_ir, load))
}

pub(super) fn cube_slice_expr_has_complex_loop_local_axes(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    expr: ValueId,
    iv_phi: ValueId,
) -> bool {
    let expr = canonical_value(fn_ir, expr);
    let ValueKind::Call { callee, args, .. } = &fn_ir.values[expr].kind else {
        return false;
    };
    if callee != "rr_idx_cube_vec_i" || args.len() < 4 {
        return false;
    }
    for arg in [args[1], args[2]] {
        let arg = canonical_value(fn_ir, arg);
        if is_iv_equivalent(fn_ir, arg, iv_phi) {
            continue;
        }
        if let ValueKind::Load { var } = &fn_ir.values[arg].kind
            && has_assignment_in_loop(fn_ir, lp, var)
        {
            return true;
        }
        if let Some(var) = fn_ir.values[arg].origin_var.as_deref()
            && has_assignment_in_loop(fn_ir, lp, var)
            && !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, &FxHashSet::default())
        {
            return true;
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_cube_slice_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
    face: ValueId,
    row: ValueId,
    size: ValueId,
    ctx: Option<ValueId>,
    start: ValueId,
    end: ValueId,
    shadow_vars: Vec<VarId>,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();

    let materialize_scalar =
        |value: ValueId,
         label: &str,
         fn_ir: &mut FnIR,
         memo: &mut FxHashMap<ValueId, ValueId>,
         interner: &mut FxHashMap<MaterializedExprKey, ValueId>| {
            match materialize_loop_invariant_scalar_expr(fn_ir, value, iv_phi, lp, memo, interner) {
                Some(v) => Some(resolve_materialized_value(fn_ir, v)),
                None => {
                    if let Some(v) =
                        recover_cube_slice_snapshot_scalar(fn_ir, lp, value, iv_phi, memo, interner)
                    {
                        return Some(v);
                    }
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: invariant scalar {} materialization ({:?})",
                            fn_ir.name, label, fn_ir.values[value].kind
                        );
                    }
                    None
                }
            }
        };

    let Some(face) = materialize_scalar(face, "face", fn_ir, &mut memo, &mut interner) else {
        return false;
    };
    let Some(row) = materialize_scalar(row, "row", fn_ir, &mut memo, &mut interner) else {
        return false;
    };
    let Some(size) = materialize_scalar(size, "size", fn_ir, &mut memo, &mut interner) else {
        return false;
    };
    let ctx = match ctx {
        Some(ctx_val) => {
            match materialize_scalar(ctx_val, "ctx", fn_ir, &mut memo, &mut interner) {
                Some(v) => Some(v),
                None => return false,
            }
        }
        None => None,
    };

    if cube_slice_expr_has_complex_loop_local_axes(fn_ir, lp, expr, iv_phi) {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: cube-slice rhs has complex loop-local axes ({:?})",
                fn_ir.name,
                fn_ir.values[canonical_value(fn_ir, expr)].kind
            );
        }
        return false;
    }

    let expr_vec = if expr_has_iv_dependency(fn_ir, expr, iv_phi) {
        match materialize_vector_expr(
            fn_ir,
            expr,
            iv_phi,
            idx_vec,
            lp,
            &mut memo,
            &mut interner,
            true,
            false,
        ) {
            Some(v) => v,
            None => {
                if trace_enabled {
                    eprintln!(
                        "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                        fn_ir.name, fn_ir.values[expr].kind
                    );
                }
                return false;
            }
        }
    } else {
        match materialize_loop_invariant_scalar_expr(
            fn_ir,
            expr,
            iv_phi,
            lp,
            &mut memo,
            &mut interner,
        ) {
            Some(v) => v,
            None => {
                if let Some(v) = materialize_vector_expr(
                    fn_ir,
                    expr,
                    iv_phi,
                    idx_vec,
                    lp,
                    &mut memo,
                    &mut interner,
                    true,
                    false,
                ) {
                    v
                } else {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: invariant scalar expr materialization ({:?})",
                            fn_ir.name, fn_ir.values[expr].kind
                        );
                    }
                    return false;
                }
            }
        }
    };

    let expr_vec = broadcast_scalar_expr_to_slice_len(fn_ir, expr_vec, start, end);

    let has_ctx = ctx.is_some();
    let mut start_args = vec![face, row, start, size];
    let mut end_args = vec![face, row, end, size];
    if let Some(ctx) = ctx {
        start_args.push(ctx);
        end_args.push(ctx);
    }
    let names = vec![None; if has_ctx { 5 } else { 4 }];
    let slice_start = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_idx_cube_vec_i".to_string(),
            args: start_args,
            names: names.clone(),
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let slice_end = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_idx_cube_vec_i".to_string(),
            args: end_args,
            names,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_assign_slice".to_string(),
            args: vec![dest, slice_start, slice_end, expr_vec],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        &shadow_vars,
        Some(slice_end),
    )
}
