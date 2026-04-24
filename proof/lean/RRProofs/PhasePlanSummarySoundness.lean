import RRProofs.PhaseOrderOptimizerSoundness
import RRProofs.PhasePlanSoundness
import RRProofs.PhasePlanLookupSoundness

namespace RRProofs.PhasePlanSummarySoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhaseOrderOptimizerSoundness
open RRProofs.PhasePlanSoundness
open RRProofs.PhasePlanLookupSoundness

structure ReducedPlanSummaryEntry where
  functionId : Nat
  summarySchedule : ReducedPhaseSchedule
  summaryProfile : ReducedPhaseProfileKind
  summaryPassGroups : List ReducedPassGroup
deriving Repr, DecidableEq

def summarizePlan (plan : ReducedFunctionPhasePlan) : ReducedPlanSummaryEntry :=
  { functionId := plan.functionId
  , summarySchedule := plan.schedule
  , summaryProfile := plan.profile
  , summaryPassGroups := plan.passGroups
  }

def planSummaryEntries
    (orderedFunctionIds : List Nat)
    (plans : List ReducedFunctionPhasePlan) : List ReducedPlanSummaryEntry :=
  orderedFunctionIds.filterMap fun functionId =>
    (lookupCollectedPlan? functionId plans).map summarizePlan

theorem summary_lookup_hit_emits_entry
    (functionId : Nat)
    (plans : List ReducedFunctionPhasePlan)
    (plan : ReducedFunctionPhasePlan)
    (hLookup : lookupCollectedPlan? functionId plans = some plan) :
    summarizePlan plan ∈ planSummaryEntries [functionId] plans := by
  simp [planSummaryEntries, hLookup]

theorem summary_lookup_miss_skips_entry
    (functionId : Nat)
    (plans : List ReducedFunctionPhasePlan)
    (hLookup : lookupCollectedPlan? functionId plans = none) :
    planSummaryEntries [functionId] plans = [] := by
  simp [planSummaryEntries, hLookup]

theorem summary_entry_exposes_schedule
    (plan : ReducedFunctionPhasePlan) :
    (summarizePlan plan).summarySchedule = plan.schedule := by
  rfl

theorem summary_entry_exposes_profile
    (plan : ReducedFunctionPhasePlan) :
    (summarizePlan plan).summaryProfile = plan.profile := by
  rfl

theorem summary_entry_exposes_pass_groups
    (plan : ReducedFunctionPhasePlan) :
    (summarizePlan plan).summaryPassGroups = plan.passGroups := by
  rfl

theorem summary_lookup_preserves_verify_ir
    (functionId : Nat)
    (plans : List ReducedFunctionPhasePlan)
    (plan : ReducedFunctionPhasePlan)
    (_hLookup : lookupCollectedPlan? functionId plans = some plan)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (planSelectedPipeline plan fn) := by
  exact selected_plan_preserves_verify_ir plan h

theorem summary_lookup_preserves_semantics
    (functionId : Nat)
    (plans : List ReducedFunctionPhasePlan)
    (plan : ReducedFunctionPhasePlan)
    (_hLookup : lookupCollectedPlan? functionId plans = some plan)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (planSelectedPipeline plan fn) env = execEntry fn env := by
  exact selected_plan_preserves_semantics plan fn env

end RRProofs.PhasePlanSummarySoundness
