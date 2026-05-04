use super::super::types::{
    FunctionPhaseFeatures, FunctionPhasePlan, PhaseOrderingMode, PhaseProfileKind, PhaseScheduleId,
    default_pass_groups_for_schedule,
};
use super::*;
use crate::mir::analyze::effects;

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct HeavyPhaseIterationResult {
    pub(crate) changed: bool,
    pub(crate) non_structural_changes: usize,
    pub(crate) structural_progress: bool,
    pub(crate) ran_structural: bool,
    pub(crate) skipped_structural: bool,
}

pub(crate) struct HeavyPhaseIterationRequest<'a> {
    pub(crate) schedule: PhaseScheduleId,
    pub(crate) fn_ir: &'a mut FnIR,
    pub(crate) callmap_user_whitelist: &'a FxHashSet<String>,
    pub(crate) loop_opt: &'a loop_opt::MirLoopOptimizer,
    pub(crate) stats: &'a mut TachyonPulseStats,
    pub(crate) pass_timings: &'a mut TachyonPassTimings,
    pub(crate) run_budgeted_passes: bool,
}

impl TachyonEngine {
    const COMPUTE_SCORE_CANONICAL_LOOP: usize = 32;
    const COMPUTE_SCORE_LOOP: usize = 16;
    const COMPUTE_SCORE_ARITH: usize = 2;
    const COMPUTE_SCORE_INTRINSIC: usize = 4;
    const COMPUTE_SCORE_INDEX: usize = 2;
    const COMPUTE_SCORE_STORE: usize = 2;

    const CONTROL_SCORE_BRANCH: usize = 18;
    const CONTROL_SCORE_PHI: usize = 8;
    const CONTROL_SCORE_SIDE_EFFECT_CALL: usize = 16;

    const PHASE_PROFILE_MARGIN: usize = 24;
    const MAX_AUTO_COMPUTE_FN_IR: usize = 256;
    const MAX_AUTO_COMPUTE_BLOCKS: usize = 16;
    const MAX_AUTO_CONTROL_FN_IR: usize = 128;
    const MAX_AUTO_CONTROL_BLOCKS: usize = 12;
    const FAST_DEV_VECTORIZE_MAX_FN_IR: usize = 128;
    const FAST_DEV_VECTORIZE_MAX_BLOCKS: usize = 12;
    const FAST_DEV_VECTORIZE_MAX_LOOPS: usize = 1;

    pub(crate) fn phase_feature_helper_is_functionally_pure(callee: &str) -> bool {
        matches!(
            callee,
            "rr_assign_slice"
                | "rr_ifelse_strict"
                | "rr_index1_read"
                | "rr_index1_read_strict"
                | "rr_index1_read_vec"
                | "rr_index1_read_vec_floor"
                | "rr_index_vec_floor"
                | "rr_gather"
                | "rr_wrap_index_vec"
                | "rr_wrap_index_vec_i"
                | "rr_idx_cube_vec_i"
                | "rr_named_list"
                | "rr_field_get"
                | "rr_field_exists"
                | "rr_list_pattern_matchable"
        )
    }

    pub(crate) fn extract_function_phase_features(fn_ir: &FnIR) -> FunctionPhaseFeatures {
        let loops = loop_analysis::LoopAnalyzer::new(fn_ir).find_loops();
        Self::extract_function_phase_features_with_loops(fn_ir, &loops)
    }

    pub(crate) fn extract_function_phase_features_with_loops(
        fn_ir: &FnIR,
        loops: &[loop_analysis::LoopInfo],
    ) -> FunctionPhaseFeatures {
        let canonical_loop_count = loops
            .iter()
            .filter(|lp| {
                lp.iv.is_some()
                    && (lp.limit.is_some() || lp.is_seq_len.is_some() || lp.is_seq_along.is_some())
            })
            .count();

        let mut branch_terms = 0usize;
        let mut phi_count = 0usize;
        let mut arithmetic_values = 0usize;
        let mut intrinsic_values = 0usize;
        let mut call_values = 0usize;
        let mut side_effecting_calls = 0usize;
        let mut index_values = 0usize;
        let mut store_instrs = 0usize;

        for block in &fn_ir.blocks {
            if matches!(block.term, Terminator::If { .. }) {
                branch_terms += 1;
            }
            for instr in &block.instrs {
                if matches!(
                    instr,
                    Instr::StoreIndex1D { .. }
                        | Instr::StoreIndex2D { .. }
                        | Instr::StoreIndex3D { .. }
                        | Instr::UnsafeRBlock { .. }
                ) {
                    store_instrs += 1;
                }
            }
        }

        for value in &fn_ir.values {
            match &value.kind {
                ValueKind::Phi { .. } => phi_count += 1,
                ValueKind::Binary { .. } | ValueKind::Unary { .. } => arithmetic_values += 1,
                ValueKind::Intrinsic { .. } => intrinsic_values += 1,
                ValueKind::Call { callee, .. } => {
                    call_values += 1;
                    if !(effects::call_is_pure(callee)
                        || Self::phase_feature_helper_is_functionally_pure(callee))
                    {
                        side_effecting_calls += 1;
                    }
                }
                ValueKind::Index1D { .. }
                | ValueKind::Index2D { .. }
                | ValueKind::Index3D { .. } => {
                    index_values += 1;
                }
                _ => {}
            }
        }

        FunctionPhaseFeatures {
            ir_size: Self::fn_ir_size(fn_ir),
            block_count: fn_ir.blocks.len(),
            loop_count: loops.len(),
            canonical_loop_count,
            branch_terms,
            phi_count,
            arithmetic_values,
            intrinsic_values,
            call_values,
            side_effecting_calls,
            index_values,
            store_instrs,
        }
    }

