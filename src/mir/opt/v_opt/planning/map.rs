use super::*;

pub(in crate::mir::opt::v_opt) fn match_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let whole_dest = |dest: ValueId| loop_covers_whole_destination(lp, fn_ir, dest, iv.init_val);

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            if let Instr::StoreIndex1D {
                base,
                idx,
                val,
                is_vector: false,
                is_safe,
                is_na_safe,
                ..
            } = instr
                && is_iv_equivalent(fn_ir, *idx, iv_phi)
                && *is_safe
                && *is_na_safe
            {
                let rhs_val = &fn_ir.values[*val];
                if let ValueKind::Binary { op, lhs, rhs } = &rhs_val.kind {
                    let lhs_idx = as_safe_loop_index(fn_ir, *lhs, iv_phi);
                    let rhs_idx = as_safe_loop_index(fn_ir, *rhs, iv_phi);

                    // x[i] OP x[i]  ->  x OP x
                    if let (Some(lbase), Some(rbase)) = (lhs_idx, rhs_idx)
                        && lbase == rbase
                        && loop_matches_vec(lp, fn_ir, lbase)
                        && whole_dest(*base)
                        && whole_dest(lbase)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                            fn_ir,
                            lp,
                            &allowed_dests,
                            *base,
                            iv_phi,
                        ) else {
                            continue;
                        };
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: lbase,
                            op: *op,
                            other: rbase,
                            shadow_vars,
                        });
                    }

                    // x[i] OP k  ->  x OP k
                    if let Some(x_base) = lhs_idx
                        && loop_matches_vec(lp, fn_ir, x_base)
                        && whole_dest(*base)
                        && whole_dest(x_base)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                            fn_ir,
                            lp,
                            &allowed_dests,
                            *base,
                            iv_phi,
                        ) else {
                            continue;
                        };
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: x_base,
                            op: *op,
                            other: *rhs,
                            shadow_vars,
                        });
                    }

                    // k OP x[i]  ->  k OP x
                    if let Some(x_base) = rhs_idx
                        && loop_matches_vec(lp, fn_ir, x_base)
                        && whole_dest(*base)
                        && whole_dest(x_base)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                            fn_ir,
                            lp,
                            &allowed_dests,
                            *base,
                            iv_phi,
                        ) else {
                            continue;
                        };
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: *lhs,
                            op: *op,
                            other: x_base,
                            shadow_vars,
                        });
                    }
                }
            }
        }
    }
    None
}

