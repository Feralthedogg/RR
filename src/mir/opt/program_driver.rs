use super::*;

#[derive(Clone, Copy)]
pub(crate) struct ProgramRunTiers {
    pub(crate) heavy: bool,
    pub(crate) full_inline: bool,
}

pub(crate) struct PreTierProgramHelpers {
    pub(crate) floor_helpers: FxHashSet<String>,
    pub(crate) proven_floor_param_slots: FxHashMap<String, FxHashSet<usize>>,
}

pub(crate) struct ProgramTierRun<'a, 'progress> {
    pub(crate) all_fns: &'a mut FxHashMap<String, FnIR>,
    pub(crate) scheduler: &'a CompilerScheduler,
    pub(crate) ordered_names: &'a [String],
    pub(crate) ordered_total: usize,
    pub(crate) stats: &'a mut TachyonPulseStats,
    pub(crate) pass_timings: &'a mut TachyonPassTimings,
    pub(crate) progress: &'a mut Option<&'progress mut dyn FnMut(TachyonProgress)>,
}

pub(crate) struct HeavyTierInputs<'a> {
    pub(crate) plan: &'a ProgramOptPlan,
    pub(crate) run_heavy_tier: bool,
    pub(crate) heavy_phase_plans: &'a FxHashMap<String, FunctionPhasePlan>,
    pub(crate) callmap_user_whitelist: &'a FxHashSet<String>,
    pub(crate) proven_floor_param_slots: &'a FxHashMap<String, FxHashSet<usize>>,
}

impl TachyonEngine {
    pub fn run_program(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        let _ = self.run_program_with_stats(all_fns);
    }

    pub fn run_program_with_stats(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
    ) -> TachyonPulseStats {
        let scheduler = CompilerScheduler::new(CompilerParallelConfig::default());
        self.run_program_with_scheduler(all_fns, &scheduler)
    }

    pub fn run_program_with_stats_progress(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        on_progress: &mut dyn FnMut(TachyonProgress),
    ) -> TachyonPulseStats {
        let scheduler = CompilerScheduler::new(CompilerParallelConfig::default());
        self.run_program_with_progress_and_scheduler(all_fns, &scheduler, on_progress)
    }

    pub fn run_program_with_stats_and_compiler_parallel(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        compiler_parallel_cfg: CompilerParallelConfig,
    ) -> TachyonPulseStats {
        let scheduler = CompilerScheduler::new(compiler_parallel_cfg);
        self.run_program_with_scheduler(all_fns, &scheduler)
    }

    pub fn run_program_with_stats_progress_and_compiler_parallel(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        compiler_parallel_cfg: CompilerParallelConfig,
        on_progress: &mut dyn FnMut(TachyonProgress),
    ) -> TachyonPulseStats {
        let scheduler = CompilerScheduler::new(compiler_parallel_cfg);
        self.run_program_with_progress_and_scheduler(all_fns, &scheduler, on_progress)
    }

    pub(crate) fn run_program_with_scheduler(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
    ) -> TachyonPulseStats {
        self.run_program_with_profile_and_scheduler(all_fns, scheduler)
            .pulse_stats
    }

    pub(crate) fn run_program_with_progress_and_scheduler(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        on_progress: &mut dyn FnMut(TachyonProgress),
    ) -> TachyonPulseStats {
        self.run_program_with_profile_and_progress_scheduler(all_fns, scheduler, on_progress)
            .pulse_stats
    }

    pub(crate) fn run_program_with_profile_and_scheduler(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
    ) -> TachyonRunProfile {
        // Proof correspondence:
        // `ProgramApiWrapperSoundness.run_program_with_profile_and_scheduler_*`
        // fixes the reduced shell theorem family for this public optimizer
        // entrypoint. The reduced model treats this as orchestration around
        // the already-composed `run_program_with_profile_inner` boundary.
        self.run_program_with_profile_inner(all_fns, scheduler, None)
    }

