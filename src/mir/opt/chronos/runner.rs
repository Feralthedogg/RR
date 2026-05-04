use super::*;

pub(in crate::mir::opt) struct ChronosPassManager {
    pub(crate) stage: ChronosStage,
    pub(crate) passes: &'static [ChronosPassSpec],
}

pub(in crate::mir::opt) struct ChronosProgramPassManager {
    pub(crate) stage: ChronosStage,
    pub(crate) passes: &'static [ChronosProgramPassSpec],
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PassDecisionStatus {
    pub(crate) enabled: bool,
    pub(crate) changed: bool,
    pub(crate) changed_count: usize,
    pub(crate) elapsed_ns: u128,
    pub(crate) reason: &'static str,
}

impl PassDecisionStatus {
    const DISABLED: Self = Self {
        enabled: false,
        changed: false,
        changed_count: 0,
        elapsed_ns: 0,
        reason: "disabled",
    };

    pub(crate) fn from_outcome(outcome: ChronosPassOutcome, elapsed_ns: u128) -> Self {
        let changed = outcome.is_changed();
        Self {
            enabled: true,
            changed,
            changed_count: outcome.changed_count,
            elapsed_ns,
            reason: if changed { "changed" } else { "no-change" },
        }
    }
}

impl ChronosPassManager {
    pub(in crate::mir::opt) const fn new(
        stage: ChronosStage,
        passes: &'static [ChronosPassSpec],
    ) -> Self {
        Self { stage, passes }
    }

    pub(in crate::mir::opt) fn run_fixed_point(
        &self,
        engine: &TachyonEngine,
        fn_ir: &mut FnIR,
        ctx: &mut ChronosContext<'_>,
        budget: ChronosBudget,
    ) -> ChronosFixedPointOutcome {
        let mut changed = true;
        let mut iterations = 0usize;
        let mut seen = FxHashSet::default();
        let mut any_changed = false;
        seen.insert(TachyonEngine::fn_ir_fingerprint(fn_ir));

        while changed && iterations < budget.max_iterations {
            iterations += 1;
            changed = false;
            let before_hash = TachyonEngine::fn_ir_fingerprint(fn_ir);

            for spec in self.passes {
                changed |= self
                    .run_function_pass(engine, fn_ir, ctx, spec)
                    .is_changed();
            }

            let after_hash = TachyonEngine::fn_ir_fingerprint(fn_ir);
            if after_hash == before_hash {
                break;
            }
            if !seen.insert(after_hash) {
                break;
            }
            changed |= after_hash != before_hash;
            any_changed |= changed;
        }

        ChronosFixedPointOutcome {
            changed: any_changed,
            iterations,
        }
    }

    pub(in crate::mir::opt) fn run_one(
        &self,
        engine: &TachyonEngine,
        fn_ir: &mut FnIR,
        ctx: &mut ChronosContext<'_>,
        spec: &ChronosPassSpec,
    ) -> ChronosPassOutcome {
        self.run_function_pass(engine, fn_ir, ctx, spec)
    }

    pub(in crate::mir::opt) fn run_sequence(
        &self,
        engine: &TachyonEngine,
        fn_ir: &mut FnIR,
        ctx: &mut ChronosContext<'_>,
    ) -> ChronosPassOutcome {
        let mut changed_count = 0usize;
        let mut changed_passes = 0usize;
        for spec in self.passes {
            let outcome = self.run_function_pass(engine, fn_ir, ctx, spec);
            changed_count = changed_count.saturating_add(outcome.changed_count);
            changed_passes = changed_passes.saturating_add(outcome.changed_passes);
        }
        ChronosPassOutcome::combined(changed_count, changed_passes)
    }