pub(in crate::mir::opt::v_opt) fn match_conditional_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let trace_enabled = vectorize_trace_enabled();

    for &bid in &lp.body {
        if bid == lp.header {
            continue;
        }
        if let Terminator::If {
            cond,
            then_bb,
            else_bb,
        } = fn_ir.blocks[bid].term
        {
            if !lp.body.contains(&then_bb) || !lp.body.contains(&else_bb) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: branch blocks leave loop body (then={}, else={})",
                        fn_ir.name, then_bb, else_bb
                    );
                }
                continue;
            }
            let cond_vec = is_condition_vectorizable(fn_ir, cond, iv_phi, lp, user_call_whitelist);
            let cond_dep = expr_has_iv_dependency(fn_ir, cond, iv_phi);
            if !cond_vec || !cond_dep {
                if trace_enabled {
                    let cond_detail = match &fn_ir.values[cond].kind {
                        ValueKind::Binary { lhs, rhs, .. } => format!(
                            "lhs={:?} rhs={:?}",
                            fn_ir.values[*lhs].kind, fn_ir.values[*rhs].kind
                        ),
                        other => format!("{:?}", other),
                    };
                    eprintln!(
                        "   [vec-cond-map] {} skip: condition rejected (vec={}, dep={}, cond={:?}, detail={})",
                        fn_ir.name, cond_vec, cond_dep, fn_ir.values[cond].kind, cond_detail
                    );
                }
                continue;
            }
            let then_store = classify_store_1d_in_block(fn_ir, then_bb);
            let else_store = classify_store_1d_in_block(fn_ir, else_bb);
            let (then_base, then_val, else_base, else_val) = match (then_store, else_store) {
                (BlockStore1DMatch::One(then_store), BlockStore1DMatch::One(else_store))
                    if !then_store.is_vector
                        && !else_store.is_vector
                        && is_iv_equivalent(fn_ir, then_store.idx, iv_phi)
                        && is_iv_equivalent(fn_ir, else_store.idx, iv_phi) =>
                {
                    (
                        then_store.base,
                        then_store.val,
                        else_store.base,
                        else_store.val,
                    )
                }
                _ => {
                    if trace_enabled {
                        match then_store {
                            BlockStore1DMatch::One(_) => {}
                            _ => eprintln!(
                                "   [vec-cond-map] {} skip: then branch has no canonical single StoreIndex1D",
                                fn_ir.name
                            ),
                        }
                        match else_store {
                            BlockStore1DMatch::One(_) => {}
                            _ => eprintln!(
                                "   [vec-cond-map] {} skip: else branch has no canonical single StoreIndex1D",
                                fn_ir.name
                            ),
                        }
                    }
                    continue;
                }
            };
            let then_base = canonical_value(fn_ir, then_base);
            let else_base = canonical_value(fn_ir, else_base);
            let dest_base = if then_base == else_base {
                then_base
            } else {
                match (
                    resolve_base_var(fn_ir, then_base),
                    resolve_base_var(fn_ir, else_base),
                ) {
                    (Some(a), Some(b)) if a == b => then_base,
                    _ => {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-cond-map] {} skip: then/else destination bases differ",
                                fn_ir.name
                            );
                        }
                        continue;
                    }
                }
            };
            let allowed_dests: Vec<VarId> =
                resolve_base_var(fn_ir, dest_base).into_iter().collect();
            let Some(shadow_vars) =
                collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest_base, iv_phi)
            else {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: loop carries non-destination state",
                        fn_ir.name
                    );
                }
                continue;
            };
            if !is_vectorizable_expr(
                fn_ir,
                then_val,
                iv_phi,
                lp,
                crate::mir::opt::v_opt::analysis::RELAXED_VECTOR_EXPR_POLICY,
            ) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: then expression is not vectorizable",
                        fn_ir.name
                    );
                }
                continue;
            }
            if !is_vectorizable_expr(
                fn_ir,
                else_val,
                iv_phi,
                lp,
                crate::mir::opt::v_opt::analysis::RELAXED_VECTOR_EXPR_POLICY,
            ) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: else expression is not vectorizable",
                        fn_ir.name
                    );
                }
                continue;
            }
            if expr_has_ambiguous_loop_local_load(fn_ir, lp, cond)
                || expr_has_ambiguous_loop_local_load(fn_ir, lp, then_val)
                || expr_has_ambiguous_loop_local_load(fn_ir, lp, else_val)
            {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: ambiguous loop-local load in conditional map",
                        fn_ir.name
                    );
                }
                continue;
            }
            return Some(VectorPlan::CondMap {
                dest: dest_base,
                cond,
                then_val,
                else_val,
                iv_phi,
                start,
                end,
                whole_dest: loop_covers_whole_destination(lp, fn_ir, dest_base, start),
                shadow_vars,
            });
        }
    }
    None
}

