use super::super::debug::{trace_proof_apply_result, trace_proof_status};
use super::super::transform::try_apply_vectorization_transactionally;
use super::super::types::ProofOutcome;
use super::{
    VOptStats, VectorPlanFamily, record_plan_counts, vector_plan_family, vector_plan_label,
};
use crate::mir::FnIR;
use crate::mir::opt::loop_analysis::LoopInfo;

pub(super) fn trace_proof_outcome(fn_ir: &FnIR, lp: &LoopInfo, outcome: &ProofOutcome) {
    match outcome {
        ProofOutcome::Certified(certified) => trace_proof_status(
            fn_ir,
            lp,
            &format!(
                "certified={} attempting-transactional-apply",
                vector_plan_label(&certified.plan)
            ),
        ),
        ProofOutcome::NotApplicable { reason } => {
            trace_proof_status(fn_ir, lp, &format!("not-applicable: {}", reason.label()))
        }
        ProofOutcome::FallbackToPattern { reason } => {
            trace_proof_status(fn_ir, lp, &format!("fallback-pattern: {}", reason.label()))
        }
    }
}

pub(super) fn try_apply_proof_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    outcome: &ProofOutcome,
    stats: &mut VOptStats,
) -> bool {
    let ProofOutcome::Certified(certified) = outcome else {
        return false;
    };

    let plan = certified.plan.clone();
    let applied = try_apply_vectorization_transactionally(fn_ir, lp, plan.clone());
    trace_proof_apply_result(
        fn_ir,
        lp,
        applied,
        &format!("plan={}", vector_plan_label(&plan)),
    );
    if !applied {
        stats.proof_apply_failed += 1;
        return false;
    }

    stats.proof_applied += 1;
    record_plan_counts(stats, fn_ir, &plan, true);
    if matches!(vector_plan_family(&plan), VectorPlanFamily::Reduction) {
        stats.reduced += 1;
    } else {
        stats.vectorized += 1;
    }
    true
}
