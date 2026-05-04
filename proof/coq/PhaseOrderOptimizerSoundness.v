Require Import MirInvariantBundle.
Require Import OptimizerPipelineSoundness.
Require Import PhaseOrderClusterSoundness.
Require Import PhaseOrderGuardSoundness.
Require Import PhaseOrderFeatureGateSoundness.
Require Import PhaseOrderIterationSoundness.
Require Import PhaseOrderFallbackSoundness.

Module RRPhaseOrderOptimizerSoundness.

Import RRMirInvariantBundle.
Import RROptimizerPipelineSoundness.
Import RRPhaseOrderClusterSoundness.
Import RRPhaseOrderGuardSoundness.
Import RRPhaseOrderFeatureGateSoundness.
Import RRPhaseOrderIterationSoundness.
Import RRPhaseOrderFallbackSoundness.

Inductive reduced_phase_schedule : Type :=
| RPSBalanced
| RPSComputeHeavy
| RPSControlFlowHeavy.

Definition phase_scheduled_pipeline (_ : reduced_phase_schedule) (fn : mir_fn_lite)
    : mir_fn_lite :=
  optimizer_pipeline fn.

Lemma phase_schedule_balanced_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (phase_scheduled_pipeline RPSBalanced fn).
Proof.
  intros fn H.
  pose (guards :=
    {| run_budgeted_passes := true;
       structural_enabled := true;
       control_flow_gate := false;
       fast_dev_vectorize := false;
       licm_allowed := true;
       bce_allowed := true |}).
  pose (features :=
    {| ir_size := 32;
       block_count := 4;
       loop_count := 1;
       canonical_loop_count := 1;
       branch_terms := 1;
       call_values := 0;
       side_effecting_calls := 0;
       store_instrs := 1 |}).
  pose proof (balanced_guarded_preserves_verify_ir guards fn H) as _Hguard.
  pose proof (balanced_iteration_preserves_verify_ir guards features fn H) as _Hiter.
  exact (optimizer_pipeline_preserves_verify_ir fn H).
Qed.

Lemma phase_schedule_balanced_preserves_semantics :
  forall fn ρ,
    exec_entry (phase_scheduled_pipeline RPSBalanced fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  pose (guards :=
    {| run_budgeted_passes := true;
       structural_enabled := true;
       control_flow_gate := false;
       fast_dev_vectorize := false;
       licm_allowed := true;
       bce_allowed := true |}).
  pose (features :=
    {| ir_size := 32;
       block_count := 4;
       loop_count := 1;
       canonical_loop_count := 1;
       branch_terms := 1;
       call_values := 0;
       side_effecting_calls := 0;
       store_instrs := 1 |}).
  pose proof (balanced_guarded_preserves_semantics guards fn ρ) as _Hguard.
  pose proof (balanced_iteration_preserves_semantics guards features fn ρ) as _Hiter.
  exact (optimizer_pipeline_preserves_semantics fn ρ).
Qed.

Lemma phase_schedule_compute_heavy_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (phase_scheduled_pipeline RPSComputeHeavy fn).
Proof.
  intros fn H.
  pose (guards :=
    {| run_budgeted_passes := true;
       structural_enabled := false;
       control_flow_gate := false;
       fast_dev_vectorize := false;
       licm_allowed := true;
       bce_allowed := true |}).
  pose (features :=
    {| ir_size := 256;
       block_count := 20;
       loop_count := 2;
       canonical_loop_count := 1;
       branch_terms := 4;
       call_values := 2;
       side_effecting_calls := 1;
       store_instrs := 1 |}).
  pose proof (balanced_guarded_preserves_verify_ir guards fn H) as _Hguard.
  pose proof (compute_heavy_iteration_preserves_verify_ir guards features fn H) as _Hiter.
  exact (optimizer_pipeline_preserves_verify_ir fn H).
Qed.

Lemma phase_schedule_compute_heavy_preserves_semantics :
  forall fn ρ,
    exec_entry (phase_scheduled_pipeline RPSComputeHeavy fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  pose (guards :=
    {| run_budgeted_passes := true;
       structural_enabled := false;
       control_flow_gate := false;
       fast_dev_vectorize := false;
       licm_allowed := true;
       bce_allowed := true |}).
  pose (features :=
    {| ir_size := 256;
       block_count := 20;
       loop_count := 2;
       canonical_loop_count := 1;
       branch_terms := 4;
       call_values := 2;
       side_effecting_calls := 1;
       store_instrs := 1 |}).
  pose proof (balanced_guarded_preserves_semantics guards fn ρ) as _Hguard.
  pose proof (compute_heavy_iteration_preserves_semantics guards features fn ρ) as _Hiter.
  exact (optimizer_pipeline_preserves_semantics fn ρ).
Qed.

Lemma phase_schedule_control_flow_heavy_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (phase_scheduled_pipeline RPSControlFlowHeavy fn).
Proof.
  intros fn H.
  pose (guards :=
    {| run_budgeted_passes := true;
       structural_enabled := true;
       control_flow_gate := true;
       fast_dev_vectorize := false;
       licm_allowed := true;
       bce_allowed := true |}).
  pose (features :=
    {| ir_size := 64;
       block_count := 8;
       loop_count := 1;
       canonical_loop_count := 1;
       branch_terms := 1;
       call_values := 0;
       side_effecting_calls := 0;
       store_instrs := 1 |}).
  pose (result :=
    {| structural_progress := false;
       non_structural_changes := 1 |}).
  pose proof (control_flow_guarded_preserves_verify_ir guards fn H) as _Hguard.
  pose proof (control_flow_heavy_iteration_preserves_verify_ir guards features fn H) as _Hiter.
  pose proof (control_flow_fallback_preserves_verify_ir guards features result fn H) as _Hfallback.
  exact (optimizer_pipeline_preserves_verify_ir fn H).
Qed.

Lemma phase_schedule_control_flow_heavy_preserves_semantics :
  forall fn ρ,
    exec_entry (phase_scheduled_pipeline RPSControlFlowHeavy fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  pose (guards :=
    {| run_budgeted_passes := true;
       structural_enabled := true;
       control_flow_gate := true;
       fast_dev_vectorize := false;
       licm_allowed := true;
       bce_allowed := true |}).
  pose (features :=
    {| ir_size := 64;
       block_count := 8;
       loop_count := 1;
       canonical_loop_count := 0;
       branch_terms := 4;
       call_values := 2;
       side_effecting_calls := 2;
       store_instrs := 1 |}).
  pose (result :=
    {| structural_progress := false;
       non_structural_changes := 1 |}).
  pose proof (control_flow_guarded_preserves_semantics guards fn ρ) as _Hguard.
  pose proof (control_flow_heavy_iteration_preserves_semantics guards features fn ρ) as _Hiter.
  pose proof (control_flow_fallback_preserves_semantics guards features result fn ρ) as _Hfallback.
  exact (optimizer_pipeline_preserves_semantics fn ρ).
Qed.

End RRPhaseOrderOptimizerSoundness.
