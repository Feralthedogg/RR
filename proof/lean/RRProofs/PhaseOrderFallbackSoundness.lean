import RRProofs.PhaseOrderIterationSoundness

namespace RRProofs.PhaseOrderFallbackSoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhaseOrderIterationSoundness
open RRProofs.PhaseOrderFeatureGateSoundness
open RRProofs.PhaseOrderGuardSoundness

structure ReducedHeavyIterationResult where
  structuralProgress : Bool
  nonStructuralChanges : Nat
deriving Repr, DecidableEq

def controlFlowShouldFallbackToBalanced (result : ReducedHeavyIterationResult) : Bool :=
  !result.structuralProgress && result.nonStructuralChanges <= 1

def controlFlowFallbackPipeline
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    (result : ReducedHeavyIterationResult)
    (fn : MirFnLite) : MirFnLite :=
  if controlFlowShouldFallbackToBalanced result then
    balancedIterationPipeline guards features fn
  else
    controlFlowHeavyIterationPipeline guards features fn

theorem control_flow_fallback_preserves_verify_ir
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    (result : ReducedHeavyIterationResult)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (controlFlowFallbackPipeline guards features result fn) := by
  unfold controlFlowFallbackPipeline
  by_cases hFallback : controlFlowShouldFallbackToBalanced result
  · simp [hFallback]
    exact balanced_iteration_preserves_verify_ir guards features h
  · simp [hFallback]
    exact control_flow_heavy_iteration_preserves_verify_ir guards features h

theorem control_flow_fallback_preserves_semantics
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    (result : ReducedHeavyIterationResult)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (controlFlowFallbackPipeline guards features result fn) env = execEntry fn env := by
  unfold controlFlowFallbackPipeline
  by_cases hFallback : controlFlowShouldFallbackToBalanced result
  · simp [hFallback]
    exact balanced_iteration_preserves_semantics guards features fn env
  · simp [hFallback]
    exact control_flow_heavy_iteration_preserves_semantics guards features fn env

end RRProofs.PhaseOrderFallbackSoundness
