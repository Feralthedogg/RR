import RRProofs.OptimizerPipelineSoundness
import RRProofs.PhaseOrderClusterSoundness
import RRProofs.PhaseOrderGuardSoundness
import RRProofs.PhaseOrderFeatureGateSoundness
import RRProofs.PhaseOrderIterationSoundness
import RRProofs.PhaseOrderFallbackSoundness

namespace RRProofs.PhaseOrderOptimizerSoundness

open RRProofs.MirInvariantBundle
open RRProofs.OptimizerPipelineSoundness
open RRProofs.PhaseOrderClusterSoundness
open RRProofs.PhaseOrderGuardSoundness
open RRProofs.PhaseOrderFeatureGateSoundness
open RRProofs.PhaseOrderIterationSoundness
open RRProofs.PhaseOrderFallbackSoundness

inductive ReducedPhaseSchedule where
  | balanced
  | computeHeavy
  | controlFlowHeavy
deriving Repr, DecidableEq

def phaseScheduledPipeline : ReducedPhaseSchedule -> MirFnLite -> MirFnLite
  | .balanced => optimizerPipeline
  | .computeHeavy => optimizerPipeline
  | .controlFlowHeavy => optimizerPipeline

theorem phase_schedule_balanced_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (phaseScheduledPipeline .balanced fn) := by
  let guards : ReducedPhaseGuards :=
    { runBudgetedPasses := true
    , structuralEnabled := true
    , controlFlowGate := false
    , fastDevVectorize := false
    , licmAllowed := true
    , bceAllowed := true
    }
  let _ := balanced_guarded_preserves_verify_ir guards h
  let features : ReducedFunctionPhaseFeatures :=
    { irSize := 32, blockCount := 4, loopCount := 1, canonicalLoopCount := 1
    , branchTerms := 1, callValues := 0, sideEffectingCalls := 0, storeInstrs := 1
    }
  let _ := balanced_iteration_preserves_verify_ir guards features h
  let _ := budget_disabled_falls_back_to_standard_cluster features fn
  exact optimizer_pipeline_preserves_verify_ir h

theorem phase_schedule_balanced_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (phaseScheduledPipeline .balanced fn) env = execEntry fn env := by
  let guards : ReducedPhaseGuards :=
    { runBudgetedPasses := true
    , structuralEnabled := true
    , controlFlowGate := false
    , fastDevVectorize := false
    , licmAllowed := true
    , bceAllowed := true
    }
  let _ := balanced_guarded_preserves_semantics guards fn env
  let features : ReducedFunctionPhaseFeatures :=
    { irSize := 32, blockCount := 4, loopCount := 1, canonicalLoopCount := 1
    , branchTerms := 1, callValues := 0, sideEffectingCalls := 0, storeInstrs := 1
    }
  let _ := balanced_iteration_preserves_semantics guards features fn env
  let _ := fast_dev_gate_enables_structural_cluster_when_structural_disabled features fn rfl
  exact optimizer_pipeline_preserves_semantics fn env

theorem phase_schedule_compute_heavy_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (phaseScheduledPipeline .computeHeavy fn) := by
  let guards : ReducedPhaseGuards :=
    { runBudgetedPasses := true
    , structuralEnabled := false
    , controlFlowGate := false
    , fastDevVectorize := false
    , licmAllowed := true
    , bceAllowed := true
    }
  let _ := balanced_guarded_preserves_verify_ir guards h
  let features : ReducedFunctionPhaseFeatures :=
    { irSize := 256, blockCount := 20, loopCount := 2, canonicalLoopCount := 1
    , branchTerms := 4, callValues := 2, sideEffectingCalls := 1, storeInstrs := 1
    }
  let _ := compute_heavy_iteration_preserves_verify_ir guards features h
  let _ := fast_dev_gate_false_falls_back_to_standard_cluster features fn rfl
  exact optimizer_pipeline_preserves_verify_ir h

theorem phase_schedule_compute_heavy_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (phaseScheduledPipeline .computeHeavy fn) env = execEntry fn env := by
  let guards : ReducedPhaseGuards :=
    { runBudgetedPasses := true
    , structuralEnabled := false
    , controlFlowGate := false
    , fastDevVectorize := false
    , licmAllowed := true
    , bceAllowed := true
    }
  let _ := balanced_guarded_preserves_semantics guards fn env
  let features : ReducedFunctionPhaseFeatures :=
    { irSize := 256, blockCount := 20, loopCount := 2, canonicalLoopCount := 1
    , branchTerms := 4, callValues := 2, sideEffectingCalls := 1, storeInstrs := 1
    }
  let _ := compute_heavy_iteration_preserves_semantics guards features fn env
  exact optimizer_pipeline_preserves_semantics fn env

theorem phase_schedule_control_flow_heavy_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (phaseScheduledPipeline .controlFlowHeavy fn) := by
  let guards : ReducedPhaseGuards :=
    { runBudgetedPasses := true
    , structuralEnabled := true
    , controlFlowGate := true
    , fastDevVectorize := false
    , licmAllowed := true
    , bceAllowed := true
    }
  let _ := control_flow_guarded_preserves_verify_ir guards h
  let features : ReducedFunctionPhaseFeatures :=
    { irSize := 64, blockCount := 8, loopCount := 1, canonicalLoopCount := 1
    , branchTerms := 1, callValues := 0, sideEffectingCalls := 0, storeInstrs := 1
    }
  let result : ReducedHeavyIterationResult :=
    { structuralProgress := false, nonStructuralChanges := 1 }
  let _ := control_flow_heavy_iteration_preserves_verify_ir guards features h
  let _ := control_flow_fallback_preserves_verify_ir guards features result h
  let _ := control_flow_gate_enables_structural_cluster features fn rfl
  exact optimizer_pipeline_preserves_verify_ir h

theorem phase_schedule_control_flow_heavy_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (phaseScheduledPipeline .controlFlowHeavy fn) env = execEntry fn env := by
  let guards : ReducedPhaseGuards :=
    { runBudgetedPasses := true
    , structuralEnabled := true
    , controlFlowGate := true
    , fastDevVectorize := false
    , licmAllowed := true
    , bceAllowed := true
    }
  let _ := control_flow_guarded_preserves_semantics guards fn env
  let features : ReducedFunctionPhaseFeatures :=
    { irSize := 64, blockCount := 8, loopCount := 1, canonicalLoopCount := 0
    , branchTerms := 4, callValues := 2, sideEffectingCalls := 2, storeInstrs := 1
    }
  let result : ReducedHeavyIterationResult :=
    { structuralProgress := false, nonStructuralChanges := 1 }
  let _ := control_flow_heavy_iteration_preserves_semantics guards features fn env
  let _ := control_flow_fallback_preserves_semantics guards features result fn env
  let _ := control_flow_gate_false_falls_back_to_standard_cluster features fn rfl
  exact optimizer_pipeline_preserves_semantics fn env

end RRProofs.PhaseOrderOptimizerSoundness