    pub(crate) fn run_function_pass(
        &self,
        engine: &TachyonEngine,
        fn_ir: &mut FnIR,
        ctx: &mut ChronosContext<'_>,
        spec: &ChronosPassSpec,
    ) -> ChronosPassOutcome {
        if spec.stage != self.stage {
            return ChronosPassOutcome::unchanged();
        }
        if !(spec.enabled)(engine, fn_ir, ctx) {
            ctx.timings.record_decision(function_decision(
                spec,
                fn_ir,
                PassDecisionStatus::DISABLED,
            ));
            return ChronosPassOutcome::unchanged();
        }

        let _metadata = (
            spec.scope,
            spec.group,
            spec.version,
            spec.proof_key,
            spec.contract,
            spec.requires,
            spec.invalidates,
            ctx.fn_ir_size,
        );
        let before_fuel = ctx.fuel.consumed();
        if ctx
            .fuel
            .consume_pass_entry(function_pass_fuel_cost(spec.id, ctx.fn_ir_size))
            .is_err()
        {
            ctx.stats.fuel_exhausted_functions += 1;
            ctx.timings.record_decision(function_decision(
                spec,
                fn_ir,
                PassDecisionStatus::DISABLED,
            ));
            if TachyonEngine::opt_fuel_trace_enabled() {
                eprintln!(
                    "   [fuel] {} exhausted before {}",
                    display_fn_name(fn_ir),
                    spec.id.timing_name()
                );
            }
            return ChronosPassOutcome::unchanged();
        }
        let opportunity = function_opportunity(engine, fn_ir, ctx, spec);
        ctx.timings.record_opportunity(opportunity);
        TachyonEngine::dump_mir_snapshot(fn_ir, spec.verify_label, spec.id.timing_name(), "before");
        let started = Instant::now();
        let outcome = (spec.run)(engine, fn_ir, ctx);
        let elapsed_ns = started.elapsed().as_nanos();
        if outcome.is_changed() {
            ctx.valid_analyses.invalidate(spec.invalidates);
            ctx.analysis_cache.invalidate(spec.invalidates);
        }
        ctx.timings
            .record(spec.id.timing_name(), elapsed_ns, outcome.is_changed());
        ctx.timings.record_decision(function_decision(
            spec,
            fn_ir,
            PassDecisionStatus::from_outcome(outcome, elapsed_ns),
        ));
        ctx.stats.fuel_consumed += ctx.fuel.consumed().saturating_sub(before_fuel);
        if ctx.fuel.exhausted() {
            ctx.stats.fuel_exhausted_functions += 1;
        }
        record_stats_delta(ctx.stats, spec.id, outcome);
        TachyonEngine::maybe_verify(fn_ir, spec.verify_label);
        TachyonEngine::dump_mir_snapshot(fn_ir, spec.verify_label, spec.id.timing_name(), "after");
        TachyonEngine::debug_stage_dump(fn_ir, spec.verify_label);
        outcome
    }
}

impl ChronosProgramPassManager {
    pub(in crate::mir::opt) const fn new(
        stage: ChronosStage,
        passes: &'static [ChronosProgramPassSpec],
    ) -> Self {
        Self { stage, passes }
    }

    pub(in crate::mir::opt) fn run_sequence(
        &self,
        engine: &TachyonEngine,
        all_fns: &mut FxHashMap<String, FnIR>,
        ctx: &mut ChronosProgramContext<'_>,
    ) -> ChronosPassOutcome {
        let mut changed_count = 0usize;
        let mut changed_passes = 0usize;
        for spec in self.passes {
            let outcome = self.run_program_pass(engine, all_fns, ctx, spec);
            changed_count = changed_count.saturating_add(outcome.changed_count);
            changed_passes = changed_passes.saturating_add(outcome.changed_passes);
        }
        ChronosPassOutcome::combined(changed_count, changed_passes)
    }

