Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.DataflowOptSoundness.
Require Import RRProofs.CfgOptSoundness.
Require Import RRProofs.LoopOptSoundness.
Require Import RRProofs.DeSsaBoundarySoundness.

Module RROptimizerPipelineSoundness.

Import RRMirInvariantBundle.
Import RRDataflowOptSoundness.
Import RRCfgOptSoundness.
Import RRLoopOptSoundness.
Import RRDeSsaBoundarySoundness.

Definition always_tier_cfg_stage (fn : mir_fn_lite) : mir_fn_lite :=
  identity_pass fn.

Definition always_tier_dataflow_stage (fn : mir_fn_lite) : mir_fn_lite :=
  identity_pass fn.

Definition always_tier_loop_stage (fn : mir_fn_lite) : mir_fn_lite :=
  identity_pass fn.

Definition always_tier_cleanup_stage (fn : mir_fn_lite) : mir_fn_lite :=
  identity_pass fn.

Definition always_tier_pipeline (fn : mir_fn_lite) : mir_fn_lite :=
  always_tier_cleanup_stage
    (always_tier_loop_stage
      (always_tier_dataflow_stage
        (always_tier_cfg_stage fn))).

Definition program_inner_pre_dessa_pipeline (fn : mir_fn_lite) : mir_fn_lite :=
  always_tier_pipeline fn.

Definition post_dessa_boundary_stage (fn : mir_fn_lite) : mir_fn_lite :=
  identity_pass fn.

Definition post_dessa_cleanup_stage (fn : mir_fn_lite) : mir_fn_lite :=
  identity_pass fn.

Definition program_post_dessa_pipeline (fn : mir_fn_lite) : mir_fn_lite :=
  post_dessa_cleanup_stage (post_dessa_boundary_stage (program_inner_pre_dessa_pipeline fn)).

Definition prepare_for_codegen_pipeline (fn : mir_fn_lite) : mir_fn_lite :=
  program_post_dessa_pipeline fn.

Definition optimizer_pipeline (fn : mir_fn_lite) : mir_fn_lite :=
  prepare_for_codegen_pipeline fn.

Lemma always_tier_pipeline_eq_identity :
  forall fn,
    always_tier_pipeline fn = identity_pass fn.
Proof.
  reflexivity.
Qed.

Lemma program_inner_pre_dessa_pipeline_eq_identity :
  forall fn,
    program_inner_pre_dessa_pipeline fn = identity_pass fn.
Proof.
  reflexivity.
Qed.

Lemma optimizer_pipeline_eq_identity :
  forall fn,
    optimizer_pipeline fn = identity_pass fn.
Proof.
  reflexivity.
Qed.

Lemma always_tier_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (always_tier_pipeline fn).
Proof.
  intros fn H.
  unfold always_tier_pipeline, always_tier_cfg_stage, always_tier_dataflow_stage,
    always_tier_loop_stage, always_tier_cleanup_stage.
  apply loop_opt_identity_preserves_verify_ir_bundle.
  apply identity_dataflow_layer_preserves_verify_ir_bundle.
  apply identity_pass_preserves_verify_ir_bundle.
  apply identity_pass_preserves_verify_ir_bundle.
  exact H.
Qed.

