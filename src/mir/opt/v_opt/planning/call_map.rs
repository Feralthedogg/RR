use super::*;

pub(in crate::mir::opt::v_opt) fn match_call_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest_base, dest_idx, rhs, is_vector, is_safe, is_na_safe) = match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    is_safe,
                    is_na_safe,
                    ..
                } => (*base, *idx, *val, *is_vector, *is_safe, *is_na_safe),
                _ => continue,
            };
            if is_vector || !is_safe || !is_na_safe || !is_iv_equivalent(fn_ir, dest_idx, iv_phi) {
                continue;
            }
            let rhs = resolve_load_alias_value(fn_ir, rhs);
            let ValueKind::Call { .. } = &fn_ir.values[rhs].kind else {
                continue;
            };
            let Some(call) = resolve_call_info(fn_ir, rhs) else {
                continue;
            };
            if !is_vector_safe_call(&call.callee, call.args.len(), user_call_whitelist) {
                continue;
            }

            let mut mapped_args = Vec::with_capacity(call.args.len());
            let mut has_vector_arg = false;
            for arg in &call.args {
                let arg = resolve_load_alias_value(fn_ir, *arg);
                if expr_has_iv_dependency(fn_ir, arg, iv_phi) {
                    if !is_vector_safe_call_chain_expr(fn_ir, arg, iv_phi, lp, user_call_whitelist)
                    {
                        mapped_args.clear();
                        break;
                    }
                    mapped_args.push(CallMapArg {
                        value: arg,
                        vectorized: true,
                    });
                    has_vector_arg = true;
                } else {
                    if !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, user_call_whitelist) {
                        mapped_args.clear();
                        break;
                    }
                    mapped_args.push(CallMapArg {
                        value: arg,
                        vectorized: false,
                    });
                }
            }
            if mapped_args.is_empty() || !has_vector_arg {
                continue;
            }
            let allowed_dests: Vec<VarId> =
                resolve_base_var(fn_ir, canonical_value(fn_ir, dest_base))
                    .into_iter()
                    .collect();
            let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                fn_ir,
                lp,
                &allowed_dests,
                canonical_value(fn_ir, dest_base),
                iv_phi,
            ) else {
                continue;
            };
            return Some(VectorPlan::CallMap {
                dest: canonical_value(fn_ir, dest_base),
                callee: call.callee,
                args: mapped_args,
                iv_phi,
                start,
                end,
                whole_dest: loop_covers_whole_destination(
                    lp,
                    fn_ir,
                    canonical_value(fn_ir, dest_base),
                    start,
                ),
                shadow_vars,
            });
        }
    }
    None
}

pub(in crate::mir::opt::v_opt) fn match_call_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut found: Option<CallMap3DMatchCandidate> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. }
                | Instr::StoreIndex1D { .. }
                | Instr::StoreIndex2D { .. }
                | Instr::UnsafeRBlock { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    if found.is_some() {
                        return None;
                    }
                    let rhs = resolve_load_alias_value(fn_ir, *val);
                    let ValueKind::Call { .. } = &fn_ir.values[rhs].kind else {
                        continue;
                    };
                    let Some(call) = resolve_call_info(fn_ir, rhs) else {
                        continue;
                    };
                    if !is_vector_safe_call(&call.callee, call.args.len(), user_call_whitelist) {
                        continue;
                    }
                    let Some((axis, fixed_a, fixed_b)) =
                        classify_3d_map_axis(fn_ir, *base, *i, *j, *k, iv_phi)
                    else {
                        continue;
                    };
                    let mut mapped_args = Vec::with_capacity(call.args.len());
                    let mut has_vector_arg = false;
                    let mut generalized = false;
                    for arg in &call.args {
                        let arg = resolve_load_alias_value(fn_ir, *arg);
                        if expr_has_iv_dependency(fn_ir, arg, iv_phi) {
                            if let Some(src_base) = axis3_vector_operand_source(
                                fn_ir, arg, axis, fixed_a, fixed_b, iv_phi,
                            ) {
                                mapped_args.push(CallMapArg {
                                    value: src_base,
                                    vectorized: true,
                                });
                            } else if is_vectorizable_expr(
                                fn_ir,
                                arg,
                                iv_phi,
                                lp,
                                crate::mir::opt::v_opt::analysis::RELAXED_VECTOR_EXPR_POLICY,
                            ) && !expr_has_unstable_loop_local_load(fn_ir, lp, arg)
                            {
                                mapped_args.push(CallMapArg {
                                    value: arg,
                                    vectorized: true,
                                });
                                generalized = true;
                            } else {
                                mapped_args.clear();
                                break;
                            }
                            has_vector_arg = true;
                        } else if is_loop_invariant_scalar_expr(
                            fn_ir,
                            arg,
                            iv_phi,
                            user_call_whitelist,
                        ) {
                            mapped_args.push(CallMapArg {
                                value: arg,
                                vectorized: false,
                            });
                        } else {
                            mapped_args.clear();
                            break;
                        }
                    }
                    if mapped_args.is_empty() || !has_vector_arg {
                        continue;
                    }
                    let allowed_dests: Vec<VarId> =
                        resolve_base_var(fn_ir, canonical_value(fn_ir, *base))
                            .into_iter()
                            .collect();
                    let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                        fn_ir,
                        lp,
                        &allowed_dests,
                        canonical_value(fn_ir, *base),
                        iv_phi,
                    ) else {
                        continue;
                    };
                    if !shadow_vars.is_empty() {
                        continue;
                    }
                    found = Some(CallMap3DMatchCandidate {
                        dest: canonical_value(fn_ir, *base),
                        axis,
                        fixed_a,
                        fixed_b,
                        callee: call.callee,
                        args: mapped_args,
                        generalized,
                    });
                }
            }
        }
    }

    let CallMap3DMatchCandidate {
        dest,
        axis,
        fixed_a,
        fixed_b,
        callee,
        args,
        generalized,
    } = found?;
    if generalized {
        Some(VectorPlan::CallMap3DGeneral {
            dest,
            callee,
            args,
            axis,
            fixed_a,
            fixed_b,
            iv_phi,
            start,
            end,
        })
    } else {
        Some(VectorPlan::CallMap3D {
            dest,
            callee,
            args,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
        })
    }
}

