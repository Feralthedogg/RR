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

Module RRPipelineCfgSubset.

Definition cfg_result_from_branches
    (cond : r_expr)
    (then_ret else_ret : r_expr) : option rvalue :=
  match eval_r_expr cond with
  | Some (RVBool true) => eval_r_expr then_ret
  | Some (RVBool false) => eval_r_expr else_ret
  | _ => None
  end.

Definition cfg_result_from_mir
    (cond : mir_expr)
    (then_ret else_ret : mir_expr) : option rvalue :=
  match eval_mir_fuel (S (mir_expr_depth cond)) cond with
  | Some (RVBool true) => eval_mir_fuel (S (mir_expr_depth then_ret)) then_ret
  | Some (RVBool false) => eval_mir_fuel (S (mir_expr_depth else_ret)) else_ret
  | _ => None
  end.

Lemma emit_r_cfg_preserves_eval :
  forall cond then_ret else_ret,
    cfg_result_from_branches (emit_r cond) (emit_r then_ret) (emit_r else_ret) =
    cfg_result_from_mir cond then_ret else_ret.
Proof.
  intros cond then_ret else_ret.
  unfold cfg_result_from_branches, cfg_result_from_mir.
  rewrite emit_r_preserves_eval.
  rewrite emit_r_preserves_eval.
  rewrite emit_r_preserves_eval.
  reflexivity.
Qed.

Definition straight_line_cfg_program_result : option rvalue :=
  eval_r_expr (RBinaryAdd (RConstInt 4) (RConstInt 3)).

Lemma straight_line_cfg_program_preserved :
  straight_line_cfg_program_result = Some (RVInt 7).
Proof.
  reflexivity.
Qed.

Definition branch_cfg_nested_record_program_result : option rvalue :=
  let cond := RConstBool true in
  let then_rec := RListLit [("inner", RListLit [("x", RConstInt 1)])] in
  let else_rec := RListLit [("inner", RListLit [("x", RConstInt 2)])] in
  let then_ret :=
    RBinaryAdd
      (RFieldGet (RFieldGet then_rec "inner") "x")
      (RConstInt 3) in
  let else_ret :=
    RBinaryAdd
      (RFieldGet (RFieldGet else_rec "inner") "x")
      (RConstInt 3) in
  match eval_r_expr cond with
  | Some (RVBool true) => eval_r_expr then_ret
  | Some (RVBool false) => eval_r_expr else_ret
  | _ => None
  end.

Lemma branch_cfg_nested_record_program_preserved :
  branch_cfg_nested_record_program_result = Some (RVInt 4).
Proof.
  unfold branch_cfg_nested_record_program_result.
  change (cfg_result_from_branches
    (emit_r (MConstBool true))
    (emit_r (MBinaryAdd
      (MFieldGet (MFieldGet (MRecordLit [("inner", MRecordLit [("x", MConstInt 1)])]) "inner") "x")
      (MConstInt 3)))
    (emit_r (MBinaryAdd
      (MFieldGet (MFieldGet (MRecordLit [("inner", MRecordLit [("x", MConstInt 2)])]) "inner") "x")
      (MConstInt 3))) = Some (RVInt 4)).
  rewrite emit_r_cfg_preserves_eval.
  reflexivity.
Qed.

End RRPipelineCfgSubset.
