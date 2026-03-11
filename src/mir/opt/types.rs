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