    pub(crate) fn compute_phase_profile_scores(features: &FunctionPhaseFeatures) -> (usize, usize) {
        let compute_score = features
            .canonical_loop_count
            .saturating_mul(Self::COMPUTE_SCORE_CANONICAL_LOOP)
            .saturating_add(features.loop_count.saturating_mul(Self::COMPUTE_SCORE_LOOP))
            .saturating_add(
                features
                    .arithmetic_values
                    .saturating_mul(Self::COMPUTE_SCORE_ARITH),
            )
            .saturating_add(
                features
                    .intrinsic_values
                    .saturating_mul(Self::COMPUTE_SCORE_INTRINSIC),
            )
            .saturating_add(
                features
                    .index_values
                    .saturating_mul(Self::COMPUTE_SCORE_INDEX),
            )
            .saturating_add(
                features
                    .store_instrs
                    .saturating_mul(Self::COMPUTE_SCORE_STORE),
            );

        let control_score = features
            .branch_terms
            .saturating_mul(Self::CONTROL_SCORE_BRANCH)
            .saturating_add(features.phi_count.saturating_mul(Self::CONTROL_SCORE_PHI))
            .saturating_add(
                features
                    .side_effecting_calls
                    .saturating_mul(Self::CONTROL_SCORE_SIDE_EFFECT_CALL),
            );

        (compute_score, control_score)
    }

    pub(crate) fn classify_phase_profile(features: &FunctionPhaseFeatures) -> PhaseProfileKind {
        // Proof correspondence:
        // `PhasePlanSoundness` fixes a reduced classification boundary for
        // `classify_phase_profile`, including concrete balanced /
        // compute-heavy / control-flow-heavy witnesses and a reduced
        // score-based classifier over plan features.
        let (compute_score, control_score) = Self::compute_phase_profile_scores(features);
        let branch_density_high = features.branch_terms.saturating_mul(3) >= features.block_count;
        let side_effects_light =
            features.side_effecting_calls.saturating_mul(4) <= features.call_values.max(1);
        let compute_schedule_safe = features.ir_size <= Self::MAX_AUTO_COMPUTE_FN_IR
            && features.block_count <= Self::MAX_AUTO_COMPUTE_BLOCKS
            && features.canonical_loop_count > 0
            && features.side_effecting_calls == 0;
        let control_schedule_safe = features.ir_size <= Self::MAX_AUTO_CONTROL_FN_IR
            && features.block_count <= Self::MAX_AUTO_CONTROL_BLOCKS
            && features.loop_count == 0;

        if compute_schedule_safe
            && side_effects_light
            && compute_score >= control_score.saturating_add(Self::PHASE_PROFILE_MARGIN)
        {
            PhaseProfileKind::ComputeHeavy
        } else if control_schedule_safe
            && (control_score >= compute_score.saturating_add(Self::PHASE_PROFILE_MARGIN)
                || branch_density_high)
        {
            PhaseProfileKind::ControlFlowHeavy
        } else {
            PhaseProfileKind::Balanced
        }
    }

    pub(crate) fn choose_phase_schedule(
        mode: PhaseOrderingMode,
        profile: PhaseProfileKind,
    ) -> PhaseScheduleId {
        // Proof correspondence:
        // `PhasePlanSoundness` fixes the reduced schedule-selection boundary
        // for this function. The reduced theorem family names the same three
        // cases: off/balanced mode forcing `Balanced`, and auto mode selecting
        // the schedule that matches the classified profile.
        match mode {
            PhaseOrderingMode::Off | PhaseOrderingMode::Balanced => PhaseScheduleId::Balanced,
            PhaseOrderingMode::Auto => match profile {
                PhaseProfileKind::Balanced => PhaseScheduleId::Balanced,
                PhaseProfileKind::ComputeHeavy => PhaseScheduleId::ComputeHeavy,
                PhaseProfileKind::ControlFlowHeavy => PhaseScheduleId::ControlFlowHeavy,
            },
        }
    }

