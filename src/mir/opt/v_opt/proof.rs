use super::analysis::{
    affine_iv_offset, as_safe_loop_index, canonical_value, classify_store_1d_in_block,
    collect_loop_shadow_vars_for_dest, expr_has_iv_dependency,
    expr_has_non_vector_safe_call_in_vector_context, induction_origin_var,
    is_condition_vectorizable, is_iv_equivalent, is_loop_invariant_scalar_expr,
    is_origin_var_iv_alias_in_loop, is_vector_safe_call, is_vector_safe_call_chain_expr,
    is_vectorizable_expr, loop_carried_phi_is_dest_last_value_shadow,
    loop_carried_phi_is_invariant_passthrough, loop_carried_phi_is_unmodified,
    loop_covers_whole_destination, loop_has_non_iv_loop_carried_state_except,
    loop_has_store_effect, preserve_phi_value, resolve_base_var, resolve_load_alias_value,
    same_base_value,
};
use super::debug::proof_engine_enabled;
use super::planning::{
    CallMapArg, ExprMapStoreCandidate, ExprMapStoreSpec, build_expr_map_entries,
    classify_expr_map_store_candidate, expr_has_non_iv_loop_state_load,
    reduction_has_extra_state_phi, reduction_has_non_acc_loop_state_assignments,
    validate_expr_map_rhs,
};
use super::reconstruct::{
    expr_has_ambiguous_loop_local_load, expr_has_unstable_loop_local_load, expr_reads_var,
    phi_state_var,
};
use super::types::{
    BlockStore1DMatch, CertifiedPlan, ProofFallbackReason, ProofOutcome, ReduceCondEntry,
};
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::{FnIR, Instr, Lit, Terminator, ValueId, ValueKind, VarId};
use crate::syntax::ast::BinOp;
use rustc_hash::FxHashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ProofConfig {
    enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Store1DSummary {
    base: ValueId,
    idx: ValueId,
    val: ValueId,
    is_safe: bool,
    is_na_safe: bool,
    is_vector: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LoopSummary {
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    store: Store1DSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CondReduceArm {
    Identity,
    Update {
        kind: super::planning::ReduceKind,
        val: ValueId,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BranchOnlyCondReductionEntry {
    plan: ReduceCondEntry,
    merge_phi: ValueId,
}

impl ProofConfig {
    fn default_enabled() -> Self {
        Self {
            enabled: proof_engine_enabled(),
        }
    }
}

pub(super) fn analyze_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> ProofOutcome {
    analyze_loop_with_config(
        fn_ir,
        lp,
        user_call_whitelist,
        ProofConfig::default_enabled(),
    )
}

fn analyze_loop_with_config(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
    config: ProofConfig,
) -> ProofOutcome {
    if !config.enabled {
        return ProofOutcome::FallbackToPattern {
            reason: ProofFallbackReason::Disabled,
        };
    }
    if lp.iv.is_none() && !loop_has_store_effect(fn_ir, lp) {
        return ProofOutcome::NotApplicable {
            reason: classify_storeless_loop_reason(fn_ir, lp, None),
        };
    }
    if loop_contains_nested_induction_like_phi(fn_ir, lp) {
        return ProofOutcome::NotApplicable {
            reason: ProofFallbackReason::UnsupportedLoopShape,
        };
    }

    let cond_reduce_result = certify_simple_cond_reduction(fn_ir, lp, user_call_whitelist);
    if let Ok(certified) = cond_reduce_result {
        return ProofOutcome::Certified(certified);
    }
    let cond_reduce_reason = cond_reduce_result.err();
    if let Some(reason) = cond_reduce_reason
        && super::debug::proof_trace_enabled()
    {
        super::debug::trace_proof_status(
            fn_ir,
            lp,
            &format!("cond-reduce-reject: {}", reason.label()),
        );
    }

    let cond_result = certify_simple_cond_map(fn_ir, lp, user_call_whitelist);
    if let Ok(certified) = cond_result {
        return ProofOutcome::Certified(certified);
    }
    let cond_reason = cond_result.err();

    let reduction_result = certify_simple_reduction(fn_ir, lp, user_call_whitelist);
    if let Ok(certified) = reduction_result {
        return ProofOutcome::Certified(certified);
    }
    let reduction_reason = reduction_result.err();

    let multi_expr_result = certify_simple_multi_expr_map(fn_ir, lp, user_call_whitelist);
    if let Ok(certified) = multi_expr_result {
        return ProofOutcome::Certified(certified);
    }
    if let Err(reason) = &multi_expr_result
        && super::debug::proof_trace_enabled()
    {
        super::debug::trace_proof_status(
            fn_ir,
            lp,
            &format!("multi-expr-reject: {}", reason.label()),
        );
    }
    let multi_expr_reason = multi_expr_result.err();

    let summary = match summarize_loop(fn_ir, lp) {
        Ok(summary) => summary,
        Err(reason) => {
            if matches!(reason, ProofFallbackReason::MissingStore) {
                let storeless_reason =
                    classify_storeless_loop_reason(fn_ir, lp, lp.iv.as_ref().map(|iv| iv.phi_val));
                let specific_reason = [
                    cond_reduce_reason,
                    cond_reason,
                    reduction_reason,
                    multi_expr_reason,
                ]
                .into_iter()
                .flatten()
                .find(|specific| {
                    !is_generic_pattern_reason(*specific)
                        && !matches!(*specific, ProofFallbackReason::UnsupportedLoopShape)
                        && !matches!(*specific, ProofFallbackReason::UnsupportedCondition)
                });
                if let Some(reason) = specific_reason {
                    if matches!(reason, ProofFallbackReason::ReductionExtraState)
                        && matches!(
                            storeless_reason,
                            ProofFallbackReason::StorelessReductionLoop
                                | ProofFallbackReason::StorelessConditionalLoop
                                | ProofFallbackReason::StorelessStateLoop
                                | ProofFallbackReason::StorelessPlainLoop
                        )
                    {
                        return ProofOutcome::NotApplicable {
                            reason: storeless_reason,
                        };
                    }
                    return ProofOutcome::FallbackToPattern { reason };
                }
                return match storeless_reason {
                    ProofFallbackReason::StorelessStateLoop
                    | ProofFallbackReason::StorelessPlainLoop => ProofOutcome::NotApplicable {
                        reason: storeless_reason,
                    },
                    _ => ProofOutcome::FallbackToPattern {
                        reason: storeless_reason,
                    },
                };
            }
            if matches!(reason, ProofFallbackReason::UnsupportedLoopShape)
                && !loop_has_store_effect(fn_ir, lp)
            {
                let storeless_reason =
                    classify_storeless_loop_reason(fn_ir, lp, lp.iv.as_ref().map(|iv| iv.phi_val));
                let has_specific_reason = [
                    cond_reduce_reason,
                    cond_reason,
                    reduction_reason,
                    multi_expr_reason,
                ]
                .into_iter()
                .flatten()
                .any(|specific| {
                    !is_generic_pattern_reason(specific)
                        && !matches!(specific, ProofFallbackReason::UnsupportedLoopShape)
                        && !matches!(specific, ProofFallbackReason::UnsupportedCondition)
                });
                if !has_specific_reason
                    && matches!(
                        storeless_reason,
                        ProofFallbackReason::StorelessConditionalLoop
                            | ProofFallbackReason::StorelessStateLoop
                            | ProofFallbackReason::StorelessPlainLoop
                    )
                {
                    return ProofOutcome::NotApplicable {
                        reason: storeless_reason,
                    };
                }
            }
            for specific in [
                cond_reduce_reason,
                cond_reason,
                reduction_reason,
                multi_expr_reason,
            ]
            .into_iter()
            .flatten()
            {
                if !is_generic_pattern_reason(specific) {
                    return ProofOutcome::FallbackToPattern { reason: specific };
                }
            }
            return ProofOutcome::FallbackToPattern { reason };
        }
    };

    let map_result = certify_simple_map(fn_ir, lp, &summary, user_call_whitelist);
    if let Ok(certified) = map_result {
        return ProofOutcome::Certified(certified);
    }
    if let Err(reason) = &map_result
        && super::debug::proof_trace_enabled()
    {
        super::debug::trace_proof_status(fn_ir, lp, &format!("map-reject: {}", reason.label()));
    }

    let shifted_result = certify_simple_shifted_map(fn_ir, lp, &summary);
    if let Ok(certified) = shifted_result {
        return ProofOutcome::Certified(certified);
    }
    if let Err(reason) = &shifted_result
        && super::debug::proof_trace_enabled()
    {
        super::debug::trace_proof_status(fn_ir, lp, &format!("shifted-reject: {}", reason.label()));
    }

    let call_result = certify_simple_call_map(fn_ir, lp, &summary, user_call_whitelist);
    if let Ok(certified) = call_result {
        return ProofOutcome::Certified(certified);
    }
    if let Err(reason) = &call_result
        && super::debug::proof_trace_enabled()
    {
        super::debug::trace_proof_status(fn_ir, lp, &format!("call-reject: {}", reason.label()));
    }

    match certify_simple_expr_map(fn_ir, lp, &summary, user_call_whitelist) {
        Ok(certified) => ProofOutcome::Certified(certified),
        Err(expr_reason) => {
            if super::debug::proof_trace_enabled() {
                super::debug::trace_proof_status(
                    fn_ir,
                    lp,
                    &format!("expr-reject: {}", expr_reason.label()),
                );
            }
            let fallback_reason = choose_fallback_reason([
                cond_reduce_reason,
                cond_reason,
                reduction_reason,
                multi_expr_reason,
                Some(expr_reason),
                call_result.err(),
                shifted_result.err(),
                map_result.err(),
            ]);
            if matches!(fallback_reason, ProofFallbackReason::ShadowState)
                && loop_is_storeful_stateful_helper(fn_ir, lp)
            {
                ProofOutcome::NotApplicable {
                    reason: ProofFallbackReason::StorefulStateLoop,
                }
            } else {
                ProofOutcome::FallbackToPattern {
                    reason: fallback_reason,
                }
            }
        }
    }
}

fn is_generic_pattern_reason(reason: ProofFallbackReason) -> bool {
    matches!(
        reason,
        ProofFallbackReason::NotYetImplemented
            | ProofFallbackReason::MissingStore
            | ProofFallbackReason::NotSimpleMap
            | ProofFallbackReason::NotSimpleCondMap
            | ProofFallbackReason::NotSimpleReduction
            | ProofFallbackReason::NotSimpleExprMap
            | ProofFallbackReason::NotSimpleCallMap
    )
}

fn classify_storeless_loop_reason(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    iv_phi: Option<ValueId>,
) -> ProofFallbackReason {
    let has_inner_branch = lp
        .body
        .iter()
        .any(|bid| *bid != lp.header && matches!(fn_ir.blocks[*bid].term, Terminator::If { .. }));
    if has_inner_branch {
        return ProofFallbackReason::StorelessConditionalLoop;
    }

    let Some(iv_phi) = iv_phi else {
        return ProofFallbackReason::StorelessPlainLoop;
    };
    let iv_phi = canonical_value(fn_ir, iv_phi);
    let mut saw_non_iv_phi = false;
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if args.is_empty() || !lp.body.contains(&phi_bb) {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        if vid == iv_phi
            || is_iv_equivalent(fn_ir, vid, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, vid, iv_phi)
        {
            continue;
        }
        saw_non_iv_phi = true;
        let Some((next_val, _)) = args.iter().find(|(_, bb)| *bb == lp.latch) else {
            continue;
        };
        if reduction_update_for_acc(fn_ir, vid, *next_val).is_some() {
            return ProofFallbackReason::StorelessReductionLoop;
        }
    }

    if saw_non_iv_phi {
        return ProofFallbackReason::StorelessStateLoop;
    }

    ProofFallbackReason::StorelessPlainLoop
}

fn choose_fallback_reason<const N: usize>(
    reasons: [Option<ProofFallbackReason>; N],
) -> ProofFallbackReason {
    for reason in reasons.into_iter().flatten() {
        if !is_generic_pattern_reason(reason) {
            return reason;
        }
    }
    reasons
        .into_iter()
        .flatten()
        .last()
        .unwrap_or(ProofFallbackReason::NotYetImplemented)
}

fn collect_store_destination_vars(fn_ir: &FnIR, lp: &LoopInfo) -> Vec<VarId> {
    let mut vars = Vec::new();
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let base = match instr {
                Instr::StoreIndex1D { base, .. } => *base,
                Instr::StoreIndex2D { base, .. } => *base,
                Instr::StoreIndex3D { base, .. } => *base,
                _ => continue,
            };
            if let Some(var) = resolve_base_var(fn_ir, base) {
                vars.push(var);
            }
        }
    }
    vars.sort_unstable();
    vars.dedup();
    vars
}

fn loop_is_storeful_stateful_helper(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    if !loop_has_store_effect(fn_ir, lp) {
        return false;
    }
    let allowed_dests = collect_store_destination_vars(fn_ir, lp);
    !allowed_dests.is_empty()
        && loop_has_non_iv_loop_carried_state_except(fn_ir, lp, &allowed_dests)
}

fn loop_contains_nested_induction_like_phi(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let Some(iv) = lp.iv.as_ref() else {
        return false;
    };
    let outer_iv = canonical_value(fn_ir, iv.phi_val);
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) || args.len() != 2 {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        if vid == outer_iv
            || is_iv_equivalent(fn_ir, vid, outer_iv)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, vid, outer_iv)
        {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, bb)| lp.body.contains(bb)) else {
            continue;
        };
        let next_val = canonical_value(fn_ir, *next_val);
        let is_step = match &fn_ir.values[next_val].kind {
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            } => {
                (canonical_value(fn_ir, *lhs) == vid && is_const_one_like(fn_ir, *rhs))
                    || (canonical_value(fn_ir, *rhs) == vid && is_const_one_like(fn_ir, *lhs))
            }
            ValueKind::Binary {
                op: BinOp::Sub,
                lhs,
                rhs,
            } => canonical_value(fn_ir, *lhs) == vid && is_const_one_like(fn_ir, *rhs),
            _ => false,
        };
        if is_step {
            return true;
        }
    }
    false
}