    pub(crate) fn run_program_with_profile_and_progress_scheduler(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        on_progress: &mut dyn FnMut(TachyonProgress),
    ) -> TachyonRunProfile {
        self.run_program_with_profile_inner(all_fns, scheduler, Some(on_progress))
    }

    pub(crate) fn emit_progress(
        progress: &mut Option<&mut dyn FnMut(TachyonProgress)>,
        tier: TachyonProgressTier,
        completed: usize,
        total: usize,
        function: &str,
    ) {
        if let Some(cb) = progress.as_deref_mut() {
            cb(TachyonProgress {
                tier,
                completed,
                total,
                function: function.to_string(),
            });
        }
    }

    pub(crate) fn timed_bool_pass<F>(
        pass_timings: &mut TachyonPassTimings,
        pass: &'static str,
        f: F,
    ) -> bool
    where
        F: FnOnce() -> bool,
    {
        let started = Instant::now();
        let changed = f();
        pass_timings.record(pass, started.elapsed().as_nanos(), changed);
        changed
    }

    pub(crate) fn timed_count_pass<F>(
        pass_timings: &mut TachyonPassTimings,
        pass: &'static str,
        f: F,
    ) -> usize
    where
        F: FnOnce() -> usize,
    {
        let started = Instant::now();
        let changed_count = f();
        pass_timings.record(pass, started.elapsed().as_nanos(), changed_count > 0);
        changed_count
    }

    pub(crate) fn take_functions_in_order(
        all_fns: &mut FxHashMap<String, FnIR>,
        ordered_names: &[String],
    ) -> Vec<(String, FnIR)> {
        let mut jobs = Vec::with_capacity(ordered_names.len());
        for name in ordered_names {
            if let Some(fn_ir) = all_fns.remove(name) {
                jobs.push((name.clone(), fn_ir));
            }
        }
        jobs
    }

    pub(crate) fn restore_functions(
        all_fns: &mut FxHashMap<String, FnIR>,
        jobs: Vec<(String, FnIR)>,
    ) {
        for (name, fn_ir) in jobs {
            all_fns.insert(name, fn_ir);
        }
    }

    pub(crate) fn fn_is_self_recursive(fn_ir: &FnIR) -> bool {
        fn_ir.values.iter().any(|value| {
            matches!(
                &value.kind,
                ValueKind::Call { callee, .. } if callee == &fn_ir.name
            )
        })
    }

    pub(crate) fn program_run_tiers(&self, plan: &ProgramOptPlan) -> ProgramRunTiers {
        let selective_enabled = Self::selective_budget_enabled();
        let heavy = !plan.selective_mode || selective_enabled;
        let inline_program_limit = if self.aggressive_opt_enabled() {
            self.configured_max_full_opt_ir().saturating_mul(2)
        } else {
            self.configured_max_full_opt_ir()
        };
        let full_inline = heavy
            && self.inline_tier_enabled()
            && ((!plan.selective_mode && plan.total_ir <= inline_program_limit)
                || (self.aggressive_opt_enabled()
                    && !plan.selected_functions.is_empty()
                    && plan.total_ir <= inline_program_limit));
        ProgramRunTiers { heavy, full_inline }
    }

    pub(crate) fn initialize_program_stats(
        &self,
        stats: &mut TachyonPulseStats,
        plan: &ProgramOptPlan,
        tiers: ProgramRunTiers,
    ) {
        stats.total_program_ir = plan.total_ir;
        stats.max_function_ir = plan.max_fn_ir;
        stats.full_opt_ir_limit = plan.program_limit;
        stats.full_opt_fn_limit = plan.fn_limit;
        stats.selective_budget_mode = plan.selective_mode && tiers.heavy;
    }

    pub(crate) fn trace_helper_rewrites(label: &str, rewrites: usize, helper_names: String) {
        if Self::wrap_trace_enabled() && rewrites > 0 {
            eprintln!(
                "   [{}] rewrote {} call site(s) using helper(s): {}",
                label, rewrites, helper_names
            );
        }
    }

