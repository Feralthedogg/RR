use super::analysis::{
    affine_iv_offset, as_safe_loop_index, canonical_value, classify_store_1d_in_block,
    collect_loop_shadow_vars_for_dest, expr_has_iv_dependency,
    expr_has_non_vector_safe_call_in_vector_context, induction_origin_var,
    is_condition_vectorizable, is_iv_equivalent, is_loop_invariant_scalar_expr,
    is_origin_var_iv_alias_in_loop, is_vector_safe_call, is_vector_safe_call_chain_expr,
    is_vectorizable_expr, loop_carried_phi_is_dest_last_value_shadow,
    loop_carried_phi_is_invariant_passthrough, loop_carried_phi_is_unmodified,
    loop_covers_whole_destination, loop_has_inner_branch,
    loop_has_non_iv_loop_carried_state_except, loop_has_store_effect, preserve_phi_value,
    resolve_base_var, resolve_load_alias_value, same_base_value,
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

#[path = "proof_reduction.rs"]
mod proof_reduction;
use self::proof_reduction::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProofConfig {
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

pub(crate) fn analyze_loop_with_config(
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
    if loop_has_inner_branch(fn_ir, lp) {
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
                Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. }
                | Instr::UnsafeRBlock { .. } => {
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
                && is_iv_equivalent(fn_ir, else_store.idx, iv_phi)
                && fn_ir.blocks[then_bb]
                    .instrs
                    .iter()
                    .all(|instr| matches!(instr, Instr::StoreIndex1D { .. }))
                && fn_ir.blocks[else_bb]
                    .instrs
                    .iter()
                    .all(|instr| matches!(instr, Instr::StoreIndex1D { .. })) =>
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
    if loop_has_inner_branch(fn_ir, lp) {
        return Err(ProofFallbackReason::NotSimpleExprMap);
    }
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
                Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. }
                | Instr::UnsafeRBlock { .. } => {
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
mod tests;
