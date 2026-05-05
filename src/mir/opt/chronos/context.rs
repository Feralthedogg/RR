use super::*;

pub(in crate::mir::opt) struct ChronosContext<'a> {
    pub(in crate::mir::opt) stats: &'a mut TachyonPulseStats,
    pub(in crate::mir::opt) timings: &'a mut TachyonPassTimings,
    pub(in crate::mir::opt) loop_optimizer: &'a loop_opt::MirLoopOptimizer,
    pub(in crate::mir::opt) user_call_whitelist: Option<&'a FxHashSet<String>>,
    pub(in crate::mir::opt) fresh_user_calls: Option<&'a FxHashSet<String>>,
    pub(in crate::mir::opt) proven_param_slots: Option<&'a FxHashSet<usize>>,
    pub(in crate::mir::opt) floor_helpers: Option<&'a FxHashSet<String>>,
    pub(in crate::mir::opt) valid_analyses: ChronosAnalysisSet,
    pub(in crate::mir::opt) analysis_cache: ChronosAnalysisCache,
    pub(in crate::mir::opt) fn_ir_size: usize,
    pub(in crate::mir::opt) run_light_sccp: bool,
    pub(in crate::mir::opt) fuel: OptimizationFuel,
}

pub(in crate::mir::opt) struct ChronosProgramContext<'a> {
    pub(in crate::mir::opt) stats: &'a mut TachyonPulseStats,
    pub(in crate::mir::opt) timings: &'a mut TachyonPassTimings,
    pub(in crate::mir::opt) inliner: Option<&'a inline::MirInliner>,
    pub(in crate::mir::opt) hot_filter: Option<&'a FxHashSet<String>>,
    pub(in crate::mir::opt) valid_analyses: ChronosAnalysisSet,
    pub(in crate::mir::opt) fuel: OptimizationFuel,
}

pub(crate) struct ChronosFunctionSequenceRequest<'a> {
    pub(in crate::mir::opt) stage: ChronosStage,
    pub(in crate::mir::opt) passes: &'static [ChronosPassSpec],
    pub(in crate::mir::opt) fn_ir: &'a mut FnIR,
    pub(in crate::mir::opt) loop_optimizer: &'a loop_opt::MirLoopOptimizer,
    pub(in crate::mir::opt) user_call_whitelist: Option<&'a FxHashSet<String>>,
    pub(in crate::mir::opt) fresh_user_calls: Option<&'a FxHashSet<String>>,
    pub(in crate::mir::opt) stats: &'a mut TachyonPulseStats,
    pub(in crate::mir::opt) timings: &'a mut TachyonPassTimings,
}
