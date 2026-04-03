//! Reduction proof helpers and conditional-reduction certification.

use super::*;
use crate::mir::opt::v_opt::{debug, planning};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CondReduceArm {
    Identity,
    Update {
        kind: planning::ReduceKind,
        val: ValueId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BranchOnlyCondReductionEntry {
    plan: ReduceCondEntry,
    merge_phi: ValueId,
}

pub(super) fn certify_simple_reduction(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Result<CertifiedPlan, ProofFallbackReason> {
    let iv = lp
        .iv
        .as_ref()
        .ok_or(ProofFallbackReason::MissingInductionVar)?;
    let iv_phi = iv.phi_val;
    if loop_has_store_effect(fn_ir, lp) {
        return Err(ProofFallbackReason::NotSimpleReduction);
    }

    let reduction_rhs_vectorizable =
        |root: ValueId| is_vectorizable_expr(fn_ir, root, iv_phi, lp, true, false);

    let mut saw_candidate = false;
    let mut specific_reason = None;
    for (id, val) in fn_ir.values.iter().enumerate() {
        if is_iv_equivalent(fn_ir, id, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, id, iv_phi)
        {
            continue;
        }
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, bb)| *bb == lp.latch) {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, bb)| *bb == lp.latch) else {
            continue;
        };

        let reduction_match = match &fn_ir.values[*next_val].kind {
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            } if operand_matches_acc_phi(fn_ir, *lhs, id)
                || operand_matches_acc_phi(fn_ir, *rhs, id) =>
            {
                Some((
                    planning::ReduceKind::Sum,
                    if operand_matches_acc_phi(fn_ir, *lhs, id) {
                        *rhs
                    } else {
                        *lhs
                    },
                ))
            }
            ValueKind::Binary {
                op: BinOp::Mul,
                lhs,
                rhs,
            } if operand_matches_acc_phi(fn_ir, *lhs, id)
                || operand_matches_acc_phi(fn_ir, *rhs, id) =>
            {
                Some((
                    planning::ReduceKind::Prod,
                    if operand_matches_acc_phi(fn_ir, *lhs, id) {
                        *rhs
                    } else {
                        *lhs
                    },
                ))
            }
            ValueKind::Call { callee, args, .. }
                if (callee == "min" || callee == "max") && args.len() == 2 =>
            {
                if operand_matches_acc_phi(fn_ir, args[0], id) {
                    Some((
                        if callee == "min" {
                            planning::ReduceKind::Min
                        } else {
                            planning::ReduceKind::Max
                        },
                        args[1],
                    ))
                } else if operand_matches_acc_phi(fn_ir, args[1], id) {
                    Some((
                        if callee == "min" {
                            planning::ReduceKind::Min
                        } else {
                            planning::ReduceKind::Max
                        },
                        args[0],
                    ))
                } else {
                    None
                }
            }
            _ => None,
        };

        let Some((kind, other)) = reduction_match else {
            continue;
        };
        saw_candidate = true;

        match certify_reduction_candidate(
            fn_ir,
            lp,
            id,
            other,
            iv_phi,
            user_call_whitelist,
            reduction_rhs_vectorizable,
        ) {
            Ok(()) => {
                return Ok(CertifiedPlan {
                    plan: planning::VectorPlan::Reduce {
                        kind,
                        acc_phi: id,
                        vec_expr: other,
                        iv_phi,
                    },
                });
            }
            Err(reason) => {
                if specific_reason.is_none() || !is_generic_pattern_reason(reason) {
                    specific_reason = Some(reason);
                }
            }
        }
    }

    if saw_candidate {
        Err(specific_reason.unwrap_or(ProofFallbackReason::UnsupportedReductionExpr))
    } else {
        Err(ProofFallbackReason::NotSimpleReduction)
    }
}

