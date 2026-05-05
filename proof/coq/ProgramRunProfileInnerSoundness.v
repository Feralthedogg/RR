Require Import MirInvariantBundle.
Require Import PhasePlanSoundness.
Require Import PhasePlanLookupSoundness.
Require Import PhasePlanSummarySoundness.
Require Import ProgramOptPlanSoundness.
Require Import OptimizerPipelineSoundness.
Require Import ProgramPhasePipelineSoundness.
Require Import ProgramTierExecutionSoundness.
Require Import ProgramPostTierStagesSoundness.
From Stdlib Require Import List Bool.
Import ListNotations.
Open Scope bool_scope.

Module RRProgramRunProfileInnerSoundness.

Import RRMirInvariantBundle.
Import RRPhasePlanSoundness.
Import RRPhasePlanLookupSoundness.
Import RRPhasePlanSummarySoundness.
Import RRProgramOptPlanSoundness.
Import RROptimizerPipelineSoundness.
Import RRProgramPhasePipelineSoundness.
Import RRProgramTierExecutionSoundness.
Import RRProgramPostTierStagesSoundness.

Definition run_program_inner_function_pipeline
    (mode : reduced_phase_ordering_mode)
    (trace_requested fast_dev run_heavy_tier : bool)
    (plan : reduced_program_opt_plan)
    (entries : list reduced_program_phase_entry)
    (entry : reduced_program_phase_entry)
    (fn : mir_fn_lite) : mir_fn_lite :=
  program_post_tier_pipeline
    (execute_program_heavy_function mode trace_requested fast_dev run_heavy_tier plan entries entry
      (always_tier_pipeline fn)).

Definition run_program_inner_summary
    (ordered_function_ids : list nat)
    (mode : reduced_phase_ordering_mode)
    (trace_requested fast_dev run_heavy_tier : bool)
    (plan : reduced_program_opt_plan)
    (entries : list reduced_program_phase_entry) : list reduced_plan_summary_entry :=
  program_phase_summary_entries ordered_function_ids mode trace_requested fast_dev run_heavy_tier plan entries.

Lemma run_program_inner_function_preserves_verify_ir :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn,
    optimizer_eligible fn ->
    optimizer_eligible
      (run_program_inner_function_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn).
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn H.
  unfold run_program_inner_function_pipeline.
  pose proof (always_tier_preserves_verify_ir fn H) as Halways.
  pose proof (execute_program_heavy_function_preserves_verify_ir
    mode trace_requested fast_dev run_heavy_tier plan entries entry
    (always_tier_pipeline fn) Halways) as Hheavy.
  exact (program_post_tier_pipeline_preserves_verify_ir
    (execute_program_heavy_function mode trace_requested fast_dev run_heavy_tier plan entries entry
      (always_tier_pipeline fn))
    Hheavy).
Qed.

Lemma run_program_inner_function_preserves_semantics :
  forall mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ,
    exec_entry
      (run_program_inner_function_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn) ρ
      = exec_entry fn ρ.
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ.
  unfold run_program_inner_function_pipeline.
  rewrite (program_post_tier_pipeline_preserves_semantics
    (execute_program_heavy_function mode trace_requested fast_dev run_heavy_tier plan entries entry
      (always_tier_pipeline fn)) ρ).
  rewrite (execute_program_heavy_function_preserves_semantics
    mode trace_requested fast_dev run_heavy_tier plan entries entry
    (always_tier_pipeline fn) ρ).
  exact (always_tier_preserves_semantics fn ρ).
Qed.

Lemma run_program_inner_summary_hit_emits_singleton :
  forall mode trace_requested fast_dev run_heavy_tier plan entries function_id selected_plan,
    lookup_collected_plan function_id
      (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries)
      = Some selected_plan ->
    In (summarize_plan selected_plan)
      (run_program_inner_summary [function_id] mode trace_requested fast_dev run_heavy_tier plan entries).
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries function_id selected_plan Hlookup.
  unfold run_program_inner_summary.
  exact (program_phase_summary_hit_emits_entry
    mode trace_requested fast_dev run_heavy_tier plan entries function_id selected_plan Hlookup).
Qed.

Lemma run_program_inner_summary_miss_skips_singleton :
  forall mode trace_requested fast_dev run_heavy_tier plan entries function_id,
    lookup_collected_plan function_id
      (collect_program_phase_plans mode trace_requested fast_dev run_heavy_tier plan entries)
      = None ->
    run_program_inner_summary [function_id] mode trace_requested fast_dev run_heavy_tier plan entries = [].
Proof.
  intros mode trace_requested fast_dev run_heavy_tier plan entries function_id Hlookup.
  unfold run_program_inner_summary.
  exact (program_phase_summary_miss_skips_entry
    mode trace_requested fast_dev run_heavy_tier plan entries function_id Hlookup).
Qed.

End RRProgramRunProfileInnerSoundness.