pub(in crate::mir::opt::v_opt) fn match_conditional_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    _user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let trace_enabled = vectorize_trace_enabled();

    for &bid in &lp.body {
        if bid == lp.header {
            continue;
        }
        let Terminator::If {
            cond,
            then_bb,
            else_bb,
        } = fn_ir.blocks[bid].term
        else {
            continue;
        };
        if !lp.body.contains(&then_bb) || !lp.body.contains(&else_bb) {
            continue;
        }
        if !expr_has_iv_dependency(fn_ir, cond, iv_phi) {
            continue;
        }
        let ValueKind::Binary {
            op: cmp_op,
            lhs,
            rhs,
        } = fn_ir.values[cond].kind
        else {
            continue;
        };
        if !matches!(
            cmp_op,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne
        ) {
            continue;
        }

        let then_store = classify_store_3d_in_block(fn_ir, then_bb);
        let else_store = classify_store_3d_in_block(fn_ir, else_bb);
        let (then_store, else_store) = match (then_store, else_store) {
            (BlockStore3DMatch::One(then_store), BlockStore3DMatch::One(else_store)) => {
                (then_store, else_store)
            }
            _ => {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond3d] {} skip: non-canonical StoreIndex3D branches",
                        fn_ir.name
                    );
                }
                continue;
            }
        };

        let then_base = canonical_value(fn_ir, then_store.base);
        let else_base = canonical_value(fn_ir, else_store.base);
        let dest_base = if then_base == else_base {
            then_base
        } else {
            match (
                resolve_base_var(fn_ir, then_base),
                resolve_base_var(fn_ir, else_base),
            ) {
                (Some(a), Some(b)) if a == b => then_base,
                _ => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-cond3d] {} skip: then/else destination bases differ",
                            fn_ir.name
                        );
                    }
                    continue;
                }
            }
        };

        let Some((then_axis, then_fixed_a, then_fixed_b)) = classify_3d_map_axis(
            fn_ir,
            then_store.base,
            then_store.i,
            then_store.j,
            then_store.k,
            iv_phi,
        ) else {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: then branch axis classify failed",
                    fn_ir.name
                );
            }
            continue;
        };
        let Some((else_axis, else_fixed_a, else_fixed_b)) = classify_3d_map_axis(
            fn_ir,
            else_store.base,
            else_store.i,
            else_store.j,
            else_store.k,
            iv_phi,
        ) else {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: else branch axis classify failed",
                    fn_ir.name
                );
            }
            continue;
        };
        if then_axis != else_axis
            || !same_loop_invariant_value(fn_ir, then_fixed_a, else_fixed_a, iv_phi)
            || !same_loop_invariant_value(fn_ir, then_fixed_b, else_fixed_b, iv_phi)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: axis/fixed dims differ across branches",
                    fn_ir.name
                );
            }
            continue;
        }

        let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest_base).into_iter().collect();
        let Some(shadow_vars) =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest_base, iv_phi)
        else {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: unsupported loop-carried state",
                    fn_ir.name
                );
            }
            continue;
        };
        if !shadow_vars.is_empty() {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: loop carries non-destination state",
                    fn_ir.name
                );
            }
            continue;
        }

        let direct_cond_lhs =
            axis3_operand_source(fn_ir, lhs, then_axis, then_fixed_a, then_fixed_b, iv_phi);
        let direct_cond_rhs =
            axis3_operand_source(fn_ir, rhs, then_axis, then_fixed_a, then_fixed_b, iv_phi);
        let direct_then = axis3_operand_source(
            fn_ir,
            then_store.val,
            then_axis,
            then_fixed_a,
            then_fixed_b,
            iv_phi,
        );
        let direct_else = axis3_operand_source(
            fn_ir,
            else_store.val,
            then_axis,
            then_fixed_a,
            then_fixed_b,
            iv_phi,
        );

        if let (Some(cond_lhs), Some(cond_rhs), Some(then_src), Some(else_src)) =
            (direct_cond_lhs, direct_cond_rhs, direct_then, direct_else)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} matched axis={:?} fixed_a={:?} fixed_b={:?}",
                    fn_ir.name, then_axis, then_fixed_a, then_fixed_b
                );
            }
            return Some(VectorPlan::CondMap3D {
                dest: dest_base,
                axis: then_axis,
                fixed_a: then_fixed_a,
                fixed_b: then_fixed_b,
                cond_lhs,
                cond_rhs,
                cmp_op,
                then_src,
                else_src,
                start,
                end,
            });
        }

        let cond_operands_ok = |root: ValueId| {
            is_vectorizable_expr(
                fn_ir,
                root,
                iv_phi,
                lp,
                crate::mir::opt::v_opt::analysis::RELAXED_VECTOR_EXPR_POLICY,
            ) || is_loop_invariant_scalar_expr(fn_ir, root, iv_phi, &FxHashSet::default())
        };
        if !cond_operands_ok(lhs) || !cond_operands_ok(rhs) {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: condition operands are not vectorizable",
                    fn_ir.name
                );
            }
            continue;
        }
        if !is_vectorizable_expr(
            fn_ir,
            then_store.val,
            iv_phi,
            lp,
            crate::mir::opt::v_opt::analysis::RELAXED_VECTOR_EXPR_POLICY,
        ) || !is_vectorizable_expr(
            fn_ir,
            else_store.val,
            iv_phi,
            lp,
            crate::mir::opt::v_opt::analysis::RELAXED_VECTOR_EXPR_POLICY,
        ) {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: branch value is not vectorizable",
                    fn_ir.name
                );
            }
            continue;
        }
        if expr_has_ambiguous_loop_local_load(fn_ir, lp, lhs)
            || expr_has_ambiguous_loop_local_load(fn_ir, lp, rhs)
            || expr_has_ambiguous_loop_local_load(fn_ir, lp, then_store.val)
            || expr_has_ambiguous_loop_local_load(fn_ir, lp, else_store.val)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: ambiguous loop-local load in generalized conditional map",
                    fn_ir.name
                );
            }
            continue;
        }

        if trace_enabled {
            eprintln!(
                "   [vec-cond3d] {} matched generalized axis={:?} fixed_a={:?} fixed_b={:?}",
                fn_ir.name, then_axis, then_fixed_a, then_fixed_b
            );
        }
        return Some(VectorPlan::CondMap3DGeneral {
            dest: dest_base,
            axis: then_axis,
            fixed_a: then_fixed_a,
            fixed_b: then_fixed_b,
            cond_lhs: lhs,
            cond_rhs: rhs,
            cmp_op,
            then_val: then_store.val,
            else_val: else_store.val,
            iv_phi,
            start,
            end,
        });
    }
    None
}