pub(super) fn certify_reduction_candidate<F>(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    other: ValueId,
    iv_phi: ValueId,
    user_call_whitelist: &FxHashSet<String>,
    reduction_rhs_vectorizable: F,
) -> Result<(), ProofFallbackReason>
where
    F: Fn(ValueId) -> bool,
{
    if loop_has_store_effect(fn_ir, lp) {
        return Err(ProofFallbackReason::NotSimpleReduction);
    }

    if reduction_has_non_acc_loop_state_assignments(fn_ir, lp, acc_phi, iv_phi)
        || reduction_has_extra_state_phi(fn_ir, lp, acc_phi, iv_phi)
    {
        return Err(ProofFallbackReason::ReductionExtraState);
    }

    let acc_reads_self = phi_state_var(fn_ir, acc_phi)
        .or_else(|| fn_ir.values[acc_phi].origin_var.clone())
        .is_some_and(|acc_var| expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default()));
    let has_iv_dep = expr_has_iv_dependency(fn_ir, other, iv_phi);
    let has_acc_var = phi_state_var(fn_ir, acc_phi)
        .or_else(|| fn_ir.values[acc_phi].origin_var.clone())
        .or_else(|| induction_origin_var(fn_ir, acc_phi))
        .is_some();
    let has_non_iv_loop_state = expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi);
    let has_unstable_loop_local = expr_has_unstable_loop_local_load(fn_ir, lp, other);
    let has_ambiguous_loop_local = expr_has_ambiguous_loop_local_load(fn_ir, lp, other);
    let has_non_vector_safe_call = expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        other,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    );
    let rhs_vectorizable = reduction_rhs_vectorizable(other);

    if !has_iv_dep
        || acc_reads_self
        || !has_acc_var
        || has_non_iv_loop_state
        || has_unstable_loop_local
        || has_ambiguous_loop_local
        || has_non_vector_safe_call
        || !rhs_vectorizable
    {
        if debug::proof_trace_enabled() {
            debug::trace_proof_status(
                fn_ir,
                lp,
                &format!(
                    "reduce-reject-detail iv_dep={} acc_reads_self={} has_acc_var={} non_iv_state={} unstable_local={} ambiguous_local={} non_vec_call={} rhs_vectorizable={} other={:?}",
                    has_iv_dep,
                    acc_reads_self,
                    has_acc_var,
                    has_non_iv_loop_state,
                    has_unstable_loop_local,
                    has_ambiguous_loop_local,
                    has_non_vector_safe_call,
                    rhs_vectorizable,
                    fn_ir.values[other].kind
                ),
            );
        }
        return Err(ProofFallbackReason::UnsupportedReductionExpr);
    }

    Ok(())
}