pub(in crate::mir::opt::v_opt) fn match_multi_expr_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut pending_entries: Vec<(ValueId, ValueId, Axis3D, ValueId, ValueId)> = Vec::new();

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. }
                | Instr::StoreIndex1D { .. }
                | Instr::StoreIndex2D { .. }
                | Instr::UnsafeRBlock { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    let (axis, fixed_a, fixed_b) =
                        classify_3d_map_axis(fn_ir, *base, *i, *j, *k, iv_phi)?;
                    let expr = resolve_load_alias_value(fn_ir, *val);
                    if !is_vectorizable_expr(
                        fn_ir,
                        expr,
                        iv_phi,
                        lp,
                        crate::mir::opt::v_opt::analysis::RELAXED_VECTOR_EXPR_POLICY,
                    ) {
                        return None;
                    }
                    if expr_has_non_vector_safe_call_in_vector_context(
                        fn_ir,
                        expr,
                        iv_phi,
                        user_call_whitelist,
                        &mut FxHashSet::default(),
                    ) {
                        return None;
                    }
                    if expr_has_unstable_loop_local_load(fn_ir, lp, expr) {
                        return None;
                    }
                    let dest = canonical_value(fn_ir, *base);
                    if expr_reads_base(fn_ir, expr, dest) {
                        return None;
                    }
                    if pending_entries
                        .iter()
                        .any(|(prev_dest, _, prev_axis, prev_a, prev_b)| {
                            (match (
                                resolve_base_var(fn_ir, *prev_dest),
                                resolve_base_var(fn_ir, dest),
                            ) {
                                (Some(a), Some(b)) => a == b,
                                _ => same_base_value(fn_ir, *prev_dest, dest),
                            }) && *prev_axis == axis
                                && same_loop_invariant_value(fn_ir, *prev_a, fixed_a, iv_phi)
                                && same_loop_invariant_value(fn_ir, *prev_b, fixed_b, iv_phi)
                        })
                    {
                        return None;
                    }
                    pending_entries.push((dest, expr, axis, fixed_a, fixed_b));
                }
            }
        }
    }

    if pending_entries.len() <= 1 {
        return None;
    }

    let allowed_dests: Vec<VarId> = pending_entries
        .iter()
        .filter_map(|(dest, ..)| resolve_base_var(fn_ir, *dest))
        .collect();
    let mut entries = Vec::with_capacity(pending_entries.len());
    for (dest, expr, axis, fixed_a, fixed_b) in pending_entries {
        let shadow_vars =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;
        entries.push(ExprMapEntry3D {
            dest,
            expr,
            axis,
            fixed_a,
            fixed_b,
            shadow_vars,
        });
    }

    Some(VectorPlan::MultiExprMap3D {
        entries,
        iv_phi,
        start,
        end,
    })
}
