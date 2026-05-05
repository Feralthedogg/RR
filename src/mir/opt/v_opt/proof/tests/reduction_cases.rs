use super::*;
#[test]
pub(crate) fn enabled_config_certifies_branch_only_cond_reduction_and_plan_applies_transactionally()
{
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
        super::super::super::planning::VectorPlan::ReduceCond { .. }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(
        applied,
        "expected certified branch-only conditional reduction plan to apply cleanly"
    );
}

#[test]
pub(crate) fn enabled_config_certifies_simple_sum_reduction_and_plan_applies_transactionally() {
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
        super::super::super::planning::VectorPlan::Reduce {
            kind: super::super::super::planning::ReduceKind::Sum,
            ..
        }
    ));

    let mut applied_ir = fn_ir.clone();
    let applied =
        try_apply_vectorization_transactionally(&mut applied_ir, &loops[0], certified.plan.clone());
    assert!(
        applied,
        "expected certified reduction plan to apply cleanly"
    );
}
