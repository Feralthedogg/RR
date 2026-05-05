Require Import MirInvariantBundle.
Require Import OptimizerPipelineSoundness.

Module RRProgramPostTierStagesSoundness.

Import RRMirInvariantBundle.
Import RROptimizerPipelineSoundness.

Definition inline_cleanup_stage (fn : mir_fn_lite) : mir_fn_lite :=
  identity_pass fn.

Definition fresh_alias_stage (fn : mir_fn_lite) : mir_fn_lite :=
  identity_pass fn.

Definition de_ssa_program_stage (fn : mir_fn_lite) : mir_fn_lite :=
  prepare_for_codegen_pipeline fn.

Definition program_post_tier_pipeline (fn : mir_fn_lite) : mir_fn_lite :=
  de_ssa_program_stage (fresh_alias_stage (inline_cleanup_stage fn)).

Lemma inline_cleanup_stage_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (inline_cleanup_stage fn).
Proof.
  intros fn H.
  exact (identity_pass_preserves_verify_ir_bundle fn H).
Qed.

Lemma inline_cleanup_stage_preserves_semantics :
  forall fn ρ,
    exec_entry (inline_cleanup_stage fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

Lemma fresh_alias_stage_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (fresh_alias_stage fn).
Proof.
  intros fn H.
  exact (identity_pass_preserves_verify_ir_bundle fn H).
Qed.

Lemma fresh_alias_stage_preserves_semantics :
  forall fn ρ,
    exec_entry (fresh_alias_stage fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

Lemma de_ssa_program_stage_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (de_ssa_program_stage fn).
Proof.
  intros fn H.
  exact (prepare_for_codegen_preserves_verify_ir fn H).
Qed.

Lemma de_ssa_program_stage_preserves_semantics :
  forall fn ρ,
    exec_entry (de_ssa_program_stage fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (prepare_for_codegen_preserves_semantics fn ρ).
Qed.

Lemma program_post_tier_pipeline_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (program_post_tier_pipeline fn).
Proof.
  intros fn H.
  unfold program_post_tier_pipeline, inline_cleanup_stage, fresh_alias_stage, de_ssa_program_stage.
  exact (prepare_for_codegen_preserves_verify_ir
    (identity_pass (identity_pass fn))
    (identity_pass_preserves_verify_ir_bundle
      (identity_pass fn)
      (identity_pass_preserves_verify_ir_bundle fn H))).
Qed.

Lemma program_post_tier_pipeline_preserves_semantics :
  forall fn ρ,
    exec_entry (program_post_tier_pipeline fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  unfold program_post_tier_pipeline, inline_cleanup_stage, fresh_alias_stage, de_ssa_program_stage.
  rewrite (prepare_for_codegen_preserves_semantics (identity_pass (identity_pass fn)) ρ).
  rewrite (identity_pass_preserves_semantics (identity_pass fn) ρ).
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

End RRProgramPostTierStagesSoundness.
