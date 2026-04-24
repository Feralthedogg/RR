Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.PhaseOrderGuardSoundness.
Require Import RRProofs.PhaseOrderFeatureGateSoundness.
Require Import RRProofs.PhaseOrderIterationSoundness.

Module RRPhaseOrderFallbackSoundness.

Import RRMirInvariantBundle.
Import RRPhaseOrderGuardSoundness.
Import RRPhaseOrderFeatureGateSoundness.
Import RRPhaseOrderIterationSoundness.

Record reduced_heavy_iteration_result : Type := {
  structural_progress : bool;
  non_structural_changes : nat;
}.

Definition control_flow_should_fallback_to_balanced
    (result : reduced_heavy_iteration_result) : bool :=
  negb (structural_progress result) && Nat.leb (non_structural_changes result) 1.

Definition control_flow_fallback_pipeline
    (guards : reduced_phase_guards)
    (features : reduced_function_phase_features)
    (result : reduced_heavy_iteration_result)
    (fn : mir_fn_lite) : mir_fn_lite :=
  if control_flow_should_fallback_to_balanced result then
    balanced_iteration_pipeline guards features fn
  else
    control_flow_heavy_iteration_pipeline guards features fn.

Lemma control_flow_fallback_preserves_verify_ir :
  forall guards features result fn,
    optimizer_eligible fn ->
    optimizer_eligible (control_flow_fallback_pipeline guards features result fn).
Proof.
  intros guards features result fn H.
  unfold control_flow_fallback_pipeline.
  destruct (control_flow_should_fallback_to_balanced result) eqn:Hfallback.
  - exact (balanced_iteration_preserves_verify_ir guards features fn H).
  - exact (control_flow_heavy_iteration_preserves_verify_ir guards features fn H).
Qed.

Lemma control_flow_fallback_preserves_semantics :
  forall guards features result fn ρ,
    exec_entry (control_flow_fallback_pipeline guards features result fn) ρ =
      exec_entry fn ρ.
Proof.
  intros guards features result fn ρ.
  unfold control_flow_fallback_pipeline.
  destruct (control_flow_should_fallback_to_balanced result) eqn:Hfallback.
  - exact (balanced_iteration_preserves_semantics guards features fn ρ).
  - exact (control_flow_heavy_iteration_preserves_semantics guards features fn ρ).
Qed.

End RRPhaseOrderFallbackSoundness.
