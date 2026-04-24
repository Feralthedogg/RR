Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.

Module RRLoweringIfPhiSubset.

Inductive src_if_expr : Type :=
| SIPure : src_expr -> src_if_expr
| SIte : src_expr -> src_expr -> src_expr -> src_if_expr.

Inductive mir_if_phi_expr : Type :=
| MIPure : mir_expr -> mir_if_phi_expr
| MIfPhi : mir_expr -> mir_expr -> mir_expr -> mir_if_phi_expr.

Definition eval_src_if_fuel (fuel : nat) (e : src_if_expr) : option rvalue :=
  match e with
  | SIPure expr => eval_src_fuel fuel expr
  | SIte cond then_expr else_expr =>
      match eval_src_fuel fuel cond with
      | Some (RVBool true) => eval_src_fuel fuel then_expr
      | Some (RVBool false) => eval_src_fuel fuel else_expr
      | _ => None
      end
  end.

Definition eval_src_if (e : src_if_expr) : option rvalue :=
  eval_src_if_fuel 32 e.

Definition eval_mir_if_phi_fuel (fuel : nat) (e : mir_if_phi_expr) : option rvalue :=
  match e with
  | MIPure expr => eval_mir_fuel fuel expr
  | MIfPhi cond then_val else_val =>
      match eval_mir_fuel fuel cond with
      | Some (RVBool true) => eval_mir_fuel fuel then_val
      | Some (RVBool false) => eval_mir_fuel fuel else_val
      | _ => None
      end
  end.

Definition eval_mir_if_phi (e : mir_if_phi_expr) : option rvalue :=
  eval_mir_if_phi_fuel 32 e.

Definition lower_if_phi (e : src_if_expr) : mir_if_phi_expr :=
  match e with
  | SIPure expr => MIPure (lower expr)
  | SIte cond then_expr else_expr =>
      MIfPhi (lower cond) (lower then_expr) (lower else_expr)
  end.

Lemma lower_if_phi_preserves_eval_fuel :
  forall fuel expr,
    eval_mir_if_phi_fuel fuel (lower_if_phi expr) =
      eval_src_if_fuel fuel expr.
Proof.
  intros fuel expr.
  destruct expr as [expr | cond then_expr else_expr]; simpl.
  - apply lower_preserves_eval_fuel.
  - rewrite !lower_preserves_eval_fuel.
    destruct (eval_src_fuel fuel cond); [destruct r|]; reflexivity.
Qed.

Lemma lower_if_phi_preserves_eval :
  forall expr,
    eval_mir_if_phi (lower_if_phi expr) = eval_src_if expr.
Proof.
  intro expr.
  exact (lower_if_phi_preserves_eval_fuel 32 expr).
Qed.

Lemma lower_if_phi_preserves_eval_pure_const :
  eval_mir_if_phi (lower_if_phi (SIPure (SConstInt 4))) =
    eval_src_if (SIPure (SConstInt 4)).
Proof.
  apply lower_if_phi_preserves_eval.
Qed.

Definition branch_record_field_src : src_if_expr :=
  SIte
    (SConstBool true)
    (SField (SRecord [("x", SConstInt 1)]) "x")
    (SField (SRecord [("x", SConstInt 2)]) "x").

Lemma branch_record_field_src_preserved :
  eval_mir_if_phi (lower_if_phi branch_record_field_src) = Some (RVInt 1).
Proof.
  reflexivity.
Qed.

Definition branch_record_field_src_false : src_if_expr :=
  SIte
    (SConstBool false)
    (SField (SRecord [("x", SConstInt 1)]) "x")
    (SField (SRecord [("x", SConstInt 2)]) "x").

Lemma branch_record_field_src_false_preserved :
  eval_mir_if_phi (lower_if_phi branch_record_field_src_false) = Some (RVInt 2).
Proof.
  reflexivity.
Qed.

Definition nested_branch_record_field_src : src_if_expr :=
  SIte
    (SConstBool true)
    (SField (SField (SRecord [("inner", SRecord [("x", SConstInt 7)])]) "inner") "x")
    (SConstInt 0).

Lemma nested_branch_record_field_src_preserved :
  eval_mir_if_phi (lower_if_phi nested_branch_record_field_src) = Some (RVInt 7).
Proof.
  reflexivity.
Qed.

Definition branch_add_src : src_if_expr :=
  SIte
    (SConstBool true)
    (SAdd (SConstInt 4) (SConstInt 5))
    (SConstInt 0).

Lemma branch_add_src_preserved :
  eval_mir_if_phi (lower_if_phi branch_add_src) = Some (RVInt 9).
Proof.
  reflexivity.
Qed.

Definition branch_add_src_false : src_if_expr :=
  SIte
    (SConstBool false)
    (SConstInt 0)
    (SAdd (SConstInt 4) (SConstInt 5)).

Lemma branch_add_src_false_preserved :
  eval_mir_if_phi (lower_if_phi branch_add_src_false) = Some (RVInt 9).
Proof.
  reflexivity.
Qed.

End RRLoweringIfPhiSubset.
