use super::*;

pub(crate) fn always_enabled(_: &TachyonEngine, _: &FnIR, _: &mut ChronosContext<'_>) -> bool {
    true
}

pub(crate) fn light_sccp_enabled(
    _: &TachyonEngine,
    _: &FnIR,
    ctx: &mut ChronosContext<'_>,
) -> bool {
    ctx.run_light_sccp
}

pub(crate) fn gvn_enabled(_: &TachyonEngine, _: &FnIR, _: &mut ChronosContext<'_>) -> bool {
    TachyonEngine::gvn_enabled()
}

pub(crate) fn licm_enabled(_: &TachyonEngine, fn_ir: &FnIR, _: &mut ChronosContext<'_>) -> bool {
    TachyonEngine::licm_enabled() && TachyonEngine::licm_allowed_for_fn(fn_ir)
}

pub(crate) fn control_licm_enabled(
    _: &TachyonEngine,
    fn_ir: &FnIR,
    ctx: &mut ChronosContext<'_>,
) -> bool {
    let features = ctx.analysis_cache.phase_features(fn_ir);
    features.canonical_loop_count > 0
        && TachyonEngine::licm_enabled()
        && TachyonEngine::licm_allowed_for_fn(fn_ir)
}

pub(crate) fn non_conservative_enabled(
    _: &TachyonEngine,
    fn_ir: &FnIR,
    _: &mut ChronosContext<'_>,
) -> bool {
    !fn_ir.requires_conservative_optimization()
}

pub(crate) const fn program_always_enabled(
    _: &TachyonEngine,
    _: &FxHashMap<String, FnIR>,
    _: &ChronosProgramContext<'_>,
) -> bool {
    true
}

pub(crate) fn run_index_canonicalize(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    ctx: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    let Some(floor_helpers) = ctx.floor_helpers else {
        return ChronosPassOutcome::unchanged();
    };
    ChronosPassOutcome::changed(TachyonEngine::canonicalize_floor_index_params(
        fn_ir,
        ctx.proven_param_slots,
        floor_helpers,
    ))
}

pub(crate) fn run_simplify_cfg(
    engine: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(engine.simplify_cfg(fn_ir))
}

pub(crate) fn run_sccp(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(sccp::MirSCCP::new().optimize(fn_ir))
}

pub(crate) fn run_intrinsics(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(intrinsics::optimize(fn_ir))
}

pub(crate) fn run_gvn(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(gvn::optimize(fn_ir))
}

pub(crate) fn run_simplify(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(simplify::optimize(fn_ir))
}

pub(crate) fn run_type_specialize(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(type_specialize::optimize(fn_ir))
}

pub(crate) fn run_poly(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    ctx: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    if !poly::poly_enabled() {
        return ChronosPassOutcome::unchanged();
    }
    let loops = ctx.analysis_cache.loops(fn_ir);
    let p_stats = poly::optimize_with_stats_with_loop_info(fn_ir, loops);
    let changed = p_stats.schedule_applied > 0;
    TachyonEngine::accumulate_poly_stats(ctx.stats, p_stats);
    ChronosPassOutcome::changed(changed)
}

pub(crate) fn run_vectorize(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    ctx: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    let Some(user_call_whitelist) = ctx.user_call_whitelist else {
        return ChronosPassOutcome::unchanged();
    };
    let loops = ctx.analysis_cache.loops(fn_ir);
    let v_stats =
        v_opt::optimize_with_stats_with_whitelist_and_loops(fn_ir, user_call_whitelist, loops);
    let changed = v_stats.changed();
    TachyonEngine::accumulate_vector_stats(ctx.stats, v_stats);
    ChronosPassOutcome::changed(changed)
}

pub(crate) fn run_unroll(
    engine: &TachyonEngine,
    fn_ir: &mut FnIR,
    ctx: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::counted(unroll::optimize(fn_ir, engine, ctx.stats))
}

pub(crate) fn run_tco(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(tco::optimize(fn_ir))
}

pub(crate) fn run_loop_opt(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    ctx: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    let loops = ctx.analysis_cache.loops(fn_ir);
    ChronosPassOutcome::counted(ctx.loop_optimizer.optimize_with_loop_info(fn_ir, loops))
}

pub(crate) fn run_licm(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    ctx: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    let loops = ctx.analysis_cache.loops(fn_ir);
    ChronosPassOutcome::changed(licm::MirLicm::new().optimize_with_loop_info(fn_ir, loops))
}