fn is_const_one_like(fn_ir: &FnIR, vid: ValueId) -> bool {
    match fn_ir.values[canonical_value(fn_ir, vid)].kind {
        ValueKind::Const(Lit::Int(1)) => true,
        ValueKind::Const(Lit::Float(v)) => (v - 1.0).abs() < f64::EPSILON,
        _ => false,
    }
}

fn summarize_loop(fn_ir: &FnIR, lp: &LoopInfo) -> Result<LoopSummary, ProofFallbackReason> {
    let iv = lp
        .iv
        .as_ref()
        .ok_or(ProofFallbackReason::MissingInductionVar)?;
    let end = lp.limit.ok_or(ProofFallbackReason::UnsupportedLoopShape)?;
    let mut store: Option<Store1DSummary> = None;
    for &bid in &lp.body {
        if bid != lp.header && matches!(fn_ir.blocks[bid].term, Terminator::If { .. }) {
            return Err(ProofFallbackReason::UnsupportedLoopShape);
        }
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_safe,
                    is_na_safe,
                    is_vector,
                    ..
                } => {
                    if store.is_some() {
                        return Err(ProofFallbackReason::MultipleStores);
                    }
                    store = Some(Store1DSummary {
                        base: *base,
                        idx: *idx,
                        val: *val,
                        is_safe: *is_safe,
                        is_na_safe: *is_na_safe,
                        is_vector: *is_vector,
                    });
                }
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => {
                    return Err(ProofFallbackReason::UnsupportedLoopShape);
                }
                Instr::Assign { .. } | Instr::Eval { .. } => {}
            }
        }
    }

    let store = store.ok_or(ProofFallbackReason::MissingStore)?;
    Ok(LoopSummary {
        iv_phi: iv.phi_val,
        start: iv.init_val,
        end,
        store,
    })
}

