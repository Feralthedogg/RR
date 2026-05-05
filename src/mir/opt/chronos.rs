use super::*;

#[path = "chronos/analysis.rs"]
pub(crate) mod analysis;
#[path = "chronos/budget.rs"]
pub(crate) mod budget;
#[path = "chronos/catalog.rs"]
pub(crate) mod catalog;
#[path = "chronos/context.rs"]
pub(crate) mod context;
#[path = "chronos/id.rs"]
pub(crate) mod id;
#[path = "chronos/outcome.rs"]
pub(crate) mod outcome;
#[path = "chronos/proof.rs"]
pub(crate) mod proof;
#[path = "chronos/runner.rs"]
pub(crate) mod runner;
#[path = "chronos/schedule.rs"]
pub(crate) mod schedule;
#[path = "chronos/spec.rs"]
pub(crate) mod spec;

pub(in crate::mir::opt) use analysis::{
    ChronosAnalysisCache, ChronosAnalysisSet, ChronosFactSnapshot,
};
pub(in crate::mir::opt) use budget::ChronosBudget;
pub(in crate::mir::opt) use catalog::{
    ALWAYS_TIER_BCE_PASS, ALWAYS_TIER_INDEX_CANONICALIZATION_PASS, ALWAYS_TIER_PASSES,
    FUNCTION_ENTRY_CANONICALIZATION_PASSES, FUNCTION_FINAL_POLISH_PASSES,
    PHASE_ORDER_BALANCED_STRUCTURAL_PASSES, PHASE_ORDER_BUDGET_PREFIX_PASSES,
    PHASE_ORDER_BUDGET_TAIL_PASSES, PHASE_ORDER_COMPUTE_PRELUDE_PASSES,
    PHASE_ORDER_CONTROL_BUDGET_PREFIX_PASSES, PHASE_ORDER_CONTROL_PRELUDE_PASSES,
    PHASE_ORDER_CONTROL_STRUCTURAL_PASSES, PHASE_ORDER_FAST_DEV_VECTORIZE_PASSES,
    PHASE_ORDER_STANDARD_BUDGET_PASSES, PHASE_ORDER_STANDARD_CORE_PASSES,
    PHASE_ORDER_STRUCTURAL_CLEANUP_PASSES, PREPARE_FOR_CODEGEN_CLEANUP_PASSES,
    PREPARE_FOR_CODEGEN_DESSA_PASSES, PROGRAM_FRESH_ALIAS_PASSES, PROGRAM_INLINE_CLEANUP_PASSES,
    PROGRAM_INLINE_PASSES, PROGRAM_OUTLINE_PASSES, PROGRAM_POST_DESSA_PASSES,
    PROGRAM_RECORD_SPECIALIZATION_PASSES,
};
pub(in crate::mir::opt) use context::{
    ChronosContext, ChronosFunctionSequenceRequest, ChronosProgramContext,
};
pub(in crate::mir::opt) use id::ChronosPassId;
pub(in crate::mir::opt) use outcome::{ChronosFixedPointOutcome, ChronosPassOutcome};
pub(in crate::mir::opt) use proof::ChronosProofKey;
pub(in crate::mir::opt) use runner::{ChronosPassManager, ChronosProgramPassManager};
pub(in crate::mir::opt) use schedule::{ChronosPassScope, ChronosStage};
pub(in crate::mir::opt) use spec::{
    ChronosBudgetClass, ChronosEnabledFn, ChronosFunctionRunner, ChronosPassContract,
    ChronosPassSpec, ChronosProgramEnabledFn, ChronosProgramPassSpec, ChronosProgramRunner,
};