pub(crate) fn run_sroa(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(sroa::optimize(fn_ir))
}

pub(crate) fn run_dce(
    engine: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(engine.dce(fn_ir))
}

pub(crate) fn run_de_ssa(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(de_ssa::run(fn_ir))
}

pub(crate) fn run_copy_cleanup(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    _: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(copy_cleanup::optimize(fn_ir))
}

pub(crate) fn run_fresh_alias(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    ctx: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    let Some(fresh_user_calls) = ctx.fresh_user_calls else {
        return ChronosPassOutcome::unchanged();
    };
    ChronosPassOutcome::changed(fresh_alias::optimize_function_with_fresh_user_calls(
        fn_ir,
        fresh_user_calls,
    ))
}

pub(crate) fn run_bce(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    ctx: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    let loops = ctx.analysis_cache.loops(fn_ir);
    ChronosPassOutcome::changed(bce::optimize_with_loop_info(fn_ir, loops))
}

pub(crate) fn run_fresh_alloc(
    _: &TachyonEngine,
    fn_ir: &mut FnIR,
    ctx: &mut ChronosContext<'_>,
) -> ChronosPassOutcome {
    let loops = ctx.analysis_cache.loops(fn_ir);
    ChronosPassOutcome::changed(fresh_alloc::optimize_with_loop_info(fn_ir, loops))
}

pub(crate) fn run_inline_program(
    _: &TachyonEngine,
    all_fns: &mut FxHashMap<String, FnIR>,
    ctx: &mut ChronosProgramContext<'_>,
) -> ChronosPassOutcome {
    let Some(inliner) = ctx.inliner else {
        return ChronosPassOutcome::unchanged();
    };
    ChronosPassOutcome::changed(inliner.optimize_with_hot_filter(all_fns, ctx.hot_filter))
}

pub(crate) fn run_record_call_specialize(
    _: &TachyonEngine,
    all_fns: &mut FxHashMap<String, FnIR>,
    _: &mut ChronosProgramContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(sroa::specialize_record_field_calls(all_fns))
}

pub(crate) fn run_record_return_specialize(
    _: &TachyonEngine,
    all_fns: &mut FxHashMap<String, FnIR>,
    _: &mut ChronosProgramContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::changed(sroa::specialize_record_return_field_calls(all_fns))
}

pub(crate) fn run_outline_program(
    engine: &TachyonEngine,
    all_fns: &mut FxHashMap<String, FnIR>,
    ctx: &mut ChronosProgramContext<'_>,
) -> ChronosPassOutcome {
    ChronosPassOutcome::counted(outline::optimize_program(all_fns, engine, ctx.stats))
}

pub(crate) const fn pass_invalidates(id: ChronosPassId) -> ChronosAnalysisSet {
    match id {
        ChronosPassId::Inline => ChronosAnalysisSet::ALL,
        ChronosPassId::RecordCallSpecialize
        | ChronosPassId::RecordReturnSpecialize
        | ChronosPassId::Outline => ChronosAnalysisSet::ALL,
        ChronosPassId::SimplifyCfg => ChronosAnalysisSet::ALL,
        ChronosPassId::IndexCanonicalize
        | ChronosPassId::Sccp
        | ChronosPassId::Intrinsics
        | ChronosPassId::Gvn
        | ChronosPassId::Simplify
        | ChronosPassId::TypeSpecialize
        | ChronosPassId::Dce
        | ChronosPassId::CopyCleanup
        | ChronosPassId::FreshAlias => ChronosAnalysisSet::VALUE_GRAPH
            .union(ChronosAnalysisSet::ALIAS_INFO)
            .union(ChronosAnalysisSet::RECORD_SHAPE),
        ChronosPassId::DeSsa => ChronosAnalysisSet::SSA_FORM
            .union(ChronosAnalysisSet::VALUE_GRAPH)
            .union(ChronosAnalysisSet::ALIAS_INFO),
        ChronosPassId::Poly
        | ChronosPassId::Vectorize
        | ChronosPassId::Tco
        | ChronosPassId::LoopOpt
        | ChronosPassId::Licm
        | ChronosPassId::Sroa
        | ChronosPassId::FreshAlloc
        | ChronosPassId::Bce
        | ChronosPassId::Unroll => ChronosAnalysisSet::ALL,
    }
}

