Require Import LoweringSubset.
Require Import LoweringIfPhiSubset.
Require Import CodegenSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRLoweringIfPhiSubset.
Import RRCodegenSubset.

Module RRPipelineIfPhiSubset.

Inductive r_if_phi_expr : Type :=
| RIPure : r_expr -> r_if_phi_expr
| RIfPhi : r_expr -> r_expr -> r_expr -> r_if_phi_expr.

Definition eval_r_if_phi (e : r_if_phi_expr) : option rvalue :=
  match e with
  | RIPure expr => eval_r_expr expr
  | RIfPhi cond then_val else_val =>
      match eval_r_expr cond with
      | Some (RVBool true) => eval_r_expr then_val
      | Some (RVBool false) => eval_r_expr else_val
      | _ => None
      end
  end.

Definition emit_r_if_phi (e : mir_if_phi_expr) : r_if_phi_expr :=
  match e with
  | MIPure expr => RIPure (emit_r expr)
  | MIfPhi cond then_val else_val =>
      RIfPhi (emit_r cond) (emit_r then_val) (emit_r else_val)
  end.

Lemma pure_const_pipeline_preserved :
  eval_r_if_phi (emit_r_if_phi (lower_if_phi (SIPure (SConstInt 4)))) = Some (RVInt 4).
Proof.
  vm_compute. reflexivity.
Qed.

Lemma branch_record_field_src_pipeline_preserved :
  eval_r_if_phi (emit_r_if_phi (lower_if_phi branch_record_field_src)) = Some (RVInt 1).
Proof.
  vm_compute. reflexivity.
Qed.

Lemma branch_record_field_src_false_pipeline_preserved :
  eval_r_if_phi (emit_r_if_phi (lower_if_phi branch_record_field_src_false)) = Some (RVInt 2).
Proof.
  vm_compute. reflexivity.
Qed.

Lemma nested_branch_record_field_src_pipeline_preserved :
  eval_r_if_phi (emit_r_if_phi (lower_if_phi nested_branch_record_field_src)) = Some (RVInt 7).
Proof.
  vm_compute. reflexivity.
Qed.

Lemma branch_add_src_pipeline_preserved :
  eval_r_if_phi (emit_r_if_phi (lower_if_phi branch_add_src)) = Some (RVInt 9).
Proof.
  vm_compute. reflexivity.
Qed.

Lemma branch_add_src_false_pipeline_preserved :
  eval_r_if_phi (emit_r_if_phi (lower_if_phi branch_add_src_false)) = Some (RVInt 9).
Proof.
  vm_compute. reflexivity.
Qed.

End RRPipelineIfPhiSubset.
