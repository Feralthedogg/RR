Require Import RRProofs.MirInvariantBundle.
Require Import RRProofs.MirSemanticsLite.
Require Import RRProofs.ProgramApiWrapperSoundness.
Require Import RRProofs.ProgramOptPlanSoundness.
Require Import RRProofs.ProgramPhasePipelineSoundness.
Require Import RRProofs.PhasePlanSoundness.
From Stdlib Require Import ZArith Bool Lia.
Open Scope Z_scope.
Open Scope bool_scope.

Module RRCompilerEndToEndSoundness.

Import RRMirInvariantBundle.
Import RRMirSemanticsLite.
Import RRProgramApiWrapperSoundness.
Import RRProgramOptPlanSoundness.
Import RRProgramPhasePipelineSoundness.
Import RRPhasePlanSoundness.

Inductive src_expr_lite : Type :=
| SELConstInt : Z -> src_expr_lite
| SELAdd : src_expr_lite -> src_expr_lite -> src_expr_lite.

Inductive r_expr_lite : Type :=
| RELConstInt : Z -> r_expr_lite
| RELAdd : r_expr_lite -> r_expr_lite -> r_expr_lite.

Fixpoint eval_src_expr_lite (expr : src_expr_lite) : Z :=
  match expr with
  | SELConstInt z => z
  | SELAdd lhs rhs => eval_src_expr_lite lhs + eval_src_expr_lite rhs
  end.

Fixpoint eval_r_expr_lite (expr : r_expr_lite) : Z :=
  match expr with
  | RELConstInt z => z
  | RELAdd lhs rhs => eval_r_expr_lite lhs + eval_r_expr_lite rhs
  end.

Fixpoint lower_emit_expr_lite (expr : src_expr_lite) : r_expr_lite :=
  match expr with
  | SELConstInt z => RELConstInt z
  | SELAdd lhs rhs => RELAdd (lower_emit_expr_lite lhs) (lower_emit_expr_lite rhs)
  end.

Lemma lower_emit_expr_lite_preserves_eval :
  forall expr,
    eval_r_expr_lite (lower_emit_expr_lite expr) = eval_src_expr_lite expr.
Proof.
  intros expr.
  induction expr as [z|lhs IHlhs rhs IHrhs]; simpl; lia.
Qed.

Record reduced_compiler_artifact : Type := {
  emitted_expr : r_expr_lite;
  optimized_fn : mir_fn_lite;
}.

Definition compile_artifact
    (src : src_expr_lite)
    (mode : reduced_phase_ordering_mode)
    (trace_requested fast_dev run_heavy_tier : bool)
    (plan : reduced_program_opt_plan)
    (entries : list reduced_program_phase_entry)
    (entry : reduced_program_phase_entry)
    (fn : mir_fn_lite) : reduced_compiler_artifact :=
  {| emitted_expr := lower_emit_expr_lite src;
     optimized_fn := run_program_pipeline mode trace_requested fast_dev run_heavy_tier plan entries entry fn |}.

Lemma compiler_frontend_preserves_eval :
  forall src,
    eval_r_expr_lite
      (emitted_expr
        (compile_artifact src RPOMBalanced false false false
          {| rpp_program_limit := 0; rpp_fn_limit := 0; rpp_total_ir := 0; rpp_max_fn_ir := 0;
             rpp_selective_mode := false; rpp_selected_functions := @nil nat |}
          (@nil reduced_program_phase_entry)
          {| rppe_function_id := 0;
             rppe_features := balanced_sample;
             rppe_ir_size := 0;
             rppe_score := 0;
             rppe_hot_weight := 0;
             rppe_present := true;
             rppe_conservative := false;
             rppe_self_recursive := false |}
          {| fn_entry := 0; fn_body_head := 0; fn_blocks := @nil mir_block;
             fn_unsupported_dynamic := false; fn_opaque_interop := false |}))
      = eval_src_expr_lite src.
Proof.
  intros src. simpl. exact (lower_emit_expr_lite_preserves_eval src).
Qed.

Lemma compiler_optimizer_preserves_verify_ir :
  forall src mode trace_requested fast_dev run_heavy_tier plan entries entry fn,
    optimizer_eligible fn ->
    optimizer_eligible (optimized_fn (compile_artifact src mode trace_requested fast_dev run_heavy_tier plan entries entry fn)).
Proof.
  intros src mode trace_requested fast_dev run_heavy_tier plan entries entry fn H.
  exact (run_program_preserves_verify_ir mode trace_requested fast_dev run_heavy_tier plan entries entry fn H).
Qed.

Lemma compiler_optimizer_preserves_semantics :
  forall src mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ,
    exec_entry (optimized_fn (compile_artifact src mode trace_requested fast_dev run_heavy_tier plan entries entry fn)) ρ
      = exec_entry fn ρ.
Proof.
  intros src mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ.
  exact (run_program_preserves_semantics mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ).
Qed.

Lemma compiler_end_to_end_preserves_observables :
  forall src mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ,
    optimizer_eligible fn ->
    eval_r_expr_lite (emitted_expr (compile_artifact src mode trace_requested fast_dev run_heavy_tier plan entries entry fn))
      = eval_src_expr_lite src /\
    exec_entry (optimized_fn (compile_artifact src mode trace_requested fast_dev run_heavy_tier plan entries entry fn)) ρ
      = exec_entry fn ρ.
Proof.
  intros src mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ H.
  split.
  - simpl. exact (lower_emit_expr_lite_preserves_eval src).
  - exact (run_program_preserves_semantics mode trace_requested fast_dev run_heavy_tier plan entries entry fn ρ).
Qed.

End RRCompilerEndToEndSoundness.