pub(super) fn certify_simple_cond_reduction(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Result<CertifiedPlan, ProofFallbackReason> {
    let iv = lp
        .iv
        .as_ref()
        .ok_or(ProofFallbackReason::MissingInductionVar)?;
    let iv_phi = iv.phi_val;
    if lp.is_seq_len.is_none() || lp.limit_adjust != 0 {
        return Err(ProofFallbackReason::UnsupportedLoopShape);
    }

    let mut branch: Option<(ValueId, usize, usize)> = None;
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
            if branch.is_some() {
                return Err(ProofFallbackReason::NotSimpleReduction);
            }
            if !lp.body.contains(&then_bb) || !lp.body.contains(&else_bb) {
                return Err(ProofFallbackReason::BranchLeavesLoopBody);
            }
            branch = Some((cond, then_bb, else_bb));
        }
    }

    let Some((cond, then_bb, else_bb)) = branch else {
        return Err(ProofFallbackReason::NotSimpleReduction);
    };
    if !is_condition_vectorizable(fn_ir, cond, iv_phi, lp, user_call_whitelist)
        || !expr_has_iv_dependency(fn_ir, cond, iv_phi)
    {
        return Err(ProofFallbackReason::UnsupportedCondition);
    }

    if let Some((dest_base, then_val, else_val)) =
        classify_cond_reduction_store_branches(fn_ir, then_bb, else_bb, iv_phi)?
    {
        if !is_vectorizable_expr(fn_ir, then_val, iv_phi, lp, true, false)
            || !is_vectorizable_expr(fn_ir, else_val, iv_phi, lp, true, false)
        {
            return Err(ProofFallbackReason::UnsupportedConditionalValues);
        }
        let Some(dest_var) = resolve_base_var(fn_ir, dest_base) else {
            return Err(ProofFallbackReason::UnresolvableDestination);
        };
        if var_read_after_loop_exit(fn_ir, lp, &dest_var) {
            return Err(ProofFallbackReason::ShadowState);
        }

        for (id, val) in fn_ir.values.iter().enumerate() {
            if is_iv_equivalent(fn_ir, id, iv_phi)
                || is_origin_var_iv_alias_in_loop(fn_ir, lp, id, iv_phi)
            {
                continue;
            }
            let ValueKind::Phi { args } = &val.kind else {
                continue;
            };
            if args.len() != 2 || !args.iter().any(|(_, bb)| *bb == lp.latch) {
                continue;
            };
            let Some((next_val, _)) = args.iter().find(|(_, bb)| *bb == lp.latch) else {
                continue;
            };
            let Some((kind, other)) = reduction_update_for_acc(fn_ir, id, *next_val) else {
                continue;
            };
            if !reduction_reads_current_dest_element(fn_ir, lp, other, dest_base, iv_phi) {
                continue;
            }
            certify_cond_reduction_candidate(
                fn_ir,
                lp,
                id,
                other,
                Some(dest_base),
                iv_phi,
                user_call_whitelist,
            )?;
            return Ok(CertifiedPlan {
                plan: planning::VectorPlan::ReduceCond {
                    kind,
                    acc_phi: id,
                    cond,
                    then_val,
                    else_val,
                    iv_phi,
                },
            });
        }
    }

    if let Some(entries) = classify_branch_only_cond_reduction_entries(fn_ir, lp, then_bb, else_bb)?
    {
        certify_branch_only_multi_cond_reduction_candidate(
            fn_ir,
            lp,
            &entries,
            iv_phi,
            user_call_whitelist,
        )?;
        for entry in &entries {
            if !is_vectorizable_expr(fn_ir, entry.plan.then_val, iv_phi, lp, true, false)
                && !is_loop_invariant_scalar_expr(
                    fn_ir,
                    entry.plan.then_val,
                    iv_phi,
                    user_call_whitelist,
                )
            {
                return Err(ProofFallbackReason::UnsupportedConditionalValues);
            }
            if !is_vectorizable_expr(fn_ir, entry.plan.else_val, iv_phi, lp, true, false)
                && !is_loop_invariant_scalar_expr(
                    fn_ir,
                    entry.plan.else_val,
                    iv_phi,
                    user_call_whitelist,
                )
            {
                return Err(ProofFallbackReason::UnsupportedConditionalValues);
            }
        }
        if entries.len() == 1 {
            let entry = entries[0];
            return Ok(CertifiedPlan {
                plan: planning::VectorPlan::ReduceCond {
                    kind: entry.plan.kind,
                    acc_phi: entry.plan.acc_phi,
                    cond,
                    then_val: entry.plan.then_val,
                    else_val: entry.plan.else_val,
                    iv_phi,
                },
            });
        }
        return Ok(CertifiedPlan {
            plan: planning::VectorPlan::MultiReduceCond {
                cond,
                entries: entries.into_iter().map(|entry| entry.plan).collect(),
                iv_phi,
            },
        });
    }

    Err(ProofFallbackReason::BranchStoreShape)
}

