import RRProofs.PhaseOrderOptimizerSoundness

namespace RRProofs.PhasePlanSoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhaseOrderOptimizerSoundness

inductive ReducedPhaseOrderingMode where
  | off
  | balanced
  | auto
deriving Repr, DecidableEq

inductive ReducedPhaseProfileKind where
  | balanced
  | computeHeavy
  | controlFlowHeavy
deriving Repr, DecidableEq

inductive ReducedPassGroup where
  | required
  | devCheap
  | releaseExpensive
  | experimental
deriving Repr, DecidableEq

structure ReducedPlanFeatures where
  irSize : Nat
  blockCount : Nat
  loopCount : Nat
  canonicalLoopCount : Nat
  branchTerms : Nat
  phiCount : Nat
  arithmeticValues : Nat
  intrinsicValues : Nat
  callValues : Nat
  sideEffectingCalls : Nat
  indexValues : Nat
  storeInstrs : Nat
deriving Repr, DecidableEq

structure ReducedFunctionPhasePlan where
  functionId : Nat
  mode : ReducedPhaseOrderingMode
  profile : ReducedPhaseProfileKind
  schedule : ReducedPhaseSchedule
  passGroups : List ReducedPassGroup
  features : ReducedPlanFeatures
  traceRequested : Bool
deriving Repr, DecidableEq

def computePhaseProfileScores (features : ReducedPlanFeatures) : Nat × Nat :=
  let computeScore :=
    features.canonicalLoopCount * 32
      + features.loopCount * 16
      + features.arithmeticValues * 2
      + features.intrinsicValues * 4
      + features.indexValues * 2
      + features.storeInstrs * 2
  let controlScore :=
    features.branchTerms * 18
      + features.phiCount * 8
      + features.sideEffectingCalls * 16
  (computeScore, controlScore)

def classifyPhaseProfile (features : ReducedPlanFeatures) : ReducedPhaseProfileKind :=
  let (computeScore, controlScore) := computePhaseProfileScores features
  let branchDensityHigh := features.branchTerms * 3 >= max features.blockCount 1
  let sideEffectsLight := features.sideEffectingCalls * 4 <= max features.callValues 1
  let computeScheduleSafe :=
    features.irSize <= 256
      && features.blockCount <= 16
      && features.canonicalLoopCount > 0
      && features.sideEffectingCalls = 0
  let controlScheduleSafe :=
    features.irSize <= 128
      && features.blockCount <= 12
      && features.loopCount = 0
  if computeScheduleSafe
      && sideEffectsLight
      && controlScore + 24 <= computeScore then
    .computeHeavy
  else if controlScheduleSafe
      && (computeScore + 24 <= controlScore || branchDensityHigh) then
    .controlFlowHeavy
  else
    .balanced

def choosePhaseSchedule
    (mode : ReducedPhaseOrderingMode)
    (profile : ReducedPhaseProfileKind) : ReducedPhaseSchedule :=
  match mode with
  | .off | .balanced => .balanced
  | .auto =>
      match profile with
      | .balanced => .balanced
      | .computeHeavy => .computeHeavy
      | .controlFlowHeavy => .controlFlowHeavy

def defaultPassGroupsForSchedule (schedule : ReducedPhaseSchedule) : List ReducedPassGroup :=
  match schedule with
  | .balanced | .controlFlowHeavy => [.required, .devCheap, .releaseExpensive]
  | .computeHeavy => [.required, .devCheap, .releaseExpensive, .experimental]

def adjustPassGroupsForFastDev (fastDev : Bool) (groups : List ReducedPassGroup) :
    List ReducedPassGroup :=
  groups.filter fun group =>
    match group with
    | .required | .devCheap => true
    | .releaseExpensive | .experimental => !fastDev

def buildFunctionPhasePlan
    (functionId : Nat)
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool)
    (features : ReducedPlanFeatures) : ReducedFunctionPhasePlan :=
  let profile := if mode = .auto then classifyPhaseProfile features else .balanced
  let schedule := choosePhaseSchedule mode profile
  { functionId
  , mode
  , profile
  , schedule
  , passGroups := adjustPassGroupsForFastDev fastDev (defaultPassGroupsForSchedule schedule)
  , features
  , traceRequested
  }

def planSelectedPipeline (plan : ReducedFunctionPhasePlan) (fn : MirFnLite) : MirFnLite :=
  phaseScheduledPipeline plan.schedule fn

