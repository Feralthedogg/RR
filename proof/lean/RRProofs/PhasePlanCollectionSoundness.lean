import RRProofs.PhasePlanSoundness

namespace RRProofs.PhasePlanCollectionSoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhasePlanSoundness

structure ReducedFunctionInventoryEntry where
  functionId : Nat
  features : ReducedPlanFeatures
  present : Bool
  conservative : Bool
  selfRecursive : Bool
  selected : Bool
deriving Repr, DecidableEq

def collectSingleFunctionPhasePlan?
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (entry : ReducedFunctionInventoryEntry) : Option ReducedFunctionPhasePlan :=
  if entry.present && !entry.conservative && !entry.selfRecursive && entry.selected then
    some (buildFunctionPhasePlan entry.functionId mode traceRequested fastDev entry.features)
  else
    none

def collectFunctionPhasePlans
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (entries : List ReducedFunctionInventoryEntry) : List ReducedFunctionPhasePlan :=
  entries.filterMap (collectSingleFunctionPhasePlan? mode traceRequested fastDev)

theorem collect_single_skips_missing
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat) (features : ReducedPlanFeatures)
    (conservative selfRecursive selected : Bool) :
    collectSingleFunctionPhasePlan? mode traceRequested fastDev
      { functionId, features, present := false, conservative, selfRecursive, selected } = none := by
  simp [collectSingleFunctionPhasePlan?]

theorem collect_single_skips_conservative
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat) (features : ReducedPlanFeatures)
    (present selfRecursive selected : Bool) :
    collectSingleFunctionPhasePlan? mode traceRequested fastDev
      { functionId, features, present, conservative := true, selfRecursive, selected } = none := by
  by_cases hPresent : present
  · simp [collectSingleFunctionPhasePlan?, hPresent]
  · simp [collectSingleFunctionPhasePlan?, hPresent]

theorem collect_single_skips_self_recursive
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat) (features : ReducedPlanFeatures)
    (present conservative selected : Bool) :
    collectSingleFunctionPhasePlan? mode traceRequested fastDev
      { functionId, features, present, conservative, selfRecursive := true, selected } = none := by
  by_cases hPresent : present
  · by_cases hConservative : conservative
    · simp [collectSingleFunctionPhasePlan?, hPresent, hConservative]
    · simp [collectSingleFunctionPhasePlan?, hPresent, hConservative]
  · simp [collectSingleFunctionPhasePlan?, hPresent]

theorem collect_single_skips_unselected
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat) (features : ReducedPlanFeatures)
    (present conservative selfRecursive : Bool) :
    collectSingleFunctionPhasePlan? mode traceRequested fastDev
      { functionId, features, present, conservative, selfRecursive, selected := false } = none := by
  by_cases hPresent : present
  · by_cases hConservative : conservative
    · simp [collectSingleFunctionPhasePlan?, hPresent, hConservative]
    · by_cases hSelf : selfRecursive
      · simp [collectSingleFunctionPhasePlan?, hPresent, hConservative, hSelf]
      · simp [collectSingleFunctionPhasePlan?, hPresent, hConservative, hSelf]
  · simp [collectSingleFunctionPhasePlan?, hPresent]

theorem collect_single_builds_plan_when_eligible
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat) (features : ReducedPlanFeatures) :
    collectSingleFunctionPhasePlan? mode traceRequested fastDev
      { functionId, features, present := true, conservative := false, selfRecursive := false, selected := true }
      = some (buildFunctionPhasePlan functionId mode traceRequested fastDev features) := by
  simp [collectSingleFunctionPhasePlan?]

theorem collected_plan_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (entry : ReducedFunctionInventoryEntry)
    (plan : ReducedFunctionPhasePlan)
    (_hCollect :
      collectSingleFunctionPhasePlan? mode traceRequested fastDev entry = some plan)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (planSelectedPipeline plan fn) := by
  exact selected_plan_preserves_verify_ir plan h

theorem collected_plan_preserves_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (entry : ReducedFunctionInventoryEntry)
    (plan : ReducedFunctionPhasePlan)
    (_hCollect :
      collectSingleFunctionPhasePlan? mode traceRequested fastDev entry = some plan)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (planSelectedPipeline plan fn) env = execEntry fn env := by
  exact selected_plan_preserves_semantics plan fn env

theorem all_collected_plans_preserve_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (entries : List ReducedFunctionInventoryEntry)
    (plan : ReducedFunctionPhasePlan)
    (_hMem : plan ∈ collectFunctionPhasePlans mode traceRequested fastDev entries)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (planSelectedPipeline plan fn) := by
  exact selected_plan_preserves_verify_ir plan h

theorem all_collected_plans_preserve_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (entries : List ReducedFunctionInventoryEntry)
    (plan : ReducedFunctionPhasePlan)
    (_hMem : plan ∈ collectFunctionPhasePlans mode traceRequested fastDev entries)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (planSelectedPipeline plan fn) env = execEntry fn env := by
  exact selected_plan_preserves_semantics plan fn env

end RRProofs.PhasePlanCollectionSoundness