    pub(crate) fn run_program_pass(
        &self,
        engine: &TachyonEngine,
        all_fns: &mut FxHashMap<String, FnIR>,
        ctx: &mut ChronosProgramContext<'_>,
        spec: &ChronosProgramPassSpec,
    ) -> ChronosPassOutcome {
        if spec.stage != self.stage {
            return ChronosPassOutcome::unchanged();
        }
        if !(spec.enabled)(engine, all_fns, ctx) {
            ctx.timings
                .record_decision(program_decision(spec, PassDecisionStatus::DISABLED));
            return ChronosPassOutcome::unchanged();
        }

        let _metadata = (
            spec.scope,
            spec.group,
            spec.version,
            spec.proof_key,
            spec.contract,
            spec.requires,
            spec.invalidates,
            ctx.valid_analyses,
        );
        let before_fuel = ctx.fuel.consumed();
        if ctx
            .fuel
            .consume_pass_entry(program_pass_fuel_cost(spec.id, all_fns))
            .is_err()
        {
            ctx.stats.fuel_consumed += ctx.fuel.consumed().saturating_sub(before_fuel);
            ctx.stats.fuel_exhausted_functions += 1;
            ctx.timings
                .record_decision(program_decision(spec, PassDecisionStatus::DISABLED));
            if TachyonEngine::opt_fuel_trace_enabled() {
                eprintln!(
                    "   [fuel] program exhausted before {}",
                    spec.id.timing_name()
                );
            }
            return ChronosPassOutcome::unchanged();
        }
        ctx.timings
            .record_opportunity(program_opportunity(engine, all_fns, spec));
        TachyonEngine::dump_program_mir_snapshots(
            all_fns,
            spec.verify_label,
            spec.id.timing_name(),
            "before",
        );
        let started = Instant::now();
        let outcome = (spec.run)(engine, all_fns, ctx);
        let elapsed_ns = started.elapsed().as_nanos();
        if outcome.is_changed() {
            ctx.valid_analyses.invalidate(spec.invalidates);
        }
        ctx.timings
            .record(spec.id.timing_name(), elapsed_ns, outcome.is_changed());
        ctx.timings.record_decision(program_decision(
            spec,
            PassDecisionStatus::from_outcome(outcome, elapsed_ns),
        ));
        ctx.stats.fuel_consumed += ctx.fuel.consumed().saturating_sub(before_fuel);
        record_stats_delta(ctx.stats, spec.id, outcome);
        verify_program_functions(all_fns, spec.verify_label);
        TachyonEngine::dump_program_mir_snapshots(
            all_fns,
            spec.verify_label,
            spec.id.timing_name(),
            "after",
        );
        outcome
    }
}

pub(crate) fn function_decision(
    spec: &ChronosPassSpec,
    fn_ir: &FnIR,
    status: PassDecisionStatus,
) -> types::TachyonPassDecision {
    types::TachyonPassDecision {
        pass: spec.id.timing_name().to_string(),
        stage: spec.stage.label().to_string(),
        scope: spec.scope.label().to_string(),
        function: Some(display_fn_name(fn_ir)),
        group: spec.group.label().to_string(),
        proof_key: spec.proof_key.0.to_string(),
        legality: spec.contract.legality.to_string(),
        profitability: spec.contract.profitability.to_string(),
        budget_class: spec.contract.budget_class.label().to_string(),
        requires: spec.requires.labels(),
        invalidates: spec.invalidates.labels(),
        enabled: status.enabled,
        changed: status.changed,
        changed_count: status.changed_count,
        elapsed_ns: status.elapsed_ns,
        reason: status.reason.to_string(),
    }
}

pub(crate) fn program_decision(
    spec: &ChronosProgramPassSpec,
    status: PassDecisionStatus,
) -> types::TachyonPassDecision {
    types::TachyonPassDecision {
        pass: spec.id.timing_name().to_string(),
        stage: spec.stage.label().to_string(),
        scope: spec.scope.label().to_string(),
        function: None,
        group: spec.group.label().to_string(),
        proof_key: spec.proof_key.0.to_string(),
        legality: spec.contract.legality.to_string(),
        profitability: spec.contract.profitability.to_string(),
        budget_class: spec.contract.budget_class.label().to_string(),
        requires: spec.requires.labels(),
        invalidates: spec.invalidates.labels(),
        enabled: status.enabled,
        changed: status.changed,
        changed_count: status.changed_count,
        elapsed_ns: status.elapsed_ns,
        reason: status.reason.to_string(),
    }
}

pub(crate) fn function_pass_fuel_cost(id: ChronosPassId, ir_size: usize) -> usize {
    let multiplier = match id {
        ChronosPassId::Poly | ChronosPassId::Vectorize => 8,
        ChronosPassId::Gvn | ChronosPassId::LoopOpt | ChronosPassId::Licm | ChronosPassId::Bce => 5,
        ChronosPassId::Sroa | ChronosPassId::FreshAlloc | ChronosPassId::Unroll => 4,
        ChronosPassId::Outline => 6,
        _ => 2,
    };
    ir_size.saturating_mul(multiplier).saturating_add(32)
}

pub(crate) fn program_pass_fuel_cost(
    id: ChronosPassId,
    all_fns: &FxHashMap<String, FnIR>,
) -> usize {
    let ir_size: usize = all_fns.values().map(TachyonEngine::fn_ir_size).sum();
    let multiplier = match id {
        ChronosPassId::Inline => 6,
        ChronosPassId::Outline => 5,
        _ => 3,
    };
    ir_size.saturating_mul(multiplier).saturating_add(128)
}

pub(crate) fn function_opportunity(
    engine: &TachyonEngine,
    fn_ir: &FnIR,
    ctx: &mut ChronosContext<'_>,
    spec: &ChronosPassSpec,
) -> types::TachyonOpportunity {
    let facts = ctx.analysis_cache.fact_snapshot(fn_ir);
    let hotness = TachyonEngine::fn_static_hotness(fn_ir);
    let size_pressure = facts
        .ir_size
        .saturating_add(facts.stores.saturating_mul(4))
        .saturating_add(facts.calls.saturating_mul(3));
    let risk = facts
        .unsafe_blocks
        .saturating_mul(64)
        .saturating_add(facts.side_effecting_calls.saturating_mul(8));
    let estimated_gain = match spec.id {
        ChronosPassId::SimplifyCfg => facts.branches.saturating_mul(3),
        ChronosPassId::Sccp | ChronosPassId::Intrinsics | ChronosPassId::Simplify => {
            facts.calls.saturating_mul(2).saturating_add(facts.branches)
        }
        ChronosPassId::Gvn => facts
            .index_values
            .saturating_mul(4)
            .saturating_add(facts.calls.saturating_mul(2)),
        ChronosPassId::LoopOpt | ChronosPassId::Licm | ChronosPassId::Bce => facts
            .canonical_loops
            .saturating_mul(16)
            .saturating_add(facts.loops.saturating_mul(6)),
        ChronosPassId::Poly | ChronosPassId::Vectorize => facts
            .canonical_loops
            .saturating_mul(if engine.aggressive_opt_enabled() {
                48
            } else {
                24
            })
            .saturating_add(facts.index_values.saturating_mul(3)),
        ChronosPassId::Sroa | ChronosPassId::FreshAlloc => {
            facts.stores.saturating_mul(6).saturating_add(facts.calls)
        }
        ChronosPassId::Dce | ChronosPassId::CopyCleanup => facts.ir_size / 8,
        ChronosPassId::DeSsa => facts.ir_size / 16,
        ChronosPassId::Unroll => facts.canonical_loops.saturating_mul(12),
        ChronosPassId::Outline => facts.ir_size / 10,
        _ => facts.ir_size / 12,
    }
    .saturating_sub(risk.min(estimated_risk_cap(spec.id)));
    types::TachyonOpportunity {
        pass: spec.id.timing_name().to_string(),
        stage: spec.stage.label().to_string(),
        function: Some(display_fn_name(fn_ir)),
        ir_size: facts.ir_size,
        estimated_gain,
        size_pressure,
        hotness,
        risk,
        reason: opportunity_reason(spec.id, facts).to_string(),
    }
}

pub(crate) fn display_fn_name(fn_ir: &FnIR) -> String {
    fn_ir
        .user_name
        .clone()
        .unwrap_or_else(|| fn_ir.name.clone())
}

pub(crate) fn program_opportunity(
    _: &TachyonEngine,
    all_fns: &FxHashMap<String, FnIR>,
    spec: &ChronosProgramPassSpec,
) -> types::TachyonOpportunity {
    let mut ir_size = 0usize;
    let mut hotness = 0usize;
    let mut risk = 0usize;
    for fn_ir in all_fns.values() {
        ir_size = ir_size.saturating_add(TachyonEngine::fn_ir_size(fn_ir));
        hotness = hotness.saturating_add(TachyonEngine::fn_static_hotness(fn_ir));
        if fn_ir.requires_conservative_optimization() {
            risk = risk.saturating_add(64);
        }
    }
    let estimated_gain = match spec.id {
        ChronosPassId::Inline => hotness / 8 + all_fns.len().saturating_mul(4),
        ChronosPassId::Outline => ir_size / 10,
        ChronosPassId::RecordCallSpecialize | ChronosPassId::RecordReturnSpecialize => {
            all_fns.len().saturating_mul(6)
        }
        _ => ir_size / 16,
    }
    .saturating_sub(risk.min(128));
    types::TachyonOpportunity {
        pass: spec.id.timing_name().to_string(),
        stage: spec.stage.label().to_string(),
        function: None,
        ir_size,
        estimated_gain,
        size_pressure: ir_size,
        hotness,
        risk,
        reason: "program-level opportunity estimate".to_string(),
    }
}

pub(crate) const fn estimated_risk_cap(id: ChronosPassId) -> usize {
    match id {
        ChronosPassId::Poly | ChronosPassId::Vectorize | ChronosPassId::Licm => 96,
        ChronosPassId::Bce | ChronosPassId::FreshAlloc | ChronosPassId::Sroa => 64,
        _ => 32,
    }
}

pub(crate) fn opportunity_reason(id: ChronosPassId, facts: ChronosFactSnapshot) -> &'static str {
    match id {
        ChronosPassId::Poly | ChronosPassId::Vectorize if facts.canonical_loops > 0 => {
            "canonical loop structural opportunity"
        }
        ChronosPassId::LoopOpt | ChronosPassId::Licm | ChronosPassId::Bce if facts.loops > 0 => {
            "loop fact opportunity"
        }
        ChronosPassId::Gvn if facts.index_values > 0 => "index/value reuse opportunity",
        ChronosPassId::Sroa | ChronosPassId::FreshAlloc if facts.stores > 0 => {
            "aggregate or allocation simplification opportunity"
        }
        ChronosPassId::Dce | ChronosPassId::CopyCleanup => "cleanup opportunity",
        ChronosPassId::Unroll => "constant trip loop unroll opportunity",
        ChronosPassId::Outline => "large function outlining opportunity",
        _ => "local pass opportunity",
    }
}