    pub(crate) fn rewrite_pre_tier_program_helpers(
        all_fns: &mut FxHashMap<String, FnIR>,
    ) -> PreTierProgramHelpers {
        let floor_helpers = Self::collect_floor_helpers(all_fns);
        let proven_floor_param_slots =
            Self::collect_proven_floor_index_param_slots(all_fns, &floor_helpers);
        if !floor_helpers.is_empty() {
            let rewrites = Self::rewrite_floor_helper_calls(all_fns, &floor_helpers);
            let helper_names = Self::sorted_names(&floor_helpers).join(", ");
            Self::trace_helper_rewrites("floor", rewrites, helper_names);
        }

        let abs_helpers = Self::collect_trivial_abs_helpers(all_fns);
        if !abs_helpers.is_empty() {
            let rewrites = Self::rewrite_trivial_abs_helper_calls(all_fns, &abs_helpers);
            let helper_names = Self::sorted_names(&abs_helpers).join(", ");
            Self::trace_helper_rewrites("abs", rewrites, helper_names);
        }

        let unit_index_helpers = Self::collect_unit_index_helpers(all_fns);
        if !unit_index_helpers.is_empty() {
            let rewrites = Self::rewrite_unit_index_helper_calls(all_fns, &unit_index_helpers);
            let helper_names = Self::sorted_names(&unit_index_helpers).join(", ");
            Self::trace_helper_rewrites("unit-index", rewrites, helper_names);
        }

        let minmax_helpers = Self::collect_trivial_minmax_helpers(all_fns);
        if !minmax_helpers.is_empty() {
            let rewrites = Self::rewrite_trivial_minmax_helper_calls(all_fns, &minmax_helpers);
            let helper_names: FxHashSet<String> = minmax_helpers.keys().cloned().collect();
            let helper_names = Self::sorted_names(&helper_names).join(", ");
            Self::trace_helper_rewrites("minmax", rewrites, helper_names);
        }

        let clamp_helpers = Self::collect_trivial_clamp_helpers(all_fns);
        if !clamp_helpers.is_empty() {
            let rewrites = Self::rewrite_trivial_clamp_helper_calls(all_fns, &clamp_helpers);
            let helper_names = Self::sorted_names(&clamp_helpers).join(", ");
            Self::trace_helper_rewrites("clamp", rewrites, helper_names);
        }

        PreTierProgramHelpers {
            floor_helpers,
            proven_floor_param_slots,
        }
    }

