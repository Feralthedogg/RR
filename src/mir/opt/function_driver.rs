use super::*;

impl TachyonEngine {
    pub fn run_function(&self, fn_ir: &mut FnIR) {
        let empty = FxHashSet::default();
        let _ = self.run_function_with_stats(fn_ir, &empty);
    }

    pub fn run_function_with_stats(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
    ) -> TachyonPulseStats {
        let floor_helpers = FxHashSet::default();
        self.run_function_with_proven_index_slots(
            fn_ir,
            callmap_user_whitelist,
            None,
            &floor_helpers,
        )
    }

    fn run_function_with_stats_with_proven(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
    ) -> TachyonPulseStats {
        let phase_plan = self.build_legacy_function_phase_plan(&fn_ir.name);
        self.run_function_with_phase_plan_with_proven(
            fn_ir,
            callmap_user_whitelist,
            proven_param_slots,
            &phase_plan,
        )
    }

    fn run_function_with_phase_plan_with_proven(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
        phase_plan: &FunctionPhasePlan,
    ) -> TachyonPulseStats {
        self.run_function_with_phase_plan_with_proven_profile(
            fn_ir,
            callmap_user_whitelist,
            proven_param_slots,
            phase_plan,
        )
        .pulse_stats
    }

    pub(super) fn run_function_with_phase_plan_with_proven_profile(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
        phase_plan: &FunctionPhasePlan,
    ) -> TachyonRunProfile {
        let floor_helpers = FxHashSet::default();
        self.run_function_with_proven_index_slots_with_phase_plan(
            fn_ir,
            callmap_user_whitelist,
            proven_param_slots,
            &floor_helpers,
            phase_plan,
        )
    }

