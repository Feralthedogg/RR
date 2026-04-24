Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.PhasePlanSoundness.
Require Import RRProofs.ProgramOptPlanSoundness.
Require Import RRProofs.ProgramPhasePipelineSoundness.
Require Import RRProofs.ProgramRunProfileInnerSoundness.

Module RRProgramApiWrapperSoundness.

Import RRMirInvariantBundle.
Import RRPhasePlanSoundness.
Import RRProgramOptPlanSoundness.
Import RRProgramPhasePipelineSoundness.
Import RRProgramRunProfileInnerSoundness.

Definition run_program_with_profile_and_scheduler_pipeline :=
  run_program_inner_function_pipeline.

Definition run_program_with_scheduler_pipeline :=
  run_program_with_profile_and_scheduler_pipeline.

Definition run_program_with_stats_pipeline :=
  run_program_with_scheduler_pipeline.

Definition run_program_pipeline :=
  run_program_with_stats_pipeline.

Lemma run_program_with_profile_and_scheduler_preserves_verify_ir :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn,
    optimizer_eligible fn ->
    optimizer_eligible
      (run_program_with_profile_and_scheduler_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn).
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn H.
  exact (run_program_inner_function_preserves_verify_ir mode trace_requested fast_dev run_heavy_tier plan entries entry fn H).
Qed.

Lemma run_program_with_profile_and_scheduler_preserves_semantics :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ,
    exec_entry
      (run_program_with_profile_and_scheduler_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn) ρ
      = exec_entry fn ρ.
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ.
  exact (run_program_inner_function_preserves_semantics mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ).
Qed.

Lemma run_program_with_scheduler_preserves_verify_ir :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn,
    optimizer_eligible fn ->
    optimizer_eligible
      (run_program_with_scheduler_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn).
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn H.
  exact (run_program_with_profile_and_scheduler_preserves_verify_ir
    mode trace_requested fast_dev run_heavy_tier plan entries entry fn H).
Qed.

Lemma run_program_with_scheduler_preserves_semantics :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ,
    exec_entry
      (run_program_with_scheduler_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn) ρ
      = exec_entry fn ρ.
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ.
  exact (run_program_with_profile_and_scheduler_preserves_semantics
    mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ).
Qed.

Lemma run_program_with_stats_preserves_verify_ir :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn,
    optimizer_eligible fn ->
    optimizer_eligible
      (run_program_with_stats_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn).
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn H.
  exact (run_program_with_scheduler_preserves_verify_ir
    mode trace_requested fast_dev run_heavy_tier plan entries entry fn H).
Qed.

Lemma run_program_with_stats_preserves_semantics :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ,
    exec_entry
      (run_program_with_stats_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn) ρ
      = exec_entry fn ρ.
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ.
  exact (run_program_with_scheduler_preserves_semantics
    mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ).
Qed.

Lemma run_program_preserves_verify_ir :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn,
    optimizer_eligible fn ->
    optimizer_eligible
      (run_program_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn).
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn H.
  exact (run_program_with_stats_preserves_verify_ir
    mode trace_requested fast_dev run_heavy_tier plan entries entry fn H).
Qed.

Lemma run_program_preserves_semantics :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ,
    exec_entry
      (run_program_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn) ρ
      = exec_entry fn ρ.
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ.
  exact (run_program_with_stats_preserves_semantics
    mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ).
Qed.

End RRProgramApiWrapperSoundness.
