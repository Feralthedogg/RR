From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Lia.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRPhiLoopCarried.

Definition var := string.
Definition state := var -> Z.

Inductive loop_expr : Type :=
| LConst : Z -> loop_expr
| LPreVar : var -> loop_expr
| LLoopVar : var -> loop_expr
| LPhi : var -> var -> loop_expr
| LAdd : loop_expr -> loop_expr -> loop_expr.

Fixpoint eval (iter : nat) (entry loop : state) (e : loop_expr) : Z :=
  match e with
  | LConst n => n
  | LPreVar x => entry x
  | LLoopVar x => loop x
  | LPhi seed carried =>
      match iter with
      | O => entry seed
      | S _ => loop carried
      end
  | LAdd lhs rhs => eval iter entry loop lhs + eval iter entry loop rhs
  end.

Fixpoint carried_deps (e : loop_expr) : list var :=
  match e with
  | LConst _ => []
  | LPreVar _ => []
  | LLoopVar x => [x]
  | LPhi _ carried => [carried]
  | LAdd lhs rhs => carried_deps lhs ++ carried_deps rhs
  end.

Definition safe_to_hoist (e : loop_expr) : Prop :=
  carried_deps e = [].

Lemma phi_depends_on_carried_after_entry :
  forall entry loop1 loop2 seed carried,
    loop1 carried <> loop2 carried ->
    eval 1 entry loop1 (LPhi seed carried) <>
    eval 1 entry loop2 (LPhi seed carried).
Proof.
  intros entry loop1 loop2 seed carried Hneq.
  simpl.
  exact Hneq.
Qed.

Lemma phi_plus_const_depends_on_carried_after_entry :
  forall entry loop1 loop2 seed carried k,
    loop1 carried <> loop2 carried ->
    eval 1 entry loop1 (LAdd (LPhi seed carried) (LConst k)) <>
    eval 1 entry loop2 (LAdd (LPhi seed carried) (LConst k)).
Proof.
  intros entry loop1 loop2 seed carried k Hneq Heq.
  simpl in Heq.
  lia.
Qed.

Lemma phi_not_safe_to_hoist :
  forall seed carried,
    ~ safe_to_hoist (LPhi seed carried).
Proof.
  intros seed carried Hsafe.
  discriminate Hsafe.
Qed.

Lemma phi_plus_const_not_safe_to_hoist :
  forall seed carried k,
    ~ safe_to_hoist (LAdd (LPhi seed carried) (LConst k)).
Proof.
  intros seed carried k Hsafe.
  discriminate Hsafe.
Qed.

Lemma hoisting_loop_phi_as_constant_is_unsound :
  forall entry loop1 loop2 seed carried,
    loop1 carried <> loop2 carried ->
    eval 2 entry loop1 (LConst (eval 1 entry loop1 (LPhi seed carried))) <>
    eval 2 entry loop2 (LConst (eval 1 entry loop2 (LPhi seed carried))).
Proof.
  intros entry loop1 loop2 seed carried Hneq.
  simpl.
  exact Hneq.
Qed.

End RRPhiLoopCarried.
