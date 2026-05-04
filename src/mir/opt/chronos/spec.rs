use super::*;

pub(in crate::mir::opt) type ChronosEnabledFn =
    for<'a> fn(&TachyonEngine, &FnIR, &mut ChronosContext<'a>) -> bool;
pub(in crate::mir::opt) type ChronosFunctionRunner =
    for<'a> fn(&TachyonEngine, &mut FnIR, &mut ChronosContext<'a>) -> ChronosPassOutcome;
pub(in crate::mir::opt) type ChronosProgramEnabledFn =
    for<'a> fn(&TachyonEngine, &FxHashMap<String, FnIR>, &ChronosProgramContext<'a>) -> bool;
pub(in crate::mir::opt) type ChronosProgramRunner = for<'a> fn(
    &TachyonEngine,
    &mut FxHashMap<String, FnIR>,
    &mut ChronosProgramContext<'a>,
) -> ChronosPassOutcome;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::mir::opt) enum ChronosBudgetClass {
    AlwaysCheap,
    LocalHeavy,
    StructuralProof,
    Interprocedural,
    FinalCleanup,
}

impl ChronosBudgetClass {
    pub(in crate::mir::opt) const fn label(self) -> &'static str {
        match self {
            Self::AlwaysCheap => "always-cheap",
            Self::LocalHeavy => "local-heavy",
            Self::StructuralProof => "structural-proof",
            Self::Interprocedural => "interprocedural",
            Self::FinalCleanup => "final-cleanup",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::mir::opt) struct ChronosPassContract {
    pub(in crate::mir::opt) legality: &'static str,
    pub(in crate::mir::opt) profitability: &'static str,
    pub(in crate::mir::opt) budget_class: ChronosBudgetClass,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::mir::opt) struct ChronosPassSpec {
    pub(in crate::mir::opt) id: ChronosPassId,
    pub(in crate::mir::opt) scope: ChronosPassScope,
    pub(in crate::mir::opt) stage: ChronosStage,
    pub(in crate::mir::opt) group: types::PassGroup,
    pub(in crate::mir::opt) version: u32,
    pub(in crate::mir::opt) proof_key: ChronosProofKey,
    pub(in crate::mir::opt) contract: ChronosPassContract,
    pub(in crate::mir::opt) requires: ChronosAnalysisSet,
    pub(in crate::mir::opt) invalidates: ChronosAnalysisSet,
    pub(in crate::mir::opt) verify_label: &'static str,
    pub(in crate::mir::opt) enabled: ChronosEnabledFn,
    pub(in crate::mir::opt) run: ChronosFunctionRunner,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::mir::opt) struct ChronosProgramPassSpec {
    pub(in crate::mir::opt) id: ChronosPassId,
    pub(in crate::mir::opt) scope: ChronosPassScope,
    pub(in crate::mir::opt) stage: ChronosStage,
    pub(in crate::mir::opt) group: types::PassGroup,
    pub(in crate::mir::opt) version: u32,
    pub(in crate::mir::opt) proof_key: ChronosProofKey,
    pub(in crate::mir::opt) contract: ChronosPassContract,
    pub(in crate::mir::opt) requires: ChronosAnalysisSet,
    pub(in crate::mir::opt) invalidates: ChronosAnalysisSet,
    pub(in crate::mir::opt) verify_label: &'static str,
    pub(in crate::mir::opt) enabled: ChronosProgramEnabledFn,
    pub(in crate::mir::opt) run: ChronosProgramRunner,
}