pub(crate) fn verify_program_functions(all_fns: &FxHashMap<String, FnIR>, verify_label: &str) {
    for name in TachyonEngine::sorted_fn_names(all_fns) {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        TachyonEngine::maybe_verify(fn_ir, verify_label);
        TachyonEngine::debug_stage_dump(fn_ir, verify_label);
    }
}

pub(crate) fn record_stats_delta(
    stats: &mut TachyonPulseStats,
    id: ChronosPassId,
    outcome: ChronosPassOutcome,
) {
    if !outcome.is_changed() {
        return;
    }
    match id {
        ChronosPassId::SimplifyCfg => stats.simplify_hits += 1,
        ChronosPassId::Sccp => stats.sccp_hits += 1,
        ChronosPassId::Intrinsics => stats.intrinsics_hits += 1,
        ChronosPassId::Gvn => stats.gvn_hits += 1,
        ChronosPassId::Simplify => stats.simplify_hits += 1,
        ChronosPassId::Inline => stats.inline_rounds += 1,
        ChronosPassId::Tco => stats.tco_hits += 1,
        ChronosPassId::LoopOpt => stats.simplified_loops += outcome.changed_count,
        ChronosPassId::Licm => stats.licm_hits += 1,
        ChronosPassId::Dce => stats.dce_hits += 1,
        ChronosPassId::DeSsa => stats.de_ssa_hits += 1,
        ChronosPassId::CopyCleanup => stats.simplify_hits += 1,
        ChronosPassId::RecordCallSpecialize | ChronosPassId::RecordReturnSpecialize => {}
        ChronosPassId::FreshAlias => {}
        ChronosPassId::IndexCanonicalize => {}
        ChronosPassId::FreshAlloc => stats.fresh_alloc_hits += 1,
        ChronosPassId::Bce => stats.bce_hits += 1,
        ChronosPassId::Unroll | ChronosPassId::Outline => {}
        ChronosPassId::TypeSpecialize
        | ChronosPassId::Poly
        | ChronosPassId::Vectorize
        | ChronosPassId::Sroa => {}
    }
}
