Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.PhaseOrderClusterSoundness.
Require Import RRProofs.PhaseOrderGuardSoundness.
Require Import RRProofs.PhaseOrderFeatureGateSoundness.

Module RRPhaseOrderIterationSoundness.

Import RRMirInvariantBundle.
Import RRPhaseOrderClusterSoundness.
Import RRPhaseOrderGuardSoundness.
Import RRPhaseOrderFeatureGateSoundness.

Definition fast_dev_subpath_pipeline (fn : mir_fn_lite) : mir_fn_lite :=
  cluster_pipeline RPCStructural fn.

Definition balanced_iteration_pipeline
    (guards : reduced_phase_guards)
    (features : reduced_function_phase_features)
    (fn : mir_fn_lite) : mir_fn_lite :=
  let after_structural :=
    if run_budgeted_passes guards then
      if structural_enabled guards then
        cluster_pipeline RPCStructural fn
      else if fast_dev_vectorize_gate features then
        cluster_pipeline RPCStructural fn
      else
        fn
    else
      fn in
  let after_cleanup :=
    if andb (run_budgeted_passes guards) (structural_enabled guards) then
      cluster_pipeline RPCCleanup after_structural
    else
      after_structural in
  cluster_pipeline RPCStandard after_cleanup.

Definition compute_heavy_iteration_pipeline
    (guards : reduced_phase_guards)
    (features : reduced_function_phase_features)
    (fn : mir_fn_lite) : mir_fn_lite :=
  let after_standard := cluster_pipeline RPCStandard fn in
  let after_structural :=
    if run_budgeted_passes guards then
      if structural_enabled guards then
        cluster_pipeline RPCStructural after_standard
      else if fast_dev_vectorize_gate features then
        cluster_pipeline RPCStructural after_standard
      else
        after_standard
    else
      after_standard in
  if andb (run_budgeted_passes guards) (structural_enabled guards) then
    cluster_pipeline RPCCleanup after_structural
  else
    after_structural.

Definition control_flow_heavy_iteration_pipeline
    (guards : reduced_phase_guards)
    (features : reduced_function_phase_features)
    (fn : mir_fn_lite) : mir_fn_lite :=
  let after_standard := cluster_pipeline RPCStandard fn in
  let after_structural :=
    if run_budgeted_passes guards then
      if andb (structural_enabled guards) (control_flow_structural_gate features) then
        cluster_pipeline RPCStructural after_standard
      else if andb (negb (structural_enabled guards)) (fast_dev_vectorize_gate features) then
        cluster_pipeline RPCStructural after_standard
      else
        after_standard
    else
      after_standard in
  if andb (run_budgeted_passes guards)
      (andb (structural_enabled guards) (control_flow_structural_gate features)) then
    cluster_pipeline RPCCleanup after_structural
  else
    after_structural.

Lemma fast_dev_subpath_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (fast_dev_subpath_pipeline fn).
Proof.
  intros fn H.
  exact (structural_cluster_preserves_verify_ir fn H).
Qed.

