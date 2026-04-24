import RRProofs.PhaseOrderFeatureGateSoundness

namespace RRProofs.PhaseOrderIterationSoundness

open RRProofs.MirInvariantBundle
open RRProofs.PhaseOrderClusterSoundness
open RRProofs.PhaseOrderGuardSoundness
open RRProofs.PhaseOrderFeatureGateSoundness

def fastDevSubpathPipeline (fn : MirFnLite) : MirFnLite :=
  clusterPipeline .structural fn

def balancedIterationPipeline
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    (fn : MirFnLite) : MirFnLite :=
  let afterStructural :=
    if guards.runBudgetedPasses then
      if guards.structuralEnabled then
        clusterPipeline .structural fn
      else if fastDevVectorizeGate features then
        clusterPipeline .structural fn
      else
        fn
    else
      fn
  let afterCleanup :=
    if guards.runBudgetedPasses && guards.structuralEnabled then
      clusterPipeline .cleanup afterStructural
    else
      afterStructural
  clusterPipeline .standard afterCleanup

def computeHeavyIterationPipeline
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    (fn : MirFnLite) : MirFnLite :=
  let afterStandard := clusterPipeline .standard fn
  let afterStructural :=
    if guards.runBudgetedPasses then
      if guards.structuralEnabled then
        clusterPipeline .structural afterStandard
      else if fastDevVectorizeGate features then
        clusterPipeline .structural afterStandard
      else
        afterStandard
    else
      afterStandard
  if guards.runBudgetedPasses && guards.structuralEnabled then
    clusterPipeline .cleanup afterStructural
  else
    afterStructural

def controlFlowHeavyIterationPipeline
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    (fn : MirFnLite) : MirFnLite :=
  let afterStandard := clusterPipeline .standard fn
  let afterStructural :=
    if guards.runBudgetedPasses then
      if guards.structuralEnabled && controlFlowStructuralGate features then
        clusterPipeline .structural afterStandard
      else if !guards.structuralEnabled && fastDevVectorizeGate features then
        clusterPipeline .structural afterStandard
      else
        afterStandard
    else
      afterStandard
  if guards.runBudgetedPasses && guards.structuralEnabled && controlFlowStructuralGate features then
    clusterPipeline .cleanup afterStructural
  else
    afterStructural

theorem fast_dev_subpath_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (fastDevSubpathPipeline fn) := by
  exact structural_cluster_preserves_verify_ir h

theorem fast_dev_subpath_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (fastDevSubpathPipeline fn) env = execEntry fn env := by
  exact structural_cluster_preserves_semantics fn env

theorem balanced_iteration_preserves_verify_ir
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (balancedIterationPipeline guards features fn) := by
  unfold balancedIterationPipeline
  by_cases hBudget : guards.runBudgetedPasses
  · simp [hBudget]
    by_cases hStruct : guards.structuralEnabled
    · simp [hStruct]
      exact standard_cluster_preserves_verify_ir
        (cleanup_cluster_preserves_verify_ir
          (structural_cluster_preserves_verify_ir h))
    · simp [hStruct]
      by_cases hFast : fastDevVectorizeGate features
      · simp [hFast]
        exact standard_cluster_preserves_verify_ir
          (structural_cluster_preserves_verify_ir h)
      · simp [hFast]
        exact standard_cluster_preserves_verify_ir h
  · simp [hBudget]
    exact standard_cluster_preserves_verify_ir h

theorem balanced_iteration_preserves_semantics
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (balancedIterationPipeline guards features fn) env = execEntry fn env := by
  unfold balancedIterationPipeline
  by_cases hBudget : guards.runBudgetedPasses
  · simp [hBudget]
    by_cases hStruct : guards.structuralEnabled
    · simp [hStruct]
      calc
        execEntry (clusterPipeline .standard (clusterPipeline .cleanup (clusterPipeline .structural fn))) env
            = execEntry (clusterPipeline .cleanup (clusterPipeline .structural fn)) env := by
                exact standard_cluster_preserves_semantics _ _
        _ = execEntry (clusterPipeline .structural fn) env := by
              exact cleanup_cluster_preserves_semantics _ _
        _ = execEntry fn env := by
              exact structural_cluster_preserves_semantics _ _
    · simp [hStruct]
      by_cases hFast : fastDevVectorizeGate features
      · simp [hFast]
        calc
          execEntry (clusterPipeline .standard (clusterPipeline .structural fn)) env
              = execEntry (clusterPipeline .structural fn) env := by
                  exact standard_cluster_preserves_semantics _ _
          _ = execEntry fn env := by
                exact structural_cluster_preserves_semantics _ _
      · simp [hFast]
        exact standard_cluster_preserves_semantics fn env
  · simp [hBudget]
    exact standard_cluster_preserves_semantics fn env

theorem compute_heavy_iteration_preserves_verify_ir
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (computeHeavyIterationPipeline guards features fn) := by
  unfold computeHeavyIterationPipeline
  by_cases hBudget : guards.runBudgetedPasses
  · simp [hBudget]
    by_cases hStruct : guards.structuralEnabled
    · simp [hStruct]
      exact cleanup_cluster_preserves_verify_ir
        (structural_cluster_preserves_verify_ir
          (standard_cluster_preserves_verify_ir h))
    · simp [hStruct]
      by_cases hFast : fastDevVectorizeGate features
      · simp [hFast]
        exact structural_cluster_preserves_verify_ir
          (standard_cluster_preserves_verify_ir h)
      · simp [hFast]
        exact standard_cluster_preserves_verify_ir h
  · simp [hBudget]
    exact standard_cluster_preserves_verify_ir h

