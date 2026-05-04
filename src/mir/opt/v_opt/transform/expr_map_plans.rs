use super::*;
pub(crate) struct ExprCallMapAutoApply<'a> {
    pub(crate) site: VectorApplySite,
    pub(crate) dest: ValueId,
    pub(crate) expr: ValueId,
    pub(crate) iv_phi: ValueId,
    pub(crate) start: ValueId,
    pub(crate) end: ValueId,
    pub(crate) whole_dest: bool,
    pub(crate) shadow_vars: &'a [VarId],
}

pub(crate) fn try_apply_expr_call_map_auto(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    plan: ExprCallMapAutoApply<'_>,
) -> Option<bool> {
    let whole_dest = plan.whole_dest && lp.limit_adjust == 0;
    let root = canonical_value(fn_ir, plan.expr);
    let (callee, args) = match &fn_ir.values[root].kind {
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            if names.iter().any(Option::is_some) {
                return None;
            }
            (callee.clone(), args.clone())
        }
        _ => return None,
    };

    let call_args: Vec<CallMapArg> = args
        .iter()
        .map(|arg| CallMapArg {
            value: *arg,
            vectorized: expr_has_iv_dependency(fn_ir, *arg, plan.iv_phi),
        })
        .collect();
    let CallMapLoweringMode::RuntimeAuto { helper_cost } =
        choose_call_map_lowering(fn_ir, &callee, &call_args, whole_dest, plan.shadow_vars)
    else {
        return None;
    };

    let end = adjusted_loop_limit(fn_ir, plan.end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return Some(false);
    };
    let idx_vec = build_loop_index_vector(fn_ir, lp)?;
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut mapped_args = Vec::with_capacity(call_args.len());
    let mut vector_args = Vec::new();

    for (arg_i, arg) in call_args.iter().enumerate() {
        let out = if arg.vectorized {
            materialize_vector_expr(
                fn_ir,
                VectorMaterializeRequest {
                    root: arg.value,
                    iv_phi: plan.iv_phi,
                    idx_vec,
                    lp,
                    policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
                },
                &mut memo,
                &mut interner,
            )?
        } else {
            materialize_loop_invariant_scalar_expr(
                fn_ir,
                arg.value,
                plan.iv_phi,
                lp,
                &mut memo,
                &mut interner,
            )
            .unwrap_or_else(|| resolve_materialized_value(fn_ir, arg.value))
        };
        let out = if arg.vectorized {
            maybe_hoist_callmap_arg_expr(fn_ir, plan.site.preheader, out, arg_i)
        } else {
            out
        };
        if arg.vectorized {
            vector_args.push(out);
        }
        mapped_args.push((out, arg.vectorized));
    }

    emit_call_map_argument_guards(
        fn_ir,
        plan.site.preheader,
        plan.dest,
        whole_dest,
        &mapped_args,
        &vector_args,
    );
    if vectorize_trace_enabled() {
        eprintln!(
            "   [vec-profit] {} expr_map runtime-auto callee={} helper_cost={} whole_dest={}",
            fn_ir.name, callee, helper_cost, whole_dest
        );
    }
    let out_val = build_call_map_auto_value(
        fn_ir,
        CallMapAutoValue {
            dest: plan.dest,
            start: plan.start,
            end,
            callee: &callee,
            helper_cost,
            mapped_args: &mapped_args,
            whole_dest,
        },
    );
    Some(finish_vector_assignment_with_shadow_states(
        fn_ir,
        plan.site,
        dest_var,
        out_val,
        plan.shadow_vars,
        Some(end),
    ))
}

pub(crate) struct ExprMapApplyPlan {
    pub(crate) site: VectorApplySite,
    pub(crate) dest: ValueId,
    pub(crate) expr: ValueId,
    pub(crate) iv_phi: ValueId,
    pub(crate) start: ValueId,
    pub(crate) end: ValueId,
    pub(crate) whole_dest: bool,
    pub(crate) shadow_vars: Vec<VarId>,
}