Lemma always_tier_preserves_semantics :
  forall fn ρ,
    exec_entry (always_tier_pipeline fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  unfold always_tier_pipeline, always_tier_cfg_stage, always_tier_dataflow_stage,
    always_tier_loop_stage, always_tier_cleanup_stage.
  repeat rewrite identity_pass_preserves_semantics.
  apply identity_pass_preserves_semantics.
Qed.

Lemma always_tier_cfg_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (always_tier_cfg_stage fn).
Proof.
  intros fn H.
  exact (identity_pass_preserves_verify_ir_bundle fn H).
Qed.

Lemma always_tier_cfg_preserves_semantics :
  forall fn ρ,
    exec_entry (always_tier_cfg_stage fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

Lemma always_tier_dataflow_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (always_tier_dataflow_stage fn).
Proof.
  intros fn H.
  exact (identity_dataflow_layer_preserves_verify_ir_bundle fn H).
Qed.

Lemma always_tier_dataflow_preserves_semantics :
  forall fn ρ,
    exec_entry (always_tier_dataflow_stage fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

Lemma always_tier_loop_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (always_tier_loop_stage fn).
Proof.
  intros fn H.
  exact (loop_opt_identity_preserves_verify_ir_bundle fn H).
Qed.

Lemma always_tier_loop_preserves_semantics :
  forall fn ρ,
    exec_entry (always_tier_loop_stage fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

Lemma always_tier_cleanup_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (always_tier_cleanup_stage fn).
Proof.
  intros fn H.
  exact (identity_pass_preserves_verify_ir_bundle fn H).
Qed.

Lemma always_tier_cleanup_preserves_semantics :
  forall fn ρ,
    exec_entry (always_tier_cleanup_stage fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

Lemma program_inner_pre_dessa_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (program_inner_pre_dessa_pipeline fn).
Proof.
  intros fn H.
  unfold program_inner_pre_dessa_pipeline.
  exact (always_tier_preserves_verify_ir fn H).
Qed.

Lemma program_inner_pre_dessa_preserves_semantics :
  forall fn ρ,
    exec_entry (program_inner_pre_dessa_pipeline fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  unfold program_inner_pre_dessa_pipeline.
  exact (always_tier_preserves_semantics fn ρ).
Qed.

Lemma program_post_dessa_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (program_post_dessa_pipeline fn).
Proof.
  intros fn H.
  unfold program_post_dessa_pipeline, post_dessa_boundary_stage, post_dessa_cleanup_stage.
  apply de_ssa_boundary_identity_preserves_verify_ir_bundle.
  apply de_ssa_boundary_identity_preserves_verify_ir_bundle.
  exact (program_inner_pre_dessa_preserves_verify_ir fn H).
Qed.

Lemma program_post_dessa_preserves_semantics :
  forall fn ρ,
    exec_entry (program_post_dessa_pipeline fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  unfold program_post_dessa_pipeline, post_dessa_boundary_stage, post_dessa_cleanup_stage.
  repeat rewrite identity_pass_preserves_semantics.
  apply program_inner_pre_dessa_preserves_semantics.
Qed.

Lemma post_dessa_boundary_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (post_dessa_boundary_stage fn).
Proof.
  intros fn H.
  exact (de_ssa_boundary_identity_preserves_verify_ir_bundle fn H).
Qed.

Lemma post_dessa_boundary_preserves_semantics :
  forall fn ρ,
    exec_entry (post_dessa_boundary_stage fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (de_ssa_boundary_identity_preserves_semantics fn ρ).
Qed.

Lemma post_dessa_cleanup_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (post_dessa_cleanup_stage fn).
Proof.
  intros fn H.
  exact (identity_pass_preserves_verify_ir_bundle fn H).
Qed.

Lemma post_dessa_cleanup_preserves_semantics :
  forall fn ρ,
    exec_entry (post_dessa_cleanup_stage fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

Lemma prepare_for_codegen_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (prepare_for_codegen_pipeline fn).
Proof.
  intros fn H.
  unfold prepare_for_codegen_pipeline.
  exact (program_post_dessa_preserves_verify_ir fn H).
Qed.

Lemma prepare_for_codegen_preserves_semantics :
  forall fn ρ,
    exec_entry (prepare_for_codegen_pipeline fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  unfold prepare_for_codegen_pipeline.
  exact (program_post_dessa_preserves_semantics fn ρ).
Qed.

Lemma optimizer_pipeline_preserves_verify_ir :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (optimizer_pipeline fn).
Proof.
  intros fn H.
  unfold optimizer_pipeline.
  exact (prepare_for_codegen_preserves_verify_ir fn H).
Qed.

Lemma optimizer_pipeline_preserves_semantics :
  forall fn ρ,
    exec_entry (optimizer_pipeline fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  unfold optimizer_pipeline.
  exact (prepare_for_codegen_preserves_semantics fn ρ).
Qed.

End RROptimizerPipelineSoundness.
