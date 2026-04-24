import RRProofs.ProgramOptPlanSoundness
import RRProofs.PhasePlanCollectionSoundness
import RRProofs.PhasePlanLookupSoundness
import RRProofs.PhasePlanSummarySoundness

namespace RRProofs.ProgramPhasePipelineSoundness

open RRProofs.MirInvariantBundle
open RRProofs.ProgramOptPlanSoundness
open RRProofs.PhasePlanSoundness
open RRProofs.PhasePlanCollectionSoundness
open RRProofs.PhasePlanLookupSoundness
open RRProofs.PhasePlanSummarySoundness

structure ReducedProgramPhaseEntry where
  functionId : Nat
  features : ReducedPlanFeatures
  irSize : Nat
  score : Nat
  hotWeight : Nat
  present : Bool
  conservative : Bool
  selfRecursive : Bool
deriving Repr, DecidableEq

def budgetEntryOf (entry : ReducedProgramPhaseEntry) : ReducedProgramFunctionEntry :=
  { functionId := entry.functionId
  , irSize := entry.irSize
  , score := entry.score
  , hotWeight := entry.hotWeight
  , conservative := entry.conservative
  }

def selectedByProgramPlan
    (plan : ReducedProgramOptPlan)
    (entry : ReducedProgramPhaseEntry) : Bool :=
  if plan.selectiveMode then
    entry.functionId ∈ plan.selectedFunctions
  else
    true

def phaseInventoryEntryOf
    (plan : ReducedProgramOptPlan)
    (entry : ReducedProgramPhaseEntry) : ReducedFunctionInventoryEntry :=
  { functionId := entry.functionId
  , features := entry.features
  , present := entry.present
  , conservative := entry.conservative
  , selfRecursive := entry.selfRecursive
  , selected := selectedByProgramPlan plan entry
  }

def collectProgramPhasePlans
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry) : List ReducedFunctionPhasePlan :=
  if runHeavyTier then
    collectFunctionPhasePlans mode traceRequested fastDev (entries.map (phaseInventoryEntryOf plan))
  else
    []

def programPhaseSummaryEntries
    (orderedFunctionIds : List Nat)
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry) : List ReducedPlanSummaryEntry :=
  planSummaryEntries orderedFunctionIds
    (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries)

theorem non_selective_plan_marks_every_entry_selected
    (plan : ReducedProgramOptPlan)
    (entry : ReducedProgramPhaseEntry)
    (h : plan.selectiveMode = false) :
    selectedByProgramPlan plan entry = true := by
  simp [selectedByProgramPlan, h]

theorem selective_plan_marks_membership_selected
    (plan : ReducedProgramOptPlan)
    (entry : ReducedProgramPhaseEntry)
    (h : plan.selectiveMode = true) :
    selectedByProgramPlan plan entry = (entry.functionId ∈ plan.selectedFunctions) := by
  simp [selectedByProgramPlan, h]

theorem heavy_tier_disabled_collects_no_phase_plans
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry) :
    collectProgramPhasePlans mode traceRequested fastDev false plan entries = [] := by
  simp [collectProgramPhasePlans]

theorem heavy_tier_disabled_emits_no_phase_summary
    (orderedFunctionIds : List Nat)
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry) :
    programPhaseSummaryEntries orderedFunctionIds mode traceRequested fastDev false plan entries = [] := by
  induction orderedFunctionIds with
  | nil =>
      simp [programPhaseSummaryEntries, collectProgramPhasePlans, planSummaryEntries]
  | cons functionId rest ih =>
      simp [programPhaseSummaryEntries, collectProgramPhasePlans, planSummaryEntries,
        lookupCollectedPlan?]

theorem program_phase_lookup_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (functionId : Nat)
    (selectedPlan : ReducedFunctionPhasePlan)
    (hLookup :
      lookupCollectedPlan? functionId
        (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries)
        = some selectedPlan)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (planSelectedPipeline selectedPlan fn) := by
  exact lookup_collected_plan_preserves_verify_ir functionId _ selectedPlan hLookup h

theorem program_phase_lookup_preserves_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (functionId : Nat)
    (selectedPlan : ReducedFunctionPhasePlan)
    (hLookup :
      lookupCollectedPlan? functionId
        (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries)
        = some selectedPlan)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (planSelectedPipeline selectedPlan fn) env = execEntry fn env := by
  exact lookup_collected_plan_preserves_semantics functionId _ selectedPlan hLookup fn env

theorem program_phase_summary_hit_emits_entry
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (functionId : Nat)
    (selectedPlan : ReducedFunctionPhasePlan)
    (hLookup :
      lookupCollectedPlan? functionId
        (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries)
        = some selectedPlan) :
    summarizePlan selectedPlan ∈
      programPhaseSummaryEntries [functionId] mode traceRequested fastDev runHeavyTier plan entries := by
  exact summary_lookup_hit_emits_entry functionId _ selectedPlan hLookup

theorem program_phase_summary_miss_skips_entry
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (functionId : Nat)
    (hLookup :
      lookupCollectedPlan? functionId
        (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries)
        = none) :
    programPhaseSummaryEntries [functionId] mode traceRequested fastDev runHeavyTier plan entries = [] := by
  exact summary_lookup_miss_skips_entry functionId _ hLookup

end RRProofs.ProgramPhasePipelineSoundness