/// Lower a canonical expr-map vectorization plan into preheader materialization
/// plus a single assignment that preserves partial-slice semantics when the
/// original scalar loop did not cover the full destination.
pub(crate) fn apply_expr_map_plan(fn_ir: &mut FnIR, lp: &LoopInfo, plan: ExprMapApplyPlan) -> bool {
    let whole_dest = plan.whole_dest && lp.limit_adjust == 0;
    if let Some(applied) = try_apply_expr_call_map_auto(
        fn_ir,
        lp,
        ExprCallMapAutoApply {
            site: plan.site,
            dest: plan.dest,
            expr: plan.expr,
            iv_phi: plan.iv_phi,
            start: plan.start,
            end: plan.end,
            whole_dest,
            shadow_vars: &plan.shadow_vars,
        },
    ) {
        return applied;
    }

    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, plan.end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
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
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: plan.expr,
            iv_phi: plan.iv_phi,
            idx_vec,
            lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                    fn_ir.name, fn_ir.values[plan.expr].kind
                );
            }
            return false;
        }
    };
    let out_val = build_expr_map_output_value(
        fn_ir,
        plan.site.preheader,
        plan.dest,
        expr_vec,
        plan.start,
        end,
        whole_dest,
    );
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        plan.site,
        dest_var,
        out_val,
        &plan.shadow_vars,
        Some(end),
    )
}

pub(crate) fn build_expr_map_output_value(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    dest: ValueId,
    expr_vec: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
) -> ValueId {
    let expr_is_scalar = is_scalar_broadcast_value(fn_ir, expr_vec);
    let expr_vec = if expr_is_scalar {
        let broadcast_end = if whole_dest && is_const_one(fn_ir, start) {
            vector_length_key(fn_ir, dest).unwrap_or(end)
        } else {
            end
        };
        broadcast_scalar_expr_to_slice_len(fn_ir, expr_vec, start, broadcast_end)
    } else if whole_dest {
        expr_vec
    } else {
        prepare_partial_slice_value(fn_ir, dest, expr_vec, start, end)
    };

    if whole_dest {
        if !expr_is_scalar && !same_length_proven(fn_ir, dest, expr_vec) {
            emit_same_len_guard(fn_ir, preheader, dest, expr_vec);
        }
        expr_vec
    } else {
        build_slice_assignment_value(fn_ir, dest, start, end, expr_vec)
    }
}

pub(crate) struct MultiExprMapStage<'a> {
    pub(crate) lp: &'a LoopInfo,
    pub(crate) site: VectorApplySite,
    pub(crate) entry_index: usize,
    pub(crate) entry: &'a ExprMapEntry,
    pub(crate) iv_phi: ValueId,
    pub(crate) idx_vec: ValueId,
    pub(crate) start: ValueId,
    pub(crate) end: ValueId,
    pub(crate) memo: &'a mut FxHashMap<ValueId, ValueId>,
    pub(crate) interner: &'a mut FxHashMap<MaterializedExprKey, ValueId>,
    pub(crate) trace_enabled: bool,
}

pub(crate) fn stage_multi_expr_map_entry(
    fn_ir: &mut FnIR,
    stage: MultiExprMapStage<'_>,
) -> Option<PreparedVectorAssignment> {
    let dest_var = resolve_base_var(fn_ir, stage.entry.dest).or_else(|| {
        if stage.trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        None
    })?;
    let expr_vec = materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: stage.entry.expr,
            iv_phi: stage.iv_phi,
            idx_vec: stage.idx_vec,
            lp: stage.lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        stage.memo,
        stage.interner,
    )
    .or_else(|| {
        if stage.trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                fn_ir.name, fn_ir.values[stage.entry.expr].kind
            );
        }
        None
    })?;
    let expr_vec = hoist_vector_expr_temp(
        fn_ir,
        stage.site.preheader,
        expr_vec,
        &format!("exprmap{}", stage.entry_index),
    );
    let out_val = build_expr_map_output_value(
        fn_ir,
        stage.site.preheader,
        stage.entry.dest,
        expr_vec,
        stage.start,
        stage.end,
        stage.entry.whole_dest && stage.lp.limit_adjust == 0,
    );
    let shadow_idx = if stage.entry.whole_dest && stage.lp.limit_adjust == 0 {
        Some(fn_ir.add_value(
            ValueKind::Len { base: out_val },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ))
    } else {
        Some(stage.end)
    };

    Some(PreparedVectorAssignment {
        dest_var,
        out_val,
        shadow_vars: stage.entry.shadow_vars.clone(),
        shadow_idx,
    })
}