fn certify_simple_reduction(
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
                    super::planning::ReduceKind::Sum,
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
                    super::planning::ReduceKind::Prod,
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
                            super::planning::ReduceKind::Min
                        } else {
                            super::planning::ReduceKind::Max
                        },
                        args[1],
                    ))
                } else if operand_matches_acc_phi(fn_ir, args[1], id) {
                    Some((
                        if callee == "min" {
                            super::planning::ReduceKind::Min
                        } else {
                            super::planning::ReduceKind::Max
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
                    plan: super::planning::VectorPlan::Reduce {
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

fn certify_reduction_candidate<F>(
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
        if super::debug::proof_trace_enabled() {
            super::debug::trace_proof_status(
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

fn certify_simple_cond_reduction(
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
                plan: super::planning::VectorPlan::ReduceCond {
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
                plan: super::planning::VectorPlan::ReduceCond {
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
            plan: super::planning::VectorPlan::MultiReduceCond {
                cond,
                entries: entries.into_iter().map(|entry| entry.plan).collect(),
                iv_phi,
            },
        });
    }

    Err(ProofFallbackReason::BranchStoreShape)
}

fn certify_cond_reduction_candidate(
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

fn certify_branch_only_multi_cond_reduction_candidate(
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

fn classify_cond_reduction_store_branches(
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

fn branch_only_cond_reduction_has_non_acc_loop_state_assignments(
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

fn reduction_update_for_acc(
    fn_ir: &FnIR,
    acc_phi: ValueId,
    next_val: ValueId,
) -> Option<(super::planning::ReduceKind, ValueId)> {
    match &fn_ir.values[next_val].kind {
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } if operand_matches_acc_phi(fn_ir, *lhs, acc_phi)
            || operand_matches_acc_phi(fn_ir, *rhs, acc_phi) =>
        {
            Some((
                super::planning::ReduceKind::Sum,
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
                super::planning::ReduceKind::Prod,
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
                        super::planning::ReduceKind::Min
                    } else {
                        super::planning::ReduceKind::Max
                    },
                    args[1],
                ))
            } else if operand_matches_acc_phi(fn_ir, args[1], acc_phi) {
                Some((
                    if callee == "min" {
                        super::planning::ReduceKind::Min
                    } else {
                        super::planning::ReduceKind::Max
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

fn classify_branch_only_cond_reduction_entries(
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

fn extract_cond_reduce_arm(fn_ir: &FnIR, acc_phi: ValueId, root: ValueId) -> Option<CondReduceArm> {
    let root = preserve_phi_value(fn_ir, root);
    if direct_acc_identity_value(fn_ir, root, acc_phi) {
        return Some(CondReduceArm::Identity);
    }
    reduction_update_for_acc(fn_ir, acc_phi, root)
        .map(|(kind, val)| CondReduceArm::Update { kind, val })
}

fn direct_acc_identity_value(fn_ir: &FnIR, root: ValueId, acc_phi: ValueId) -> bool {
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

fn resolve_cond_reduce_branch_values(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    then_arm: CondReduceArm,
    else_arm: CondReduceArm,
) -> Option<(super::planning::ReduceKind, ValueId, ValueId)> {
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

fn cond_reduce_identity_value(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    kind: super::planning::ReduceKind,
) -> Option<ValueId> {
    let ValueKind::Phi { args } = &fn_ir.values[preserve_phi_value(fn_ir, acc_phi)].kind else {
        return None;
    };
    let seed = args
        .iter()
        .find(|(_, pred)| !lp.body.contains(pred))
        .map(|(arg, _)| canonical_value(fn_ir, *arg))?;
    match kind {
        super::planning::ReduceKind::Sum => match &fn_ir.values[seed].kind {
            ValueKind::Const(crate::mir::Lit::Int(0))
            | ValueKind::Const(crate::mir::Lit::Float(0.0)) => Some(seed),
            _ => None,
        },
        super::planning::ReduceKind::Prod => match &fn_ir.values[seed].kind {
            ValueKind::Const(crate::mir::Lit::Int(1))
            | ValueKind::Const(crate::mir::Lit::Float(1.0)) => Some(seed),
            _ => None,
        },
        _ => None,
    }
}

fn cond_reduction_has_blocking_extra_state_phi(
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
        if super::debug::proof_trace_enabled() {
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

fn reduction_reads_current_dest_element(
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

fn operand_matches_acc_phi(fn_ir: &FnIR, operand: ValueId, acc_phi: ValueId) -> bool {
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

fn var_read_after_loop_exit(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> bool {
    fn value_has_direct_post_exit_load(fn_ir: &FnIR, root: ValueId, var: &str) -> bool {
        match &fn_ir.values[root].kind {
            ValueKind::Load { var: load_var } => load_var == var,
            ValueKind::Binary { lhs, rhs, .. } => {
                value_has_direct_post_exit_load(fn_ir, *lhs, var)
                    || value_has_direct_post_exit_load(fn_ir, *rhs, var)
            }
            ValueKind::Unary { rhs, .. } => value_has_direct_post_exit_load(fn_ir, *rhs, var),
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

fn certify_simple_cond_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Result<CertifiedPlan, ProofFallbackReason> {
    let iv = lp
        .iv
        .as_ref()
        .ok_or(ProofFallbackReason::MissingInductionVar)?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit.ok_or(ProofFallbackReason::UnsupportedLoopShape)?;
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
                return Err(ProofFallbackReason::UnsupportedLoopShape);
            }
            if !lp.body.contains(&then_bb) || !lp.body.contains(&else_bb) {
                return Err(ProofFallbackReason::BranchLeavesLoopBody);
            }
            branch = Some((cond, then_bb, else_bb));
        }
    }

    let Some((cond, then_bb, else_bb)) = branch else {
        return Err(ProofFallbackReason::NotSimpleCondMap);
    };

    if !is_condition_vectorizable(fn_ir, cond, iv_phi, lp, user_call_whitelist)
        || !expr_has_iv_dependency(fn_ir, cond, iv_phi)
    {
        return Err(ProofFallbackReason::UnsupportedCondition);
    }

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
        _ => return Err(ProofFallbackReason::BranchStoreShape),
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
            (Some(lhs), Some(rhs)) if lhs == rhs => then_base,
            _ => return Err(ProofFallbackReason::MismatchedBranchDestinations),
        }
    };

    let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest_base).into_iter().collect();
    let shadow_vars =
        collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest_base, iv_phi)
            .ok_or(ProofFallbackReason::ShadowState)?;

    if !is_vectorizable_expr(fn_ir, then_store.val, iv_phi, lp, true, false)
        || !is_vectorizable_expr(fn_ir, else_store.val, iv_phi, lp, true, false)
    {
        return Err(ProofFallbackReason::UnsupportedConditionalValues);
    }

    Ok(CertifiedPlan {
        plan: super::planning::VectorPlan::CondMap {
            dest: dest_base,
            cond,
            then_val: then_store.val,
            else_val: else_store.val,
            iv_phi,
            start,
            end,
            whole_dest: loop_covers_whole_destination(lp, fn_ir, dest_base, start),
            shadow_vars,
        },
    })
}

fn certify_simple_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    summary: &LoopSummary,
    user_call_whitelist: &FxHashSet<String>,
) -> Result<CertifiedPlan, ProofFallbackReason> {
    let store = summary.store;
    if store.is_vector
        || !store.is_safe
        || !store.is_na_safe
        || !is_iv_equivalent(fn_ir, store.idx, summary.iv_phi)
    {
        return Err(ProofFallbackReason::NonCanonicalStore);
    }

    let Some(dest_var) = resolve_base_var(fn_ir, store.base) else {
        return Err(ProofFallbackReason::UnresolvableDestination);
    };
    if !loop_covers_whole_destination(lp, fn_ir, store.base, summary.start) {
        return Err(ProofFallbackReason::NotWholeDestination);
    }

    let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[store.val].kind else {
        return Err(ProofFallbackReason::NotSimpleMap);
    };

    let lhs_idx = as_safe_loop_index(fn_ir, *lhs, summary.iv_phi);
    let rhs_idx = as_safe_loop_index(fn_ir, *rhs, summary.iv_phi);

    let (src, other) = if let (Some(lhs_base), Some(rhs_base)) = (lhs_idx, rhs_idx) {
        if lhs_base != rhs_base {
            return Err(ProofFallbackReason::NotSimpleMap);
        }
        if !loop_covers_whole_destination(lp, fn_ir, lhs_base, summary.start) {
            return Err(ProofFallbackReason::NotWholeDestination);
        }
        (lhs_base, rhs_base)
    } else if let Some(lhs_base) = lhs_idx {
        if !loop_covers_whole_destination(lp, fn_ir, lhs_base, summary.start) {
            return Err(ProofFallbackReason::NotWholeDestination);
        }
        if !is_loop_invariant_scalar_expr(fn_ir, *rhs, summary.iv_phi, user_call_whitelist) {
            return Err(ProofFallbackReason::UnsupportedMapOperands);
        }
        (lhs_base, *rhs)
    } else if let Some(rhs_base) = rhs_idx {
        if !loop_covers_whole_destination(lp, fn_ir, rhs_base, summary.start) {
            return Err(ProofFallbackReason::NotWholeDestination);
        }
        if !is_loop_invariant_scalar_expr(fn_ir, *lhs, summary.iv_phi, user_call_whitelist) {
            return Err(ProofFallbackReason::UnsupportedMapOperands);
        }
        (*lhs, rhs_base)
    } else {
        return Err(ProofFallbackReason::NotSimpleMap);
    };

    let allowed_dests: Vec<VarId> = vec![dest_var];
    let shadow_vars =
        collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, store.base, summary.iv_phi)
            .ok_or(ProofFallbackReason::ShadowState)?;

    Ok(CertifiedPlan {
        plan: super::planning::VectorPlan::Map {
            dest: store.base,
            src,
            op: *op,
            other,
            shadow_vars,
        },
    })
}

fn certify_simple_shifted_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    summary: &LoopSummary,
) -> Result<CertifiedPlan, ProofFallbackReason> {
    let store = summary.store;
    if store.is_vector
        || !store.is_safe
        || !store.is_na_safe
        || !is_iv_equivalent(fn_ir, store.idx, summary.iv_phi)
    {
        return Err(ProofFallbackReason::NonCanonicalStore);
    }

    let rhs = resolve_load_alias_value(fn_ir, store.val);
    let ValueKind::Index1D {
        base: src_base,
        idx: src_idx,
        is_safe,
        is_na_safe,
    } = fn_ir.values[rhs].kind.clone()
    else {
        return Err(ProofFallbackReason::NotSimpleMap);
    };
    if !is_safe || !is_na_safe {
        return Err(ProofFallbackReason::UnsupportedMapOperands);
    }

    let Some(offset) = affine_iv_offset(fn_ir, src_idx, summary.iv_phi) else {
        return Err(ProofFallbackReason::NotSimpleMap);
    };
    if offset == 0 {
        return Err(ProofFallbackReason::NotSimpleMap);
    }

    let dest = canonical_value(fn_ir, store.base);
    let src = canonical_value(fn_ir, src_base);
    if same_base_value(fn_ir, dest, src) && offset < 0 {
        return Err(ProofFallbackReason::UnsupportedMapOperands);
    }

    let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest).into_iter().collect();
    if loop_has_non_iv_loop_carried_state_except(fn_ir, lp, &allowed_dests) {
        return Err(ProofFallbackReason::ShadowState);
    }

    Ok(CertifiedPlan {
        plan: super::planning::VectorPlan::ShiftedMap {
            dest,
            src,
            start: summary.start,
            end: summary.end,
            offset,
        },
    })
}

fn certify_simple_multi_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Result<CertifiedPlan, ProofFallbackReason> {
    let iv = lp
        .iv
        .as_ref()
        .ok_or(ProofFallbackReason::MissingInductionVar)?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit.ok_or(ProofFallbackReason::UnsupportedLoopShape)?;
    let mut found: Vec<(ValueId, ValueId)> = Vec::new();

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } | Instr::Eval { .. } => {}
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => {
                    return Err(ProofFallbackReason::NotSimpleExprMap);
                }
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => {
                    let Some(candidate) = classify_expr_map_store_candidate(
                        fn_ir,
                        lp,
                        user_call_whitelist,
                        ExprMapStoreSpec {
                            base: *base,
                            idx: *idx,
                            expr: *val,
                            is_vector: *is_vector,
                        },
                        iv_phi,
                    ) else {
                        return Err(ProofFallbackReason::NotSimpleExprMap);
                    };
                    let ExprMapStoreCandidate::Standard { dest, expr } = candidate else {
                        return Err(ProofFallbackReason::NotSimpleExprMap);
                    };
                    if found.iter().any(|(existing_dest, _)| {
                        match (
                            resolve_base_var(fn_ir, *existing_dest),
                            resolve_base_var(fn_ir, dest),
                        ) {
                            (Some(a), Some(b)) => a == b,
                            _ => same_base_value(fn_ir, *existing_dest, dest),
                        }
                    }) {
                        return Err(ProofFallbackReason::NotSimpleExprMap);
                    }
                    found.push((dest, expr));
                }
            }
        }
    }

    if found.len() <= 1 {
        return Err(ProofFallbackReason::NotSimpleExprMap);
    }

    let entries = build_expr_map_entries(fn_ir, lp, iv_phi, start, found)
        .ok_or(ProofFallbackReason::ShadowState)?;
    Ok(CertifiedPlan {
        plan: super::planning::VectorPlan::MultiExprMap {
            entries,
            iv_phi,
            start,
            end,
        },
    })
}

fn certify_simple_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    summary: &LoopSummary,
    user_call_whitelist: &FxHashSet<String>,
) -> Result<CertifiedPlan, ProofFallbackReason> {
    let store = summary.store;
    if store.is_vector || !is_iv_equivalent(fn_ir, store.idx, summary.iv_phi) {
        return Err(ProofFallbackReason::NonCanonicalStore);
    }
    let Some(dest_var) = resolve_base_var(fn_ir, store.base) else {
        return Err(ProofFallbackReason::UnresolvableDestination);
    };
    let whole_dest = loop_covers_whole_destination(lp, fn_ir, store.base, summary.start);

    let expr = resolve_load_alias_value(fn_ir, store.val);
    if !validate_expr_map_rhs(
        fn_ir,
        lp,
        user_call_whitelist,
        store.base,
        expr,
        summary.iv_phi,
    ) {
        return Err(ProofFallbackReason::NotSimpleExprMap);
    }

    let allowed_dests: Vec<VarId> = vec![dest_var];
    let shadow_vars =
        collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, store.base, summary.iv_phi)
            .ok_or(ProofFallbackReason::ShadowState)?;

    Ok(CertifiedPlan {
        plan: super::planning::VectorPlan::ExprMap {
            dest: store.base,
            expr,
            iv_phi: summary.iv_phi,
            start: summary.start,
            end: summary.end,
            whole_dest,
            shadow_vars,
        },
    })
}

fn certify_simple_call_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    summary: &LoopSummary,
    user_call_whitelist: &FxHashSet<String>,
) -> Result<CertifiedPlan, ProofFallbackReason> {
    let store = summary.store;
    if store.is_vector || !is_iv_equivalent(fn_ir, store.idx, summary.iv_phi) {
        return Err(ProofFallbackReason::NonCanonicalStore);
    }
    let Some(dest_var) = resolve_base_var(fn_ir, store.base) else {
        return Err(ProofFallbackReason::UnresolvableDestination);
    };
    let whole_dest = loop_covers_whole_destination(lp, fn_ir, store.base, summary.start);

    let expr = resolve_load_alias_value(fn_ir, store.val);
    let ValueKind::Call { callee, args, .. } = &fn_ir.values[expr].kind else {
        return Err(ProofFallbackReason::NotSimpleCallMap);
    };
    if !is_vector_safe_call(callee, args.len(), user_call_whitelist) {
        return Err(ProofFallbackReason::UnsupportedCallMapArgs);
    }

    let mut mapped_args = Vec::with_capacity(args.len());
    let mut has_vector_arg = false;
    for arg in args {
        let arg = resolve_load_alias_value(fn_ir, *arg);
        if expr_has_iv_dependency(fn_ir, arg, summary.iv_phi) {
            if !is_vector_safe_call_chain_expr(fn_ir, arg, summary.iv_phi, lp, user_call_whitelist)
            {
                return Err(ProofFallbackReason::UnsupportedCallMapArgs);
            }
            mapped_args.push(CallMapArg {
                value: arg,
                vectorized: true,
            });
            has_vector_arg = true;
        } else {
            if !is_loop_invariant_scalar_expr(fn_ir, arg, summary.iv_phi, user_call_whitelist) {
                return Err(ProofFallbackReason::UnsupportedCallMapArgs);
            }
            mapped_args.push(CallMapArg {
                value: arg,
                vectorized: false,
            });
        }
    }
    if mapped_args.is_empty() || !has_vector_arg {
        return Err(ProofFallbackReason::UnsupportedCallMapArgs);
    }

    let allowed_dests: Vec<VarId> = vec![dest_var];
    let shadow_vars =
        collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, store.base, summary.iv_phi)
            .ok_or(ProofFallbackReason::ShadowState)?;

    Ok(CertifiedPlan {
        plan: super::planning::VectorPlan::CallMap {
            dest: store.base,
            callee: callee.clone(),
            args: mapped_args,
            iv_phi: summary.iv_phi,
            start: summary.start,
            end: summary.end,
            whole_dest,
            shadow_vars,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::super::transform::try_apply_vectorization_transactionally;
    use super::*;
    use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo};
    use crate::mir::{BinOp, Facts};
    use crate::utils::Span;

    fn dummy_loop() -> LoopInfo {
        LoopInfo {
            header: 0,
            latch: 0,
            exits: Vec::new(),
            body: FxHashSet::default(),
            is_seq_len: None,
            is_seq_along: None,
            iv: None,
            limit: None,
            limit_adjust: 0,
        }
    }

    #[test]
    fn disabled_config_falls_back_with_disabled_reason() {
        let fn_ir = FnIR::new("proof_dummy".to_string(), vec![]);
        let loop_info = dummy_loop();
        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loop_info,
            &FxHashSet::default(),
            ProofConfig { enabled: false },
        );
        assert!(matches!(
            outcome,
            ProofOutcome::FallbackToPattern {
                reason: ProofFallbackReason::Disabled
            }
        ));
    }

    #[test]
    fn enabled_config_falls_back_with_missing_induction_var_reason() {
        let fn_ir = FnIR::new("proof_dummy".to_string(), vec![]);
        let loop_info = dummy_loop();
        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loop_info,
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        assert!(matches!(
            outcome,
            ProofOutcome::NotApplicable {
                reason: ProofFallbackReason::StorelessPlainLoop
            }
        ));
    }

    fn simple_map_fn() -> FnIR {
        base_single_store_loop_fn("proof_map", |fn_ir, load_x, load_i, one| {
            let read_x = fn_ir.add_value(
                ValueKind::Index1D {
                    base: load_x,
                    idx: load_i,
                    is_safe: true,
                    is_na_safe: true,
                },
                Span::default(),
                Facts::empty(),
                None,
            );
            fn_ir.add_value(
                ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: read_x,
                    rhs: one,
                },
                Span::default(),
                Facts::empty(),
                None,
            )
        })
    }

    fn simple_expr_map_fn() -> FnIR {
        base_single_store_loop_fn("proof_expr_map", |fn_ir, load_x, load_i, one| {
            let two = fn_ir.add_value(
                ValueKind::Const(crate::mir::Lit::Int(2)),
                Span::default(),
                Facts::empty(),
                None,
            );
            let read_x = fn_ir.add_value(
                ValueKind::Index1D {
                    base: load_x,
                    idx: load_i,
                    is_safe: true,
                    is_na_safe: true,
                },
                Span::default(),
                Facts::empty(),
                None,
            );
            let plus = fn_ir.add_value(
                ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: read_x,
                    rhs: one,
                },
                Span::default(),
                Facts::empty(),
                None,
            );
            fn_ir.add_value(
                ValueKind::Binary {
                    op: BinOp::Mul,
                    lhs: plus,
                    rhs: two,
                },
                Span::default(),
                Facts::empty(),
                None,
            )
        })
    }

    fn simple_call_map_fn() -> FnIR {
        base_single_store_loop_fn("proof_call_map", |fn_ir, load_x, load_i, _one| {
            let read_x = fn_ir.add_value(
                ValueKind::Index1D {
                    base: load_x,
                    idx: load_i,
                    is_safe: true,
                    is_na_safe: true,
                },
                Span::default(),
                Facts::empty(),
                None,
            );
            fn_ir.add_value(
                ValueKind::Call {
                    callee: "abs".to_string(),
                    args: vec![read_x],
                    names: vec![None],
                },
                Span::default(),
                Facts::empty(),
                None,
            )
        })
    }

    fn simple_shifted_map_fn() -> FnIR {
        let mut fn_ir = FnIR::new("proof_shifted_map".to_string(), vec!["x".to_string()]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        let param_x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let load_i = fn_ir.add_value(
            ValueKind::Load {
                var: "i".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let len_x = fn_ir.add_value(
            ValueKind::Len { base: load_x },
            Span::default(),
            Facts::empty(),
            None,
        );
        let loop_end = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Sub,
                lhs: len_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: load_i,
                rhs: loop_end,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rhs_idx = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: load_i,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rhs = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: rhs_idx,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let next_i = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: load_i,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: param_x,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: one,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };
        fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
            base: load_x,
            idx: load_i,
            val: rhs,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: next_i,
            span: Span::default(),
        });
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

        fn_ir
    }

    fn simple_multi_expr_map_fn() -> FnIR {
        let mut fn_ir = FnIR::new(
            "proof_multi_expr_map".to_string(),
            vec!["x".to_string(), "y".to_string()],
        );
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        let param_x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let param_y = fn_ir.add_value(
            ValueKind::Param { index: 1 },
            Span::default(),
            Facts::empty(),
            Some("y".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let load_y = fn_ir.add_value(
            ValueKind::Load {
                var: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("y".to_string()),
        );
        let load_i = fn_ir.add_value(
            ValueKind::Load {
                var: "i".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let len_x = fn_ir.add_value(
            ValueKind::Len { base: load_x },
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: load_i,
                rhs: len_x,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: load_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let next_x = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let read_y = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_y,
                idx: load_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let next_y = fn_ir.add_value(
            ValueKind::Call {
                callee: "abs".to_string(),
                args: vec![read_y],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let next_i = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: load_i,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: param_x,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "y".to_string(),
            src: param_y,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: one,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };
        fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
            base: load_x,
            idx: load_i,
            val: next_x,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
            base: load_y,
            idx: load_i,
            val: next_y,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: next_i,
            span: Span::default(),
        });
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

        fn_ir
    }

    fn partial_range_single_store_loop_fn<F>(name: &str, build_rhs: F) -> FnIR
    where
        F: Fn(&mut FnIR, ValueId, ValueId, ValueId) -> ValueId,
    {
        let mut fn_ir = FnIR::new(name.to_string(), vec!["x".to_string()]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        let param_x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let load_i = fn_ir.add_value(
            ValueKind::Load {
                var: "i".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let len_x = fn_ir.add_value(
            ValueKind::Len { base: load_x },
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Lt,
                lhs: load_i,
                rhs: len_x,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rhs = build_rhs(&mut fn_ir, load_x, load_i, one);
        let next_i = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: load_i,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: param_x,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: two,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };
        fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
            base: load_x,
            idx: load_i,
            val: rhs,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: next_i,
            span: Span::default(),
        });
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

        fn_ir
    }

    fn partial_expr_map_fn() -> FnIR {
        partial_range_single_store_loop_fn(
            "proof_partial_expr_map",
            |fn_ir, load_x, load_i, one| {
                let read_x = fn_ir.add_value(
                    ValueKind::Index1D {
                        base: load_x,
                        idx: load_i,
                        is_safe: true,
                        is_na_safe: true,
                    },
                    Span::default(),
                    Facts::empty(),
                    None,
                );
                fn_ir.add_value(
                    ValueKind::Binary {
                        op: BinOp::Add,
                        lhs: read_x,
                        rhs: one,
                    },
                    Span::default(),
                    Facts::empty(),
                    None,
                )
            },
        )
    }

    fn partial_call_map_fn() -> FnIR {
        partial_range_single_store_loop_fn(
            "proof_partial_call_map",
            |fn_ir, load_x, load_i, _one| {
                let read_x = fn_ir.add_value(
                    ValueKind::Index1D {
                        base: load_x,
                        idx: load_i,
                        is_safe: true,
                        is_na_safe: true,
                    },
                    Span::default(),
                    Facts::empty(),
                    None,
                );
                fn_ir.add_value(
                    ValueKind::Call {
                        callee: "abs".to_string(),
                        args: vec![read_x],
                        names: vec![None],
                    },
                    Span::default(),
                    Facts::empty(),
                    None,
                )
            },
        )
    }

    fn simple_cond_map_fn() -> FnIR {
        let mut fn_ir = FnIR::new("proof_cond_map".to_string(), vec!["x".to_string()]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let branch = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        let latch = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        let param_x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let load_i = fn_ir.add_value(
            ValueKind::Load {
                var: "i".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let len_x = fn_ir.add_value(
            ValueKind::Len { base: load_x },
            Span::default(),
            Facts::empty(),
            None,
        );
        let loop_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: load_i,
                rhs: len_x,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: load_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let branch_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Gt,
                lhs: read_x,
                rhs: zero,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let then_rhs = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let else_rhs = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Sub,
                lhs: read_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let next_i = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: load_i,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: param_x,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: one,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond: loop_cond,
            then_bb: branch,
            else_bb: exit,
        };
        fn_ir.blocks[branch].term = Terminator::If {
            cond: branch_cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].instrs.push(Instr::StoreIndex1D {
            base: load_x,
            idx: load_i,
            val: then_rhs,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[then_bb].term = Terminator::Goto(latch);
        fn_ir.blocks[else_bb].instrs.push(Instr::StoreIndex1D {
            base: load_x,
            idx: load_i,
            val: else_rhs,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[else_bb].term = Terminator::Goto(latch);
        fn_ir.blocks[latch].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: next_i,
            span: Span::default(),
        });
        fn_ir.blocks[latch].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

        fn_ir
    }

    fn simple_cond_reduction_fn() -> FnIR {
        let mut fn_ir = FnIR::new("proof_cond_reduce".to_string(), vec!["x".to_string()]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let branch = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        let latch = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        let param_x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let phi_i = fn_ir.add_value(
            ValueKind::Phi { args: vec![] },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        fn_ir.values[phi_i].phi_block = Some(header);
        let phi_acc = fn_ir.add_value(
            ValueKind::Phi { args: vec![] },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        fn_ir.values[phi_acc].phi_block = Some(header);
        let len_x = fn_ir.add_value(
            ValueKind::Len { base: load_x },
            Span::default(),
            Facts::empty(),
            None,
        );
        let loop_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: phi_i,
                rhs: len_x,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: phi_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let branch_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Gt,
                lhs: read_x,
                rhs: zero,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let then_rhs = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let else_rhs = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Sub,
                lhs: read_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let reduced_read = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: phi_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let next_acc = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi_acc,
                rhs: reduced_read,
            },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        let next_i = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi_i,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
            args.push((one, entry));
            args.push((next_i, latch));
        }
        if let ValueKind::Phi { args } = &mut fn_ir.values[phi_acc].kind {
            args.push((zero, entry));
            args.push((next_acc, latch));
        }

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: param_x,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond: loop_cond,
            then_bb: branch,
            else_bb: exit,
        };
        fn_ir.blocks[branch].term = Terminator::If {
            cond: branch_cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].instrs.push(Instr::StoreIndex1D {
            base: load_x,
            idx: phi_i,
            val: then_rhs,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[then_bb].term = Terminator::Goto(latch);
        fn_ir.blocks[else_bb].instrs.push(Instr::StoreIndex1D {
            base: load_x,
            idx: phi_i,
            val: else_rhs,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[else_bb].term = Terminator::Goto(latch);
        fn_ir.blocks[latch].instrs.push(Instr::Assign {
            dst: "acc".to_string(),
            src: next_acc,
            span: Span::default(),
        });
        fn_ir.blocks[latch].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: next_i,
            span: Span::default(),
        });
        fn_ir.blocks[latch].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(Some(phi_acc));

        fn_ir
    }

    fn simple_branch_only_cond_reduction_fn() -> FnIR {
        let mut fn_ir = FnIR::new(
            "proof_branch_only_cond_reduce".to_string(),
            vec!["x".to_string()],
        );
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let branch = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        let latch = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        let param_x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let phi_i = fn_ir.add_value(
            ValueKind::Phi { args: vec![] },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        fn_ir.values[phi_i].phi_block = Some(header);
        let phi_acc = fn_ir.add_value(
            ValueKind::Phi { args: vec![] },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        fn_ir.values[phi_acc].phi_block = Some(header);
        let len_x = fn_ir.add_value(
            ValueKind::Len { base: load_x },
            Span::default(),
            Facts::empty(),
            None,
        );
        let loop_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: phi_i,
                rhs: len_x,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: phi_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let branch_cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Gt,
                lhs: read_x,
                rhs: zero,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let inc = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let next_acc_then = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi_acc,
                rhs: inc,
            },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        let merged_acc = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(next_acc_then, then_bb), (phi_acc, else_bb)],
            },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        fn_ir.values[merged_acc].phi_block = Some(latch);
        let next_i = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi_i,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );

        if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
            args.push((one, entry));
            args.push((next_i, latch));
        }
        if let ValueKind::Phi { args } = &mut fn_ir.values[phi_acc].kind {
            args.push((zero, entry));
            args.push((merged_acc, latch));
        }

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: param_x,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond: loop_cond,
            then_bb: branch,
            else_bb: exit,
        };
        fn_ir.blocks[branch].term = Terminator::If {
            cond: branch_cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].instrs.push(Instr::Assign {
            dst: "acc".to_string(),
            src: next_acc_then,
            span: Span::default(),
        });
        fn_ir.blocks[then_bb].term = Terminator::Goto(latch);
        fn_ir.blocks[else_bb].term = Terminator::Goto(latch);
        fn_ir.blocks[latch].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: next_i,
            span: Span::default(),
        });
        fn_ir.blocks[latch].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(Some(phi_acc));

        fn_ir
    }

    fn simple_sum_reduction_fn() -> FnIR {
        let mut fn_ir = FnIR::new("proof_reduce_sum".to_string(), vec!["x".to_string()]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        let param_x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let phi_i = fn_ir.add_value(
            ValueKind::Phi { args: vec![] },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        fn_ir.values[phi_i].phi_block = Some(header);
        let phi_acc = fn_ir.add_value(
            ValueKind::Phi { args: vec![] },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        fn_ir.values[phi_acc].phi_block = Some(header);
        let len_x = fn_ir.add_value(
            ValueKind::Len { base: load_x },
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: phi_i,
                rhs: len_x,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let read_x = fn_ir.add_value(
            ValueKind::Index1D {
                base: load_x,
                idx: phi_i,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let next_acc = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi_acc,
                rhs: read_x,
            },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        let next_i = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi_i,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );

        if let ValueKind::Phi { args } = &mut fn_ir.values[phi_i].kind {
            args.push((one, entry));
            args.push((next_i, body));
        }
        if let ValueKind::Phi { args } = &mut fn_ir.values[phi_acc].kind {
            args.push((zero, entry));
            args.push((next_acc, body));
        }

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: param_x,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Goto(header);
        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "acc".to_string(),
            src: next_acc,
            span: Span::default(),
        });
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: next_i,
            span: Span::default(),
        });
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(Some(phi_acc));

        fn_ir
    }

    fn base_single_store_loop_fn<F>(name: &str, build_rhs: F) -> FnIR
    where
        F: Fn(&mut FnIR, ValueId, ValueId, ValueId) -> ValueId,
    {
        let mut fn_ir = FnIR::new(name.to_string(), vec!["x".to_string()]);
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = header;

        let param_x = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(crate::mir::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let load_i = fn_ir.add_value(
            ValueKind::Load {
                var: "i".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let len_x = fn_ir.add_value(
            ValueKind::Len { base: load_x },
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: load_i,
                rhs: len_x,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rhs = build_rhs(&mut fn_ir, load_x, load_i, one);
        let next_i = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: load_i,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: param_x,
            span: Span::default(),
        });
        fn_ir.blocks[entry].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: one,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::Goto(header);

        fn_ir.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };

        fn_ir.blocks[body].instrs.push(Instr::StoreIndex1D {
            base: load_x,
            idx: load_i,
            val: rhs,
            is_safe: true,
            is_na_safe: true,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[body].instrs.push(Instr::Assign {
            dst: "i".to_string(),
            src: next_i,
            span: Span::default(),
        });
        fn_ir.blocks[body].term = Terminator::Goto(header);
        fn_ir.blocks[exit].term = Terminator::Return(Some(load_x));

        fn_ir
    }

    #[test]
    fn enabled_config_certifies_simple_map_and_plan_applies_transactionally() {
        let fn_ir = simple_map_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::Map { op: BinOp::Add, .. }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(applied, "expected certified map plan to apply cleanly");
    }

    #[test]
    fn enabled_config_certifies_simple_expr_map_and_plan_applies_transactionally() {
        let fn_ir = simple_expr_map_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::ExprMap { .. }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(applied, "expected certified expr-map plan to apply cleanly");
    }

    #[test]
    fn enabled_config_certifies_simple_call_map_and_plan_applies_transactionally() {
        let fn_ir = simple_call_map_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::CallMap { .. }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(applied, "expected certified call-map plan to apply cleanly");
    }

    #[test]
    fn enabled_config_certifies_simple_shifted_map_and_plan_applies_transactionally() {
        let fn_ir = simple_shifted_map_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::ShiftedMap { offset: 1, .. }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(
            applied,
            "expected certified shifted-map plan to apply cleanly"
        );
    }

    #[test]
    fn enabled_config_certifies_simple_multi_expr_map_and_plan_applies_transactionally() {
        let fn_ir = simple_multi_expr_map_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::MultiExprMap { .. }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(
            applied,
            "expected certified multi-expr-map plan to apply cleanly"
        );
    }

    #[test]
    fn enabled_config_certifies_partial_expr_map_and_plan_applies_transactionally() {
        let fn_ir = partial_expr_map_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::ExprMap {
                whole_dest: false,
                ..
            }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(
            applied,
            "expected certified partial expr-map plan to apply cleanly"
        );
    }

    #[test]
    fn enabled_config_certifies_partial_call_map_and_plan_applies_transactionally() {
        let fn_ir = partial_call_map_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::CallMap {
                whole_dest: false,
                ..
            }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(
            applied,
            "expected certified partial call-map plan to apply cleanly"
        );
    }

    #[test]
    fn enabled_config_certifies_simple_cond_map_and_plan_applies_transactionally() {
        let fn_ir = simple_cond_map_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::CondMap { .. }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(applied, "expected certified cond-map plan to apply cleanly");
    }

    #[test]
    fn enabled_config_certifies_simple_cond_reduction_and_plan_applies_transactionally() {
        let fn_ir = simple_cond_reduction_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::ReduceCond { .. }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(
            applied,
            "expected certified conditional reduction plan to apply cleanly"
        );
    }

    #[test]
    fn enabled_config_certifies_branch_only_cond_reduction_and_plan_applies_transactionally() {
        let fn_ir = simple_branch_only_cond_reduction_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::ReduceCond { .. }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(
            applied,
            "expected certified branch-only conditional reduction plan to apply cleanly"
        );
    }

    #[test]
    fn enabled_config_certifies_simple_sum_reduction_and_plan_applies_transactionally() {
        let fn_ir = simple_sum_reduction_fn();
        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        assert_eq!(loops.len(), 1);

        let outcome = analyze_loop_with_config(
            &fn_ir,
            &loops[0],
            &FxHashSet::default(),
            ProofConfig { enabled: true },
        );
        let ProofOutcome::Certified(certified) = outcome else {
            panic!("expected certified proof outcome");
        };
        assert!(matches!(
            certified.plan,
            super::super::planning::VectorPlan::Reduce {
                kind: super::super::planning::ReduceKind::Sum,
                ..
            }
        ));

        let mut applied_ir = fn_ir.clone();
        let applied = try_apply_vectorization_transactionally(
            &mut applied_ir,
            &loops[0],
            certified.plan.clone(),
        );
        assert!(
            applied,
            "expected certified reduction plan to apply cleanly"
        );
    }
}