pub(crate) const fn pass_requires(id: ChronosPassId) -> ChronosAnalysisSet {
    match id {
        ChronosPassId::Inline
        | ChronosPassId::Outline
        | ChronosPassId::RecordCallSpecialize
        | ChronosPassId::RecordReturnSpecialize => ChronosAnalysisSet::CALL_GRAPH
            .union(ChronosAnalysisSet::VALUE_GRAPH)
            .union(ChronosAnalysisSet::EFFECTS),
        ChronosPassId::Poly | ChronosPassId::Vectorize | ChronosPassId::Unroll => {
            ChronosAnalysisSet::CONTROL_FLOW
                .union(ChronosAnalysisSet::LOOP_INFO)
                .union(ChronosAnalysisSet::RANGE_BOUNDS)
                .union(ChronosAnalysisSet::EFFECTS)
                .union(ChronosAnalysisSet::DOMINANCE)
        }
        ChronosPassId::LoopOpt | ChronosPassId::Licm => ChronosAnalysisSet::CONTROL_FLOW
            .union(ChronosAnalysisSet::LOOP_INFO)
            .union(ChronosAnalysisSet::EFFECTS)
            .union(ChronosAnalysisSet::DOMINANCE),
        ChronosPassId::FreshAlloc => ChronosAnalysisSet::LOOP_INFO
            .union(ChronosAnalysisSet::ALIAS_INFO)
            .union(ChronosAnalysisSet::ESCAPE_INFO),
        ChronosPassId::Bce => ChronosAnalysisSet::CONTROL_FLOW
            .union(ChronosAnalysisSet::LOOP_INFO)
            .union(ChronosAnalysisSet::RANGE_BOUNDS)
            .union(ChronosAnalysisSet::DOMINANCE),
        ChronosPassId::Sroa => ChronosAnalysisSet::VALUE_GRAPH
            .union(ChronosAnalysisSet::ALIAS_INFO)
            .union(ChronosAnalysisSet::ESCAPE_INFO)
            .union(ChronosAnalysisSet::RECORD_SHAPE),
        ChronosPassId::Gvn => ChronosAnalysisSet::VALUE_GRAPH
            .union(ChronosAnalysisSet::ALIAS_INFO)
            .union(ChronosAnalysisSet::EFFECTS),
        ChronosPassId::Dce => ChronosAnalysisSet::VALUE_GRAPH.union(ChronosAnalysisSet::EFFECTS),
        ChronosPassId::DeSsa | ChronosPassId::CopyCleanup => {
            ChronosAnalysisSet::SSA_FORM.union(ChronosAnalysisSet::VALUE_GRAPH)
        }
        ChronosPassId::SimplifyCfg | ChronosPassId::Tco => {
            ChronosAnalysisSet::CONTROL_FLOW.union(ChronosAnalysisSet::DOMINANCE)
        }
        ChronosPassId::IndexCanonicalize
        | ChronosPassId::Sccp
        | ChronosPassId::Intrinsics
        | ChronosPassId::Simplify
        | ChronosPassId::TypeSpecialize
        | ChronosPassId::FreshAlias => ChronosAnalysisSet::VALUE_GRAPH,
    }
}

