use super::*;

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

    fn emit_progress(
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

    pub(super) fn timed_bool_pass<F>(
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

    pub(super) fn timed_count_pass<F>(
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

    fn take_functions_in_order(
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

    fn restore_functions(all_fns: &mut FxHashMap<String, FnIR>, jobs: Vec<(String, FnIR)>) {
        for (name, fn_ir) in jobs {
            all_fns.insert(name, fn_ir);
        }
    }

    pub(super) fn fn_is_self_recursive(fn_ir: &FnIR) -> bool {
        fn_ir.values.iter().any(|value| {
            matches!(
                &value.kind,
                ValueKind::Call { callee, .. } if callee == &fn_ir.name
            )
        })
    }

    fn run_program_with_profile_inner(
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
        let mut profile = TachyonRunProfile::default();
        let stats = &mut profile.pulse_stats;
        let pass_timings = &mut profile.pass_timings;
        profile.active_pass_groups = self.active_pass_group_labels();
        let plan = Self::build_opt_plan(all_fns);
        let selective_enabled = Self::selective_budget_enabled();
        let run_heavy_tier = !plan.selective_mode || selective_enabled;
        let run_full_inline_tier = run_heavy_tier
            && self.inline_tier_enabled()
            && !plan.selective_mode
            && plan.total_ir <= Self::max_full_opt_ir();
        stats.total_program_ir = plan.total_ir;
        stats.max_function_ir = plan.max_fn_ir;
        stats.full_opt_ir_limit = plan.program_limit;
        stats.full_opt_fn_limit = plan.fn_limit;
        stats.selective_budget_mode = plan.selective_mode && selective_enabled;
        let ordered_names = Self::sorted_fn_names(all_fns);
        let ordered_total = ordered_names.len();
        let floor_helpers = Self::collect_floor_helpers(all_fns);
        let proven_floor_param_slots =
            Self::collect_proven_floor_index_param_slots(all_fns, &floor_helpers);
        if !floor_helpers.is_empty() {
            let rewrites = Self::rewrite_floor_helper_calls(all_fns, &floor_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&floor_helpers).join(", ");
                eprintln!(
                    "   [floor] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let abs_helpers = Self::collect_trivial_abs_helpers(all_fns);
        if !abs_helpers.is_empty() {
            let rewrites = Self::rewrite_trivial_abs_helper_calls(all_fns, &abs_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&abs_helpers).join(", ");
                eprintln!(
                    "   [abs] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let unit_index_helpers = Self::collect_unit_index_helpers(all_fns);
        if !unit_index_helpers.is_empty() {
            let rewrites = Self::rewrite_unit_index_helper_calls(all_fns, &unit_index_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&unit_index_helpers).join(", ");
                eprintln!(
                    "   [unit-index] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let minmax_helpers = Self::collect_trivial_minmax_helpers(all_fns);
        if !minmax_helpers.is_empty() {
            let rewrites = Self::rewrite_trivial_minmax_helper_calls(all_fns, &minmax_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names: FxHashSet<String> = minmax_helpers.keys().cloned().collect();
                let helper_names = Self::sorted_names(&helper_names).join(", ");
                eprintln!(
                    "   [minmax] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let clamp_helpers = Self::collect_trivial_clamp_helpers(all_fns);
        if !clamp_helpers.is_empty() {
            let rewrites = Self::rewrite_trivial_clamp_helper_calls(all_fns, &clamp_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&clamp_helpers).join(", ");
                eprintln!(
                    "   [clamp] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }

        // Tier A (always): lightweight canonicalization for every safe function.
        let tier_a_total_ir: usize = ordered_names
            .iter()
            .filter_map(|name| all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let tier_a_jobs = Self::take_functions_in_order(all_fns, &ordered_names);
        let tier_a_results = scheduler.map_stage(
            CompilerParallelStage::TachyonAlways,
            tier_a_jobs,
            tier_a_total_ir,
            |(name, mut fn_ir)| {
                let local_profile = self.run_always_tier_with_profile(
                    &mut fn_ir,
                    proven_floor_param_slots.get(&name),
                    &floor_helpers,
                );
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_tier_a = Vec::with_capacity(tier_a_results.len());
        for (idx, (name, fn_ir, local_profile)) in tier_a_results.into_iter().enumerate() {
            stats.accumulate(local_profile.pulse_stats);
            pass_timings.accumulate(local_profile.pass_timings);
            Self::emit_progress(
                &mut progress,
                TachyonProgressTier::Always,
                idx + 1,
                ordered_total,
                &name,
            );
            restored_tier_a.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_tier_a);
        Self::debug_wrap_candidates(all_fns);

        let heavy_phase_plans = if run_heavy_tier {
            let selected_functions = if plan.selective_mode {
                Some(&plan.selected_functions)
            } else {
                None
            };
            // Proof correspondence:
            // `ProgramPhasePipelineSoundness` fixes the reduced program-level
            // composition boundary here:
            // `ProgramOptPlan -> selected_functions ->
            // collect_function_phase_plans -> plan_summary`.
            // The reduced model keeps the same heavy-tier disabled/empty case,
            // selected-function gating, lookup reuse, and summary emission
            // boundaries over the collected plan set.
            self.collect_function_phase_plans(all_fns, &ordered_names, selected_functions)
        } else {
            FxHashMap::default()
        };
        profile.plan_summary = self.plan_summary_lines(&ordered_names, &heavy_phase_plans);

        let wrap_index_helpers = if run_heavy_tier {
            Self::collect_wrap_index_helpers(all_fns)
        } else {
            FxHashSet::default()
        };
        if !wrap_index_helpers.is_empty() {
            let rewrites = Self::rewrite_wrap_index_helper_calls(all_fns, &wrap_index_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&wrap_index_helpers).join(", ");
                eprintln!(
                    "   [wrap] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }
        let periodic_index_helpers = if run_heavy_tier {
            Self::collect_periodic_index_helpers(all_fns)
        } else {
            FxHashMap::default()
        };
        if !periodic_index_helpers.is_empty() {
            let rewrites =
                Self::rewrite_periodic_index_helper_calls(all_fns, &periodic_index_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names: FxHashSet<String> =
                    periodic_index_helpers.keys().cloned().collect();
                let helper_names = Self::sorted_names(&helper_names).join(", ");
                eprintln!(
                    "   [wrap1d] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }

        let cube_index_helpers = if run_heavy_tier {
            Self::collect_cube_index_helpers(all_fns)
        } else {
            FxHashSet::default()
        };
        if !cube_index_helpers.is_empty() {
            let rewrites = Self::rewrite_cube_index_helper_calls(all_fns, &cube_index_helpers);
            if Self::wrap_trace_enabled() && rewrites > 0 {
                let helper_names = Self::sorted_names(&cube_index_helpers).join(", ");
                eprintln!(
                    "   [cube] rewrote {} call site(s) using helper(s): {}",
                    rewrites, helper_names
                );
            }
        }

        let heavy_targets_exist =
            run_heavy_tier && (!plan.selective_mode || !plan.selected_functions.is_empty());
        let callmap_user_whitelist = if heavy_targets_exist {
            Self::collect_callmap_user_whitelist(all_fns)
        } else {
            FxHashSet::default()
        };

        // Tier B (selective-heavy): optimize full pass pipeline only for selected functions.
        let tier_b_total_ir: usize = ordered_names
            .iter()
            .filter_map(|name| all_fns.get(name))
            .map(Self::fn_ir_size)
            .sum();
        let tier_b_jobs = Self::take_functions_in_order(all_fns, &ordered_names);
        let tier_b_results = scheduler.map_stage(
            CompilerParallelStage::TachyonHeavy,
            tier_b_jobs,
            tier_b_total_ir,
            |(name, mut fn_ir)| {
                // Proof correspondence:
                // `ProgramTierExecutionSoundness` fixes the reduced per-
                // function execution boundary for this closure. The reduced
                // model keeps the same branch split:
                // conservative skip, self-recursive skip, heavy-tier-disabled
                // skip, budget skip, collected-plan hit, and legacy-plan
                // fallback.
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
                let selected = !plan.selective_mode || plan.selected_functions.contains(&name);
                if !run_heavy_tier || !selected {
                    local_profile.pulse_stats.skipped_functions += 1;
                    let reason = if !run_heavy_tier {
                        "SkipOpt/HeavyTierDisabled"
                    } else {
                        "SkipOpt/Budget"
                    };
                    let _ = Self::verify_or_reject(&mut fn_ir, reason);
                    return (name, fn_ir, local_profile);
                }
                local_profile.pulse_stats.optimized_functions += 1;
                let phase_plan = heavy_phase_plans
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| self.build_legacy_function_phase_plan(&name));
                let phase_profile = self.run_function_with_phase_plan_with_proven_profile(
                    &mut fn_ir,
                    &callmap_user_whitelist,
                    proven_floor_param_slots.get(&name),
                    &phase_plan,
                );
                local_profile
                    .pulse_stats
                    .accumulate(phase_profile.pulse_stats);
                local_profile
                    .pass_timings
                    .accumulate(phase_profile.pass_timings);
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_tier_b = Vec::with_capacity(tier_b_results.len());
        for (idx, (name, fn_ir, local_profile)) in tier_b_results.into_iter().enumerate() {
            stats.accumulate(local_profile.pulse_stats);
            pass_timings.accumulate(local_profile.pass_timings);
            Self::emit_progress(
                &mut progress,
                TachyonProgressTier::Heavy,
                idx + 1,
                ordered_total,
                &name,
            );
            restored_tier_b.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_tier_b);

        // Tier C (full-program): bounded inter-procedural inlining.
        // Proof correspondence:
        // `ProgramPostTierStagesSoundness.inline_cleanup_stage_*` fixes the
        // reduced stage boundary for the inline cleanup slice below
        // (`simplify_cfg -> dce` after an inlining round).
        if run_full_inline_tier {
            let mut changed = true;
            let mut iter = 0;
            let inliner = if self.fast_dev_enabled() {
                inline::MirInliner::new_fast_dev()
            } else {
                inline::MirInliner::new()
            };
            let hot_filter = if plan.selective_mode {
                Some(&plan.selected_functions)
            } else {
                None
            };
            while changed && iter < self.configured_max_inline_rounds() {
                changed = false;
                iter += 1;
                // Inlining needs access to the whole map
                let local_changed = Self::timed_bool_pass(pass_timings, "inline", || {
                    inliner.optimize_with_hot_filter(all_fns, hot_filter)
                });
                let ordered_names = Self::sorted_fn_names(all_fns);
                for name in &ordered_names {
                    let Some(fn_ir) = all_fns.get(name) else {
                        continue;
                    };
                    Self::maybe_verify(fn_ir, "After Inlining");
                }
                if local_changed {
                    stats.inline_rounds += 1;
                    changed = true;
                    // Re-optimize each function if inlining happened
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
                            let mut local_profile = TachyonRunProfile::default();
                            if fn_ir.requires_conservative_optimization() {
                                Self::maybe_verify(
                                    &fn_ir,
                                    "After Inline Cleanup (Skipped: ConservativeInterop)",
                                );
                                return (name, fn_ir, local_profile);
                            }
                            let inline_sc_changed = Self::timed_bool_pass(
                                &mut local_profile.pass_timings,
                                "simplify_cfg",
                                || self.simplify_cfg(&mut fn_ir),
                            );
                            let inline_dce_changed = Self::timed_bool_pass(
                                &mut local_profile.pass_timings,
                                "dce",
                                || self.dce(&mut fn_ir),
                            );
                            if inline_sc_changed || inline_dce_changed {
                                local_profile.pulse_stats.inline_cleanup_hits += 1;
                            }
                            if inline_sc_changed {
                                local_profile.pulse_stats.simplify_hits += 1;
                            }
                            if inline_dce_changed {
                                local_profile.pulse_stats.dce_hits += 1;
                            }
                            Self::maybe_verify(&fn_ir, "After Inline Cleanup");
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
            }
        }

        let fresh_user_calls =
            fresh_alias::collect_fresh_returning_user_functions_for_parallel(all_fns);
        // Proof correspondence:
        // `ProgramPostTierStagesSoundness.fresh_alias_stage_*` fixes the
        // reduced stage boundary for the fresh-alias cleanup pass applied
        // across the restored program map here.
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
                let mut local_timings = TachyonPassTimings::default();
                let _changed = Self::timed_bool_pass(&mut local_timings, "fresh_alias", || {
                    fresh_alias::optimize_function_with_fresh_user_calls(
                        &mut fn_ir,
                        &fresh_user_calls,
                    )
                });
                (name, fn_ir, local_timings)
            },
        );
        let mut restored_fresh_alias = Vec::with_capacity(fresh_alias_results.len());
        for (name, fn_ir, local_timings) in fresh_alias_results {
            pass_timings.accumulate(local_timings);
            restored_fresh_alias.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_fresh_alias);

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
                let de_ssa_changed =
                    Self::timed_bool_pass(&mut local_profile.pass_timings, "de_ssa", || {
                        de_ssa::run(&mut fn_ir)
                    });
                if de_ssa_changed {
                    local_profile.pulse_stats.de_ssa_hits += 1;
                }
                let copy_cleanup_changed = if fn_ir.requires_conservative_optimization() {
                    false
                } else {
                    Self::timed_bool_pass(&mut local_profile.pass_timings, "copy_cleanup", || {
                        copy_cleanup::optimize(&mut fn_ir)
                    })
                };
                if copy_cleanup_changed {
                    local_profile.pulse_stats.simplify_hits += 1;
                }
                if !fn_ir.requires_conservative_optimization() {
                    let sc_changed = Self::timed_bool_pass(
                        &mut local_profile.pass_timings,
                        "simplify_cfg",
                        || self.simplify_cfg(&mut fn_ir),
                    );
                    let dce_changed =
                        Self::timed_bool_pass(&mut local_profile.pass_timings, "dce", || {
                            self.dce(&mut fn_ir)
                        });
                    if sc_changed {
                        local_profile.pulse_stats.simplify_hits += 1;
                    }
                    if dce_changed {
                        local_profile.pulse_stats.dce_hits += 1;
                    }
                }
                let _ = Self::verify_or_reject(&mut fn_ir, "After De-SSA");
                (name, fn_ir, local_profile)
            },
        );
        let mut restored_de_ssa = Vec::with_capacity(de_ssa_results.len());
        for (idx, (name, fn_ir, local_profile)) in de_ssa_results.into_iter().enumerate() {
            stats.accumulate(local_profile.pulse_stats);
            pass_timings.accumulate(local_profile.pass_timings);
            Self::emit_progress(
                &mut progress,
                TachyonProgressTier::DeSsa,
                idx + 1,
                de_ssa_total,
                &name,
            );
            restored_de_ssa.push((name, fn_ir));
        }
        Self::restore_functions(all_fns, restored_de_ssa);
        profile
    }
}
