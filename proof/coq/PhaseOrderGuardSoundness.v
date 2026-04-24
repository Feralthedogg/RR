Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.PhaseOrderClusterSoundness.

Module RRPhaseOrderGuardSoundness.

Import RRMirInvariantBundle.
Import RRPhaseOrderClusterSoundness.

Record reduced_phase_guards : Type := {
  run_budgeted_passes : bool;
  structural_enabled : bool;
  control_flow_gate : bool;
  fast_dev_vectorize : bool;
  licm_allowed : bool;
  bce_allowed : bool;
}.

Definition balanced_guarded_pipeline (guards : reduced_phase_guards) (fn : mir_fn_lite)
    : mir_fn_lite :=
  if run_budgeted_passes guards then
    if structural_enabled guards then
      cluster_pipeline RPCStructural fn
    else if fast_dev_vectorize guards then
      cluster_pipeline RPCStructural fn
    else
      cluster_pipeline RPCStandard fn
  else
    cluster_pipeline RPCStandard fn.

Definition control_flow_guarded_pipeline (guards : reduced_phase_guards) (fn : mir_fn_lite)
    : mir_fn_lite :=
  if andb (run_budgeted_passes guards)
       (andb (structural_enabled guards) (control_flow_gate guards))
  then cluster_pipeline RPCStructural fn
  else cluster_pipeline RPCStandard fn.

Definition cleanup_guarded_pipeline (_guards : reduced_phase_guards) (fn : mir_fn_lite)
    : mir_fn_lite :=
  cluster_pipeline RPCCleanup fn.

Lemma balanced_guarded_preserves_verify_ir :
  forall guards fn,
    optimizer_eligible fn ->
    optimizer_eligible (balanced_guarded_pipeline guards fn).
Proof.
  intros guards fn H.
  unfold balanced_guarded_pipeline.
  destruct (run_budgeted_passes guards);
  destruct (structural_enabled guards);
  destruct (fast_dev_vectorize guards);
  try exact (structural_cluster_preserves_verify_ir fn H);
  exact (standard_cluster_preserves_verify_ir fn H).
Qed.

Lemma balanced_guarded_preserves_semantics :
  forall guards fn ρ,
    exec_entry (balanced_guarded_pipeline guards fn) ρ = exec_entry fn ρ.
Proof.
  intros guards fn ρ.
  unfold balanced_guarded_pipeline.
  destruct (run_budgeted_passes guards);
  destruct (structural_enabled guards);
  destruct (fast_dev_vectorize guards);
  try exact (structural_cluster_preserves_semantics fn ρ);
  exact (standard_cluster_preserves_semantics fn ρ).
Qed.

Lemma control_flow_guarded_preserves_verify_ir :
  forall guards fn,
    optimizer_eligible fn ->
    optimizer_eligible (control_flow_guarded_pipeline guards fn).
Proof.
  intros guards fn H.
  unfold control_flow_guarded_pipeline.
  destruct (andb (run_budgeted_passes guards)
            (andb (structural_enabled guards) (control_flow_gate guards)));
  [exact (structural_cluster_preserves_verify_ir fn H)
  | exact (standard_cluster_preserves_verify_ir fn H)].
Qed.

Lemma control_flow_guarded_preserves_semantics :
  forall guards fn ρ,
    exec_entry (control_flow_guarded_pipeline guards fn) ρ = exec_entry fn ρ.
Proof.
  intros guards fn ρ.
  unfold control_flow_guarded_pipeline.
  destruct (andb (run_budgeted_passes guards)
            (andb (structural_enabled guards) (control_flow_gate guards)));
  [exact (structural_cluster_preserves_semantics fn ρ)
  | exact (standard_cluster_preserves_semantics fn ρ)].
Qed.

Lemma cleanup_guarded_preserves_verify_ir :
  forall guards fn,
    optimizer_eligible fn ->
    optimizer_eligible (cleanup_guarded_pipeline guards fn).
Proof.
  intros guards fn H.
  exact (cleanup_cluster_preserves_verify_ir fn H).
Qed.

Lemma cleanup_guarded_preserves_semantics :
  forall guards fn ρ,
    exec_entry (cleanup_guarded_pipeline guards fn) ρ = exec_entry fn ρ.
Proof.
  intros guards fn ρ.
  exact (cleanup_cluster_preserves_semantics fn ρ).
Qed.

End RRPhaseOrderGuardSoundness.