pub(crate) const fn pass_contract(
    id: ChronosPassId,
    group: types::PassGroup,
) -> ChronosPassContract {
    let budget_class = match id {
        ChronosPassId::Inline
        | ChronosPassId::Outline
        | ChronosPassId::RecordCallSpecialize
        | ChronosPassId::RecordReturnSpecialize => ChronosBudgetClass::Interprocedural,
        ChronosPassId::Poly | ChronosPassId::Vectorize | ChronosPassId::Unroll => {
            ChronosBudgetClass::StructuralProof
        }
        ChronosPassId::SimplifyCfg
        | ChronosPassId::Sccp
        | ChronosPassId::Intrinsics
        | ChronosPassId::Simplify
        | ChronosPassId::TypeSpecialize
        | ChronosPassId::Tco
        | ChronosPassId::Dce
        | ChronosPassId::CopyCleanup
        | ChronosPassId::IndexCanonicalize => ChronosBudgetClass::AlwaysCheap,
        ChronosPassId::DeSsa | ChronosPassId::FreshAlias => ChronosBudgetClass::FinalCleanup,
        ChronosPassId::Gvn
        | ChronosPassId::LoopOpt
        | ChronosPassId::Licm
        | ChronosPassId::Sroa
        | ChronosPassId::FreshAlloc
        | ChronosPassId::Bce => ChronosBudgetClass::LocalHeavy,
    };
    let legality = match id {
        ChronosPassId::Poly | ChronosPassId::Vectorize => "proof-driven structural legality",
        ChronosPassId::Unroll => "constant-trip loop clone preserves counted iteration order",
        ChronosPassId::Outline => "single-entry region helper preserves live-in/live-out boundary",
        ChronosPassId::Licm => "loop invariant with effect and alias guards",
        ChronosPassId::Bce => "range/dominance bounds proof",
        ChronosPassId::FreshAlloc | ChronosPassId::Sroa => "escape and record-shape proof",
        ChronosPassId::Inline => "call graph rewrite preserves callee body semantics",
        ChronosPassId::RecordCallSpecialize | ChronosPassId::RecordReturnSpecialize => {
            "record field specialization preserves observable record semantics"
        }
        ChronosPassId::DeSsa | ChronosPassId::CopyCleanup => {
            "parallel-copy phi materialization boundary"
        }
        ChronosPassId::Gvn => "effect-aware value equivalence",
        ChronosPassId::Dce => "effect-free unused computation removal",
        _ => "local MIR equivalence",
    };
    let profitability = match group {
        types::PassGroup::Required => "required normalization",
        types::PassGroup::DevCheap => "cheap local cleanup",
        types::PassGroup::ReleaseExpensive => "budgeted release cost model",
        types::PassGroup::Experimental => "O3 aggressive opportunity model",
    };
    ChronosPassContract {
        legality,
        profitability,
        budget_class,
    }
}

pub(crate) const fn function_spec(
    id: ChronosPassId,
    stage: ChronosStage,
    group: types::PassGroup,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    ChronosPassSpec {
        id,
        scope: ChronosPassScope::FunctionMir,
        stage,
        group,
        version: 1,
        proof_key: ChronosProofKey(proof_key),
        contract: pass_contract(id, group),
        requires: pass_requires(id),
        invalidates: pass_invalidates(id),
        verify_label,
        enabled,
        run,
    }
}

pub(crate) const fn program_spec(
    id: ChronosPassId,
    stage: ChronosStage,
    group: types::PassGroup,
    verify_label: &'static str,
    enabled: ChronosProgramEnabledFn,
    run: ChronosProgramRunner,
    proof_key: &'static str,
) -> ChronosProgramPassSpec {
    ChronosProgramPassSpec {
        id,
        scope: ChronosPassScope::ProgramMir,
        stage,
        group,
        version: 1,
        proof_key: ChronosProofKey(proof_key),
        contract: pass_contract(id, group),
        requires: pass_requires(id),
        invalidates: pass_invalidates(id),
        verify_label,
        enabled,
        run,
    }
}

pub(crate) const fn always_tier_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::AlwaysTier,
        types::PassGroup::Required,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn function_entry_canonicalization_spec(
    verify_label: &'static str,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        ChronosPassId::IndexCanonicalize,
        ChronosStage::FunctionEntryCanonicalization,
        types::PassGroup::Required,
        verify_label,
        always_enabled,
        run_index_canonicalize,
        proof_key,
    )
}

