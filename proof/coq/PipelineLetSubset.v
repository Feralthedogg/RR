Require Import LoweringSubset.
Require Import CodegenSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRCodegenSubset.

Module RRPipelineLetSubset.

Lemma lower_emit_preserves_eval :
  forall expr,
    eval_r_expr (emit_r (lower expr)) =
    eval_src_fuel (S (mir_expr_depth (lower expr))) expr.
Proof.
  intro expr.
  rewrite emit_r_preserves_eval.
  apply lower_preserves_eval_fuel.
Qed.

Definition let_record_field_src : mir_expr :=
  MFieldGet (MRecordLit [("x", MConstInt 4)]) "x".

Lemma let_record_field_src_pipeline_preserved :
  eval_r_expr (emit_r let_record_field_src) = Some (RVInt 4).
Proof.
  reflexivity.
Qed.

Definition let_nested_record_field_src : mir_expr :=
  MFieldGet (MFieldGet (MRecordLit [("inner", MRecordLit [("x", MConstInt 4)])]) "inner") "x".

Lemma let_nested_record_field_src_pipeline_preserved :
  eval_r_expr (emit_r let_nested_record_field_src) = Some (RVInt 4).
Proof.
  reflexivity.
Qed.

Definition let_add_src : mir_expr :=
  MBinaryAdd (MConstInt 4) (MConstInt 3).

Lemma let_add_src_pipeline_preserved :
  eval_r_expr (emit_r let_add_src) = Some (RVInt 7).
Proof.
  reflexivity.
Qed.

Definition let_nested_record_field_add_src : mir_expr :=
  MBinaryAdd
    (MFieldGet (MFieldGet (MRecordLit [("inner", MRecordLit [("x", MConstInt 4)])]) "inner") "x")
    (MConstInt 3).

Lemma let_nested_record_field_add_src_pipeline_preserved :
  eval_r_expr (emit_r let_nested_record_field_add_src) = Some (RVInt 7).
Proof.
  reflexivity.
Qed.

End RRPipelineLetSubset.