Lemma fast_dev_subpath_preserves_semantics :
  forall fn ρ,
    exec_entry (fast_dev_subpath_pipeline fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (structural_cluster_preserves_semantics fn ρ).
Qed.

Lemma balanced_iteration_preserves_verify_ir :
  forall guards features fn,
    optimizer_eligible fn ->
    optimizer_eligible (balanced_iteration_pipeline guards features fn).
Proof.
  intros guards features fn H.
  unfold balanced_iteration_pipeline.
  destruct (run_budgeted_passes guards) eqn:Hbudget.
  - destruct (structural_enabled guards) eqn:Hstruct.
    + exact (standard_cluster_preserves_verify_ir _
        (cleanup_cluster_preserves_verify_ir _
          (structural_cluster_preserves_verify_ir _ H))).
    + destruct (fast_dev_vectorize_gate features) eqn:Hfast.
      * exact (standard_cluster_preserves_verify_ir _
          (structural_cluster_preserves_verify_ir _ H)).
      * exact (standard_cluster_preserves_verify_ir _ H).
  - exact (standard_cluster_preserves_verify_ir _ H).
Qed.

Lemma balanced_iteration_preserves_semantics :
  forall guards features fn ρ,
    exec_entry (balanced_iteration_pipeline guards features fn) ρ = exec_entry fn ρ.
Proof.
  intros guards features fn ρ.
  unfold balanced_iteration_pipeline.
  destruct (run_budgeted_passes guards) eqn:Hbudget.
  - destruct (structural_enabled guards) eqn:Hstruct.
    + simpl.
      change (exec_entry
        (cluster_pipeline RPCStandard
          (cluster_pipeline RPCCleanup (cluster_pipeline RPCStructural fn))) ρ =
        exec_entry fn ρ).
      rewrite (standard_cluster_preserves_semantics
        (cluster_pipeline RPCCleanup (cluster_pipeline RPCStructural fn)) ρ).
      rewrite (cleanup_cluster_preserves_semantics
        (cluster_pipeline RPCStructural fn) ρ).
      exact (structural_cluster_preserves_semantics fn ρ).
    + destruct (fast_dev_vectorize_gate features) eqn:Hfast.
      * simpl.
        change (exec_entry
          (cluster_pipeline RPCStandard (cluster_pipeline RPCStructural fn)) ρ =
          exec_entry fn ρ).
        rewrite (standard_cluster_preserves_semantics
          (cluster_pipeline RPCStructural fn) ρ).
        exact (structural_cluster_preserves_semantics fn ρ).
      * exact (standard_cluster_preserves_semantics fn ρ).
  - exact (standard_cluster_preserves_semantics fn ρ).
Qed.

Lemma compute_heavy_iteration_preserves_verify_ir :
  forall guards features fn,
    optimizer_eligible fn ->
    optimizer_eligible (compute_heavy_iteration_pipeline guards features fn).
Proof.
  intros guards features fn H.
  unfold compute_heavy_iteration_pipeline.
  destruct (run_budgeted_passes guards) eqn:Hbudget.
  - destruct (structural_enabled guards) eqn:Hstruct.
    + exact (cleanup_cluster_preserves_verify_ir _
        (structural_cluster_preserves_verify_ir _
          (standard_cluster_preserves_verify_ir _ H))).
    + destruct (fast_dev_vectorize_gate features) eqn:Hfast.
      * exact (structural_cluster_preserves_verify_ir _
          (standard_cluster_preserves_verify_ir _ H)).
      * exact (standard_cluster_preserves_verify_ir _ H).
  - exact (standard_cluster_preserves_verify_ir _ H).
Qed.

Lemma compute_heavy_iteration_preserves_semantics :
  forall guards features fn ρ,
    exec_entry (compute_heavy_iteration_pipeline guards features fn) ρ = exec_entry fn ρ.
Proof.
  intros guards features fn ρ.
  unfold compute_heavy_iteration_pipeline.
  destruct (run_budgeted_passes guards) eqn:Hbudget.
  - destruct (structural_enabled guards) eqn:Hstruct.
    + simpl.
      change (exec_entry
        (cluster_pipeline RPCCleanup
          (cluster_pipeline RPCStructural (cluster_pipeline RPCStandard fn))) ρ =
        exec_entry fn ρ).
      rewrite (cleanup_cluster_preserves_semantics
        (cluster_pipeline RPCStructural (cluster_pipeline RPCStandard fn)) ρ).
      rewrite (structural_cluster_preserves_semantics
        (cluster_pipeline RPCStandard fn) ρ).
      exact (standard_cluster_preserves_semantics fn ρ).
    + destruct (fast_dev_vectorize_gate features) eqn:Hfast.
      * simpl.
        change (exec_entry
          (cluster_pipeline RPCStructural (cluster_pipeline RPCStandard fn)) ρ =
          exec_entry fn ρ).
        rewrite (structural_cluster_preserves_semantics
          (cluster_pipeline RPCStandard fn) ρ).
        exact (standard_cluster_preserves_semantics fn ρ).
      * exact (standard_cluster_preserves_semantics fn ρ).
  - exact (standard_cluster_preserves_semantics fn ρ).
Qed.

Lemma control_flow_heavy_iteration_preserves_verify_ir :
  forall guards features fn,
    optimizer_eligible fn ->
    optimizer_eligible (control_flow_heavy_iteration_pipeline guards features fn).
Proof.
  intros guards features fn H.
  unfold control_flow_heavy_iteration_pipeline.
  destruct (run_budgeted_passes guards) eqn:Hbudget.
  - destruct (andb (structural_enabled guards) (control_flow_structural_gate features)) eqn:Hstruct.
    + exact (cleanup_cluster_preserves_verify_ir _
        (structural_cluster_preserves_verify_ir _
          (standard_cluster_preserves_verify_ir _ H))).
    + destruct (andb (negb (structural_enabled guards)) (fast_dev_vectorize_gate features)) eqn:Hfast.
      * exact (structural_cluster_preserves_verify_ir _
          (standard_cluster_preserves_verify_ir _ H)).
      * exact (standard_cluster_preserves_verify_ir _ H).
  - exact (standard_cluster_preserves_verify_ir _ H).
Qed.

Lemma control_flow_heavy_iteration_preserves_semantics :
  forall guards features fn ρ,
    exec_entry (control_flow_heavy_iteration_pipeline guards features fn) ρ = exec_entry fn ρ.
Proof.
  intros guards features fn ρ.
  unfold control_flow_heavy_iteration_pipeline.
  destruct (run_budgeted_passes guards) eqn:Hbudget.
  - destruct (andb (structural_enabled guards) (control_flow_structural_gate features)) eqn:Hstruct.
    + simpl.
      change (exec_entry
        (cluster_pipeline RPCCleanup
          (cluster_pipeline RPCStructural (cluster_pipeline RPCStandard fn))) ρ =
        exec_entry fn ρ).
      rewrite (cleanup_cluster_preserves_semantics
        (cluster_pipeline RPCStructural (cluster_pipeline RPCStandard fn)) ρ).
      rewrite (structural_cluster_preserves_semantics
        (cluster_pipeline RPCStandard fn) ρ).
      exact (standard_cluster_preserves_semantics fn ρ).
    + destruct (andb (negb (structural_enabled guards)) (fast_dev_vectorize_gate features)) eqn:Hfast.
      * simpl.
        change (exec_entry
          (cluster_pipeline RPCStructural (cluster_pipeline RPCStandard fn)) ρ =
          exec_entry fn ρ).
        rewrite (structural_cluster_preserves_semantics
          (cluster_pipeline RPCStandard fn) ρ).
        exact (standard_cluster_preserves_semantics fn ρ).
      * exact (standard_cluster_preserves_semantics fn ρ).
  - exact (standard_cluster_preserves_semantics fn ρ).
Qed.

End RRPhaseOrderIterationSoundness.