pub(super) fn certify_cond_reduction_candidate(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    other: ValueId,
    dest_base: Option<ValueId>,
    iv_phi: ValueId,
    user_call_whitelist: &FxHashSet<String>,
) -> Result<(), ProofFallbackReason> {
    if reduction_has_non_acc_loop_state_assignments(fn_ir, lp, acc_phi, iv_phi)
        || cond_reduction_has_blocking_extra_state_phi(
            fn_ir,
            lp,
            &[acc_phi],
            dest_base,
            &[],
            iv_phi,
        )
    {
        return Err(ProofFallbackReason::ReductionExtraState);
    }

    let acc_reads_self = phi_state_var(fn_ir, acc_phi)
        .or_else(|| fn_ir.values[acc_phi].origin_var.clone())
        .is_some_and(|acc_var| expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default()));
    let reads_current_dest = dest_base.is_some_and(|dest_base| {
        reduction_reads_current_dest_element(fn_ir, lp, other, dest_base, iv_phi)
    });

    if !(expr_has_iv_dependency(fn_ir, other, iv_phi) || reads_current_dest)
        || acc_reads_self
        || phi_state_var(fn_ir, acc_phi)
            .or_else(|| fn_ir.values[acc_phi].origin_var.clone())
            .or_else(|| induction_origin_var(fn_ir, acc_phi))
            .is_none()
        || expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
        || expr_has_unstable_loop_local_load(fn_ir, lp, other)
        || expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
        || expr_has_non_vector_safe_call_in_vector_context(
            fn_ir,
            other,
            iv_phi,
            user_call_whitelist,
            &mut FxHashSet::default(),
        )
    {
        return Err(ProofFallbackReason::UnsupportedReductionExpr);
    }

    Ok(())
}

pub(super) fn certify_branch_only_multi_cond_reduction_candidate(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    entries: &[BranchOnlyCondReductionEntry],
    iv_phi: ValueId,
    user_call_whitelist: &FxHashSet<String>,
) -> Result<(), ProofFallbackReason> {
    let allowed_acc_phis = entries
        .iter()
        .map(|entry| canonical_value(fn_ir, entry.plan.acc_phi))
        .collect::<Vec<_>>();
    let allowed_merge_phis = entries
        .iter()
        .map(|entry| canonical_value(fn_ir, entry.merge_phi))
        .collect::<Vec<_>>();
    if branch_only_cond_reduction_has_non_acc_loop_state_assignments(
        fn_ir,
        lp,
        &allowed_acc_phis,
        iv_phi,
    ) || cond_reduction_has_blocking_extra_state_phi(
        fn_ir,
        lp,
        &allowed_acc_phis,
        None,
        &allowed_merge_phis,
        iv_phi,
    ) {
        return Err(ProofFallbackReason::ReductionExtraState);
    }

    for entry in entries {
        let Some(acc_var) = phi_state_var(fn_ir, entry.plan.acc_phi)
            .or_else(|| fn_ir.values[entry.plan.acc_phi].origin_var.clone())
            .or_else(|| induction_origin_var(fn_ir, entry.plan.acc_phi))
        else {
            return Err(ProofFallbackReason::UnsupportedReductionExpr);
        };
        for value in [entry.plan.then_val, entry.plan.else_val] {
            if expr_reads_var(fn_ir, value, &acc_var, &mut FxHashSet::default())
                || expr_has_non_iv_loop_state_load(fn_ir, lp, value, iv_phi)
                || expr_has_unstable_loop_local_load(fn_ir, lp, value)
                || expr_has_ambiguous_loop_local_load(fn_ir, lp, value)
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    value,
                    iv_phi,
                    user_call_whitelist,
                    &mut FxHashSet::default(),
                )
            {
                return Err(ProofFallbackReason::UnsupportedReductionExpr);
            }
        }
    }
    Ok(())
}

pub(super) fn classify_cond_reduction_store_branches(
    fn_ir: &FnIR,
    then_bb: usize,
    else_bb: usize,
    iv_phi: ValueId,
) -> Result<Option<(ValueId, ValueId, ValueId)>, ProofFallbackReason> {
    let (then_store, else_store) = match (
        classify_store_1d_in_block(fn_ir, then_bb),
        classify_store_1d_in_block(fn_ir, else_bb),
    ) {
        (BlockStore1DMatch::One(then_store), BlockStore1DMatch::One(else_store))
            if !then_store.is_vector
                && !else_store.is_vector
                && is_iv_equivalent(fn_ir, then_store.idx, iv_phi)
                && is_iv_equivalent(fn_ir, else_store.idx, iv_phi) =>
        {
            (then_store, else_store)
        }
        (BlockStore1DMatch::None, BlockStore1DMatch::None) => return Ok(None),
        _ => return Err(ProofFallbackReason::BranchStoreShape),
    };
    let then_base = canonical_value(fn_ir, then_store.base);
    let else_base = canonical_value(fn_ir, else_store.base);
    if then_base != else_base {
        return Err(ProofFallbackReason::MismatchedBranchDestinations);
    }
    Ok(Some((then_base, then_store.val, else_store.val)))
}

