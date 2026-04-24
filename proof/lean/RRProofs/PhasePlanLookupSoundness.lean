import RRProofs.PhasePlanCollectionSoundness

namespace RRProofs.PhasePlanLookupSoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhasePlanSoundness
open RRProofs.PhasePlanCollectionSoundness

def lookupCollectedPlan?
    (functionId : Nat)
    (plans : List ReducedFunctionPhasePlan) : Option ReducedFunctionPhasePlan :=
  plans.find? fun plan => plan.functionId = functionId

theorem lookup_singleton_eligible_returns_plan
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat)
    (features : ReducedPlanFeatures) :
    lookupCollectedPlan? functionId
      (collectFunctionPhasePlans mode traceRequested fastDev
        [{ functionId, features, present := true, conservative := false
         , selfRecursive := false, selected := true }])
      = some (buildFunctionPhasePlan functionId mode traceRequested fastDev features) := by
  simp [lookupCollectedPlan?, collectFunctionPhasePlans, collectSingleFunctionPhasePlan?,
    buildFunctionPhasePlan]

theorem lookup_singleton_missing_returns_none
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat)
    (features : ReducedPlanFeatures) :
    lookupCollectedPlan? functionId
      (collectFunctionPhasePlans mode traceRequested fastDev
        [{ functionId, features, present := false, conservative := false
         , selfRecursive := false, selected := true }])
      = none := by
  simp [lookupCollectedPlan?, collectFunctionPhasePlans, collectSingleFunctionPhasePlan?]

theorem lookup_singleton_conservative_returns_none
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat)
    (features : ReducedPlanFeatures) :
    lookupCollectedPlan? functionId
      (collectFunctionPhasePlans mode traceRequested fastDev
        [{ functionId, features, present := true, conservative := true
         , selfRecursive := false, selected := true }])
      = none := by
  simp [lookupCollectedPlan?, collectFunctionPhasePlans, collectSingleFunctionPhasePlan?]

theorem lookup_singleton_self_recursive_returns_none
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat)
    (features : ReducedPlanFeatures) :
    lookupCollectedPlan? functionId
      (collectFunctionPhasePlans mode traceRequested fastDev
        [{ functionId, features, present := true, conservative := false
         , selfRecursive := true, selected := true }])
      = none := by
  simp [lookupCollectedPlan?, collectFunctionPhasePlans, collectSingleFunctionPhasePlan?]

theorem lookup_singleton_unselected_returns_none
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (functionId : Nat)
    (features : ReducedPlanFeatures) :
    lookupCollectedPlan? functionId
      (collectFunctionPhasePlans mode traceRequested fastDev
        [{ functionId, features, present := true, conservative := false
         , selfRecursive := false, selected := false }])
      = none := by
  simp [lookupCollectedPlan?, collectFunctionPhasePlans, collectSingleFunctionPhasePlan?]

theorem lookup_collected_plan_preserves_verify_ir
    (functionId : Nat)
    (plans : List ReducedFunctionPhasePlan)
    (plan : ReducedFunctionPhasePlan)
    (_hLookup : lookupCollectedPlan? functionId plans = some plan)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (planSelectedPipeline plan fn) := by
  exact selected_plan_preserves_verify_ir plan h

theorem lookup_collected_plan_preserves_semantics
    (functionId : Nat)
    (plans : List ReducedFunctionPhasePlan)
    (plan : ReducedFunctionPhasePlan)
    (_hLookup : lookupCollectedPlan? functionId plans = some plan)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (planSelectedPipeline plan fn) env = execEntry fn env := by
  exact selected_plan_preserves_semantics plan fn env

end RRProofs.PhasePlanLookupSoundness
