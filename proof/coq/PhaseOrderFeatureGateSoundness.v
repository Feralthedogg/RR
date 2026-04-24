Require Import RRProofs.PhaseOrderClusterSoundness.
Require Import RRProofs.PhaseOrderGuardSoundness.

Module RRPhaseOrderFeatureGateSoundness.

Import RRPhaseOrderClusterSoundness.
Import RRPhaseOrderGuardSoundness.

Record reduced_function_phase_features : Type := {
  ir_size : nat;
  block_count : nat;
  loop_count : nat;
  canonical_loop_count : nat;
  branch_terms : nat;
  call_values : nat;
  side_effecting_calls : nat;
  store_instrs : nat;
}.

Definition phase_branch_density_high (features : reduced_function_phase_features) : bool :=
  Nat.leb (Nat.max (block_count features) 1) (branch_terms features * 3).

Definition control_flow_structural_gate (features : reduced_function_phase_features) : bool :=
  let branch_density_high := phase_branch_density_high features in
  let side_effects_dominant :=
    Nat.ltb (Nat.max (call_values features) 1) (side_effecting_calls features * 2) in
  Nat.ltb 0 (canonical_loop_count features)
    && negb branch_density_high
    && negb side_effects_dominant.

Definition fast_dev_vectorize_gate (features : reduced_function_phase_features) : bool :=
  Nat.ltb 0 (canonical_loop_count features)
    && Nat.leb (loop_count features) 1
    && Nat.leb (ir_size features) 128
    && Nat.leb (block_count features) 12
    && Nat.leb (branch_terms features) 2
    && Nat.leb (side_effecting_calls features) 1
    && Nat.ltb 0 (store_instrs features).

Definition guards_from_features
    (run_budgeted structural_enabled licm_allowed bce_allowed : bool)
    (features : reduced_function_phase_features) : reduced_phase_guards :=
  {| run_budgeted_passes := run_budgeted;
     structural_enabled := structural_enabled;
     control_flow_gate := control_flow_structural_gate features;
     fast_dev_vectorize := fast_dev_vectorize_gate features;
     licm_allowed := licm_allowed;
     bce_allowed := bce_allowed |}.

Lemma control_flow_gate_enables_structural_cluster :
  forall features fn,
    control_flow_structural_gate features = true ->
    control_flow_guarded_pipeline (guards_from_features true true true true features) fn =
    cluster_pipeline RPCStructural fn.
Proof.
  intros features fn Hgate.
  unfold control_flow_guarded_pipeline, guards_from_features.
  simpl. rewrite Hgate. reflexivity.
Qed.

Lemma control_flow_gate_false_falls_back_to_standard_cluster :
  forall features fn,
    control_flow_structural_gate features = false ->
    control_flow_guarded_pipeline (guards_from_features true true true true features) fn =
    cluster_pipeline RPCStandard fn.
Proof.
  intros features fn Hgate.
  unfold control_flow_guarded_pipeline, guards_from_features.
  simpl. rewrite Hgate. reflexivity.
Qed.

Lemma fast_dev_gate_enables_structural_cluster_when_structural_disabled :
  forall features fn,
    fast_dev_vectorize_gate features = true ->
    balanced_guarded_pipeline (guards_from_features true false true true features) fn =
    cluster_pipeline RPCStructural fn.
Proof.
  intros features fn Hgate.
  unfold balanced_guarded_pipeline, guards_from_features.
  simpl. rewrite Hgate. reflexivity.
Qed.

Lemma fast_dev_gate_false_falls_back_to_standard_cluster :
  forall features fn,
    fast_dev_vectorize_gate features = false ->
    balanced_guarded_pipeline (guards_from_features true false true true features) fn =
    cluster_pipeline RPCStandard fn.
Proof.
  intros features fn Hgate.
  unfold balanced_guarded_pipeline, guards_from_features.
  simpl. rewrite Hgate. reflexivity.
Qed.

Lemma budget_disabled_falls_back_to_standard_cluster :
  forall features fn,
    balanced_guarded_pipeline (guards_from_features false true true true features) fn =
    cluster_pipeline RPCStandard fn.
Proof.
  intros features fn.
  unfold balanced_guarded_pipeline, guards_from_features.
  simpl. reflexivity.
Qed.

End RRPhaseOrderFeatureGateSoundness.
