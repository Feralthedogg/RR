import RRProofs.MirInvariantBundle
import RRProofs.PhaseOrderClusterSoundness

namespace RRProofs.PhaseOrderGuardSoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhaseOrderClusterSoundness

structure ReducedPhaseGuards where
  runBudgetedPasses : Bool
  structuralEnabled : Bool
  controlFlowGate : Bool
  fastDevVectorize : Bool
  licmAllowed : Bool
  bceAllowed : Bool
deriving Repr, DecidableEq

def balancedGuardedPipeline (guards : ReducedPhaseGuards) (fn : MirFnLite) : MirFnLite :=
  if guards.runBudgetedPasses then
    if guards.structuralEnabled then
      clusterPipeline .structural fn
    else if guards.fastDevVectorize then
      clusterPipeline .structural fn
    else
      clusterPipeline .standard fn
  else
    clusterPipeline .standard fn

def controlFlowGuardedPipeline (guards : ReducedPhaseGuards) (fn : MirFnLite) : MirFnLite :=
  if guards.runBudgetedPasses && guards.structuralEnabled && guards.controlFlowGate then
    clusterPipeline .structural fn
  else
    clusterPipeline .standard fn

def cleanupGuardedPipeline (_guards : ReducedPhaseGuards) (fn : MirFnLite) : MirFnLite :=
  clusterPipeline .cleanup fn

theorem balanced_guarded_preserves_verify_ir
    (guards : ReducedPhaseGuards)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (balancedGuardedPipeline guards fn) := by
  unfold balancedGuardedPipeline
  by_cases hBudget : guards.runBudgetedPasses
  · simp [hBudget]
    by_cases hStruct : guards.structuralEnabled
    · simp [hStruct]
      exact structural_cluster_preserves_verify_ir h
    · simp [hStruct]
      by_cases hFast : guards.fastDevVectorize
      · simp [hFast]
        exact structural_cluster_preserves_verify_ir h
      · simp [hFast]
        exact standard_cluster_preserves_verify_ir h
  · simp [hBudget]
    exact standard_cluster_preserves_verify_ir h

theorem balanced_guarded_preserves_semantics
    (guards : ReducedPhaseGuards)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (balancedGuardedPipeline guards fn) env = execEntry fn env := by
  unfold balancedGuardedPipeline
  by_cases hBudget : guards.runBudgetedPasses
  · simp [hBudget]
    by_cases hStruct : guards.structuralEnabled
    · simp [hStruct]
      exact structural_cluster_preserves_semantics fn env
    · simp [hStruct]
      by_cases hFast : guards.fastDevVectorize
      · simp [hFast]
        exact structural_cluster_preserves_semantics fn env
      · simp [hFast]
        exact standard_cluster_preserves_semantics fn env
  · simp [hBudget]
    exact standard_cluster_preserves_semantics fn env

theorem control_flow_guarded_preserves_verify_ir
    (guards : ReducedPhaseGuards)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (controlFlowGuardedPipeline guards fn) := by
  unfold controlFlowGuardedPipeline
  by_cases hGate : guards.runBudgetedPasses && guards.structuralEnabled && guards.controlFlowGate
  · simp [hGate]
    exact structural_cluster_preserves_verify_ir h
  · simp [hGate]
    exact standard_cluster_preserves_verify_ir h

theorem control_flow_guarded_preserves_semantics
    (guards : ReducedPhaseGuards)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (controlFlowGuardedPipeline guards fn) env = execEntry fn env := by
  unfold controlFlowGuardedPipeline
  by_cases hGate : guards.runBudgetedPasses && guards.structuralEnabled && guards.controlFlowGate
  · simp [hGate]
    exact structural_cluster_preserves_semantics fn env
  · simp [hGate]
    exact standard_cluster_preserves_semantics fn env

theorem cleanup_guarded_preserves_verify_ir
    (guards : ReducedPhaseGuards)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (cleanupGuardedPipeline guards fn) := by
  exact cleanup_cluster_preserves_verify_ir h

theorem cleanup_guarded_preserves_semantics
    (guards : ReducedPhaseGuards)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (cleanupGuardedPipeline guards fn) env = execEntry fn env := by
  exact cleanup_cluster_preserves_semantics fn env

end RRProofs.PhaseOrderGuardSoundness
