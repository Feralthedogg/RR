Require Import MirInvariantBundle.
Require Import OptimizerPipelineSoundness.

Module RRPhaseOrderClusterSoundness.

Import RRMirInvariantBundle.
Import RROptimizerPipelineSoundness.

Inductive reduced_phase_cluster : Type :=
| RPCStructural
| RPCStandard
| RPCCleanup.

Definition cluster_pipeline (cluster : reduced_phase_cluster) (fn : mir_fn_lite) : mir_fn_lite :=
  match cluster with
  | RPCStructural => always_tier_loop_stage fn
  | RPCStandard =>
      always_tier_loop_stage (always_tier_dataflow_stage (always_tier_cfg_stage fn))
  | RPCCleanup =>
      post_dessa_cleanup_stage (post_dessa_boundary_stage fn)
  end.

Lemma structural_cluster_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (cluster_pipeline RPCStructural fn).
Proof.
  intros fn H.
  exact (always_tier_loop_preserves_verify_ir fn H).
Qed.

Lemma structural_cluster_preserves_semantics :
  forall fn ρ,
    exec_entry (cluster_pipeline RPCStructural fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (always_tier_loop_preserves_semantics fn ρ).
Qed.

Lemma standard_cluster_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (cluster_pipeline RPCStandard fn).
Proof.
  intros fn H.
  exact (always_tier_loop_preserves_verify_ir fn
    (always_tier_dataflow_preserves_verify_ir fn
      (always_tier_cfg_preserves_verify_ir fn H))).
Qed.

Lemma standard_cluster_preserves_semantics :
  forall fn ρ,
    exec_entry (cluster_pipeline RPCStandard fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  unfold cluster_pipeline.
  rewrite always_tier_cfg_preserves_semantics.
  rewrite always_tier_dataflow_preserves_semantics.
  apply always_tier_loop_preserves_semantics.
Qed.

Lemma cleanup_cluster_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (cluster_pipeline RPCCleanup fn).
Proof.
  intros fn H.
  exact (post_dessa_cleanup_preserves_verify_ir fn
    (post_dessa_boundary_preserves_verify_ir fn H)).
Qed.

Lemma cleanup_cluster_preserves_semantics :
  forall fn ρ,
    exec_entry (cluster_pipeline RPCCleanup fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  unfold cluster_pipeline.
  rewrite post_dessa_boundary_preserves_semantics.
  apply post_dessa_cleanup_preserves_semantics.
Qed.

End RRPhaseOrderClusterSoundness.
