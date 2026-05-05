use super::super::debug::VectorizeSkipReason;

#[derive(Debug, Default, Clone, Copy)]
pub struct VOptStats {
    pub vectorized: usize,
    pub reduced: usize,
    pub loops_seen: usize,
    pub skipped: usize,
    pub skip_no_iv: usize,
    pub skip_non_canonical_bound: usize,
    pub skip_unsupported_cfg_shape: usize,
    pub skip_indirect_index_access: usize,
    pub skip_store_effects: usize,
    pub skip_no_supported_pattern: usize,
    pub candidate_total: usize,
    pub candidate_reductions: usize,
    pub candidate_conditionals: usize,
    pub candidate_recurrences: usize,
    pub candidate_shifted: usize,
    pub candidate_call_maps: usize,
    pub candidate_expr_maps: usize,
    pub candidate_scatters: usize,
    pub candidate_cube_slices: usize,
    pub candidate_basic_maps: usize,
    pub candidate_multi_exprs: usize,
    pub candidate_2d: usize,
    pub candidate_3d: usize,
    pub candidate_call_map_direct: usize,
    pub candidate_call_map_runtime: usize,
    pub applied_total: usize,
    pub applied_reductions: usize,
    pub applied_conditionals: usize,
    pub applied_recurrences: usize,
    pub applied_shifted: usize,
    pub applied_call_maps: usize,
    pub applied_expr_maps: usize,
    pub applied_scatters: usize,
    pub applied_cube_slices: usize,
    pub applied_basic_maps: usize,
    pub applied_multi_exprs: usize,
    pub applied_2d: usize,
    pub applied_3d: usize,
    pub applied_call_map_direct: usize,
    pub applied_call_map_runtime: usize,
    pub legacy_poly_fallback_candidate_total: usize,
    pub legacy_poly_fallback_candidate_reductions: usize,
    pub legacy_poly_fallback_candidate_maps: usize,
    pub legacy_poly_fallback_applied_total: usize,
    pub legacy_poly_fallback_applied_reductions: usize,
    pub legacy_poly_fallback_applied_maps: usize,
    pub trip_tier_tiny: usize,
    pub trip_tier_small: usize,
    pub trip_tier_medium: usize,
    pub trip_tier_large: usize,
    pub proof_certified: usize,
    pub proof_applied: usize,
    pub proof_apply_failed: usize,
    pub proof_fallback_pattern: usize,
    pub proof_fallback_reason_counts: [usize; super::super::PROOF_FALLBACK_REASON_COUNT],
}

impl VOptStats {
    pub fn changed(self) -> bool {
        self.vectorized > 0 || self.reduced > 0
    }

    pub(super) fn record_skip(&mut self, reason: VectorizeSkipReason) {
        self.skipped += 1;
        match reason {
            VectorizeSkipReason::NoIv => self.skip_no_iv += 1,
            VectorizeSkipReason::NonCanonicalBound => self.skip_non_canonical_bound += 1,
            VectorizeSkipReason::UnsupportedCfgShape => self.skip_unsupported_cfg_shape += 1,
            VectorizeSkipReason::IndirectIndexAccess => self.skip_indirect_index_access += 1,
            VectorizeSkipReason::StoreEffects => self.skip_store_effects += 1,
            VectorizeSkipReason::NoSupportedPattern => self.skip_no_supported_pattern += 1,
        }
    }

    pub(super) fn record_trip_tier(&mut self, tier: u8) {
        match tier {
            0 => self.trip_tier_tiny += 1,
            1 => self.trip_tier_small += 1,
            2 => self.trip_tier_medium += 1,
            _ => self.trip_tier_large += 1,
        }
    }
}