    fn run_function_with_proven_index_slots(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> TachyonPulseStats {
        let phase_plan = self.build_legacy_function_phase_plan(&fn_ir.name);
        self.run_function_with_proven_index_slots_with_phase_plan(
            fn_ir,
            callmap_user_whitelist,
            proven_param_slots,
            floor_helpers,
            &phase_plan,
        )
        .pulse_stats
    }

    fn run_function_with_proven_index_slots_with_phase_plan(
        &self,
        fn_ir: &mut FnIR,
        callmap_user_whitelist: &FxHashSet<String>,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
        phase_plan: &FunctionPhasePlan,
    ) -> TachyonRunProfile {
        let mut profile = TachyonRunProfile::default();
        let stats = &mut profile.pulse_stats;
        let pass_timings = &mut profile.pass_timings;
        match phase_plan.profile {
            types::PhaseProfileKind::Balanced => stats.phase_profile_balanced_functions += 1,
            types::PhaseProfileKind::ComputeHeavy => {
                stats.phase_profile_compute_heavy_functions += 1
            }
            types::PhaseProfileKind::ControlFlowHeavy => {
                stats.phase_profile_control_flow_heavy_functions += 1
            }
        }
        let mut changed = true;
        let loop_opt = loop_opt::MirLoopOptimizer::new();
        let mut iterations = 0;
        let mut seen_hashes = FxHashSet::default();
        let start_time = Instant::now();
        let fn_ir_size = Self::fn_ir_size(fn_ir);
        let max_iters = if fn_ir_size > 2200 {
            4
        } else if fn_ir_size > 1400 {
            8
        } else if fn_ir_size > 900 {
            12
        } else {
            self.configured_max_opt_iterations()
        };
        let heavy_pass_budgeted = fn_ir_size > self.configured_heavy_pass_fn_ir();

        // Initial Verify
        if !Self::verify_or_reject(fn_ir, "Start") {
            return profile;
        }
        Self::debug_stage_dump(fn_ir, "Start");
        let canonicalized_index_params =
            Self::canonicalize_floor_index_params(fn_ir, proven_param_slots, floor_helpers);
        if canonicalized_index_params {
            Self::maybe_verify(fn_ir, "After ParamIndexCanonicalize");
            Self::debug_stage_dump(fn_ir, "After ParamIndexCanonicalize");
        }
        seen_hashes.insert(Self::fn_ir_fingerprint(fn_ir));
        let mut current_schedule = phase_plan.schedule;
        let mut fallback_used = false;
        let mut control_flow_structural_skipped = false;

        while changed && iterations < max_iters {
            if start_time.elapsed().as_millis() > self.configured_max_fn_opt_ms() {
                break;
            }
            changed = false;
            iterations += 1;
            let before_hash = Self::fn_ir_fingerprint(fn_ir);
            let pre_iteration_fn_ir =
                if matches!(current_schedule, PhaseScheduleId::ControlFlowHeavy)
                    && iterations == 1
                    && !fallback_used
                {
                    Some(fn_ir.clone())
                } else {
                    None
                };
            let pre_iteration_stats = if pre_iteration_fn_ir.is_some() {
                Some(*stats)
            } else {
                None
            };

            let run_budgeted_passes = !(heavy_pass_budgeted && iterations > 1);
            let iteration_result = self.run_heavy_phase_schedule_iteration(
                current_schedule,
                fn_ir,
                callmap_user_whitelist,
                &loop_opt,
                stats,
                pass_timings,
                run_budgeted_passes,
            );
            changed |= iteration_result.changed;
            control_flow_structural_skipped |= iteration_result.skipped_structural;
            // check_elimination remains disabled.

            let after_hash = Self::fn_ir_fingerprint(fn_ir);
            if matches!(current_schedule, PhaseScheduleId::ControlFlowHeavy)
                && iterations == 1
                && !fallback_used
                && Self::control_flow_should_fallback_to_balanced(iteration_result)
            {
                if let Some(saved_fn_ir) = pre_iteration_fn_ir {
                    *fn_ir = saved_fn_ir;
                }
                if let Some(saved_stats) = pre_iteration_stats {
                    *stats = saved_stats;
                }
                if phase_plan.trace_requested {
                    eprintln!(
                        "   [phase-order] {} fallback control-flow-heavy -> balanced non_structural_changes={} structural_progress={} skipped_structural={}",
                        fn_ir.name,
                        iteration_result.non_structural_changes,
                        iteration_result.structural_progress,
                        iteration_result.skipped_structural
                    );
                }
                current_schedule = PhaseScheduleId::Balanced;
                fallback_used = true;
                stats.phase_schedule_fallbacks += 1;
                changed = true;
                continue;
            }
            if after_hash == before_hash {
                break;
            }
            if !seen_hashes.insert(after_hash) {
                // Degenerate oscillation guard.
                break;
            }
            changed |= after_hash != before_hash;
        }
        if control_flow_structural_skipped {
            stats.control_flow_structural_skip_functions += 1;
        }

        // Final polishing pass
        let mut polishing = true;
        let mut polish_guard = 0usize;
        let mut polish_seen: FxHashSet<u64> = FxHashSet::default();
        while polishing && polish_guard < 16 {
            if start_time.elapsed().as_millis() > self.configured_max_fn_opt_ms() {
                break;
            }
            polish_guard += 1;
            let before_polish = Self::fn_ir_fingerprint(fn_ir);
            polishing =
                Self::timed_bool_pass(pass_timings, "simplify_cfg", || self.simplify_cfg(fn_ir));
            if polishing {
                stats.simplify_hits += 1;
            }
            let dce_changed = Self::timed_bool_pass(pass_timings, "dce", || self.dce(fn_ir));
            if dce_changed {
                stats.dce_hits += 1;
            }
            polishing |= dce_changed;
            let after_polish = Self::fn_ir_fingerprint(fn_ir);
            if after_polish == before_polish || !polish_seen.insert(after_polish) {
                break;
            }
        }
        let _ = Self::verify_or_reject(fn_ir, "End");
        Self::debug_stage_dump(fn_ir, "End");
        profile
    }

    // Backward-compat wrappers.
    pub fn prepare_for_codegen(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.stabilize_for_codegen(all_fns);
    }

    pub fn optimize_all(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.run_program(all_fns);
    }

    pub fn optimize_function(&self, fn_ir: &mut FnIR) {
        self.run_function(fn_ir);
    }

    pub(super) fn collect_callmap_user_whitelist(
        all_fns: &FxHashMap<String, FnIR>,
    ) -> FxHashSet<String> {
        callmap::collect_callmap_user_whitelist(all_fns)
    }

    pub(super) fn is_callmap_vector_safe_user_fn(
        name: &str,
        fn_ir: &FnIR,
        user_whitelist: &FxHashSet<String>,
    ) -> bool {
        callmap::is_callmap_vector_safe_user_fn(name, fn_ir, user_whitelist)
    }

    pub(super) fn is_vector_safe_user_expr(
        fn_ir: &FnIR,
        vid: ValueId,
        user_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        callmap::is_vector_safe_user_expr(fn_ir, vid, user_whitelist, seen)
    }
}
