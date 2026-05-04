Require Import MirInvariantBundle.
Require Import OptimizerPipelineSoundness.
Require Import ProgramPostTierStagesSoundness.

Module RRChronosPassManagerSoundness.

Import RRMirInvariantBundle.
Import RROptimizerPipelineSoundness.
Import RRProgramPostTierStagesSoundness.

Inductive chronos_stage_lite : Type :=
  | ChronosFunctionEntryCanonicalization
  | ChronosAlwaysTier
  | ChronosPhaseOrderStandard
  | ChronosPhaseOrderUnroll
  | ChronosFunctionFinalPolish
  | ChronosProgramOutlining
  | ChronosProgramInline
  | ChronosProgramRecordSpecialization
  | ChronosProgramInlineCleanup
  | ChronosProgramFreshAlias
  | ChronosProgramPostDeSsa
  | ChronosPrepareForCodegen.

Definition chronos_stage_pipeline
    (stage : chronos_stage_lite) (fn : mir_fn_lite) : mir_fn_lite :=
  match stage with
  | ChronosFunctionEntryCanonicalization => identity_pass fn
  | ChronosAlwaysTier => always_tier_pipeline fn
  | ChronosPhaseOrderStandard => program_inner_pre_dessa_pipeline fn
  | ChronosPhaseOrderUnroll => identity_pass fn
  | ChronosFunctionFinalPolish => always_tier_pipeline fn
  | ChronosProgramOutlining => identity_pass fn
  | ChronosProgramInline => identity_pass fn
  | ChronosProgramRecordSpecialization => identity_pass fn
  | ChronosProgramInlineCleanup => inline_cleanup_stage fn
  | ChronosProgramFreshAlias => fresh_alias_stage fn
  | ChronosProgramPostDeSsa => program_post_dessa_pipeline fn
  | ChronosPrepareForCodegen => prepare_for_codegen_pipeline fn
  end.

Definition chronos_reduced_schedule (fn : mir_fn_lite) : mir_fn_lite :=
  prepare_for_codegen_pipeline
    (program_post_tier_pipeline
      (program_inner_pre_dessa_pipeline fn)).

Lemma chronos_stage_preserves_verify_ir :
  forall stage fn,
    optimizer_eligible fn ->
    optimizer_eligible (chronos_stage_pipeline stage fn).
Proof.
  intros stage fn H.
  destruct stage; simpl.
  - exact (identity_pass_preserves_verify_ir_bundle fn H).
  - exact (always_tier_preserves_verify_ir fn H).
  - exact (program_inner_pre_dessa_preserves_verify_ir fn H).
  - exact (identity_pass_preserves_verify_ir_bundle fn H).
  - exact (always_tier_preserves_verify_ir fn H).
  - exact (identity_pass_preserves_verify_ir_bundle fn H).
  - exact (identity_pass_preserves_verify_ir_bundle fn H).
  - exact (identity_pass_preserves_verify_ir_bundle fn H).
  - exact (inline_cleanup_stage_preserves_verify_ir fn H).
  - exact (fresh_alias_stage_preserves_verify_ir fn H).
  - exact (program_post_dessa_preserves_verify_ir fn H).
  - exact (prepare_for_codegen_preserves_verify_ir fn H).
Qed.

Lemma chronos_stage_preserves_semantics :
  forall stage fn ρ,
    exec_entry (chronos_stage_pipeline stage fn) ρ = exec_entry fn ρ.
Proof.
  intros stage fn ρ.
  destruct stage; simpl.
  - exact (identity_pass_preserves_semantics fn ρ).
  - exact (always_tier_preserves_semantics fn ρ).
  - exact (program_inner_pre_dessa_preserves_semantics fn ρ).
  - exact (identity_pass_preserves_semantics fn ρ).
  - exact (always_tier_preserves_semantics fn ρ).
  - exact (identity_pass_preserves_semantics fn ρ).
  - exact (identity_pass_preserves_semantics fn ρ).
  - exact (identity_pass_preserves_semantics fn ρ).
  - exact (inline_cleanup_stage_preserves_semantics fn ρ).
  - exact (fresh_alias_stage_preserves_semantics fn ρ).
  - exact (program_post_dessa_preserves_semantics fn ρ).
  - exact (prepare_for_codegen_preserves_semantics fn ρ).
Qed.

Lemma chronos_reduced_schedule_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (chronos_reduced_schedule fn).
Proof.
  intros fn H.
  unfold chronos_reduced_schedule.
  exact (prepare_for_codegen_preserves_verify_ir
    (program_post_tier_pipeline (program_inner_pre_dessa_pipeline fn))
    (program_post_tier_pipeline_preserves_verify_ir
      (program_inner_pre_dessa_pipeline fn)
      (program_inner_pre_dessa_preserves_verify_ir fn H))).
Qed.

Lemma chronos_reduced_schedule_preserves_semantics :
  forall fn ρ,
    exec_entry (chronos_reduced_schedule fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  unfold chronos_reduced_schedule.
  rewrite (prepare_for_codegen_preserves_semantics
    (program_post_tier_pipeline (program_inner_pre_dessa_pipeline fn)) ρ).
  rewrite (program_post_tier_pipeline_preserves_semantics
    (program_inner_pre_dessa_pipeline fn) ρ).
  exact (program_inner_pre_dessa_preserves_semantics fn ρ).
Qed.

Definition fuel_exhausted_skip (fn : mir_fn_lite) : mir_fn_lite :=
  identity_pass fn.

Lemma fuel_exhausted_skip_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (fuel_exhausted_skip fn).
Proof.
  intros fn H.
  exact (identity_pass_preserves_verify_ir_bundle fn H).
Qed.

Lemma fuel_exhausted_skip_preserves_semantics :
  forall fn ρ,
    exec_entry (fuel_exhausted_skip fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

End RRChronosPassManagerSoundness.