def computeHeavySample : ReducedPlanFeatures :=
  { irSize := 64
  , blockCount := 6
  , loopCount := 2
  , canonicalLoopCount := 2
  , branchTerms := 1
  , phiCount := 1
  , arithmeticValues := 12
  , intrinsicValues := 4
  , callValues := 0
  , sideEffectingCalls := 0
  , indexValues := 3
  , storeInstrs := 2
  }

def controlFlowHeavySample : ReducedPlanFeatures :=
  { irSize := 48
  , blockCount := 4
  , loopCount := 0
  , canonicalLoopCount := 0
  , branchTerms := 3
  , phiCount := 2
  , arithmeticValues := 0
  , intrinsicValues := 0
  , callValues := 2
  , sideEffectingCalls := 2
  , indexValues := 0
  , storeInstrs := 0
  }

def balancedSample : ReducedPlanFeatures :=
  { irSize := 200
  , blockCount := 20
  , loopCount := 1
  , canonicalLoopCount := 0
  , branchTerms := 2
  , phiCount := 1
  , arithmeticValues := 1
  , intrinsicValues := 0
  , callValues := 2
  , sideEffectingCalls := 1
  , indexValues := 0
  , storeInstrs := 0
  }

theorem compute_heavy_sample_classifies_compute_heavy :
    classifyPhaseProfile computeHeavySample = .computeHeavy := by
  decide

theorem control_flow_heavy_sample_classifies_control_flow_heavy :
    classifyPhaseProfile controlFlowHeavySample = .controlFlowHeavy := by
  decide

theorem balanced_sample_classifies_balanced :
    classifyPhaseProfile balancedSample = .balanced := by
  decide

theorem choose_phase_schedule_off_is_balanced
    (profile : ReducedPhaseProfileKind) :
    choosePhaseSchedule .off profile = .balanced := by
  cases profile <;> rfl

theorem choose_phase_schedule_balanced_mode_is_balanced
    (profile : ReducedPhaseProfileKind) :
    choosePhaseSchedule .balanced profile = .balanced := by
  cases profile <;> rfl

theorem choose_phase_schedule_auto_uses_profile
    (profile : ReducedPhaseProfileKind) :
    choosePhaseSchedule .auto profile =
      match profile with
      | .balanced => .balanced
      | .computeHeavy => .computeHeavy
      | .controlFlowHeavy => .controlFlowHeavy := by
  cases profile <;> rfl

theorem fast_dev_group_filter_drops_expensive_groups
    (schedule : ReducedPhaseSchedule) :
    adjustPassGroupsForFastDev true (defaultPassGroupsForSchedule schedule)
      = [.required, .devCheap] := by
  cases schedule <;> rfl

theorem build_phase_plan_non_auto_uses_balanced_profile
    (functionId : Nat) (traceRequested fastDev : Bool)
    (features : ReducedPlanFeatures) :
    (buildFunctionPhasePlan functionId .balanced traceRequested fastDev features).profile = .balanced := by
  simp [buildFunctionPhasePlan]

theorem build_phase_plan_auto_uses_classified_profile
    (functionId : Nat) (traceRequested fastDev : Bool)
    (features : ReducedPlanFeatures) :
    (buildFunctionPhasePlan functionId .auto traceRequested fastDev features).profile
      = classifyPhaseProfile features := by
  simp [buildFunctionPhasePlan]

theorem build_phase_plan_schedule_matches_choice
    (functionId : Nat) (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev : Bool) (features : ReducedPlanFeatures) :
    (buildFunctionPhasePlan functionId mode traceRequested fastDev features).schedule
      = choosePhaseSchedule mode
          ((buildFunctionPhasePlan functionId mode traceRequested fastDev features).profile) := by
  unfold buildFunctionPhasePlan
  split <;> rfl

theorem selected_plan_preserves_verify_ir
    (plan : ReducedFunctionPhasePlan)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (planSelectedPipeline plan fn) := by
  unfold planSelectedPipeline
  cases plan.schedule with
  | balanced => exact phase_schedule_balanced_preserves_verify_ir h
  | computeHeavy => exact phase_schedule_compute_heavy_preserves_verify_ir h
  | controlFlowHeavy => exact phase_schedule_control_flow_heavy_preserves_verify_ir h

theorem selected_plan_preserves_semantics
    (plan : ReducedFunctionPhasePlan)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (planSelectedPipeline plan fn) env = execEntry fn env := by
  unfold planSelectedPipeline
  cases plan.schedule with
  | balanced => exact phase_schedule_balanced_preserves_semantics fn env
  | computeHeavy => exact phase_schedule_compute_heavy_preserves_semantics fn env
  | controlFlowHeavy => exact phase_schedule_control_flow_heavy_preserves_semantics fn env

end RRProofs.PhasePlanSoundness
