use super::*;

impl TachyonEngine {
    pub(crate) fn run_always_tier_with_stats(
        &self,
        fn_ir: &mut FnIR,
        proven_param_slots: Option<&FxHashSet<usize>>,
        floor_helpers: &FxHashSet<String>,
    ) -> TachyonPulseStats {
        self.run_always_tier_with_profile(fn_ir, proven_param_slots, floor_helpers)
            .pulse_stats
    }

    pub(crate) fn run_always_tier_with_profile(
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
        stats.always_tier_functions = 1;
        let fn_ir_size = Self::fn_ir_size(fn_ir);
        let run_light_sccp = fn_ir_size <= self.configured_heavy_pass_fn_ir().saturating_mul(2);
        let loop_opt = loop_opt::MirLoopOptimizer::new();
        let mut ctx = chronos::ChronosContext {
            stats,
            timings: pass_timings,
            loop_optimizer: &loop_opt,
            user_call_whitelist: None,
            fresh_user_calls: None,
            proven_param_slots,
            floor_helpers: Some(floor_helpers),
            valid_analyses: chronos::ChronosAnalysisSet::ALL,
            analysis_cache: chronos::ChronosAnalysisCache::default(),
            fn_ir_size,
            run_light_sccp,
            fuel: self.fuel_for_function(fn_ir_size),
        };
        let _ = chronos::ChronosPassManager::new(
            chronos::ChronosStage::AlwaysTier,
            chronos::ALWAYS_TIER_PASSES,
        )
        .run_one(
            self,
            fn_ir,
            &mut ctx,
            &chronos::ALWAYS_TIER_INDEX_CANONICALIZATION_PASS,
        );
        let manager = chronos::ChronosPassManager::new(
            chronos::ChronosStage::AlwaysTier,
            chronos::ALWAYS_TIER_PASSES,
        );
        let fixed_point = manager.run_fixed_point(
            self,
            fn_ir,
            &mut ctx,
            chronos::ChronosBudget::fixed_point(self.configured_always_tier_max_iters()),
        );
        let _ = (fixed_point.changed, fixed_point.iterations);

        // Apply one bounded BCE sweep after convergence so skipped heavy-tier functions
        // can still get guard elimination opportunities without large compile-time spikes.
        if fn_ir_size <= self.configured_always_bce_fn_ir() {
            let _ = manager.run_one(self, fn_ir, &mut ctx, &chronos::ALWAYS_TIER_BCE_PASS);
        }

        let _ = Self::verify_or_reject(fn_ir, "AlwaysTier/End");
        profile
    }
}