    pub(crate) fn emit_phase_plan_trace(plan: &FunctionPhasePlan) {
        let Some(features) = plan.features else {
            return;
        };
        eprintln!(
            "   [phase-order] {} mode={} profile={} schedule={} ir={} blocks={} loops={} canon_loops={} branches={} phi={} arith={} intrinsics={} calls={} sidefx_calls={} index={} stores={}",
            plan.function,
            plan.mode.label(),
            plan.profile.label(),
            plan.schedule.label(),
            features.ir_size,
            features.block_count,
            features.loop_count,
            features.canonical_loop_count,
            features.branch_terms,
            features.phi_count,
            features.arithmetic_values,
            features.intrinsic_values,
            features.call_values,
            features.side_effecting_calls,
            features.index_values,
            features.store_instrs
        );
    }

    pub(crate) fn phase_branch_density_high(features: &FunctionPhaseFeatures) -> bool {
        features.branch_terms.saturating_mul(3) >= features.block_count.max(1)
    }

    pub(crate) fn control_flow_structural_gate(features: &FunctionPhaseFeatures) -> bool {
        let branch_density_high = Self::phase_branch_density_high(features);
        let side_effects_dominant =
            features.side_effecting_calls.saturating_mul(2) > features.call_values.max(1);
        features.canonical_loop_count > 0 && !branch_density_high && !side_effects_dominant
    }

    pub(crate) fn fast_dev_vectorize_gate(features: &FunctionPhaseFeatures) -> bool {
        features.canonical_loop_count > 0
            && features.loop_count <= Self::FAST_DEV_VECTORIZE_MAX_LOOPS
            && features.ir_size <= Self::FAST_DEV_VECTORIZE_MAX_FN_IR
            && features.block_count <= Self::FAST_DEV_VECTORIZE_MAX_BLOCKS
            && features.branch_terms <= 2
            && features.side_effecting_calls <= 1
            && features.store_instrs > 0
    }

    pub(crate) fn fast_dev_vectorize_enabled_for_fn(&self, fn_ir: &FnIR) -> bool {
        self.fast_dev_enabled()
            && Self::fast_dev_vectorize_gate(&Self::extract_function_phase_features(fn_ir))
    }