pub(super) fn branch_only_cond_reduction_has_non_acc_loop_state_assignments(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    allowed_acc_phis: &[ValueId],
    iv_phi: ValueId,
) -> bool {
    let allowed_acc_vars = allowed_acc_phis
        .iter()
        .filter_map(|acc_phi| {
            phi_state_var(fn_ir, *acc_phi).or_else(|| fn_ir.values[*acc_phi].origin_var.clone())
        })
        .collect::<FxHashSet<_>>();
    let iv_var = induction_origin_var(fn_ir, iv_phi);
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if allowed_acc_vars.contains(dst)
                || iv_var.as_deref() == Some(dst.as_str())
                || dst.starts_with(".arg_")
            {
                continue;
            }
            if expr_has_non_iv_loop_state_load(fn_ir, lp, *src, iv_phi) {
                return true;
            }
        }
    }
    false
}

pub(super) fn reduction_update_for_acc(
    fn_ir: &FnIR,
    acc_phi: ValueId,
    next_val: ValueId,
) -> Option<(planning::ReduceKind, ValueId)> {
    match &fn_ir.values[next_val].kind {
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } if operand_matches_acc_phi(fn_ir, *lhs, acc_phi)
            || operand_matches_acc_phi(fn_ir, *rhs, acc_phi) =>
        {
            Some((
                planning::ReduceKind::Sum,
                if operand_matches_acc_phi(fn_ir, *lhs, acc_phi) {
                    *rhs
                } else {
                    *lhs
                },
            ))
        }
        ValueKind::Binary {
            op: BinOp::Mul,
            lhs,
            rhs,
        } if operand_matches_acc_phi(fn_ir, *lhs, acc_phi)
            || operand_matches_acc_phi(fn_ir, *rhs, acc_phi) =>
        {
            Some((
                planning::ReduceKind::Prod,
                if operand_matches_acc_phi(fn_ir, *lhs, acc_phi) {
                    *rhs
                } else {
                    *lhs
                },
            ))
        }
        ValueKind::Call { callee, args, .. }
            if (callee == "min" || callee == "max") && args.len() == 2 =>
        {
            if operand_matches_acc_phi(fn_ir, args[0], acc_phi) {
                Some((
                    if callee == "min" {
                        planning::ReduceKind::Min
                    } else {
                        planning::ReduceKind::Max
                    },
                    args[1],
                ))
            } else if operand_matches_acc_phi(fn_ir, args[1], acc_phi) {
                Some((
                    if callee == "min" {
                        planning::ReduceKind::Min
                    } else {
                        planning::ReduceKind::Max
                    },
                    args[0],
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn classify_branch_only_cond_reduction_entries(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    then_bb: usize,
    else_bb: usize,
) -> Result<Option<Vec<BranchOnlyCondReductionEntry>>, ProofFallbackReason> {
    let (Terminator::Goto(then_join), Terminator::Goto(else_join)) =
        (&fn_ir.blocks[then_bb].term, &fn_ir.blocks[else_bb].term)
    else {
        return Ok(None);
    };
    if then_join != else_join || *then_join != lp.latch {
        return Ok(None);
    }
    let mut out = Vec::new();
    for (id, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, bb)| *bb == lp.latch) {
            continue;
        }
        let Some((loop_arg, _)) = args.iter().find(|(_, bb)| *bb == lp.latch) else {
            continue;
        };
        let merge_phi = preserve_phi_value(fn_ir, *loop_arg);
        let ValueKind::Phi { args: merge_args } = &fn_ir.values[merge_phi].kind else {
            continue;
        };
        if fn_ir.values[merge_phi].phi_block != Some(lp.latch) {
            continue;
        }
        let Some((then_root, _)) = merge_args.iter().find(|(_, bb)| *bb == then_bb) else {
            continue;
        };
        let Some((else_root, _)) = merge_args.iter().find(|(_, bb)| *bb == else_bb) else {
            continue;
        };
        let then_arm = extract_cond_reduce_arm(fn_ir, id, *then_root);
        let else_arm = extract_cond_reduce_arm(fn_ir, id, *else_root);
        let (then_arm, else_arm) = match (then_arm, else_arm) {
            (Some(then_arm), Some(else_arm)) => (then_arm, else_arm),
            _ => continue,
        };
        let Some((kind, then_val, else_val)) =
            resolve_cond_reduce_branch_values(fn_ir, lp, id, then_arm, else_arm)
        else {
            continue;
        };
        out.push(BranchOnlyCondReductionEntry {
            plan: ReduceCondEntry {
                kind,
                acc_phi: id,
                then_val,
                else_val,
            },
            merge_phi,
        });
    }
    if out.is_empty() {
        Ok(None)
    } else {
        Ok(Some(out))
    }
}

pub(super) fn extract_cond_reduce_arm(
    fn_ir: &FnIR,
    acc_phi: ValueId,
    root: ValueId,
) -> Option<CondReduceArm> {
    let root = preserve_phi_value(fn_ir, root);
    if direct_acc_identity_value(fn_ir, root, acc_phi) {
        return Some(CondReduceArm::Identity);
    }
    reduction_update_for_acc(fn_ir, acc_phi, root)
        .map(|(kind, val)| CondReduceArm::Update { kind, val })
}

pub(super) fn direct_acc_identity_value(fn_ir: &FnIR, root: ValueId, acc_phi: ValueId) -> bool {
    let root = preserve_phi_value(fn_ir, root);
    if canonical_value(fn_ir, root) == canonical_value(fn_ir, acc_phi) {
        return true;
    }
    let Some(acc_var) =
        phi_state_var(fn_ir, acc_phi).or_else(|| fn_ir.values[acc_phi].origin_var.clone())
    else {
        return false;
    };
    match &fn_ir.values[root].kind {
        ValueKind::Load { var } => var == &acc_var,
        ValueKind::Phi { .. } => fn_ir.values[root].origin_var.as_deref() == Some(acc_var.as_str()),
        _ => false,
    }
}

pub(super) fn resolve_cond_reduce_branch_values(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    then_arm: CondReduceArm,
    else_arm: CondReduceArm,
) -> Option<(planning::ReduceKind, ValueId, ValueId)> {
    match (then_arm, else_arm) {
        (
            CondReduceArm::Update {
                kind: then_kind,
                val: then_val,
            },
            CondReduceArm::Update {
                kind: else_kind,
                val: else_val,
            },
        ) if then_kind == else_kind => Some((then_kind, then_val, else_val)),
        (
            CondReduceArm::Update {
                kind,
                val: then_val,
            },
            CondReduceArm::Identity,
        ) => cond_reduce_identity_value(fn_ir, lp, acc_phi, kind)
            .map(|else_val| (kind, then_val, else_val)),
        (
            CondReduceArm::Identity,
            CondReduceArm::Update {
                kind,
                val: else_val,
            },
        ) => cond_reduce_identity_value(fn_ir, lp, acc_phi, kind)
            .map(|then_val| (kind, then_val, else_val)),
        _ => None,
    }
}

pub(super) fn cond_reduce_identity_value(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    kind: planning::ReduceKind,
) -> Option<ValueId> {
    let ValueKind::Phi { args } = &fn_ir.values[preserve_phi_value(fn_ir, acc_phi)].kind else {
        return None;
    };
    let seed = args
        .iter()
        .find(|(_, pred)| !lp.body.contains(pred))
        .map(|(arg, _)| canonical_value(fn_ir, *arg))?;
    match kind {
        planning::ReduceKind::Sum => match &fn_ir.values[seed].kind {
            ValueKind::Const(crate::mir::Lit::Int(0))
            | ValueKind::Const(crate::mir::Lit::Float(0.0)) => Some(seed),
            _ => None,
        },
        planning::ReduceKind::Prod => match &fn_ir.values[seed].kind {
            ValueKind::Const(crate::mir::Lit::Int(1))
            | ValueKind::Const(crate::mir::Lit::Float(1.0)) => Some(seed),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn cond_reduction_has_blocking_extra_state_phi(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    allowed_acc_phis: &[ValueId],
    dest_base: Option<ValueId>,
    allowed_merge_phis: &[ValueId],
    iv_phi: ValueId,
) -> bool {
    let allowed_acc_phis = allowed_acc_phis
        .iter()
        .map(|vid| canonical_value(fn_ir, *vid))
        .collect::<FxHashSet<_>>();
    let allowed_merge_phis = allowed_merge_phis
        .iter()
        .map(|vid| canonical_value(fn_ir, *vid))
        .collect::<FxHashSet<_>>();
    let mut seen = FxHashSet::default();
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        if allowed_acc_phis.contains(&vid)
            || allowed_merge_phis.contains(&vid)
            || is_iv_equivalent(fn_ir, vid, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, vid, iv_phi)
            || !seen.insert(vid)
        {
            continue;
        }
        if loop_carried_phi_is_unmodified(fn_ir, vid)
            || loop_carried_phi_is_invariant_passthrough(fn_ir, lp, vid, iv_phi)
            || dest_base.is_some_and(|dest_base| {
                loop_carried_phi_is_dest_last_value_shadow(fn_ir, lp, vid, dest_base, iv_phi)
            })
        {
            continue;
        }
        if debug::proof_trace_enabled() {
            let phi_args = args
                .iter()
                .map(|(arg, pred)| {
                    let arg = canonical_value(fn_ir, *arg);
                    format!(
                        "{}@{} kind={:?} origin={:?}",
                        arg, pred, fn_ir.values[arg].kind, fn_ir.values[arg].origin_var
                    )
                })
                .collect::<Vec<_>>()
                .join(" | ");
            eprintln!(
                "   [vec-proof] {} reduction-extra-state phi={} origin={:?} unmodified={} invariant_passthrough={} dest_shadow={} args=[{}]",
                fn_ir.name,
                vid,
                value.origin_var,
                loop_carried_phi_is_unmodified(fn_ir, vid),
                loop_carried_phi_is_invariant_passthrough(fn_ir, lp, vid, iv_phi),
                dest_base.is_some_and(|dest_base| {
                    loop_carried_phi_is_dest_last_value_shadow(fn_ir, lp, vid, dest_base, iv_phi)
                }),
                phi_args
            );
        }
        return true;
    }
    false
}

pub(super) fn reduction_reads_current_dest_element(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    root: ValueId,
    dest_base: ValueId,
    iv_phi: ValueId,
) -> bool {
    let root = resolve_load_alias_value(fn_ir, root);
    let iv_origin = induction_origin_var(fn_ir, iv_phi);
    match &fn_ir.values[root].kind {
        ValueKind::Index1D { base, idx, .. } => {
            same_base_value(fn_ir, *base, dest_base)
                && (is_iv_equivalent(fn_ir, *idx, iv_phi)
                    || is_origin_var_iv_alias_in_loop(fn_ir, lp, *idx, iv_phi)
                    || iv_origin.as_deref()
                        == fn_ir.values[canonical_value(fn_ir, *idx)]
                            .origin_var
                            .as_deref())
        }
        _ => false,
    }
}

pub(super) fn operand_matches_acc_phi(fn_ir: &FnIR, operand: ValueId, acc_phi: ValueId) -> bool {
    if canonical_value(fn_ir, operand) == canonical_value(fn_ir, acc_phi) {
        return true;
    }
    let Some(acc_var) =
        phi_state_var(fn_ir, acc_phi).or_else(|| fn_ir.values[acc_phi].origin_var.clone())
    else {
        return false;
    };
    matches!(
        &fn_ir.values[canonical_value(fn_ir, operand)].kind,
        ValueKind::Load { var } if var == &acc_var
    ) || fn_ir.values[canonical_value(fn_ir, operand)]
        .origin_var
        .as_deref()
        == Some(acc_var.as_str())
}

pub(super) fn var_read_after_loop_exit(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> bool {
    fn value_has_direct_post_exit_load(fn_ir: &FnIR, root: ValueId, var: &str) -> bool {
        match &fn_ir.values[root].kind {
            ValueKind::Load { var: load_var } => load_var == var,
            ValueKind::Binary { lhs, rhs, .. } => {
                value_has_direct_post_exit_load(fn_ir, *lhs, var)
                    || value_has_direct_post_exit_load(fn_ir, *rhs, var)
            }
            ValueKind::Unary { rhs, .. } => value_has_direct_post_exit_load(fn_ir, *rhs, var),
            ValueKind::RecordLit { fields } => fields
                .iter()
                .any(|(_, value)| value_has_direct_post_exit_load(fn_ir, *value, var)),
            ValueKind::FieldGet { base, .. } => value_has_direct_post_exit_load(fn_ir, *base, var),
            ValueKind::FieldSet { base, value, .. } => {
                value_has_direct_post_exit_load(fn_ir, *base, var)
                    || value_has_direct_post_exit_load(fn_ir, *value, var)
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
                .iter()
                .any(|arg| value_has_direct_post_exit_load(fn_ir, *arg, var)),
            ValueKind::Index1D { base, idx, .. } => {
                value_has_direct_post_exit_load(fn_ir, *base, var)
                    || value_has_direct_post_exit_load(fn_ir, *idx, var)
            }
            ValueKind::Index2D { base, r, c } => {
                value_has_direct_post_exit_load(fn_ir, *base, var)
                    || value_has_direct_post_exit_load(fn_ir, *r, var)
                    || value_has_direct_post_exit_load(fn_ir, *c, var)
            }
            ValueKind::Index3D { base, i, j, k } => {
                value_has_direct_post_exit_load(fn_ir, *base, var)
                    || value_has_direct_post_exit_load(fn_ir, *i, var)
                    || value_has_direct_post_exit_load(fn_ir, *j, var)
                    || value_has_direct_post_exit_load(fn_ir, *k, var)
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                value_has_direct_post_exit_load(fn_ir, *base, var)
            }
            ValueKind::Range { start, end } => {
                value_has_direct_post_exit_load(fn_ir, *start, var)
                    || value_has_direct_post_exit_load(fn_ir, *end, var)
            }
            ValueKind::Phi { .. }
            | ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::RSymbol { .. } => false,
        }
    }

    let mut seen = FxHashSet::default();
    let mut stack = lp.exits.clone();
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        let Some(block) = fn_ir.blocks.get(bid) else {
            continue;
        };
        for instr in &block.instrs {
            let reads = match instr {
                Instr::Assign { dst, src, .. } => {
                    dst != var && value_has_direct_post_exit_load(fn_ir, *src, var)
                }
                Instr::Eval { val, .. } => value_has_direct_post_exit_load(fn_ir, *val, var),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    value_has_direct_post_exit_load(fn_ir, *base, var)
                        || value_has_direct_post_exit_load(fn_ir, *idx, var)
                        || value_has_direct_post_exit_load(fn_ir, *val, var)
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    value_has_direct_post_exit_load(fn_ir, *base, var)
                        || value_has_direct_post_exit_load(fn_ir, *r, var)
                        || value_has_direct_post_exit_load(fn_ir, *c, var)
                        || value_has_direct_post_exit_load(fn_ir, *val, var)
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    value_has_direct_post_exit_load(fn_ir, *base, var)
                        || value_has_direct_post_exit_load(fn_ir, *i, var)
                        || value_has_direct_post_exit_load(fn_ir, *j, var)
                        || value_has_direct_post_exit_load(fn_ir, *k, var)
                        || value_has_direct_post_exit_load(fn_ir, *val, var)
                }
            };
            if reads {
                return true;
            }
        }
        match block.term {
            Terminator::Goto(next) => stack.push(next),
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                if value_has_direct_post_exit_load(fn_ir, cond, var) {
                    return true;
                }
                stack.push(then_bb);
                stack.push(else_bb);
            }
            Terminator::Return(Some(ret)) => {
                if value_has_direct_post_exit_load(fn_ir, ret, var) {
                    return true;
                }
            }
            Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }
    false
}
