import RRProofs.PhaseOrderGuardSoundness

namespace RRProofs.PhaseOrderFeatureGateSoundness

open RRProofs.PhaseOrderGuardSoundness
open RRProofs.PhaseOrderClusterSoundness
open RRProofs.MirInvariantBundle

structure ReducedFunctionPhaseFeatures where
  irSize : Nat
  blockCount : Nat
  loopCount : Nat
  canonicalLoopCount : Nat
  branchTerms : Nat
  callValues : Nat
  sideEffectingCalls : Nat
  storeInstrs : Nat
deriving Repr, DecidableEq

def phaseBranchDensityHigh (features : ReducedFunctionPhaseFeatures) : Bool :=
  features.branchTerms * 3 >= max features.blockCount 1

def controlFlowStructuralGate (features : ReducedFunctionPhaseFeatures) : Bool :=
  let branchDensityHigh := phaseBranchDensityHigh features
  let sideEffectsDominant := features.sideEffectingCalls * 2 > max features.callValues 1
  features.canonicalLoopCount > 0 && !branchDensityHigh && !sideEffectsDominant

def fastDevVectorizeGate (features : ReducedFunctionPhaseFeatures) : Bool :=
  features.canonicalLoopCount > 0
    && features.loopCount <= 1
    && features.irSize <= 128
    && features.blockCount <= 12
    && features.branchTerms <= 2
    && features.sideEffectingCalls <= 1
    && features.storeInstrs > 0

def guardsFromFeatures
    (runBudgeted structuralEnabled licmAllowed bceAllowed : Bool)
    (features : ReducedFunctionPhaseFeatures) : ReducedPhaseGuards :=
  { runBudgetedPasses := runBudgeted
  , structuralEnabled := structuralEnabled
  , controlFlowGate := controlFlowStructuralGate features
  , fastDevVectorize := fastDevVectorizeGate features
  , licmAllowed := licmAllowed
  , bceAllowed := bceAllowed
  }

theorem control_flow_gate_enables_structural_cluster
    (features : ReducedFunctionPhaseFeatures) (fn : MirFnLite)
    (hGate : controlFlowStructuralGate features = true) :
    controlFlowGuardedPipeline (guardsFromFeatures true true true true features) fn =
      clusterPipeline .structural fn := by
  unfold controlFlowGuardedPipeline guardsFromFeatures
  simp [hGate]

theorem control_flow_gate_false_falls_back_to_standard_cluster
    (features : ReducedFunctionPhaseFeatures) (fn : MirFnLite)
    (hGate : controlFlowStructuralGate features = false) :
    controlFlowGuardedPipeline (guardsFromFeatures true true true true features) fn =
      clusterPipeline .standard fn := by
  unfold controlFlowGuardedPipeline guardsFromFeatures
  simp [hGate]

theorem fast_dev_gate_enables_structural_cluster_when_structural_disabled
    (features : ReducedFunctionPhaseFeatures) (fn : MirFnLite)
    (hGate : fastDevVectorizeGate features = true) :
    balancedGuardedPipeline (guardsFromFeatures true false true true features) fn =
      clusterPipeline .structural fn := by
  unfold balancedGuardedPipeline guardsFromFeatures
  simp [hGate]

theorem fast_dev_gate_false_falls_back_to_standard_cluster
    (features : ReducedFunctionPhaseFeatures) (fn : MirFnLite)
    (hGate : fastDevVectorizeGate features = false) :
    balancedGuardedPipeline (guardsFromFeatures true false true true features) fn =
      clusterPipeline .standard fn := by
  unfold balancedGuardedPipeline guardsFromFeatures
  simp [hGate]

theorem budget_disabled_falls_back_to_standard_cluster
    (features : ReducedFunctionPhaseFeatures) (fn : MirFnLite) :
    balancedGuardedPipeline (guardsFromFeatures false true true true features) fn =
      clusterPipeline .standard fn := by
  unfold balancedGuardedPipeline guardsFromFeatures
  simp

end RRProofs.PhaseOrderFeatureGateSoundness
