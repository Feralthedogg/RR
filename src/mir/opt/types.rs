use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ClampBound {
    ConstOne,
    ConstSix,
    Var(String),
}

#[derive(Debug, Clone)]
pub(super) struct CubeIndexReturnVars {
    pub(super) face_var: String,
    pub(super) x_var: String,
    pub(super) y_var: String,
    pub(super) size_var: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TachyonProgressTier {
    Always,
    Heavy,
    DeSsa,
}

impl TachyonProgressTier {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::Heavy => "heavy",
            Self::DeSsa => "de-ssa",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TachyonProgress {
    pub tier: TachyonProgressTier,
    pub completed: usize,
    pub total: usize,
    pub function: String,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TachyonPulseStats {
    pub vectorized: usize,
    pub reduced: usize,
    pub vector_loops_seen: usize,
    pub vector_skipped: usize,
    pub vector_skip_no_iv: usize,
    pub vector_skip_non_canonical_bound: usize,
    pub vector_skip_unsupported_cfg_shape: usize,
    pub vector_skip_indirect_index_access: usize,
    pub vector_skip_store_effects: usize,
    pub vector_skip_no_supported_pattern: usize,
    pub vector_candidate_total: usize,
    pub vector_candidate_reductions: usize,
    pub vector_candidate_conditionals: usize,
    pub vector_candidate_recurrences: usize,
    pub vector_candidate_shifted: usize,
    pub vector_candidate_call_maps: usize,
    pub vector_candidate_expr_maps: usize,
    pub vector_candidate_scatters: usize,
    pub vector_candidate_cube_slices: usize,
    pub vector_candidate_basic_maps: usize,
    pub vector_candidate_multi_exprs: usize,
    pub vector_candidate_2d: usize,
    pub vector_candidate_3d: usize,
    pub vector_candidate_call_map_direct: usize,
    pub vector_candidate_call_map_runtime: usize,
    pub vector_applied_total: usize,
    pub vector_applied_reductions: usize,
    pub vector_applied_conditionals: usize,
    pub vector_applied_recurrences: usize,
    pub vector_applied_shifted: usize,
    pub vector_applied_call_maps: usize,
    pub vector_applied_expr_maps: usize,
    pub vector_applied_scatters: usize,
    pub vector_applied_cube_slices: usize,
    pub vector_applied_basic_maps: usize,
    pub vector_applied_multi_exprs: usize,
    pub vector_applied_2d: usize,
    pub vector_applied_3d: usize,
    pub vector_applied_call_map_direct: usize,
    pub vector_applied_call_map_runtime: usize,
    pub vector_trip_tier_tiny: usize,
    pub vector_trip_tier_small: usize,
    pub vector_trip_tier_medium: usize,
    pub vector_trip_tier_large: usize,
    pub simplified_loops: usize,
    pub tco_hits: usize,
    pub sccp_hits: usize,
    pub intrinsics_hits: usize,
    pub gvn_hits: usize,
    pub licm_hits: usize,
    pub fresh_alloc_hits: usize,
    pub bce_hits: usize,
    pub simplify_hits: usize,
    pub dce_hits: usize,
    pub inline_rounds: usize,
    pub inline_cleanup_hits: usize,
    pub de_ssa_hits: usize,
    pub always_tier_functions: usize,
    pub optimized_functions: usize,
    pub skipped_functions: usize,
    pub full_opt_ir_limit: usize,
    pub full_opt_fn_limit: usize,
    pub total_program_ir: usize,
    pub max_function_ir: usize,
    pub selective_budget_mode: bool,
}

impl TachyonPulseStats {
    pub(super) fn accumulate(&mut self, other: Self) {
        self.vectorized += other.vectorized;
        self.reduced += other.reduced;
        self.vector_loops_seen += other.vector_loops_seen;
        self.vector_skipped += other.vector_skipped;
        self.vector_skip_no_iv += other.vector_skip_no_iv;
        self.vector_skip_non_canonical_bound += other.vector_skip_non_canonical_bound;
        self.vector_skip_unsupported_cfg_shape += other.vector_skip_unsupported_cfg_shape;
        self.vector_skip_indirect_index_access += other.vector_skip_indirect_index_access;
        self.vector_skip_store_effects += other.vector_skip_store_effects;
        self.vector_skip_no_supported_pattern += other.vector_skip_no_supported_pattern;
        self.vector_candidate_total += other.vector_candidate_total;
        self.vector_candidate_reductions += other.vector_candidate_reductions;
        self.vector_candidate_conditionals += other.vector_candidate_conditionals;
        self.vector_candidate_recurrences += other.vector_candidate_recurrences;
        self.vector_candidate_shifted += other.vector_candidate_shifted;
        self.vector_candidate_call_maps += other.vector_candidate_call_maps;
        self.vector_candidate_expr_maps += other.vector_candidate_expr_maps;
        self.vector_candidate_scatters += other.vector_candidate_scatters;
        self.vector_candidate_cube_slices += other.vector_candidate_cube_slices;
        self.vector_candidate_basic_maps += other.vector_candidate_basic_maps;
        self.vector_candidate_multi_exprs += other.vector_candidate_multi_exprs;
        self.vector_candidate_2d += other.vector_candidate_2d;
        self.vector_candidate_3d += other.vector_candidate_3d;
        self.vector_candidate_call_map_direct += other.vector_candidate_call_map_direct;
        self.vector_candidate_call_map_runtime += other.vector_candidate_call_map_runtime;
        self.vector_applied_total += other.vector_applied_total;
        self.vector_applied_reductions += other.vector_applied_reductions;
        self.vector_applied_conditionals += other.vector_applied_conditionals;
        self.vector_applied_recurrences += other.vector_applied_recurrences;
        self.vector_applied_shifted += other.vector_applied_shifted;
        self.vector_applied_call_maps += other.vector_applied_call_maps;
        self.vector_applied_expr_maps += other.vector_applied_expr_maps;
        self.vector_applied_scatters += other.vector_applied_scatters;
        self.vector_applied_cube_slices += other.vector_applied_cube_slices;
        self.vector_applied_basic_maps += other.vector_applied_basic_maps;
        self.vector_applied_multi_exprs += other.vector_applied_multi_exprs;
        self.vector_applied_2d += other.vector_applied_2d;
        self.vector_applied_3d += other.vector_applied_3d;
        self.vector_applied_call_map_direct += other.vector_applied_call_map_direct;
        self.vector_applied_call_map_runtime += other.vector_applied_call_map_runtime;
        self.vector_trip_tier_tiny += other.vector_trip_tier_tiny;
        self.vector_trip_tier_small += other.vector_trip_tier_small;
        self.vector_trip_tier_medium += other.vector_trip_tier_medium;
        self.vector_trip_tier_large += other.vector_trip_tier_large;
        self.simplified_loops += other.simplified_loops;
        self.tco_hits += other.tco_hits;
        self.sccp_hits += other.sccp_hits;
        self.intrinsics_hits += other.intrinsics_hits;
        self.gvn_hits += other.gvn_hits;
        self.licm_hits += other.licm_hits;
        self.fresh_alloc_hits += other.fresh_alloc_hits;
        self.bce_hits += other.bce_hits;
        self.simplify_hits += other.simplify_hits;
        self.dce_hits += other.dce_hits;
        self.inline_rounds += other.inline_rounds;
        self.inline_cleanup_hits += other.inline_cleanup_hits;
        self.de_ssa_hits += other.de_ssa_hits;
        self.always_tier_functions += other.always_tier_functions;
        self.optimized_functions += other.optimized_functions;
        self.skipped_functions += other.skipped_functions;
    }

    pub fn to_json_string(&self) -> String {
        format!(
            concat!(
                "{{\n",
                "  \"vectorized\": {},\n",
                "  \"reduced\": {},\n",
                "  \"vector_loops_seen\": {},\n",
                "  \"vector_skipped\": {},\n",
                "  \"vector_skip_no_iv\": {},\n",
                "  \"vector_skip_non_canonical_bound\": {},\n",
                "  \"vector_skip_unsupported_cfg_shape\": {},\n",
                "  \"vector_skip_indirect_index_access\": {},\n",
                "  \"vector_skip_store_effects\": {},\n",
                "  \"vector_skip_no_supported_pattern\": {},\n",
                "  \"vector_candidate_total\": {},\n",
                "  \"vector_candidate_reductions\": {},\n",
                "  \"vector_candidate_conditionals\": {},\n",
                "  \"vector_candidate_recurrences\": {},\n",
                "  \"vector_candidate_shifted\": {},\n",
                "  \"vector_candidate_call_maps\": {},\n",
                "  \"vector_candidate_expr_maps\": {},\n",
                "  \"vector_candidate_scatters\": {},\n",
                "  \"vector_candidate_cube_slices\": {},\n",
                "  \"vector_candidate_basic_maps\": {},\n",
                "  \"vector_candidate_multi_exprs\": {},\n",
                "  \"vector_candidate_2d\": {},\n",
                "  \"vector_candidate_3d\": {},\n",
                "  \"vector_candidate_call_map_direct\": {},\n",
                "  \"vector_candidate_call_map_runtime\": {},\n",
                "  \"vector_applied_total\": {},\n",
                "  \"vector_applied_reductions\": {},\n",
                "  \"vector_applied_conditionals\": {},\n",
                "  \"vector_applied_recurrences\": {},\n",
                "  \"vector_applied_shifted\": {},\n",
                "  \"vector_applied_call_maps\": {},\n",
                "  \"vector_applied_expr_maps\": {},\n",
                "  \"vector_applied_scatters\": {},\n",
                "  \"vector_applied_cube_slices\": {},\n",
                "  \"vector_applied_basic_maps\": {},\n",
                "  \"vector_applied_multi_exprs\": {},\n",
                "  \"vector_applied_2d\": {},\n",
                "  \"vector_applied_3d\": {},\n",
                "  \"vector_applied_call_map_direct\": {},\n",
                "  \"vector_applied_call_map_runtime\": {},\n",
                "  \"vector_trip_tier_tiny\": {},\n",
                "  \"vector_trip_tier_small\": {},\n",
                "  \"vector_trip_tier_medium\": {},\n",
                "  \"vector_trip_tier_large\": {},\n",
                "  \"simplified_loops\": {},\n",
                "  \"tco_hits\": {},\n",
                "  \"sccp_hits\": {},\n",
                "  \"intrinsics_hits\": {},\n",
                "  \"gvn_hits\": {},\n",
                "  \"licm_hits\": {},\n",
                "  \"fresh_alloc_hits\": {},\n",
                "  \"bce_hits\": {},\n",
                "  \"simplify_hits\": {},\n",
                "  \"dce_hits\": {},\n",
                "  \"inline_rounds\": {},\n",
                "  \"inline_cleanup_hits\": {},\n",
                "  \"de_ssa_hits\": {},\n",
                "  \"always_tier_functions\": {},\n",
                "  \"optimized_functions\": {},\n",
                "  \"skipped_functions\": {},\n",
                "  \"full_opt_ir_limit\": {},\n",
                "  \"full_opt_fn_limit\": {},\n",
                "  \"total_program_ir\": {},\n",
                "  \"max_function_ir\": {},\n",
                "  \"selective_budget_mode\": {}\n",
                "}}"
            ),
            self.vectorized,
            self.reduced,
            self.vector_loops_seen,
            self.vector_skipped,
            self.vector_skip_no_iv,
            self.vector_skip_non_canonical_bound,
            self.vector_skip_unsupported_cfg_shape,
            self.vector_skip_indirect_index_access,
            self.vector_skip_store_effects,
            self.vector_skip_no_supported_pattern,
            self.vector_candidate_total,
            self.vector_candidate_reductions,
            self.vector_candidate_conditionals,
            self.vector_candidate_recurrences,
            self.vector_candidate_shifted,
            self.vector_candidate_call_maps,
            self.vector_candidate_expr_maps,
            self.vector_candidate_scatters,
            self.vector_candidate_cube_slices,
            self.vector_candidate_basic_maps,
            self.vector_candidate_multi_exprs,
            self.vector_candidate_2d,
            self.vector_candidate_3d,
            self.vector_candidate_call_map_direct,
            self.vector_candidate_call_map_runtime,
            self.vector_applied_total,
            self.vector_applied_reductions,
            self.vector_applied_conditionals,
            self.vector_applied_recurrences,
            self.vector_applied_shifted,
            self.vector_applied_call_maps,
            self.vector_applied_expr_maps,
            self.vector_applied_scatters,
            self.vector_applied_cube_slices,
            self.vector_applied_basic_maps,
            self.vector_applied_multi_exprs,
            self.vector_applied_2d,
            self.vector_applied_3d,
            self.vector_applied_call_map_direct,
            self.vector_applied_call_map_runtime,
            self.vector_trip_tier_tiny,
            self.vector_trip_tier_small,
            self.vector_trip_tier_medium,
            self.vector_trip_tier_large,
            self.simplified_loops,
            self.tco_hits,
            self.sccp_hits,
            self.intrinsics_hits,
            self.gvn_hits,
            self.licm_hits,
            self.fresh_alloc_hits,
            self.bce_hits,
            self.simplify_hits,
            self.dce_hits,
            self.inline_rounds,
            self.inline_cleanup_hits,
            self.de_ssa_hits,
            self.always_tier_functions,
            self.optimized_functions,
            self.skipped_functions,
            self.full_opt_ir_limit,
            self.full_opt_fn_limit,
            self.total_program_ir,
            self.max_function_ir,
            self.selective_budget_mode
        )
    }
}

#[derive(Debug, Clone)]
pub(super) struct FunctionBudgetProfile {
    pub(super) name: String,
    pub(super) ir_size: usize,
    pub(super) score: usize,
    pub(super) weighted_score: usize,
    pub(super) density: usize,
    pub(super) hot_weight: usize,
    pub(super) within_fn_limit: bool,
}

#[derive(Debug, Clone)]
pub(super) struct ProgramOptPlan {
    pub(super) program_limit: usize,
    pub(super) fn_limit: usize,
    pub(super) total_ir: usize,
    pub(super) max_fn_ir: usize,
    pub(super) selective_mode: bool,
    pub(super) selected_functions: FxHashSet<String>,
}