pub(crate) const fn phase_order_standard_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PhaseOrderStandard,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn phase_order_compute_prelude_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PhaseOrderComputePrelude,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn phase_order_control_prelude_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PhaseOrderControlPrelude,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn phase_order_budget_prefix_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PhaseOrderBudgetPrefix,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn phase_order_budget_tail_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PhaseOrderBudgetTail,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn balanced_structural_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PhaseOrderBalancedStructural,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn control_structural_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PhaseOrderControlStructural,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn fast_dev_vectorize_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PhaseOrderFastDevVectorize,
        types::PassGroup::DevCheap,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn structural_cleanup_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PhaseOrderStructuralCleanup,
        types::PassGroup::DevCheap,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn function_final_polish_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::FunctionFinalPolish,
        types::PassGroup::Required,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn program_inline_cleanup_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::ProgramInlineCleanup,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn program_inline_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosProgramEnabledFn,
    run: ChronosProgramRunner,
    proof_key: &'static str,
) -> ChronosProgramPassSpec {
    program_spec(
        id,
        ChronosStage::ProgramInline,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn program_record_specialization_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosProgramEnabledFn,
    run: ChronosProgramRunner,
    proof_key: &'static str,
) -> ChronosProgramPassSpec {
    program_spec(
        id,
        ChronosStage::ProgramRecordSpecialization,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn program_outline_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosProgramEnabledFn,
    run: ChronosProgramRunner,
    proof_key: &'static str,
) -> ChronosProgramPassSpec {
    program_spec(
        id,
        ChronosStage::ProgramOutlining,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn program_fresh_alias_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::ProgramFreshAlias,
        types::PassGroup::ReleaseExpensive,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn program_post_de_ssa_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::ProgramPostDeSsa,
        types::PassGroup::Required,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(crate) const fn prepare_for_codegen_spec(
    id: ChronosPassId,
    verify_label: &'static str,
    enabled: ChronosEnabledFn,
    run: ChronosFunctionRunner,
    proof_key: &'static str,
) -> ChronosPassSpec {
    function_spec(
        id,
        ChronosStage::PrepareForCodegen,
        types::PassGroup::Required,
        verify_label,
        enabled,
        run,
        proof_key,
    )
}

pub(in crate::mir::opt) const ALWAYS_TIER_PASSES: &[ChronosPassSpec] = &[
    always_tier_spec(
        ChronosPassId::SimplifyCfg,
        "After AlwaysTier/SimplifyCFG",
        always_enabled,
        run_simplify_cfg,
        "CfgOptSoundness.always_tier_simplify_cfg",
    ),
    always_tier_spec(
        ChronosPassId::Sccp,
        "After AlwaysTier/SCCP",
        light_sccp_enabled,
        run_sccp,
        "DataflowOptSoundness.always_tier_sccp",
    ),
    always_tier_spec(
        ChronosPassId::Intrinsics,
        "After AlwaysTier/Intrinsics",
        light_sccp_enabled,
        run_intrinsics,
        "DataflowOptSoundness.always_tier_intrinsics",
    ),
    always_tier_spec(
        ChronosPassId::TypeSpecialize,
        "After AlwaysTier/TypeSpecialize",
        always_enabled,
        run_type_specialize,
        "DataflowOptSoundness.always_tier_type_specialize",
    ),
    always_tier_spec(
        ChronosPassId::Tco,
        "After AlwaysTier/TCO",
        always_enabled,
        run_tco,
        "LoopOptSoundness.always_tier_tco",
    ),
    always_tier_spec(
        ChronosPassId::LoopOpt,
        "After AlwaysTier/LoopOpt",
        always_enabled,
        run_loop_opt,
        "LoopOptSoundness.always_tier_loop_opt",
    ),
    always_tier_spec(
        ChronosPassId::Sroa,
        "After AlwaysTier/SROA",
        always_enabled,
        run_sroa,
        "SroaSoundness.always_tier_sroa",
    ),
    always_tier_spec(
        ChronosPassId::Dce,
        "After AlwaysTier/DCE",
        always_enabled,
        run_dce,
        "DataflowOptSoundness.always_tier_dce",
    ),
];

pub(in crate::mir::opt) const ALWAYS_TIER_INDEX_CANONICALIZATION_PASS: ChronosPassSpec =
    always_tier_spec(
        ChronosPassId::IndexCanonicalize,
        "After AlwaysTier/ParamIndexCanonicalize",
        always_enabled,
        run_index_canonicalize,
        "DataflowOptSoundness.always_tier_index_canonicalize",
    );

pub(in crate::mir::opt) const ALWAYS_TIER_BCE_PASS: ChronosPassSpec = always_tier_spec(
    ChronosPassId::Bce,
    "After AlwaysTier/BCE",
    always_enabled,
    run_bce,
    "LoopOptSoundness.always_tier_bce",
);

pub(in crate::mir::opt) const FUNCTION_ENTRY_CANONICALIZATION_PASSES: &[ChronosPassSpec] =
    &[function_entry_canonicalization_spec(
        "After ParamIndexCanonicalize",
        "DataflowOptSoundness.function_entry_index_canonicalize",
    )];

pub(in crate::mir::opt) const PHASE_ORDER_STANDARD_CORE_PASSES: &[ChronosPassSpec] = &[
    phase_order_standard_spec(
        ChronosPassId::SimplifyCfg,
        "After SimplifyCFG",
        always_enabled,
        run_simplify_cfg,
        "PhaseOrderClusterSoundness.standard_cluster_simplify_cfg",
    ),
    phase_order_standard_spec(
        ChronosPassId::Sccp,
        "After SCCP",
        always_enabled,
        run_sccp,
        "PhaseOrderClusterSoundness.standard_cluster_sccp",
    ),
    phase_order_standard_spec(
        ChronosPassId::Intrinsics,
        "After Intrinsics",
        always_enabled,
        run_intrinsics,
        "PhaseOrderClusterSoundness.standard_cluster_intrinsics",
    ),
    phase_order_standard_spec(
        ChronosPassId::Gvn,
        "After GVN",
        gvn_enabled,
        run_gvn,
        "PhaseOrderClusterSoundness.standard_cluster_gvn",
    ),
    phase_order_standard_spec(
        ChronosPassId::Simplify,
        "After Simplify",
        always_enabled,
        run_simplify,
        "PhaseOrderClusterSoundness.standard_cluster_simplify",
    ),
    phase_order_standard_spec(
        ChronosPassId::Sroa,
        "After SROA",
        always_enabled,
        run_sroa,
        "PhaseOrderClusterSoundness.standard_cluster_sroa",
    ),
    phase_order_standard_spec(
        ChronosPassId::Dce,
        "After DCE",
        always_enabled,
        run_dce,
        "PhaseOrderClusterSoundness.standard_cluster_dce",
    ),
];

pub(in crate::mir::opt) const PHASE_ORDER_STANDARD_BUDGET_PASSES: &[ChronosPassSpec] = &[
    phase_order_standard_spec(
        ChronosPassId::LoopOpt,
        "After LoopOpt",
        always_enabled,
        run_loop_opt,
        "PhaseOrderClusterSoundness.standard_cluster_loop_opt",
    ),
    phase_order_standard_spec(
        ChronosPassId::Licm,
        "After LICM",
        licm_enabled,
        run_licm,
        "PhaseOrderClusterSoundness.standard_cluster_licm",
    ),
    phase_order_standard_spec(
        ChronosPassId::FreshAlloc,
        "After FreshAlloc",
        always_enabled,
        run_fresh_alloc,
        "PhaseOrderClusterSoundness.standard_cluster_fresh_alloc",
    ),
    phase_order_standard_spec(
        ChronosPassId::Bce,
        "After BCE",
        always_enabled,
        run_bce,
        "PhaseOrderClusterSoundness.standard_cluster_bce",
    ),
    phase_order_standard_spec(
        ChronosPassId::Unroll,
        "After FullUnroll",
        always_enabled,
        run_unroll,
        "PhaseOrderClusterSoundness.standard_cluster_unroll",
    ),
];

pub(in crate::mir::opt) const PHASE_ORDER_COMPUTE_PRELUDE_PASSES: &[ChronosPassSpec] = &[
    phase_order_compute_prelude_spec(
        ChronosPassId::SimplifyCfg,
        "After SimplifyCFG",
        always_enabled,
        run_simplify_cfg,
        "PhaseOrderIterationSoundness.compute_heavy_simplify_cfg",
    ),
    phase_order_compute_prelude_spec(
        ChronosPassId::Sccp,
        "After SCCP",
        always_enabled,
        run_sccp,
        "PhaseOrderIterationSoundness.compute_heavy_sccp",
    ),
    phase_order_compute_prelude_spec(
        ChronosPassId::Intrinsics,
        "After Intrinsics",
        always_enabled,
        run_intrinsics,
        "PhaseOrderIterationSoundness.compute_heavy_intrinsics",
    ),
    phase_order_compute_prelude_spec(
        ChronosPassId::Gvn,
        "After GVN",
        gvn_enabled,
        run_gvn,
        "PhaseOrderIterationSoundness.compute_heavy_gvn",
    ),
    phase_order_compute_prelude_spec(
        ChronosPassId::Simplify,
        "After Simplify",
        always_enabled,
        run_simplify,
        "PhaseOrderIterationSoundness.compute_heavy_simplify",
    ),
    phase_order_compute_prelude_spec(
        ChronosPassId::Sroa,
        "After SROA",
        always_enabled,
        run_sroa,
        "PhaseOrderIterationSoundness.compute_heavy_sroa",
    ),
    phase_order_compute_prelude_spec(
        ChronosPassId::Dce,
        "After DCE",
        always_enabled,
        run_dce,
        "PhaseOrderIterationSoundness.compute_heavy_dce",
    ),
];