    pub(crate) fn run_fast_dev_vectorize_subpath(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
    ) -> bool {
        // Proof correspondence:
        // `PhaseOrderIterationSoundness.fast_dev_subpath_preserves_*`
        // fixes the reduced theorem family for this entrypoint. The reduced
        // model currently approximates this Rust slice by the same structural
        // cluster used elsewhere, but with a dedicated theorem name so the
        // dispatch boundary stays 1:1 with the production helper.
        let loop_opt = loop_opt::MirLoopOptimizer::new();
        self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
            stage: chronos::ChronosStage::PhaseOrderFastDevVectorize,
            passes: chronos::PHASE_ORDER_FAST_DEV_VECTORIZE_PASSES,
            fn_ir,
            loop_optimizer: &loop_opt,
            user_call_whitelist: Some(callmap_user_whitelist),
            fresh_user_calls: None,
            stats,
            timings: pass_timings,
        })
        .is_changed()
    }

    pub(crate) fn control_flow_should_fallback_to_balanced(
        result: HeavyPhaseIterationResult,
    ) -> bool {
        // Proof correspondence:
        // `PhaseOrderFallbackSoundness.control_flow_fallback_preserves_*`
        // fixes the reduced theorem boundary for this predicate. The reduced
        // model keeps only `structural_progress` and `non_structural_changes`,
        // then proves that falling back to the balanced path remains within
        // the same invariant-preserving / semantics-preserving optimizer spine.
        !result.structural_progress && result.non_structural_changes <= 1
    }

    pub(crate) fn build_function_phase_plan_from_features(
        &self,
        function: &str,
        mode: PhaseOrderingMode,
        trace_requested: bool,
        features: FunctionPhaseFeatures,
    ) -> FunctionPhasePlan {
        // Proof correspondence:
        // `PhasePlanSoundness` fixes the reduced `classify -> choose schedule
        // -> build plan` boundary for this helper, including default pass
        // groups, fast-dev filtering, and plan-selected schedule soundness.
        let profile = if matches!(mode, PhaseOrderingMode::Auto) {
            Self::classify_phase_profile(&features)
        } else {
            PhaseProfileKind::Balanced
        };
        let schedule = Self::choose_phase_schedule(mode, profile);
        let mut pass_groups = default_pass_groups_for_schedule(schedule);
        if self.aggressive_opt_enabled()
            && !pass_groups.contains(&super::types::PassGroup::Experimental)
        {
            pass_groups.push(super::types::PassGroup::Experimental);
        }
        FunctionPhasePlan {
            function: function.to_string(),
            mode,
            profile,
            schedule,
            pass_groups: self.adjust_pass_groups_for_mode(&pass_groups),
            features: Some(features),
            trace_requested,
        }
    }

    pub(crate) fn build_function_phase_plan(
        &self,
        function: &str,
        fn_ir: &FnIR,
    ) -> FunctionPhasePlan {
        let mode = self.resolved_phase_ordering_mode();
        let trace_requested = Self::phase_ordering_trace_enabled();
        let features = Self::extract_function_phase_features(fn_ir);
        self.build_function_phase_plan_from_features(function, mode, trace_requested, features)
    }

    pub(crate) fn collect_function_phase_plans(
        &self,
        all_fns: &FxHashMap<String, FnIR>,
        ordered_names: &[String],
        selected_functions: Option<&FxHashSet<String>>,
    ) -> FxHashMap<String, FunctionPhasePlan> {
        // Proof correspondence:
        // `PhasePlanCollectionSoundness` fixes the reduced collection boundary
        // for this helper. The reduced model keeps the same skip/filter shape:
        // missing function entry, conservative-optimization requirement,
        // self-recursive function, and selected-function filter miss, then
        // reuses `PhasePlanSoundness` for every collected plan.
        let mut plans = FxHashMap::default();
        for name in ordered_names {
            let Some(fn_ir) = all_fns.get(name) else {
                continue;
            };
            if fn_ir.requires_conservative_optimization() || Self::fn_is_self_recursive(fn_ir) {
                continue;
            }
            if selected_functions.is_some_and(|selected| !selected.contains(name)) {
                continue;
            }
            let plan = self.build_function_phase_plan(name, fn_ir);
            if plan.trace_requested {
                Self::emit_phase_plan_trace(&plan);
            }
            plans.insert(name.clone(), plan);
        }
        plans
    }

    pub(crate) fn run_heavy_phase_schedule_iteration(
        &self,
        request: HeavyPhaseIterationRequest<'_>,
    ) -> HeavyPhaseIterationResult {
        let HeavyPhaseIterationRequest {
            schedule,
            fn_ir,
            callmap_user_whitelist,
            loop_opt,
            stats,
            pass_timings,
            run_budgeted_passes,
        } = request;
        // Proof correspondence:
        // `PhaseOrderOptimizerSoundness` fixes a reduced schedule family for
        // the three Rust phase-order profiles dispatched here:
        // `Balanced`, `ComputeHeavy`, and `ControlFlowHeavy`.
        // Those reduced theorems currently refine the optimizer-only spine
        // theorem names without yet modeling the full per-pass delta between
        // the three production schedules.
        match schedule {
            PhaseScheduleId::Balanced => self.run_balanced_heavy_phase_iteration(
                fn_ir,
                callmap_user_whitelist,
                loop_opt,
                stats,
                pass_timings,
                run_budgeted_passes,
            ),
            PhaseScheduleId::ComputeHeavy => self.run_compute_heavy_phase_iteration(
                fn_ir,
                callmap_user_whitelist,
                loop_opt,
                stats,
                pass_timings,
                run_budgeted_passes,
            ),
            PhaseScheduleId::ControlFlowHeavy => self.run_control_flow_heavy_phase_iteration(
                fn_ir,
                callmap_user_whitelist,
                loop_opt,
                stats,
                pass_timings,
                run_budgeted_passes,
            ),
        }
    }

    pub(crate) fn run_chronos_function_sequence(
        &self,
        request: chronos::ChronosFunctionSequenceRequest<'_>,
    ) -> chronos::ChronosPassOutcome {
        let chronos::ChronosFunctionSequenceRequest {
            stage,
            passes,
            fn_ir,
            loop_optimizer,
            user_call_whitelist,
            fresh_user_calls,
            stats,
            timings,
        } = request;
        let mut ctx = chronos::ChronosContext {
            stats,
            timings,
            loop_optimizer,
            user_call_whitelist,
            fresh_user_calls,
            proven_param_slots: None,
            floor_helpers: None,
            valid_analyses: chronos::ChronosAnalysisSet::ALL,
            analysis_cache: chronos::ChronosAnalysisCache::default(),
            fn_ir_size: Self::fn_ir_size(fn_ir),
            run_light_sccp: true,
            fuel: self.fuel_for_function(Self::fn_ir_size(fn_ir)),
        };
        chronos::ChronosPassManager::new(stage, passes).run_sequence(self, fn_ir, &mut ctx)
    }

    pub(crate) fn record_non_structural_chronos_result(
        result: &mut HeavyPhaseIterationResult,
        outcome: chronos::ChronosPassOutcome,
    ) {
        if !outcome.is_changed() {
            return;
        }
        result.changed = true;
        result.non_structural_changes = result
            .non_structural_changes
            .saturating_add(outcome.changed_passes);
    }

    pub(crate) fn run_compute_heavy_phase_iteration(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        loop_opt: &loop_opt::MirLoopOptimizer,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
        run_budgeted_passes: bool,
    ) -> HeavyPhaseIterationResult {
        // Proof correspondence:
        // `PhaseOrderIterationSoundness.compute_heavy_iteration_preserves_*`
        // names the reduced theorem family for this entrypoint. It composes
        // the reduced standard cluster with the structural/cleanup and
        // fast-dev fallback boundaries already fixed in the cluster/guard/
        // feature-gate layers.
        let mut result = HeavyPhaseIterationResult::default();

        let prelude_outcome =
            self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                stage: chronos::ChronosStage::PhaseOrderComputePrelude,
                passes: chronos::PHASE_ORDER_COMPUTE_PRELUDE_PASSES,
                fn_ir,
                loop_optimizer: loop_opt,
                user_call_whitelist: None,
                fresh_user_calls: None,
                stats,
                timings: pass_timings,
            });
        Self::record_non_structural_chronos_result(&mut result, prelude_outcome);

        if run_budgeted_passes {
            let budget_prefix_outcome =
                self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                    stage: chronos::ChronosStage::PhaseOrderBudgetPrefix,
                    passes: chronos::PHASE_ORDER_BUDGET_PREFIX_PASSES,
                    fn_ir,
                    loop_optimizer: loop_opt,
                    user_call_whitelist: None,
                    fresh_user_calls: None,
                    stats,
                    timings: pass_timings,
                });
            Self::record_non_structural_chronos_result(&mut result, budget_prefix_outcome);

            let pass_changed = if self.structural_optimizations_enabled() {
                self.run_balanced_structural_cluster(
                    fn_ir,
                    callmap_user_whitelist,
                    stats,
                    pass_timings,
                )
            } else {
                false
            };
            if pass_changed {
                result.changed = true;
                result.structural_progress = true;
                result.ran_structural = true;
                if self.run_balanced_structural_cleanup(fn_ir, stats, pass_timings) {
                    result.changed = true;
                    result.structural_progress = true;
                }
            } else {
                result.ran_structural = true;
            }

            if !self.structural_optimizations_enabled()
                && self.fast_dev_vectorize_enabled_for_fn(fn_ir)
                && self.run_fast_dev_vectorize_subpath(
                    fn_ir,
                    callmap_user_whitelist,
                    stats,
                    pass_timings,
                )
            {
                result.changed = true;
                result.non_structural_changes += 1;
            }

            let budget_tail_outcome =
                self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                    stage: chronos::ChronosStage::PhaseOrderBudgetTail,
                    passes: chronos::PHASE_ORDER_BUDGET_TAIL_PASSES,
                    fn_ir,
                    loop_optimizer: loop_opt,
                    user_call_whitelist: None,
                    fresh_user_calls: None,
                    stats,
                    timings: pass_timings,
                });
            Self::record_non_structural_chronos_result(&mut result, budget_tail_outcome);
        }

        result
    }

    pub(crate) fn run_balanced_heavy_phase_iteration(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        loop_opt: &loop_opt::MirLoopOptimizer,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
        run_budgeted_passes: bool,
    ) -> HeavyPhaseIterationResult {
        // Proof correspondence:
        // `PhaseOrderIterationSoundness.balanced_iteration_preserves_*`
        // names the reduced theorem family for this balanced entrypoint,
        // including the structural cluster, optional cleanup, fast-dev
        // fallback, and the trailing standard cluster.
        let mut result = HeavyPhaseIterationResult::default();
        let pass_changed = if run_budgeted_passes {
            if self.structural_optimizations_enabled() {
                self.run_balanced_structural_cluster(
                    fn_ir,
                    callmap_user_whitelist,
                    stats,
                    pass_timings,
                )
            } else {
                false
            }
        } else {
            false
        };

        if pass_changed {
            result.changed = true;
            result.structural_progress = true;
            result.ran_structural = true;
            if self.run_balanced_structural_cleanup(fn_ir, stats, pass_timings) {
                result.changed = true;
                result.structural_progress = true;
            }
        }

        if run_budgeted_passes {
            result.ran_structural = true;
        }

        if run_budgeted_passes
            && !self.structural_optimizations_enabled()
            && self.fast_dev_vectorize_enabled_for_fn(fn_ir)
            && self.run_fast_dev_vectorize_subpath(
                fn_ir,
                callmap_user_whitelist,
                stats,
                pass_timings,
            )
        {
            result.changed = true;
            result.non_structural_changes += 1;
        }

        let standard_changed = self.run_balanced_standard_cluster(
            fn_ir,
            loop_opt,
            stats,
            pass_timings,
            run_budgeted_passes,
        );
        if standard_changed {
            result.changed = true;
            result.non_structural_changes += 1;
        }
        result
    }

    pub(crate) fn run_control_flow_heavy_phase_iteration(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        loop_opt: &loop_opt::MirLoopOptimizer,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
        run_budgeted_passes: bool,
    ) -> HeavyPhaseIterationResult {
        // Proof correspondence:
        // `PhaseOrderIterationSoundness.control_flow_heavy_iteration_preserves_*`
        // names the reduced theorem family for this entrypoint. The reduced
        // version keeps the same standard-prelude then structural/cleanup
        // dispatch shape, keyed by the control-flow gate and fast-dev fallback.
        let mut result = HeavyPhaseIterationResult::default();

        let prelude_outcome =
            self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                stage: chronos::ChronosStage::PhaseOrderControlPrelude,
                passes: chronos::PHASE_ORDER_CONTROL_PRELUDE_PASSES,
                fn_ir,
                loop_optimizer: loop_opt,
                user_call_whitelist: None,
                fresh_user_calls: None,
                stats,
                timings: pass_timings,
            });
        Self::record_non_structural_chronos_result(&mut result, prelude_outcome);

        if run_budgeted_passes {
            let budget_prefix_outcome =
                self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                    stage: chronos::ChronosStage::PhaseOrderBudgetPrefix,
                    passes: chronos::PHASE_ORDER_CONTROL_BUDGET_PREFIX_PASSES,
                    fn_ir,
                    loop_optimizer: loop_opt,
                    user_call_whitelist: None,
                    fresh_user_calls: None,
                    stats,
                    timings: pass_timings,
                });
            Self::record_non_structural_chronos_result(&mut result, budget_prefix_outcome);

            let structural_features = Self::extract_function_phase_features(fn_ir);
            if self.structural_optimizations_enabled()
                && Self::control_flow_structural_gate(&structural_features)
            {
                result.ran_structural = true;
                let poly_changed = self.run_control_flow_structural_cluster(
                    fn_ir,
                    callmap_user_whitelist,
                    stats,
                    pass_timings,
                );
                if poly_changed {
                    result.changed = true;
                    result.structural_progress = true;
                    if self.run_balanced_structural_cleanup(fn_ir, stats, pass_timings) {
                        result.changed = true;
                        result.structural_progress = true;
                    }
                }
            } else {
                result.skipped_structural = true;
            }

            if !self.structural_optimizations_enabled()
                && self.fast_dev_vectorize_enabled_for_fn(fn_ir)
                && self.run_fast_dev_vectorize_subpath(
                    fn_ir,
                    callmap_user_whitelist,
                    stats,
                    pass_timings,
                )
            {
                result.changed = true;
                result.non_structural_changes += 1;
            }

            let budget_tail_outcome =
                self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                    stage: chronos::ChronosStage::PhaseOrderBudgetTail,
                    passes: chronos::PHASE_ORDER_BUDGET_TAIL_PASSES,
                    fn_ir,
                    loop_optimizer: loop_opt,
                    user_call_whitelist: None,
                    fresh_user_calls: None,
                    stats,
                    timings: pass_timings,
                });
            Self::record_non_structural_chronos_result(&mut result, budget_tail_outcome);
        }

        result
    }

    pub(crate) fn run_balanced_structural_cluster(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
    ) -> bool {
        // Proof correspondence:
        // `PhaseOrderClusterSoundness` names this reduced boundary as the
        // `structural_cluster_*` theorem family. In Rust this cluster covers
        // the structural/vectorization-oriented slice:
        // `type_specialize -> poly -> vectorize -> type_specialize -> tco`.
        let loop_opt = loop_opt::MirLoopOptimizer::new();
        self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
            stage: chronos::ChronosStage::PhaseOrderBalancedStructural,
            passes: chronos::PHASE_ORDER_BALANCED_STRUCTURAL_PASSES,
            fn_ir,
            loop_optimizer: &loop_opt,
            user_call_whitelist: Some(callmap_user_whitelist),
            fresh_user_calls: None,
            stats,
            timings: pass_timings,
        })
        .is_changed()
    }

    pub(crate) fn run_control_flow_structural_cluster(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
    ) -> bool {
        // Proof correspondence:
        // this is the control-flow-heavy structural cluster matched by the same
        // reduced `structural_cluster_*` theorem family, but with the narrower
        // Rust pass slice:
        // `poly -> vectorize -> type_specialize`.
        let loop_opt = loop_opt::MirLoopOptimizer::new();
        self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
            stage: chronos::ChronosStage::PhaseOrderControlStructural,
            passes: chronos::PHASE_ORDER_CONTROL_STRUCTURAL_PASSES,
            fn_ir,
            loop_optimizer: &loop_opt,
            user_call_whitelist: Some(callmap_user_whitelist),
            fresh_user_calls: None,
            stats,
            timings: pass_timings,
        })
        .is_changed()
    }

    pub(crate) fn run_balanced_structural_cleanup(
        &self,
        fn_ir: &mut FnIR,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
    ) -> bool {
        // Proof correspondence:
        // `PhaseOrderClusterSoundness.cleanup_cluster_*` approximates this
        // reduced cleanup boundary:
        // `simplify_cfg -> sroa -> dce`.
        let loop_opt = loop_opt::MirLoopOptimizer::new();
        self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
            stage: chronos::ChronosStage::PhaseOrderStructuralCleanup,
            passes: chronos::PHASE_ORDER_STRUCTURAL_CLEANUP_PASSES,
            fn_ir,
            loop_optimizer: &loop_opt,
            user_call_whitelist: None,
            fresh_user_calls: None,
            stats,
            timings: pass_timings,
        })
        .is_changed()
    }

    pub(crate) fn run_balanced_standard_cluster(
        &self,
        fn_ir: &mut FnIR,
        loop_opt: &loop_opt::MirLoopOptimizer,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
        run_budgeted_passes: bool,
    ) -> bool {
        // Proof correspondence:
        // `PhaseOrderClusterSoundness.standard_cluster_*` approximates this
        // reduced compute/dataflow/loop cluster:
        // `simplify_cfg -> sccp -> intrinsics -> gvn -> simplify -> sroa -> dce`
        // plus, when budgeted, `loop_opt -> licm -> fresh_alloc -> bce`.
        let mut changed = self
            .run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                stage: chronos::ChronosStage::PhaseOrderStandard,
                passes: chronos::PHASE_ORDER_STANDARD_CORE_PASSES,
                fn_ir,
                loop_optimizer: loop_opt,
                user_call_whitelist: None,
                fresh_user_calls: None,
                stats,
                timings: pass_timings,
            })
            .is_changed();

        if run_budgeted_passes {
            changed |= self
                .run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                    stage: chronos::ChronosStage::PhaseOrderStandard,
                    passes: chronos::PHASE_ORDER_STANDARD_BUDGET_PASSES,
                    fn_ir,
                    loop_optimizer: loop_opt,
                    user_call_whitelist: None,
                    fresh_user_calls: None,
                    stats,
                    timings: pass_timings,
                })
                .is_changed();
        }

        changed
    }
    pub(crate) fn accumulate_poly_stats(stats: &mut TachyonPulseStats, p_stats: poly::PolyStats) {
        stats.poly_loops_seen += p_stats.loops_seen;
        stats.poly_scops_detected += p_stats.scops_detected;
        stats.poly_rejected_cfg_shape += p_stats.rejected_cfg_shape;
        stats.poly_rejected_non_affine += p_stats.rejected_non_affine;
        stats.poly_rejected_effects += p_stats.rejected_effects;
        stats.poly_affine_stmt_count += p_stats.affine_stmt_count;
        stats.poly_access_relation_count += p_stats.access_relation_count;
        stats.poly_dependence_solved += p_stats.dependence_solved;
        stats.poly_schedule_attempted += p_stats.schedule_attempted;
        stats.poly_schedule_applied += p_stats.schedule_applied;
        stats.poly_schedule_attempted_identity += p_stats.schedule_attempted_identity;
        stats.poly_schedule_attempted_interchange += p_stats.schedule_attempted_interchange;
        stats.poly_schedule_attempted_skew2d += p_stats.schedule_attempted_skew2d;
        stats.poly_schedule_attempted_tile1d += p_stats.schedule_attempted_tile1d;
        stats.poly_schedule_attempted_tile2d += p_stats.schedule_attempted_tile2d;
        stats.poly_schedule_attempted_tile3d += p_stats.schedule_attempted_tile3d;
        stats.poly_schedule_applied_identity += p_stats.schedule_applied_identity;
        stats.poly_schedule_applied_interchange += p_stats.schedule_applied_interchange;
        stats.poly_schedule_applied_skew2d += p_stats.schedule_applied_skew2d;
        stats.poly_schedule_applied_tile1d += p_stats.schedule_applied_tile1d;
        stats.poly_schedule_applied_tile2d += p_stats.schedule_applied_tile2d;
        stats.poly_schedule_applied_tile3d += p_stats.schedule_applied_tile3d;
        stats.poly_schedule_auto_fuse_selected += p_stats.schedule_auto_fuse_selected;
        stats.poly_schedule_auto_fission_selected += p_stats.schedule_auto_fission_selected;
        stats.poly_schedule_auto_skew2d_selected += p_stats.schedule_auto_skew2d_selected;
        stats.poly_schedule_backend_hint_selected += p_stats.schedule_backend_hint_selected;
    }

    pub(crate) fn accumulate_vector_stats(
        stats: &mut TachyonPulseStats,
        v_stats: v_opt::VOptStats,
    ) {
        stats.vectorized += v_stats.vectorized;
        stats.reduced += v_stats.reduced;
        stats.vector_loops_seen += v_stats.loops_seen;
        stats.vector_skipped += v_stats.skipped;
        stats.vector_skip_no_iv += v_stats.skip_no_iv;
        stats.vector_skip_non_canonical_bound += v_stats.skip_non_canonical_bound;
        stats.vector_skip_unsupported_cfg_shape += v_stats.skip_unsupported_cfg_shape;
        stats.vector_skip_indirect_index_access += v_stats.skip_indirect_index_access;
        stats.vector_skip_store_effects += v_stats.skip_store_effects;
        stats.vector_skip_no_supported_pattern += v_stats.skip_no_supported_pattern;
        stats.vector_candidate_total += v_stats.candidate_total;
        stats.vector_candidate_reductions += v_stats.candidate_reductions;
        stats.vector_candidate_conditionals += v_stats.candidate_conditionals;
        stats.vector_candidate_recurrences += v_stats.candidate_recurrences;
        stats.vector_candidate_shifted += v_stats.candidate_shifted;
        stats.vector_candidate_call_maps += v_stats.candidate_call_maps;
        stats.vector_candidate_expr_maps += v_stats.candidate_expr_maps;
        stats.vector_candidate_scatters += v_stats.candidate_scatters;
        stats.vector_candidate_cube_slices += v_stats.candidate_cube_slices;
        stats.vector_candidate_basic_maps += v_stats.candidate_basic_maps;
        stats.vector_candidate_multi_exprs += v_stats.candidate_multi_exprs;
        stats.vector_candidate_2d += v_stats.candidate_2d;
        stats.vector_candidate_3d += v_stats.candidate_3d;
        stats.vector_candidate_call_map_direct += v_stats.candidate_call_map_direct;
        stats.vector_candidate_call_map_runtime += v_stats.candidate_call_map_runtime;
        stats.vector_applied_total += v_stats.applied_total;
        stats.vector_applied_reductions += v_stats.applied_reductions;
        stats.vector_applied_conditionals += v_stats.applied_conditionals;
        stats.vector_applied_recurrences += v_stats.applied_recurrences;
        stats.vector_applied_shifted += v_stats.applied_shifted;
        stats.vector_applied_call_maps += v_stats.applied_call_maps;
        stats.vector_applied_expr_maps += v_stats.applied_expr_maps;
        stats.vector_applied_scatters += v_stats.applied_scatters;
        stats.vector_applied_cube_slices += v_stats.applied_cube_slices;
        stats.vector_applied_basic_maps += v_stats.applied_basic_maps;
        stats.vector_applied_multi_exprs += v_stats.applied_multi_exprs;
        stats.vector_applied_2d += v_stats.applied_2d;
        stats.vector_applied_3d += v_stats.applied_3d;
        stats.vector_applied_call_map_direct += v_stats.applied_call_map_direct;
        stats.vector_applied_call_map_runtime += v_stats.applied_call_map_runtime;
        stats.vector_legacy_poly_fallback_candidate_total +=
            v_stats.legacy_poly_fallback_candidate_total;
        stats.vector_legacy_poly_fallback_candidate_reductions +=
            v_stats.legacy_poly_fallback_candidate_reductions;
        stats.vector_legacy_poly_fallback_candidate_maps +=
            v_stats.legacy_poly_fallback_candidate_maps;
        stats.vector_legacy_poly_fallback_applied_total +=
            v_stats.legacy_poly_fallback_applied_total;
        stats.vector_legacy_poly_fallback_applied_reductions +=
            v_stats.legacy_poly_fallback_applied_reductions;
        stats.vector_legacy_poly_fallback_applied_maps += v_stats.legacy_poly_fallback_applied_maps;
        stats.vector_trip_tier_tiny += v_stats.trip_tier_tiny;
        stats.vector_trip_tier_small += v_stats.trip_tier_small;
        stats.vector_trip_tier_medium += v_stats.trip_tier_medium;
        stats.vector_trip_tier_large += v_stats.trip_tier_large;
        stats.proof_certified += v_stats.proof_certified;
        stats.proof_applied += v_stats.proof_applied;
        stats.proof_apply_failed += v_stats.proof_apply_failed;
        stats.proof_fallback_pattern += v_stats.proof_fallback_pattern;
        for (dst, src) in stats
            .proof_fallback_reason_counts
            .iter_mut()
            .zip(v_stats.proof_fallback_reason_counts)
        {
            *dst += src;
        }
    }
}
