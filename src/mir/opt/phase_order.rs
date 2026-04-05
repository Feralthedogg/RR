use super::types::{
    FunctionPhaseFeatures, FunctionPhasePlan, PhaseOrderingMode, PhaseProfileKind, PhaseScheduleId,
};
use super::*;
use crate::mir::analyze::effects;

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct HeavyPhaseIterationResult {
    pub(super) changed: bool,
    pub(super) non_structural_changes: usize,
    pub(super) structural_progress: bool,
    pub(super) ran_structural: bool,
    pub(super) skipped_structural: bool,
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

    fn phase_feature_helper_is_functionally_pure(callee: &str) -> bool {
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
        )
    }

    pub(super) fn extract_function_phase_features(fn_ir: &FnIR) -> FunctionPhaseFeatures {
        let loops = loop_analysis::LoopAnalyzer::new(fn_ir).find_loops();
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

    fn compute_phase_profile_scores(features: &FunctionPhaseFeatures) -> (usize, usize) {
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

    pub(super) fn classify_phase_profile(features: &FunctionPhaseFeatures) -> PhaseProfileKind {
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

    pub(super) fn choose_phase_schedule(
        mode: PhaseOrderingMode,
        profile: PhaseProfileKind,
    ) -> PhaseScheduleId {
        match mode {
            PhaseOrderingMode::Off | PhaseOrderingMode::Balanced => PhaseScheduleId::Balanced,
            PhaseOrderingMode::Auto => match profile {
                PhaseProfileKind::Balanced => PhaseScheduleId::Balanced,
                PhaseProfileKind::ComputeHeavy => PhaseScheduleId::ComputeHeavy,
                PhaseProfileKind::ControlFlowHeavy => PhaseScheduleId::ControlFlowHeavy,
            },
        }
    }

    fn emit_phase_plan_trace(plan: &FunctionPhasePlan) {
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

    fn phase_branch_density_high(features: &FunctionPhaseFeatures) -> bool {
        features.branch_terms.saturating_mul(3) >= features.block_count.max(1)
    }

    fn control_flow_structural_gate(features: &FunctionPhaseFeatures) -> bool {
        let branch_density_high = Self::phase_branch_density_high(features);
        let side_effects_dominant =
            features.side_effecting_calls.saturating_mul(2) > features.call_values.max(1);
        features.canonical_loop_count > 0 && !branch_density_high && !side_effects_dominant
    }

    pub(super) fn control_flow_should_fallback_to_balanced(
        result: HeavyPhaseIterationResult,
    ) -> bool {
        !result.structural_progress && result.non_structural_changes <= 1
    }

    fn build_function_phase_plan_from_features(
        function: &str,
        mode: PhaseOrderingMode,
        trace_requested: bool,
        features: FunctionPhaseFeatures,
    ) -> FunctionPhasePlan {
        let profile = if matches!(mode, PhaseOrderingMode::Auto) {
            Self::classify_phase_profile(&features)
        } else {
            PhaseProfileKind::Balanced
        };
        let schedule = Self::choose_phase_schedule(mode, profile);
        FunctionPhasePlan {
            function: function.to_string(),
            mode,
            profile,
            schedule,
            features: Some(features),
            trace_requested,
        }
    }

    pub(super) fn build_function_phase_plan(
        &self,
        function: &str,
        fn_ir: &FnIR,
    ) -> FunctionPhasePlan {
        let mode = self.resolved_phase_ordering_mode();
        let trace_requested = Self::phase_ordering_trace_enabled();
        let features = Self::extract_function_phase_features(fn_ir);
        Self::build_function_phase_plan_from_features(function, mode, trace_requested, features)
    }

    pub(super) fn collect_function_phase_plans(
        &self,
        all_fns: &FxHashMap<String, FnIR>,
        ordered_names: &[String],
        selected_functions: Option<&FxHashSet<String>>,
    ) -> FxHashMap<String, FunctionPhasePlan> {
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

    pub(super) fn run_heavy_phase_schedule_iteration(
        &self,
        schedule: PhaseScheduleId,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        loop_opt: &loop_opt::MirLoopOptimizer,
        stats: &mut TachyonPulseStats,
        run_budgeted_passes: bool,
    ) -> HeavyPhaseIterationResult {
        match schedule {
            PhaseScheduleId::Balanced => self.run_balanced_heavy_phase_iteration(
                fn_ir,
                callmap_user_whitelist,
                loop_opt,
                stats,
                run_budgeted_passes,
            ),
            PhaseScheduleId::ComputeHeavy => self.run_compute_heavy_phase_iteration(
                fn_ir,
                callmap_user_whitelist,
                loop_opt,
                stats,
                run_budgeted_passes,
            ),
            PhaseScheduleId::ControlFlowHeavy => self.run_control_flow_heavy_phase_iteration(
                fn_ir,
                callmap_user_whitelist,
                loop_opt,
                stats,
                run_budgeted_passes,
            ),
        }
    }

    fn run_compute_heavy_phase_iteration(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        loop_opt: &loop_opt::MirLoopOptimizer,
        stats: &mut TachyonPulseStats,
        run_budgeted_passes: bool,
    ) -> HeavyPhaseIterationResult {
        let mut result = HeavyPhaseIterationResult::default();

        let sc_changed = self.simplify_cfg(fn_ir);
        if sc_changed {
            stats.simplify_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After SimplifyCFG");
        Self::debug_stage_dump(fn_ir, "After SimplifyCFG");
        if sc_changed {
            result.changed = true;
            result.non_structural_changes += 1;
        }

        let sccp_changed = sccp::MirSCCP::new().optimize(fn_ir);
        if sccp_changed {
            stats.sccp_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After SCCP");
        Self::debug_stage_dump(fn_ir, "After SCCP");
        if sccp_changed {
            result.changed = true;
            result.non_structural_changes += 1;
        }

        let intr_changed = intrinsics::optimize(fn_ir);
        if intr_changed {
            stats.intrinsics_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After Intrinsics");
        Self::debug_stage_dump(fn_ir, "After Intrinsics");
        if intr_changed {
            result.changed = true;
            result.non_structural_changes += 1;
        }

        let gvn_changed = if Self::gvn_enabled() {
            let changed = gvn::optimize(fn_ir);
            if changed {
                stats.gvn_hits += 1;
            }
            changed
        } else {
            false
        };
        Self::maybe_verify(fn_ir, "After GVN");
        Self::debug_stage_dump(fn_ir, "After GVN");
        if gvn_changed {
            result.changed = true;
            result.non_structural_changes += 1;
        }

        let simplify_changed = simplify::optimize(fn_ir);
        if simplify_changed {
            stats.simplify_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After Simplify");
        Self::debug_stage_dump(fn_ir, "After Simplify");
        if simplify_changed {
            result.changed = true;
            result.non_structural_changes += 1;
        }

        let dce_changed = self.dce(fn_ir);
        if dce_changed {
            stats.dce_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After DCE");
        Self::debug_stage_dump(fn_ir, "After DCE");
        if dce_changed {
            result.changed = true;
            result.non_structural_changes += 1;
        }

        if run_budgeted_passes {
            let loop_changed_count = loop_opt.optimize_with_count(fn_ir);
            stats.simplified_loops += loop_changed_count;
            let loop_changed = loop_changed_count > 0;
            Self::maybe_verify(fn_ir, "After LoopOpt");
            Self::debug_stage_dump(fn_ir, "After LoopOpt");
            if loop_changed {
                result.changed = true;
                result.non_structural_changes += 1;
            }

            let licm_changed = if Self::licm_enabled() && Self::licm_allowed_for_fn(fn_ir) {
                let changed = licm::MirLicm::new().optimize(fn_ir);
                if changed {
                    stats.licm_hits += 1;
                }
                changed
            } else {
                false
            };
            Self::maybe_verify(fn_ir, "After LICM");
            Self::debug_stage_dump(fn_ir, "After LICM");
            if licm_changed {
                result.changed = true;
                result.non_structural_changes += 1;
            }

            let pass_changed =
                self.run_balanced_structural_cluster(fn_ir, callmap_user_whitelist, stats);
            if pass_changed {
                result.changed = true;
                result.structural_progress = true;
                result.ran_structural = true;
                if self.run_balanced_structural_cleanup(fn_ir, stats) {
                    result.changed = true;
                    result.structural_progress = true;
                }
            } else {
                result.ran_structural = true;
            }

            let fresh_changed = fresh_alloc::optimize(fn_ir);
            if fresh_changed {
                stats.fresh_alloc_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After FreshAlloc");
            Self::debug_stage_dump(fn_ir, "After FreshAlloc");
            if fresh_changed {
                result.changed = true;
                result.non_structural_changes += 1;
            }

            let bce_changed = bce::optimize(fn_ir);
            if bce_changed {
                stats.bce_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After BCE");
            Self::debug_stage_dump(fn_ir, "After BCE");
            if bce_changed {
                result.changed = true;
                result.non_structural_changes += 1;
            }
        }

        result
    }

    fn run_balanced_heavy_phase_iteration(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        loop_opt: &loop_opt::MirLoopOptimizer,
        stats: &mut TachyonPulseStats,
        run_budgeted_passes: bool,
    ) -> HeavyPhaseIterationResult {
        let mut result = HeavyPhaseIterationResult::default();
        let pass_changed = if run_budgeted_passes {
            self.run_balanced_structural_cluster(fn_ir, callmap_user_whitelist, stats)
        } else {
            false
        };

        if pass_changed {
            result.changed = true;
            result.structural_progress = true;
            result.ran_structural = true;
            if self.run_balanced_structural_cleanup(fn_ir, stats) {
                result.changed = true;
                result.structural_progress = true;
            }
        }

        if run_budgeted_passes {
            result.ran_structural = true;
        }

        let standard_changed =
            self.run_balanced_standard_cluster(fn_ir, loop_opt, stats, run_budgeted_passes);
        if standard_changed {
            result.changed = true;
            result.non_structural_changes += 1;
        }
        result
    }

    fn run_control_flow_heavy_phase_iteration(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        loop_opt: &loop_opt::MirLoopOptimizer,
        stats: &mut TachyonPulseStats,
        run_budgeted_passes: bool,
    ) -> HeavyPhaseIterationResult {
        let mut result = HeavyPhaseIterationResult::default();

        let sc_changed = self.simplify_cfg(fn_ir);
        if sc_changed {
            stats.simplify_hits += 1;
            result.changed = true;
            result.non_structural_changes += 1;
        }
        Self::maybe_verify(fn_ir, "After SimplifyCFG");
        Self::debug_stage_dump(fn_ir, "After SimplifyCFG");

        let sccp_changed = sccp::MirSCCP::new().optimize(fn_ir);
        if sccp_changed {
            stats.sccp_hits += 1;
            result.changed = true;
            result.non_structural_changes += 1;
        }
        Self::maybe_verify(fn_ir, "After SCCP");
        Self::debug_stage_dump(fn_ir, "After SCCP");

        let intr_changed = intrinsics::optimize(fn_ir);
        if intr_changed {
            stats.intrinsics_hits += 1;
            result.changed = true;
            result.non_structural_changes += 1;
        }
        Self::maybe_verify(fn_ir, "After Intrinsics");
        Self::debug_stage_dump(fn_ir, "After Intrinsics");

        let type_spec_changed = type_specialize::optimize(fn_ir);
        if type_spec_changed {
            result.changed = true;
            result.non_structural_changes += 1;
        }
        Self::maybe_verify(fn_ir, "After TypeSpecialize");
        Self::debug_stage_dump(fn_ir, "After TypeSpecialize");

        let simplify_changed = simplify::optimize(fn_ir);
        if simplify_changed {
            stats.simplify_hits += 1;
            result.changed = true;
            result.non_structural_changes += 1;
        }
        Self::maybe_verify(fn_ir, "After Simplify");
        Self::debug_stage_dump(fn_ir, "After Simplify");

        let dce_changed = self.dce(fn_ir);
        if dce_changed {
            stats.dce_hits += 1;
            result.changed = true;
            result.non_structural_changes += 1;
        }
        Self::maybe_verify(fn_ir, "After DCE");
        Self::debug_stage_dump(fn_ir, "After DCE");

        let tco_changed = tco::optimize(fn_ir);
        if tco_changed {
            stats.tco_hits += 1;
            result.changed = true;
            result.non_structural_changes += 1;
        }
        Self::maybe_verify(fn_ir, "After TCO");
        Self::debug_stage_dump(fn_ir, "After TCO");

        let gvn_changed = if Self::gvn_enabled() {
            let changed = gvn::optimize(fn_ir);
            if changed {
                stats.gvn_hits += 1;
                result.changed = true;
                result.non_structural_changes += 1;
            }
            changed
        } else {
            false
        };
        Self::maybe_verify(fn_ir, "After GVN");
        Self::debug_stage_dump(fn_ir, "After GVN");
        let _ = gvn_changed;

        if run_budgeted_passes {
            let loop_changed_count = loop_opt.optimize_with_count(fn_ir);
            if loop_changed_count > 0 {
                stats.simplified_loops += loop_changed_count;
                result.changed = true;
                result.non_structural_changes += 1;
            }
            Self::maybe_verify(fn_ir, "After LoopOpt");
            Self::debug_stage_dump(fn_ir, "After LoopOpt");

            let post_loop_features = Self::extract_function_phase_features(fn_ir);
            let licm_changed = if post_loop_features.canonical_loop_count > 0
                && Self::licm_enabled()
                && Self::licm_allowed_for_fn(fn_ir)
            {
                let changed = licm::MirLicm::new().optimize(fn_ir);
                if changed {
                    stats.licm_hits += 1;
                    result.changed = true;
                    result.non_structural_changes += 1;
                }
                changed
            } else {
                false
            };
            Self::maybe_verify(fn_ir, "After LICM");
            Self::debug_stage_dump(fn_ir, "After LICM");
            let _ = licm_changed;

            let structural_features = Self::extract_function_phase_features(fn_ir);
            if Self::control_flow_structural_gate(&structural_features) {
                result.ran_structural = true;
                let poly_changed =
                    self.run_control_flow_structural_cluster(fn_ir, callmap_user_whitelist, stats);
                if poly_changed {
                    result.changed = true;
                    result.structural_progress = true;
                    if self.run_balanced_structural_cleanup(fn_ir, stats) {
                        result.changed = true;
                        result.structural_progress = true;
                    }
                }
            } else {
                result.skipped_structural = true;
            }

            let fresh_changed = fresh_alloc::optimize(fn_ir);
            if fresh_changed {
                stats.fresh_alloc_hits += 1;
                result.changed = true;
                result.non_structural_changes += 1;
            }
            Self::maybe_verify(fn_ir, "After FreshAlloc");
            Self::debug_stage_dump(fn_ir, "After FreshAlloc");

            let bce_changed = bce::optimize(fn_ir);
            if bce_changed {
                stats.bce_hits += 1;
                result.changed = true;
                result.non_structural_changes += 1;
            }
            Self::maybe_verify(fn_ir, "After BCE");
            Self::debug_stage_dump(fn_ir, "After BCE");
        }

        result
    }

    fn run_balanced_structural_cluster(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        stats: &mut TachyonPulseStats,
    ) -> bool {
        let mut changed = false;

        let type_spec_changed = type_specialize::optimize(fn_ir);
        Self::maybe_verify(fn_ir, "After TypeSpecialize");
        Self::debug_stage_dump(fn_ir, "After TypeSpecialize");
        changed |= type_spec_changed;

        let p_stats = poly::optimize_with_stats(fn_ir);
        Self::accumulate_poly_stats(stats, p_stats);
        changed |= p_stats.schedule_applied > 0;

        let v_stats = v_opt::optimize_with_stats_with_whitelist(fn_ir, callmap_user_whitelist);
        let v_changed = v_stats.changed();
        Self::accumulate_vector_stats(stats, v_stats);
        Self::maybe_verify(fn_ir, "After Vectorization");
        Self::debug_stage_dump(fn_ir, "After Vectorization");
        changed |= v_changed;

        let type_spec_post_vec = type_specialize::optimize(fn_ir);
        Self::maybe_verify(fn_ir, "After TypeSpecialize(PostVec)");
        Self::debug_stage_dump(fn_ir, "After TypeSpecialize(PostVec)");
        changed |= type_spec_post_vec;

        let tco_changed = tco::optimize(fn_ir);
        if tco_changed {
            stats.tco_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After TCO");
        Self::debug_stage_dump(fn_ir, "After TCO");
        changed |= tco_changed;

        changed
    }

    fn run_control_flow_structural_cluster(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        stats: &mut TachyonPulseStats,
    ) -> bool {
        let mut changed = false;

        let p_stats = poly::optimize_with_stats(fn_ir);
        Self::accumulate_poly_stats(stats, p_stats);
        changed |= p_stats.schedule_applied > 0;

        let v_stats = v_opt::optimize_with_stats_with_whitelist(fn_ir, callmap_user_whitelist);
        let v_changed = v_stats.changed();
        Self::accumulate_vector_stats(stats, v_stats);
        Self::maybe_verify(fn_ir, "After Vectorization");
        Self::debug_stage_dump(fn_ir, "After Vectorization");
        changed |= v_changed;

        let type_spec_post_vec = type_specialize::optimize(fn_ir);
        Self::maybe_verify(fn_ir, "After TypeSpecialize(PostVec)");
        Self::debug_stage_dump(fn_ir, "After TypeSpecialize(PostVec)");
        changed |= type_spec_post_vec;

        changed
    }

    fn run_balanced_structural_cleanup(
        &self,
        fn_ir: &mut FnIR,
        stats: &mut TachyonPulseStats,
    ) -> bool {
        let mut changed = false;

        let sc_changed = self.simplify_cfg(fn_ir);
        if sc_changed {
            stats.simplify_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After Structural SimplifyCFG");
        Self::debug_stage_dump(fn_ir, "After Structural SimplifyCFG");
        changed |= sc_changed;

        let dce_changed = self.dce(fn_ir);
        if dce_changed {
            stats.dce_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After Structural DCE");
        Self::debug_stage_dump(fn_ir, "After Structural DCE");
        changed |= dce_changed;

        changed
    }

    fn run_balanced_standard_cluster(
        &self,
        fn_ir: &mut FnIR,
        loop_opt: &loop_opt::MirLoopOptimizer,
        stats: &mut TachyonPulseStats,
        run_budgeted_passes: bool,
    ) -> bool {
        let mut changed = false;

        let sc_changed = self.simplify_cfg(fn_ir);
        if sc_changed {
            stats.simplify_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After SimplifyCFG");
        Self::debug_stage_dump(fn_ir, "After SimplifyCFG");
        changed |= sc_changed;

        let sccp_changed = sccp::MirSCCP::new().optimize(fn_ir);
        if sccp_changed {
            stats.sccp_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After SCCP");
        Self::debug_stage_dump(fn_ir, "After SCCP");
        changed |= sccp_changed;

        let intr_changed = intrinsics::optimize(fn_ir);
        if intr_changed {
            stats.intrinsics_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After Intrinsics");
        Self::debug_stage_dump(fn_ir, "After Intrinsics");
        changed |= intr_changed;

        let gvn_changed = if Self::gvn_enabled() {
            let changed = gvn::optimize(fn_ir);
            if changed {
                stats.gvn_hits += 1;
            }
            changed
        } else {
            false
        };
        Self::maybe_verify(fn_ir, "After GVN");
        Self::debug_stage_dump(fn_ir, "After GVN");
        changed |= gvn_changed;

        let simplify_changed = simplify::optimize(fn_ir);
        if simplify_changed {
            stats.simplify_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After Simplify");
        Self::debug_stage_dump(fn_ir, "After Simplify");
        changed |= simplify_changed;

        let dce_changed = self.dce(fn_ir);
        if dce_changed {
            stats.dce_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After DCE");
        Self::debug_stage_dump(fn_ir, "After DCE");
        changed |= dce_changed;

        if run_budgeted_passes {
            let loop_changed_count = loop_opt.optimize_with_count(fn_ir);
            stats.simplified_loops += loop_changed_count;
            let loop_changed = loop_changed_count > 0;
            Self::maybe_verify(fn_ir, "After LoopOpt");
            Self::debug_stage_dump(fn_ir, "After LoopOpt");
            changed |= loop_changed;

            let licm_changed = if Self::licm_enabled() && Self::licm_allowed_for_fn(fn_ir) {
                let changed = licm::MirLicm::new().optimize(fn_ir);
                if changed {
                    stats.licm_hits += 1;
                }
                changed
            } else {
                false
            };
            Self::maybe_verify(fn_ir, "After LICM");
            Self::debug_stage_dump(fn_ir, "After LICM");
            changed |= licm_changed;

            let fresh_changed = fresh_alloc::optimize(fn_ir);
            if fresh_changed {
                stats.fresh_alloc_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After FreshAlloc");
            Self::debug_stage_dump(fn_ir, "After FreshAlloc");
            changed |= fresh_changed;

            let bce_changed = bce::optimize(fn_ir);
            if bce_changed {
                stats.bce_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After BCE");
            Self::debug_stage_dump(fn_ir, "After BCE");
            changed |= bce_changed;
        }

        changed
    }

    fn accumulate_poly_stats(stats: &mut TachyonPulseStats, p_stats: poly::PolyStats) {
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

    fn accumulate_vector_stats(stats: &mut TachyonPulseStats, v_stats: v_opt::VOptStats) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Span;

    fn sample_feature_fn() -> FnIR {
        let mut fn_ir = FnIR::new("phase_features".to_string(), vec!["x".to_string()]);
        let entry = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let param = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let zero = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let _phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(zero, entry), (one, then_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let binary = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: zero,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let _unary = fn_ir.add_value(
            ValueKind::Unary {
                op: UnaryOp::Neg,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let _intrinsic = fn_ir.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecAbsF64,
                args: vec![param],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let pure_call = fn_ir.add_value(
            ValueKind::Call {
                callee: "abs".to_string(),
                args: vec![one],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let impure_call = fn_ir.add_value(
            ValueKind::Call {
                callee: "print".to_string(),
                args: vec![one],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let _index = fn_ir.add_value(
            ValueKind::Index1D {
                base: param,
                idx: one,
                is_safe: false,
                is_na_safe: false,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Lt,
                lhs: zero,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        fn_ir.blocks[entry].instrs.push(Instr::StoreIndex1D {
            base: param,
            idx: one,
            val: impure_call,
            is_safe: false,
            is_na_safe: false,
            is_vector: false,
            span: Span::default(),
        });
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].term = Terminator::Return(Some(binary));
        fn_ir.blocks[else_bb].term = Terminator::Return(Some(pure_call));
        fn_ir
    }

    #[test]
    fn extract_function_phase_features_counts_basic_shapes() {
        let fn_ir = sample_feature_fn();
        let features = TachyonEngine::extract_function_phase_features(&fn_ir);
        assert_eq!(features.ir_size, TachyonEngine::fn_ir_size(&fn_ir));
        assert_eq!(features.block_count, 3);
        assert_eq!(features.loop_count, 0);
        assert_eq!(features.canonical_loop_count, 0);
        assert_eq!(features.branch_terms, 1);
        assert_eq!(features.phi_count, 1);
        assert_eq!(features.arithmetic_values, 3);
        assert_eq!(features.intrinsic_values, 1);
        assert_eq!(features.call_values, 2);
        assert_eq!(features.side_effecting_calls, 1);
        assert_eq!(features.index_values, 1);
        assert_eq!(features.store_instrs, 1);
    }

    #[test]
    fn collect_function_phase_plans_only_keeps_selected_safe_candidates() {
        let mut all_fns = FxHashMap::default();
        all_fns.insert("selected".to_string(), sample_feature_fn());

        let mut self_recursive = FnIR::new("self_recursive".to_string(), vec![]);
        let entry = self_recursive.add_block();
        self_recursive.entry = entry;
        self_recursive.body_head = entry;
        let call = self_recursive.add_value(
            ValueKind::Call {
                callee: "self_recursive".to_string(),
                args: Vec::new(),
                names: Vec::new(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        self_recursive.blocks[entry].term = Terminator::Return(Some(call));
        all_fns.insert("self_recursive".to_string(), self_recursive);

        let mut conservative = FnIR::new("conservative".to_string(), vec![]);
        let entry = conservative.add_block();
        conservative.entry = entry;
        conservative.body_head = entry;
        conservative.blocks[entry].term = Terminator::Return(None);
        conservative.mark_unsupported_dynamic("test".to_string());
        all_fns.insert("conservative".to_string(), conservative);

        let ordered = vec![
            "conservative".to_string(),
            "selected".to_string(),
            "self_recursive".to_string(),
        ];
        let selected = FxHashSet::from_iter(["selected".to_string(), "self_recursive".to_string()]);

        let plans =
            TachyonEngine::new().collect_function_phase_plans(&all_fns, &ordered, Some(&selected));
        assert_eq!(plans.len(), 1);
        let plan = plans
            .get("selected")
            .expect("selected function should be planned");
        assert_eq!(plan.function, "selected");
        assert_eq!(plan.profile, PhaseProfileKind::Balanced);
        assert_eq!(plan.schedule, PhaseScheduleId::Balanced);
        assert!(plan.features.is_some());
    }

    #[test]
    fn classify_phase_profile_marks_compute_heavy_features() {
        let features = FunctionPhaseFeatures {
            ir_size: 180,
            block_count: 8,
            loop_count: 3,
            canonical_loop_count: 2,
            branch_terms: 1,
            phi_count: 2,
            arithmetic_values: 24,
            intrinsic_values: 6,
            call_values: 2,
            side_effecting_calls: 0,
            index_values: 8,
            store_instrs: 4,
        };
        assert_eq!(
            TachyonEngine::classify_phase_profile(&features),
            PhaseProfileKind::ComputeHeavy
        );
    }

    #[test]
    fn classify_phase_profile_marks_control_flow_heavy_features() {
        let features = FunctionPhaseFeatures {
            ir_size: 120,
            block_count: 6,
            loop_count: 0,
            canonical_loop_count: 0,
            branch_terms: 4,
            phi_count: 5,
            arithmetic_values: 2,
            intrinsic_values: 0,
            call_values: 3,
            side_effecting_calls: 2,
            index_values: 0,
            store_instrs: 0,
        };
        assert_eq!(
            TachyonEngine::classify_phase_profile(&features),
            PhaseProfileKind::ControlFlowHeavy
        );
    }

    #[test]
    fn build_phase_plan_in_auto_mode_exposes_classified_schedule() {
        let features = FunctionPhaseFeatures {
            ir_size: 220,
            block_count: 9,
            loop_count: 2,
            canonical_loop_count: 2,
            branch_terms: 1,
            phi_count: 1,
            arithmetic_values: 18,
            intrinsic_values: 4,
            call_values: 1,
            side_effecting_calls: 0,
            index_values: 4,
            store_instrs: 2,
        };
        let plan = TachyonEngine::build_function_phase_plan_from_features(
            "auto_fn",
            PhaseOrderingMode::Auto,
            true,
            features,
        );
        assert_eq!(plan.profile, PhaseProfileKind::ComputeHeavy);
        assert_eq!(plan.schedule, PhaseScheduleId::ComputeHeavy);
        assert!(plan.trace_requested);
    }

    #[test]
    fn build_phase_plan_in_non_auto_modes_stays_balanced() {
        let features = FunctionPhaseFeatures {
            ir_size: 120,
            block_count: 4,
            loop_count: 2,
            canonical_loop_count: 2,
            branch_terms: 0,
            phi_count: 0,
            arithmetic_values: 8,
            intrinsic_values: 2,
            call_values: 0,
            side_effecting_calls: 0,
            index_values: 2,
            store_instrs: 1,
        };
        for mode in [PhaseOrderingMode::Off, PhaseOrderingMode::Balanced] {
            let plan = TachyonEngine::build_function_phase_plan_from_features(
                "non_auto", mode, false, features,
            );
            assert_eq!(plan.profile, PhaseProfileKind::Balanced);
            assert_eq!(plan.schedule, PhaseScheduleId::Balanced);
        }
    }

    #[test]
    fn control_flow_structural_gate_requires_canonical_low_branch_features() {
        let mut features = FunctionPhaseFeatures {
            ir_size: 160,
            block_count: 8,
            loop_count: 2,
            canonical_loop_count: 1,
            branch_terms: 1,
            phi_count: 3,
            arithmetic_values: 6,
            intrinsic_values: 0,
            call_values: 1,
            side_effecting_calls: 0,
            index_values: 2,
            store_instrs: 1,
        };
        assert!(TachyonEngine::control_flow_structural_gate(&features));
        features.branch_terms = 4;
        assert!(!TachyonEngine::control_flow_structural_gate(&features));
    }

    #[test]
    fn control_flow_fallback_triggers_only_on_low_progress() {
        assert!(TachyonEngine::control_flow_should_fallback_to_balanced(
            HeavyPhaseIterationResult {
                changed: false,
                non_structural_changes: 1,
                structural_progress: false,
                ran_structural: false,
                skipped_structural: true,
            }
        ));
        assert!(!TachyonEngine::control_flow_should_fallback_to_balanced(
            HeavyPhaseIterationResult {
                changed: true,
                non_structural_changes: 3,
                structural_progress: false,
                ran_structural: false,
                skipped_structural: true,
            }
        ));
        assert!(!TachyonEngine::control_flow_should_fallback_to_balanced(
            HeavyPhaseIterationResult {
                changed: true,
                non_structural_changes: 0,
                structural_progress: true,
                ran_structural: true,
                skipped_structural: false,
            }
        ));
    }
}
