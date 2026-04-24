import RRProofs.OptimizerPipelineSoundness

namespace RRProofs.PhaseOrderClusterSoundness

open RRProofs.MirInvariantBundle
open RRProofs.OptimizerPipelineSoundness

inductive ReducedPhaseCluster where
  | structural
  | standard
  | cleanup
deriving Repr, DecidableEq

def clusterPipeline : ReducedPhaseCluster -> MirFnLite -> MirFnLite
  | .structural => alwaysTierLoopStage
  | .standard =>
      fun fn => alwaysTierLoopStage (alwaysTierDataflowStage (alwaysTierCfgStage fn))
  | .cleanup =>
      fun fn => postDeSsaCleanupStage (postDeSsaBoundaryStage fn)

theorem structural_cluster_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (clusterPipeline .structural fn) := by
  exact always_tier_loop_preserves_verify_ir h

theorem structural_cluster_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (clusterPipeline .structural fn) env = execEntry fn env := by
  exact always_tier_loop_preserves_semantics fn env

theorem standard_cluster_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (clusterPipeline .standard fn) := by
  exact always_tier_loop_preserves_verify_ir
    (always_tier_dataflow_preserves_verify_ir
      (always_tier_cfg_preserves_verify_ir h))

theorem standard_cluster_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (clusterPipeline .standard fn) env = execEntry fn env := by
  unfold clusterPipeline
  simp [always_tier_cfg_preserves_semantics, always_tier_dataflow_preserves_semantics,
    always_tier_loop_preserves_semantics]

theorem cleanup_cluster_preserves_verify_ir
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (clusterPipeline .cleanup fn) := by
  exact post_dessa_cleanup_preserves_verify_ir
    (post_dessa_boundary_preserves_verify_ir h)

theorem cleanup_cluster_preserves_semantics
    (fn : MirFnLite) (env : RRProofs.MirSemanticsLite.Env) :
    execEntry (clusterPipeline .cleanup fn) env = execEntry fn env := by
  unfold clusterPipeline
  simp [post_dessa_boundary_preserves_semantics, post_dessa_cleanup_preserves_semantics]

end RRProofs.PhaseOrderClusterSoundness
