import RRProofs.ProgramOptPlanSoundness
import RRProofs.PhasePlanLookupSoundness
import RRProofs.ProgramPhasePipelineSoundness

namespace RRProofs.ProgramTierExecutionSoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhasePlanSoundness
open RRProofs.ProgramOptPlanSoundness
open RRProofs.PhasePlanLookupSoundness
open RRProofs.ProgramPhasePipelineSoundness

inductive ReducedHeavyTierDecision where
  | skipConservative
  | skipSelfRecursive
  | skipHeavyTierDisabled
  | skipBudget
  | useCollectedPlan
  | useLegacyPlan
deriving Repr, DecidableEq

def legacyFunctionPhasePlan
    (functionId : Nat)
    (mode : ReducedPhaseOrderingMode)
    (traceRequested : Bool)
    (features : ReducedPlanFeatures) : ReducedFunctionPhasePlan :=
  { functionId
  , mode
  , profile := .balanced
  , schedule := .balanced
  , passGroups := defaultPassGroupsForSchedule .balanced
  , features
  , traceRequested
  }

def executeHeavyTierDecision
    (mode : ReducedPhaseOrderingMode)
    (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    (decision : ReducedHeavyTierDecision)
    (selectedPlan? : Option ReducedFunctionPhasePlan)
    (fn : MirFnLite) : MirFnLite :=
  match decision with
  | .skipConservative
  | .skipSelfRecursive
  | .skipHeavyTierDisabled
  | .skipBudget => fn
  | .useCollectedPlan =>
      match selectedPlan? with
      | some selectedPlan => planSelectedPipeline selectedPlan fn
      | none => fn
  | .useLegacyPlan =>
      planSelectedPipeline (legacyFunctionPhasePlan entry.functionId mode traceRequested entry.features) fn

def executeProgramHeavyFunction
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) : MirFnLite :=
  if entry.conservative then
    executeHeavyTierDecision mode traceRequested entry .skipConservative none fn
  else if entry.selfRecursive then
    executeHeavyTierDecision mode traceRequested entry .skipSelfRecursive none fn
  else if !runHeavyTier then
    executeHeavyTierDecision mode traceRequested entry .skipHeavyTierDisabled none fn
  else if !(selectedByProgramPlan plan entry) then
    executeHeavyTierDecision mode traceRequested entry .skipBudget none fn
  else
    match lookupCollectedPlan? entry.functionId
        (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries) with
    | some selectedPlan =>
        executeHeavyTierDecision mode traceRequested entry .useCollectedPlan (some selectedPlan) fn
    | none =>
        executeHeavyTierDecision mode traceRequested entry .useLegacyPlan none fn

theorem skip_conservative_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (executeHeavyTierDecision mode traceRequested entry .skipConservative none fn) := by
  simpa [executeHeavyTierDecision]

theorem skip_conservative_preserves_semantics
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (executeHeavyTierDecision mode traceRequested entry .skipConservative none fn) env
      = execEntry fn env := by
  simp [executeHeavyTierDecision]

theorem skip_self_recursive_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (executeHeavyTierDecision mode traceRequested entry .skipSelfRecursive none fn) := by
  simpa [executeHeavyTierDecision]

theorem skip_self_recursive_preserves_semantics
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (executeHeavyTierDecision mode traceRequested entry .skipSelfRecursive none fn) env
      = execEntry fn env := by
  simp [executeHeavyTierDecision]

theorem skip_heavy_tier_disabled_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (executeHeavyTierDecision mode traceRequested entry .skipHeavyTierDisabled none fn) := by
  simpa [executeHeavyTierDecision]

theorem skip_heavy_tier_disabled_preserves_semantics
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (executeHeavyTierDecision mode traceRequested entry .skipHeavyTierDisabled none fn) env
      = execEntry fn env := by
  simp [executeHeavyTierDecision]

theorem skip_budget_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (executeHeavyTierDecision mode traceRequested entry .skipBudget none fn) := by
  simpa [executeHeavyTierDecision]

theorem skip_budget_preserves_semantics
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (executeHeavyTierDecision mode traceRequested entry .skipBudget none fn) env
      = execEntry fn env := by
  simp [executeHeavyTierDecision]

