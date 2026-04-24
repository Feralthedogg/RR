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

Module RRPipelineAssignPhiSubset.

Inductive mir_assign_phi_expr : Type :=
| MAssignPhi : string -> mir_expr -> mir_expr -> mir_expr -> mir_expr -> mir_assign_phi_expr.

Inductive r_assign_phi_expr : Type :=
| RAssignPhi : string -> r_expr -> r_expr -> r_expr -> r_expr -> r_assign_phi_expr.

Definition eval_mir_assign_phi_fuel (fuel : nat) (e : mir_assign_phi_expr) : option rvalue :=
  match e with
  | MAssignPhi _ cond then_val else_val body =>
      match eval_mir_fuel fuel cond with
      | Some (RVBool true) =>
          match eval_mir_fuel fuel then_val, eval_mir_fuel fuel body with
          | Some _, Some v => Some v
          | _, _ => None
          end
      | Some (RVBool false) =>
          match eval_mir_fuel fuel else_val, eval_mir_fuel fuel body with
          | Some _, Some v => Some v
          | _, _ => None
          end
      | _ => None
      end
  end.

Definition eval_r_assign_phi (e : r_assign_phi_expr) : option rvalue :=
  match e with
  | RAssignPhi _ cond then_val else_val body =>
      match eval_r_expr cond with
      | Some (RVBool true) =>
          match eval_r_expr then_val, eval_r_expr body with
          | Some _, Some v => Some v
          | _, _ => None
          end
      | Some (RVBool false) =>
          match eval_r_expr else_val, eval_r_expr body with
          | Some _, Some v => Some v
          | _, _ => None
          end
      | _ => None
      end
  end.

Definition emit_r_assign_phi (e : mir_assign_phi_expr) : r_assign_phi_expr :=
  match e with
  | MAssignPhi name cond then_val else_val body =>
      RAssignPhi name (emit_r cond) (emit_r then_val) (emit_r else_val) (emit_r body)
  end.

Definition eval_mir_assign_phi_codegen (e : mir_assign_phi_expr) : option rvalue :=
  match e with
  | MAssignPhi _ cond then_val else_val body =>
      match eval_mir_fuel (S (mir_expr_depth cond)) cond with
      | Some (RVBool true) =>
          match eval_mir_fuel (S (mir_expr_depth then_val)) then_val,
                eval_mir_fuel (S (mir_expr_depth body)) body with
          | Some _, Some v => Some v
          | _, _ => None
          end
      | Some (RVBool false) =>
          match eval_mir_fuel (S (mir_expr_depth else_val)) else_val,
                eval_mir_fuel (S (mir_expr_depth body)) body with
          | Some _, Some v => Some v
          | _, _ => None
          end
      | _ => None
      end
  end.

Lemma emit_r_assign_phi_preserves_eval :
  forall expr,
    eval_r_assign_phi (emit_r_assign_phi expr) = eval_mir_assign_phi_codegen expr.
Proof.
  intros expr.
  destruct expr as [name cond then_val else_val body].
  unfold eval_mir_assign_phi_codegen.
  simpl.
  rewrite emit_r_preserves_eval.
  rewrite emit_r_preserves_eval.
  rewrite emit_r_preserves_eval.
  rewrite emit_r_preserves_eval.
  reflexivity.
Qed.

Definition branch_assigned_local_src : r_assign_phi_expr :=
  RAssignPhi "x"
    (RConstBool true)
    (RConstInt 1)
    (RConstInt 2)
    (RBinaryAdd (RConstInt 1) (RConstInt 3)).

Lemma branch_assigned_local_src_pipeline_preserved :
  eval_r_assign_phi branch_assigned_local_src = Some (RVInt 4).
Proof.
  change (eval_r_assign_phi (emit_r_assign_phi
    (MAssignPhi "x" (MConstBool true) (MConstInt 1) (MConstInt 2)
      (MBinaryAdd (MConstInt 1) (MConstInt 3)))) = Some (RVInt 4)).
  rewrite emit_r_assign_phi_preserves_eval.
  reflexivity.
Qed.

Definition branch_assigned_record_field_src : r_assign_phi_expr :=
  RAssignPhi "rec"
    (RConstBool true)
    (RListLit [("x", RConstInt 1)])
    (RListLit [("x", RConstInt 2)])
    (RBinaryAdd (RFieldGet (RListLit [("x", RConstInt 1)]) "x") (RConstInt 3)).

Lemma branch_assigned_record_field_src_pipeline_preserved :
  eval_r_assign_phi branch_assigned_record_field_src = Some (RVInt 4).
Proof.
  change (eval_r_assign_phi (emit_r_assign_phi
    (MAssignPhi "rec"
      (MConstBool true)
      (MRecordLit [("x", MConstInt 1)])
      (MRecordLit [("x", MConstInt 2)])
      (MBinaryAdd (MFieldGet (MRecordLit [("x", MConstInt 1)]) "x") (MConstInt 3)))) = Some (RVInt 4)).
  rewrite emit_r_assign_phi_preserves_eval.
  reflexivity.
Qed.

Definition branch_assigned_nested_record_field_src : r_assign_phi_expr :=
  RAssignPhi "rec"
    (RConstBool true)
    (RListLit [("inner", RListLit [("x", RConstInt 1)])])
    (RListLit [("inner", RListLit [("x", RConstInt 2)])])
    (RBinaryAdd
      (RFieldGet (RFieldGet (RListLit [("inner", RListLit [("x", RConstInt 1)])]) "inner") "x")
      (RConstInt 3)).

Lemma branch_assigned_nested_record_field_src_pipeline_preserved :
  eval_r_assign_phi branch_assigned_nested_record_field_src = Some (RVInt 4).
Proof.
  change (eval_r_assign_phi (emit_r_assign_phi
    (MAssignPhi "rec"
      (MConstBool true)
      (MRecordLit [("inner", MRecordLit [("x", MConstInt 1)])])
      (MRecordLit [("inner", MRecordLit [("x", MConstInt 2)])])
      (MBinaryAdd
        (MFieldGet (MFieldGet (MRecordLit [("inner", MRecordLit [("x", MConstInt 1)])]) "inner") "x")
        (MConstInt 3)))) = Some (RVInt 4)).
  rewrite emit_r_assign_phi_preserves_eval.
  reflexivity.
Qed.

End RRPipelineAssignPhiSubset.
