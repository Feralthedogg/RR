use super::*;

impl TachyonEngine {
    pub(super) fn run_always_tier_with_stats(
        &self,
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> TachyonPulseStats {
        self.run_always_tier_with_profile(fn_ir, proven_param_slots, floor_helpers)
            .pulse_stats
    }

    pub(super) fn run_always_tier_with_profile(
        &self,
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> TachyonRunProfile {
        let mut profile = TachyonRunProfile::default();
        let stats = &mut profile.pulse_stats;
        let pass_timings = &mut profile.pass_timings;
        if fn_ir.requires_conservative_optimization() {
            return profile;
        }
        if !Self::verify_or_reject(fn_ir, "AlwaysTier/Start") {
            return profile;
        }
        let canonicalized_index_params =
            Self::canonicalize_floor_index_params(fn_ir, proven_param_slots, floor_helpers);
        if canonicalized_index_params {
            Self::maybe_verify(fn_ir, "After AlwaysTier/ParamIndexCanonicalize");
        }

        stats.always_tier_functions = 1;
        let mut changed = true;
        let mut iter = 0usize;
        let max_iters = self.configured_always_tier_max_iters();
        let mut seen = FxHashSet::default();
        seen.insert(Self::fn_ir_fingerprint(fn_ir));
        let fn_ir_size = Self::fn_ir_size(fn_ir);
        let run_light_sccp = fn_ir_size <= self.configured_heavy_pass_fn_ir().saturating_mul(2);
        let loop_opt = loop_opt::MirLoopOptimizer::new();

        while changed && iter < max_iters {
            iter += 1;
            changed = false;
            let before_hash = Self::fn_ir_fingerprint(fn_ir);

            // Proof correspondence:
            // `DataflowOptSoundness` approximates the local expression/value
            // rewrite slice in this always-tier loop (`sccp`, `gvn`, `dce`,
            // plus canonicalization-style simplifications).
            // `CfgOptSoundness` approximates `simplify_cfg` / reduced entry
            // retarget / dead-block cleanup style rewrites.
            // `LoopOptSoundness` approximates the loop-focused slice
            // (`tco`, `loop_opt`, bounded `bce`) under a reduced MIR model.
            let sc_changed =
                Self::timed_bool_pass(pass_timings, "simplify_cfg", || self.simplify_cfg(fn_ir));
            if sc_changed {
                stats.simplify_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/SimplifyCFG");

            if run_light_sccp {
                let sccp_changed = Self::timed_bool_pass(pass_timings, "sccp", || {
                    sccp::MirSCCP::new().optimize(fn_ir)
                });
                if sccp_changed {
                    stats.sccp_hits += 1;
                    changed = true;
                }
                Self::maybe_verify(fn_ir, "After AlwaysTier/SCCP");

                let intr_changed = Self::timed_bool_pass(pass_timings, "intrinsics", || {
                    intrinsics::optimize(fn_ir)
                });
                if intr_changed {
                    stats.intrinsics_hits += 1;
                    changed = true;
                }
                Self::maybe_verify(fn_ir, "After AlwaysTier/Intrinsics");
            }

            let type_spec_changed = Self::timed_bool_pass(pass_timings, "type_specialize", || {
                type_specialize::optimize(fn_ir)
            });
            if type_spec_changed {
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/TypeSpecialize");

            let tco_changed = Self::timed_bool_pass(pass_timings, "tco", || tco::optimize(fn_ir));
            if tco_changed {
                stats.tco_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/TCO");

            let loop_changed_count = Self::timed_count_pass(pass_timings, "loop_opt", || {
                loop_opt.optimize_with_count(fn_ir)
            });
            if loop_changed_count > 0 {
                stats.simplified_loops += loop_changed_count;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/LoopOpt");

            let dce_changed = Self::timed_bool_pass(pass_timings, "dce", || self.dce(fn_ir));
            if dce_changed {
                stats.dce_hits += 1;
                changed = true;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/DCE");

            let after_hash = Self::fn_ir_fingerprint(fn_ir);
            if after_hash == before_hash {
                break;
            }
            if !seen.insert(after_hash) {
                break;
            }
        }

        // Apply one bounded BCE sweep after convergence so skipped heavy-tier functions
        // can still get guard elimination opportunities without large compile-time spikes.
        if fn_ir_size <= self.configured_always_bce_fn_ir() {
            let bce_changed = Self::timed_bool_pass(pass_timings, "bce", || bce::optimize(fn_ir));
            if bce_changed {
                stats.bce_hits += 1;
            }
            Self::maybe_verify(fn_ir, "After AlwaysTier/BCE");
        }

        let _ = Self::verify_or_reject(fn_ir, "AlwaysTier/End");
        profile
    }
}