    pub(crate) fn run_always_program_tier(
        &self,
        run: ProgramTierRun<'_, '_>,
        helpers: &PreTierProgramHelpers,
    ) {
        let tier_a_total_ir: usize = run
            .ordered_names
            .iter()
            .filter_map(|name| run.all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let tier_a_jobs = Self::take_functions_in_order(run.all_fns, run.ordered_names);
        let tier_a_results = run.scheduler.map_stage(
            CompilerParallelStage::TachyonAlways,
            tier_a_jobs,
            tier_a_total_ir,
            |(name, mut fn_ir)| {
                let local_profile = self.run_always_tier_with_profile(
                    &mut fn_ir,
                    helpers.proven_floor_param_slots.get(&name),
                    &helpers.floor_helpers,
                );
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_tier_a = Vec::with_capacity(tier_a_results.len());
        for (idx, (name, fn_ir, local_profile)) in tier_a_results.into_iter().enumerate() {
            run.stats.accumulate(local_profile.pulse_stats);
            run.pass_timings.accumulate(local_profile.pass_timings);
            Self::emit_progress(
                run.progress,
                TachyonProgressTier::Always,
                idx + 1,
                run.ordered_total,
                &name,
            );
            restored_tier_a.push((name, fn_ir));
        }
        Self::restore_functions(run.all_fns, restored_tier_a);
    }

    pub(crate) fn run_record_specialization_stage(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
    ) {
        let mut record_specialization_ctx = chronos::ChronosProgramContext {
            stats,
            timings: pass_timings,
            inliner: None,
            hot_filter: None,
            valid_analyses: chronos::ChronosAnalysisSet::ALL,
            fuel: OptimizationFuel::disabled(),
        };
        chronos::ChronosProgramPassManager::new(
            chronos::ChronosStage::ProgramRecordSpecialization,
            chronos::PROGRAM_RECORD_SPECIALIZATION_PASSES,
        )
        .run_sequence(self, all_fns, &mut record_specialization_ctx);
    }

    pub(crate) fn run_outlining_program_stage(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
    ) {
        let total_ir = all_fns.values().map(Self::fn_ir_size).sum();
        let mut outline_ctx = chronos::ChronosProgramContext {
            stats,
            timings: pass_timings,
            inliner: None,
            hot_filter: None,
            valid_analyses: chronos::ChronosAnalysisSet::ALL,
            fuel: self.fuel_for_function(total_ir),
        };
        chronos::ChronosProgramPassManager::new(
            chronos::ChronosStage::ProgramOutlining,
            chronos::PROGRAM_OUTLINE_PASSES,
        )
        .run_sequence(self, all_fns, &mut outline_ctx);
    }

    pub(crate) fn collect_program_heavy_phase_plans(
        &self,
        all_fns: &FxHashMap<String, FnIR>,
        ordered_names: &[String],
        plan: &ProgramOptPlan,
        run_heavy_tier: bool,
    ) -> FxHashMap<String, FunctionPhasePlan> {
        if !run_heavy_tier {
            return FxHashMap::default();
        }
        let selected_functions = if plan.selective_mode {
            Some(&plan.selected_functions)
        } else {
            None
        };
        // Proof correspondence:
        // `ProgramPhasePipelineSoundness` fixes the reduced program-level
        // composition boundary here: `ProgramOptPlan -> selected_functions ->
        // collect_function_phase_plans -> plan_summary`.
        self.collect_function_phase_plans(all_fns, ordered_names, selected_functions)
    }

    pub(crate) fn rewrite_heavy_index_helpers(
        all_fns: &mut FxHashMap<String, FnIR>,
        run_heavy_tier: bool,
    ) {
        let wrap_index_helpers = if run_heavy_tier {
            Self::collect_wrap_index_helpers(all_fns)
        } else {
            FxHashSet::default()
        };
        if !wrap_index_helpers.is_empty() {
            let rewrites = Self::rewrite_wrap_index_helper_calls(all_fns, &wrap_index_helpers);
            let helper_names = Self::sorted_names(&wrap_index_helpers).join(", ");
            Self::trace_helper_rewrites("wrap", rewrites, helper_names);
        }

        let periodic_index_helpers = if run_heavy_tier {
            Self::collect_periodic_index_helpers(all_fns)
        } else {
            FxHashMap::default()
        };
        if !periodic_index_helpers.is_empty() {
            let rewrites =
                Self::rewrite_periodic_index_helper_calls(all_fns, &periodic_index_helpers);
            let helper_names: FxHashSet<String> = periodic_index_helpers.keys().cloned().collect();
            let helper_names = Self::sorted_names(&helper_names).join(", ");
            Self::trace_helper_rewrites("wrap1d", rewrites, helper_names);
        }

        let cube_index_helpers = if run_heavy_tier {
            Self::collect_cube_index_helpers(all_fns)
        } else {
            FxHashSet::default()
        };
        if !cube_index_helpers.is_empty() {
            let rewrites = Self::rewrite_cube_index_helper_calls(all_fns, &cube_index_helpers);
            let helper_names = Self::sorted_names(&cube_index_helpers).join(", ");
            Self::trace_helper_rewrites("cube", rewrites, helper_names);
        }
    }

    pub(crate) fn collect_heavy_callmap_whitelist(
        all_fns: &FxHashMap<String, FnIR>,
        plan: &ProgramOptPlan,
        run_heavy_tier: bool,
    ) -> FxHashSet<String> {
        let heavy_targets_exist =
            run_heavy_tier && (!plan.selective_mode || !plan.selected_functions.is_empty());
        if heavy_targets_exist {
            Self::collect_callmap_user_whitelist(all_fns)
        } else {
            FxHashSet::default()
        }
    }

    pub(crate) fn run_heavy_program_tier(
        &self,
        run: ProgramTierRun<'_, '_>,
        inputs: HeavyTierInputs<'_>,
    ) {
        let tier_b_total_ir: usize = run
            .ordered_names
            .iter()
            .filter_map(|name| run.all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let tier_b_jobs = Self::take_functions_in_order(run.all_fns, run.ordered_names);
        let tier_b_results = run.scheduler.map_stage(
            CompilerParallelStage::TachyonHeavy,
            tier_b_jobs,
            tier_b_total_ir,
            |(name, fn_ir)| self.run_heavy_function_job(name, fn_ir, &inputs),
        );
        let mut restored_tier_b = Vec::with_capacity(tier_b_results.len());
        for (idx, (name, fn_ir, local_profile)) in tier_b_results.into_iter().enumerate() {
            run.stats.accumulate(local_profile.pulse_stats);
            run.pass_timings.accumulate(local_profile.pass_timings);
            Self::emit_progress(
                run.progress,
                TachyonProgressTier::Heavy,
                idx + 1,
                run.ordered_total,
                &name,
            );
            restored_tier_b.push((name, fn_ir));
        }
        Self::restore_functions(run.all_fns, restored_tier_b);
    }

    pub(crate) fn run_heavy_function_job(
        &self,
        name: String,
        mut fn_ir: FnIR,
        inputs: &HeavyTierInputs<'_>,
    ) -> (String, FnIR, TachyonRunProfile) {
        // Proof correspondence:
        // `ProgramTierExecutionSoundness` fixes the reduced per-function
        // execution boundary: conservative skip, self-recursive skip,
        // heavy-tier-disabled skip, budget skip, collected-plan hit, and
        // legacy-plan fallback.
        let mut local_profile = TachyonRunProfile::default();
        if fn_ir.requires_conservative_optimization() {
            local_profile.pulse_stats.skipped_functions += 1;
            let _ = Self::verify_or_reject(&mut fn_ir, "SkipOpt/ConservativeInterop");
            return (name, fn_ir, local_profile);
        }
        if Self::fn_is_self_recursive(&fn_ir) {
            local_profile.pulse_stats.skipped_functions += 1;
            let _ = Self::verify_or_reject(&mut fn_ir, "SkipOpt/SelfRecursive");
            return (name, fn_ir, local_profile);
        }
        let selected =
            !inputs.plan.selective_mode || inputs.plan.selected_functions.contains(&name);
        if !inputs.run_heavy_tier || !selected {
            local_profile.pulse_stats.skipped_functions += 1;
            let reason = if !inputs.run_heavy_tier {
                "SkipOpt/HeavyTierDisabled"
            } else {
                "SkipOpt/Budget"
            };
            let _ = Self::verify_or_reject(&mut fn_ir, reason);
            return (name, fn_ir, local_profile);
        }
        local_profile.pulse_stats.optimized_functions += 1;
        let phase_plan = inputs
            .heavy_phase_plans
            .get(&name)
            .cloned()
            .unwrap_or_else(|| self.build_legacy_function_phase_plan(&name));
        let phase_profile = self.run_function_with_phase_plan_with_proven_profile(
            &mut fn_ir,
            inputs.callmap_user_whitelist,
            inputs.proven_floor_param_slots.get(&name),
            &phase_plan,
        );
        local_profile
            .pulse_stats
            .accumulate(phase_profile.pulse_stats);
        local_profile
            .pass_timings
            .accumulate(phase_profile.pass_timings);
        (name, fn_ir, local_profile)
    }

    pub(crate) fn run_full_inline_program_tier(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        plan: &ProgramOptPlan,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
    ) {
        let mut changed = true;
        let mut iter = 0;
        let inliner = if self.fast_dev_enabled() {
            inline::MirInliner::new_fast_dev()
        } else if self.aggressive_opt_enabled() {
            inline::MirInliner::new_aggressive()
        } else {
            inline::MirInliner::new()
        };
        let hot_filter = if plan.selective_mode {
            Some(&plan.selected_functions)
        } else {
            None
        };
        let inline_manager = chronos::ChronosProgramPassManager::new(
            chronos::ChronosStage::ProgramInline,
            chronos::PROGRAM_INLINE_PASSES,
        );
        while changed && iter < self.configured_max_inline_rounds() {
            changed = false;
            iter += 1;
            let mut inline_ctx = chronos::ChronosProgramContext {
                stats,
                timings: pass_timings,
                inliner: Some(&inliner),
                hot_filter,
                valid_analyses: chronos::ChronosAnalysisSet::ALL,
                fuel: OptimizationFuel::disabled(),
            };
            let local_changed = inline_manager
                .run_sequence(self, all_fns, &mut inline_ctx)
                .is_changed();
            if local_changed {
                changed = true;
                self.run_inline_cleanup_program_stage(all_fns, scheduler, stats, pass_timings);
            }
        }
    }

    pub(crate) fn run_inline_cleanup_program_stage(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
    ) {
        let ordered_names = Self::sorted_fn_names(all_fns);
        let cleanup_total_ir: usize = ordered_names
            .iter()
            .filter_map(|name| all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let cleanup_jobs = Self::take_functions_in_order(all_fns, &ordered_names);
        let cleanup_results = scheduler.map_stage(
            CompilerParallelStage::TachyonInlineCleanup,
            cleanup_jobs,
            cleanup_total_ir,
            |(name, mut fn_ir)| {
                let local_profile = self.run_inline_cleanup_function_job(&mut fn_ir);
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_cleanup = Vec::with_capacity(cleanup_results.len());
        for (name, fn_ir, local_profile) in cleanup_results {
            stats.accumulate(local_profile.pulse_stats);
            pass_timings.accumulate(local_profile.pass_timings);
            restored_cleanup.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_cleanup);
    }

    pub(crate) fn run_inline_cleanup_function_job(&self, fn_ir: &mut FnIR) -> TachyonRunProfile {
        let mut local_profile = TachyonRunProfile::default();
        if fn_ir.requires_conservative_optimization() {
            Self::maybe_verify(fn_ir, "After Inline Cleanup (Skipped: ConservativeInterop)");
            return local_profile;
        }
        let loop_opt = loop_opt::MirLoopOptimizer::new();
        let cleanup_outcome =
            self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                stage: chronos::ChronosStage::ProgramInlineCleanup,
                passes: chronos::PROGRAM_INLINE_CLEANUP_PASSES,
                fn_ir,
                loop_optimizer: &loop_opt,
                user_call_whitelist: None,
                fresh_user_calls: None,
                stats: &mut local_profile.pulse_stats,
                timings: &mut local_profile.pass_timings,
            });
        if cleanup_outcome.is_changed() {
            local_profile.pulse_stats.inline_cleanup_hits += 1;
        }
        Self::maybe_verify(fn_ir, "After Inline Cleanup");
        local_profile
    }

    pub(crate) fn run_fresh_alias_program_stage(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
    ) {
        let fresh_user_calls =
            fresh_alias::collect_fresh_returning_user_functions_for_parallel(all_fns);
        let fresh_alias_names = Self::sorted_fn_names(all_fns);
        let fresh_alias_total_ir: usize = fresh_alias_names
            .iter()
            .filter_map(|name| all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let fresh_alias_jobs = Self::take_functions_in_order(all_fns, &fresh_alias_names);
        let fresh_alias_results = scheduler.map_stage(
            CompilerParallelStage::TachyonFreshAlias,
            fresh_alias_jobs,
            fresh_alias_total_ir,
            |(name, mut fn_ir)| {
                let mut local_profile = TachyonRunProfile::default();
                let loop_opt = loop_opt::MirLoopOptimizer::new();
                let _ =
                    self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                        stage: chronos::ChronosStage::ProgramFreshAlias,
                        passes: chronos::PROGRAM_FRESH_ALIAS_PASSES,
                        fn_ir: &mut fn_ir,
                        loop_optimizer: &loop_opt,
                        user_call_whitelist: None,
                        fresh_user_calls: Some(&fresh_user_calls),
                        stats: &mut local_profile.pulse_stats,
                        timings: &mut local_profile.pass_timings,
                    });
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_fresh_alias = Vec::with_capacity(fresh_alias_results.len());
        for (name, fn_ir, local_profile) in fresh_alias_results {
            stats.accumulate(local_profile.pulse_stats);
            pass_timings.accumulate(local_profile.pass_timings);
            restored_fresh_alias.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_fresh_alias);
    }

    pub(crate) fn run_de_ssa_program_stage(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        stats: &mut TachyonPulseStats,
        pass_timings: &mut TachyonPassTimings,
        progress: &mut Option<&mut dyn FnMut(TachyonProgress)>,
    ) {
        let ordered_names = Self::sorted_fn_names(all_fns);
        let de_ssa_total = ordered_names.len();
        let de_ssa_total_ir: usize = ordered_names
            .iter()
            .filter_map(|name| all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let de_ssa_jobs = Self::take_functions_in_order(all_fns, &ordered_names);
        let de_ssa_results = scheduler.map_stage(
            CompilerParallelStage::TachyonDeSsa,
            de_ssa_jobs,
            de_ssa_total_ir,
            |(name, mut fn_ir)| {
                let mut local_profile = TachyonRunProfile::default();
                let loop_opt = loop_opt::MirLoopOptimizer::new();
                let _ =
                    self.run_chronos_function_sequence(chronos::ChronosFunctionSequenceRequest {
                        stage: chronos::ChronosStage::ProgramPostDeSsa,
                        passes: chronos::PROGRAM_POST_DESSA_PASSES,
                        fn_ir: &mut fn_ir,
                        loop_optimizer: &loop_opt,
                        user_call_whitelist: None,
                        fresh_user_calls: None,
                        stats: &mut local_profile.pulse_stats,
                        timings: &mut local_profile.pass_timings,
                    });
                let _ = Self::verify_or_reject(&mut fn_ir, "After De-SSA");
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_de_ssa = Vec::with_capacity(de_ssa_results.len());
        for (idx, (name, fn_ir, local_profile)) in de_ssa_results.into_iter().enumerate() {
            stats.accumulate(local_profile.pulse_stats);
            pass_timings.accumulate(local_profile.pass_timings);
            Self::emit_progress(
                progress,
                TachyonProgressTier::DeSsa,
                idx + 1,
                de_ssa_total,
                &name,
            );
            restored_de_ssa.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_de_ssa);
    }

    pub(crate) fn run_program_with_profile_inner(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        scheduler: &CompilerScheduler,
        mut progress: Option<&mut dyn FnMut(TachyonProgress)>,
    ) -> TachyonRunProfile {
        // Proof correspondence:
        // `ProgramRunProfileInnerSoundness` fixes the reduced wrapper theorem
        // family for this whole function. The reduced model composes:
        // always-tier execution, heavy-tier plan flow, per-function heavy-tier
        // execution, plan-summary emission, and the post-tier cleanup/de-ssa
        // tail into one `run_program_with_profile_inner`-shaped boundary.
        let mut profile = TachyonRunProfile {
            active_pass_groups: self.active_pass_group_labels(),
            ..TachyonRunProfile::default()
        };
        let plan = self.build_opt_plan(all_fns);
        let tiers = self.program_run_tiers(&plan);
        self.initialize_program_stats(&mut profile.pulse_stats, &plan, tiers);
        let ordered_names = Self::sorted_fn_names(all_fns);
        let ordered_total = ordered_names.len();
        let pre_tier_helpers = Self::rewrite_pre_tier_program_helpers(all_fns);
        self.run_always_program_tier(
            ProgramTierRun {
                all_fns,
                scheduler,
                ordered_names: &ordered_names,
                ordered_total,
                stats: &mut profile.pulse_stats,
                pass_timings: &mut profile.pass_timings,
                progress: &mut progress,
            },
            &pre_tier_helpers,
        );
        Self::debug_wrap_candidates(all_fns);
        Self::debug_sroa_candidates(all_fns);
        self.run_record_specialization_stage(
            all_fns,
            &mut profile.pulse_stats,
            &mut profile.pass_timings,
        );
        self.run_outlining_program_stage(
            all_fns,
            &mut profile.pulse_stats,
            &mut profile.pass_timings,
        );

        let heavy_phase_plans =
            self.collect_program_heavy_phase_plans(all_fns, &ordered_names, &plan, tiers.heavy);
        profile.plan_summary = self.plan_summary_lines(&ordered_names, &heavy_phase_plans);
        Self::rewrite_heavy_index_helpers(all_fns, tiers.heavy);
        let callmap_user_whitelist =
            Self::collect_heavy_callmap_whitelist(all_fns, &plan, tiers.heavy);

        self.run_heavy_program_tier(
            ProgramTierRun {
                all_fns,
                scheduler,
                ordered_names: &ordered_names,
                ordered_total,
                stats: &mut profile.pulse_stats,
                pass_timings: &mut profile.pass_timings,
                progress: &mut progress,
            },
            HeavyTierInputs {
                plan: &plan,
                run_heavy_tier: tiers.heavy,
                heavy_phase_plans: &heavy_phase_plans,
                callmap_user_whitelist: &callmap_user_whitelist,
                proven_floor_param_slots: &pre_tier_helpers.proven_floor_param_slots,
            },
        );

        // Tier C (full-program): bounded inter-procedural inlining.
        // Proof correspondence:
        // `ProgramPostTierStagesSoundness.inline_cleanup_stage_*` fixes the
        // reduced stage boundary for the inline cleanup slice below
        // (`simplify_cfg -> sroa -> dce` after an inlining round).
        if tiers.full_inline {
            self.run_full_inline_program_tier(
                all_fns,
                scheduler,
                &plan,
                &mut profile.pulse_stats,
                &mut profile.pass_timings,
            );
        }

        // Proof correspondence:
        // `ProgramPostTierStagesSoundness.fresh_alias_stage_*` fixes the
        // reduced stage boundary for the fresh-alias cleanup pass applied
        // across the restored program map here.
        self.run_fresh_alias_program_stage(
            all_fns,
            scheduler,
            &mut profile.pulse_stats,
            &mut profile.pass_timings,
        );

        // 3. De-SSA (Phi elimination via parallel copy) before codegen.
        // Proof correspondence:
        // `DeSsaBoundarySoundness` models the reduced redundant-copy
        // elimination boundary here, while `OptimizerPipelineSoundness`
        // exposes the staged theorem family that crosses this point:
        // `program_post_dessa_*` for the `de_ssa` + cleanup boundary and
        // `prepare_for_codegen_*` for the full pre-emission normalization
        // slice. `DeSsaSubset` remains the lower-level reduced theorem for the
        // canonical copy-boundary matcher itself.
        // `ProgramPostTierStagesSoundness.de_ssa_program_stage_*` then lifts
        // this same reduced boundary into the post-heavy, program-level tail
        // stage family used by `run_program_with_profile_inner`.
        self.run_de_ssa_program_stage(
            all_fns,
            scheduler,
            &mut profile.pulse_stats,
            &mut profile.pass_timings,
            &mut progress,
        );
        profile
    }
}