theorem compute_heavy_iteration_preserves_semantics
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (computeHeavyIterationPipeline guards features fn) env = execEntry fn env := by
  unfold computeHeavyIterationPipeline
  by_cases hBudget : guards.runBudgetedPasses
  · simp [hBudget]
    by_cases hStruct : guards.structuralEnabled
    · simp [hStruct]
      calc
        execEntry (clusterPipeline .cleanup (clusterPipeline .structural (clusterPipeline .standard fn))) env
            = execEntry (clusterPipeline .structural (clusterPipeline .standard fn)) env := by
                exact cleanup_cluster_preserves_semantics _ _
        _ = execEntry (clusterPipeline .standard fn) env := by
              exact structural_cluster_preserves_semantics _ _
        _ = execEntry fn env := by
              exact standard_cluster_preserves_semantics _ _
    · simp [hStruct]
      by_cases hFast : fastDevVectorizeGate features
      · simp [hFast]
        calc
          execEntry (clusterPipeline .structural (clusterPipeline .standard fn)) env
              = execEntry (clusterPipeline .standard fn) env := by
                  exact structural_cluster_preserves_semantics _ _
          _ = execEntry fn env := by
                exact standard_cluster_preserves_semantics _ _
      · simp [hFast]
        exact standard_cluster_preserves_semantics fn env
  · simp [hBudget]
    exact standard_cluster_preserves_semantics fn env

theorem control_flow_heavy_iteration_preserves_verify_ir
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (controlFlowHeavyIterationPipeline guards features fn) := by
  unfold controlFlowHeavyIterationPipeline
  by_cases hBudget : guards.runBudgetedPasses
  · simp [hBudget]
    by_cases hStruct : guards.structuralEnabled
    · by_cases hGate : controlFlowStructuralGate features
      · simp [hStruct, hGate]
        exact cleanup_cluster_preserves_verify_ir
          (structural_cluster_preserves_verify_ir
            (standard_cluster_preserves_verify_ir h))
      · by_cases hFast : fastDevVectorizeGate features
        · simp [hStruct, hGate, hFast]
          exact standard_cluster_preserves_verify_ir h
        · simp [hStruct, hGate, hFast]
          exact standard_cluster_preserves_verify_ir h
    · by_cases hGate : controlFlowStructuralGate features
      · by_cases hFast : fastDevVectorizeGate features
        · simp [hStruct, hGate, hFast]
          exact structural_cluster_preserves_verify_ir
            (standard_cluster_preserves_verify_ir h)
        · simp [hStruct, hGate, hFast]
          exact standard_cluster_preserves_verify_ir h
      · by_cases hFast : fastDevVectorizeGate features
        · simp [hStruct, hGate, hFast]
          exact structural_cluster_preserves_verify_ir
            (standard_cluster_preserves_verify_ir h)
        · simp [hStruct, hGate, hFast]
          exact standard_cluster_preserves_verify_ir h
  · simp [hBudget]
    exact standard_cluster_preserves_verify_ir h

theorem control_flow_heavy_iteration_preserves_semantics
    (guards : ReducedPhaseGuards)
    (features : ReducedFunctionPhaseFeatures)
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (controlFlowHeavyIterationPipeline guards features fn) env = execEntry fn env := by
  unfold controlFlowHeavyIterationPipeline
  by_cases hBudget : guards.runBudgetedPasses
  · simp [hBudget]
    by_cases hStruct : guards.structuralEnabled
    · by_cases hGate : controlFlowStructuralGate features
      · simp [hStruct, hGate]
        calc
          execEntry (clusterPipeline .cleanup (clusterPipeline .structural (clusterPipeline .standard fn))) env
              = execEntry (clusterPipeline .structural (clusterPipeline .standard fn)) env := by
                  exact cleanup_cluster_preserves_semantics _ _
          _ = execEntry (clusterPipeline .standard fn) env := by
                exact structural_cluster_preserves_semantics _ _
          _ = execEntry fn env := by
                exact standard_cluster_preserves_semantics _ _
      · by_cases hFast : fastDevVectorizeGate features
        · simp [hStruct, hGate, hFast]
          exact standard_cluster_preserves_semantics fn env
        · simp [hStruct, hGate, hFast]
          exact standard_cluster_preserves_semantics fn env
    · by_cases hGate : controlFlowStructuralGate features
      · by_cases hFast : fastDevVectorizeGate features
        · simp [hStruct, hGate, hFast]
          calc
            execEntry (clusterPipeline .structural (clusterPipeline .standard fn)) env
                = execEntry (clusterPipeline .standard fn) env := by
                    exact structural_cluster_preserves_semantics _ _
            _ = execEntry fn env := by
                  exact standard_cluster_preserves_semantics _ _
        · simp [hStruct, hGate, hFast]
          exact standard_cluster_preserves_semantics fn env
      · by_cases hFast : fastDevVectorizeGate features
        · simp [hStruct, hGate, hFast]
          calc
            execEntry (clusterPipeline .structural (clusterPipeline .standard fn)) env
                = execEntry (clusterPipeline .standard fn) env := by
                    exact structural_cluster_preserves_semantics _ _
            _ = execEntry fn env := by
                  exact standard_cluster_preserves_semantics _ _
        · simp [hStruct, hGate, hFast]
          exact standard_cluster_preserves_semantics fn env
  · simp [hBudget]
    exact standard_cluster_preserves_semantics fn env

end RRProofs.PhaseOrderIterationSoundness