theorem legacy_plan_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (executeHeavyTierDecision mode traceRequested entry .useLegacyPlan none fn) := by
  exact selected_plan_preserves_verify_ir _ h

theorem legacy_plan_preserves_semantics
    (mode : ReducedPhaseOrderingMode) (traceRequested : Bool)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (executeHeavyTierDecision mode traceRequested entry .useLegacyPlan none fn) env
      = execEntry fn env := by
  exact selected_plan_preserves_semantics _ fn env

theorem collected_plan_hit_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (selectedPlan : ReducedFunctionPhasePlan)
    (hLookup :
      lookupCollectedPlan? entry.functionId
        (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries)
        = some selectedPlan)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible
      (executeHeavyTierDecision mode traceRequested entry .useCollectedPlan (some selectedPlan) fn) := by
  simpa [executeHeavyTierDecision] using
    program_phase_lookup_preserves_verify_ir mode traceRequested fastDev runHeavyTier plan entries
      entry.functionId selectedPlan hLookup h

theorem collected_plan_hit_preserves_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (selectedPlan : ReducedFunctionPhasePlan)
    (hLookup :
      lookupCollectedPlan? entry.functionId
        (collectProgramPhasePlans mode traceRequested fastDev runHeavyTier plan entries)
        = some selectedPlan)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry
      (executeHeavyTierDecision mode traceRequested entry .useCollectedPlan (some selectedPlan) fn) env
      = execEntry fn env := by
  simpa [executeHeavyTierDecision] using
    program_phase_lookup_preserves_semantics mode traceRequested fastDev runHeavyTier plan entries
      entry.functionId selectedPlan hLookup fn env

theorem execute_program_heavy_function_preserves_verify_ir
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (executeProgramHeavyFunction mode traceRequested fastDev runHeavyTier plan entries entry fn) := by
  unfold executeProgramHeavyFunction
  by_cases hConservative : entry.conservative
  · simp [hConservative, executeHeavyTierDecision]
    exact h
  · by_cases hSelf : entry.selfRecursive
    · simp [hConservative, hSelf, executeHeavyTierDecision]
      exact h
    · by_cases hHeavy : runHeavyTier
      · by_cases hSelected : selectedByProgramPlan plan entry
        · simp [hConservative, hSelf, hHeavy, hSelected]
          split
          · rename_i selectedPlan hLookup
            exact collected_plan_hit_preserves_verify_ir mode traceRequested fastDev true plan entries entry selectedPlan hLookup h
          · exact legacy_plan_preserves_verify_ir mode traceRequested entry h
        · simp [hConservative, hSelf, hHeavy, hSelected, executeHeavyTierDecision]
          exact h
      · simp [hConservative, hSelf, hHeavy, executeHeavyTierDecision]
        exact h

theorem execute_program_heavy_function_preserves_semantics
    (mode : ReducedPhaseOrderingMode)
    (traceRequested fastDev runHeavyTier : Bool)
    (plan : ReducedProgramOptPlan)
    (entries : List ReducedProgramPhaseEntry)
    (entry : ReducedProgramPhaseEntry)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (executeProgramHeavyFunction mode traceRequested fastDev runHeavyTier plan entries entry fn) env
      = execEntry fn env := by
  unfold executeProgramHeavyFunction
  by_cases hConservative : entry.conservative
  · simp [hConservative, executeHeavyTierDecision]
  · by_cases hSelf : entry.selfRecursive
    · simp [hConservative, hSelf, executeHeavyTierDecision]
    · by_cases hHeavy : runHeavyTier
      · by_cases hSelected : selectedByProgramPlan plan entry
        · simp [hConservative, hSelf, hHeavy, hSelected]
          split
          · rename_i selectedPlan hLookup
            exact collected_plan_hit_preserves_semantics mode traceRequested fastDev true plan entries entry selectedPlan hLookup fn env
          · exact legacy_plan_preserves_semantics mode traceRequested entry fn env
        · simp [hConservative, hSelf, hHeavy, hSelected, executeHeavyTierDecision]
      · simp [hConservative, hSelf, hHeavy, executeHeavyTierDecision]

end RRProofs.ProgramTierExecutionSoundness
