use super::*;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::mir::opt) enum ChronosStage {
    FunctionEntryCanonicalization,
    AlwaysTier,
    PhaseOrderStandard,
    PhaseOrderComputePrelude,
    PhaseOrderControlPrelude,
    PhaseOrderBudgetPrefix,
    PhaseOrderBudgetTail,
    PhaseOrderBalancedStructural,
    PhaseOrderControlStructural,
    PhaseOrderFastDevVectorize,
    PhaseOrderStructuralCleanup,
    FunctionFinalPolish,
    ProgramInline,
    ProgramOutlining,
    ProgramRecordSpecialization,
    ProgramInlineCleanup,
    ProgramFreshAlias,
    ProgramPostDeSsa,
    PrepareForCodegen,
}

impl ChronosStage {
    pub(in crate::mir::opt) const fn label(self) -> &'static str {
        match self {
            Self::FunctionEntryCanonicalization => "function-entry-canonicalization",
            Self::AlwaysTier => "always-tier",
            Self::PhaseOrderStandard => "phase-order-standard",
            Self::PhaseOrderComputePrelude => "phase-order-compute-prelude",
            Self::PhaseOrderControlPrelude => "phase-order-control-prelude",
            Self::PhaseOrderBudgetPrefix => "phase-order-budget-prefix",
            Self::PhaseOrderBudgetTail => "phase-order-budget-tail",
            Self::PhaseOrderBalancedStructural => "phase-order-balanced-structural",
            Self::PhaseOrderControlStructural => "phase-order-control-structural",
            Self::PhaseOrderFastDevVectorize => "phase-order-fast-dev-vectorize",
            Self::PhaseOrderStructuralCleanup => "phase-order-structural-cleanup",
            Self::FunctionFinalPolish => "function-final-polish",
            Self::ProgramInline => "program-inline",
            Self::ProgramOutlining => "program-outlining",
            Self::ProgramRecordSpecialization => "program-record-specialization",
            Self::ProgramInlineCleanup => "program-inline-cleanup",
            Self::ProgramFreshAlias => "program-fresh-alias",
            Self::ProgramPostDeSsa => "program-post-de-ssa",
            Self::PrepareForCodegen => "prepare-for-codegen",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::mir::opt) enum ChronosPassScope {
    FunctionMir,
    ProgramMir,
}

impl ChronosPassScope {
    pub(in crate::mir::opt) const fn label(self) -> &'static str {
        match self {
            Self::FunctionMir => "function-mir",
            Self::ProgramMir => "program-mir",
        }
    }
}