pub(crate) fn apply_multi_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entries: Vec<ExprMapEntry>,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    if trace_enabled {
        let dests: Vec<String> = entries
            .iter()
            .map(|entry| {
                resolve_base_var(fn_ir, entry.dest).unwrap_or_else(|| format!("<v{}>", entry.dest))
            })
            .collect();
        eprintln!(
            "   [vec-apply-expr] {} multi_expr_map entries={:?}",
            fn_ir.name, dests
        );
    }
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
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
    let mut staged = Vec::with_capacity(entries.len());

    for (entry_index, entry) in entries.iter().enumerate() {
        let Some(assignment) = stage_multi_expr_map_entry(
            fn_ir,
            MultiExprMapStage {
                lp,
                site,
                entry_index,
                entry,
                iv_phi,
                idx_vec,
                start,
                end,
                memo: &mut memo,
                interner: &mut interner,
                trace_enabled,
            },
        ) else {
            return false;
        };
        staged.push(assignment);
    }
    emit_prepared_vector_assignments(fn_ir, site, staged)
}

pub(crate) struct MultiExprMap3DStage<'a> {
    pub(crate) lp: &'a LoopInfo,
    pub(crate) site: VectorApplySite,
    pub(crate) entry_index: usize,
    pub(crate) entry: &'a ExprMapEntry3D,
    pub(crate) iv_phi: ValueId,
    pub(crate) idx_vec: ValueId,
    pub(crate) start: ValueId,
    pub(crate) end: ValueId,
    pub(crate) memo: &'a mut FxHashMap<ValueId, ValueId>,
    pub(crate) interner: &'a mut FxHashMap<MaterializedExprKey, ValueId>,
    pub(crate) trace_enabled: bool,
}

pub(crate) fn stage_multi_expr_map_3d_entry(
    fn_ir: &mut FnIR,
    stage: MultiExprMap3DStage<'_>,
) -> Option<PreparedVectorAssignment> {
    let dest_var = resolve_base_var(fn_ir, stage.entry.dest).or_else(|| {
        if stage.trace_enabled {
            eprintln!(
                "   [vec-apply-expr3d] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        None
    })?;
    let expr_vec = materialize_vector_expr(
        fn_ir,
        VectorMaterializeRequest {
            root: stage.entry.expr,
            iv_phi: stage.iv_phi,
            idx_vec: stage.idx_vec,
            lp: stage.lp,
            policy: RELAXED_VECTOR_MATERIALIZE_POLICY,
        },
        stage.memo,
        stage.interner,
    )
    .or_else(|| {
        if stage.trace_enabled {
            eprintln!(
                "   [vec-apply-expr3d] {} fail: materialize_vector_expr({:?})",
                fn_ir.name, fn_ir.values[stage.entry.expr].kind
            );
        }
        None
    })?;
    let expr_vec = hoist_vector_expr_temp(
        fn_ir,
        stage.site.preheader,
        expr_vec,
        &format!("exprmap3d{}", stage.entry_index),
    );
    let helper = match stage.entry.axis {
        Axis3D::Dim1 => "rr_dim1_assign_values",
        Axis3D::Dim2 => "rr_dim2_assign_values",
        Axis3D::Dim3 => "rr_dim3_assign_values",
    };
    let fixed_a = resolve_materialized_value(fn_ir, stage.entry.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, stage.entry.fixed_b);
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![
                stage.entry.dest,
                expr_vec,
                fixed_a,
                fixed_b,
                stage.start,
                stage.end,
            ],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );

    Some(PreparedVectorAssignment {
        dest_var,
        out_val,
        shadow_vars: stage.entry.shadow_vars.clone(),
        shadow_idx: Some(stage.end),
    })
}

pub(crate) fn apply_multi_expr_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entries: Vec<ExprMapEntry3D>,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr3d] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut staged = Vec::with_capacity(entries.len());

    for (entry_index, entry) in entries.iter().enumerate() {
        let Some(assignment) = stage_multi_expr_map_3d_entry(
            fn_ir,
            MultiExprMap3DStage {
                lp,
                site,
                entry_index,
                entry,
                iv_phi,
                idx_vec,
                start,
                end,
                memo: &mut memo,
                interner: &mut interner,
                trace_enabled,
            },
        ) else {
            return false;
        };
        staged.push(assignment);
    }
    emit_prepared_vector_assignments(fn_ir, site, staged)
}
