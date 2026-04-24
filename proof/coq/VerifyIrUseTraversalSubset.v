Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrMustDefFixedPointSubset.
Require Import VerifyIrMustDefConvergenceSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import Bool.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrMustDefFixedPointSubset.
Import RRVerifyIrMustDefConvergenceSubset.

Module RRVerifyIrUseTraversalSubset.

Inductive use_expr : Type :=
| UEConst
| UELoad : string -> use_expr
| UEWrap : use_expr -> use_expr
| UEPair : use_expr -> use_expr -> use_expr
| UEPhi : use_expr -> use_expr.

Fixpoint first_undefined_load (defined : def_set) (follow_phi : bool)
    (e : use_expr) : option string :=
  match e with
  | UEConst => None
  | UELoad v =>
      if in_dec String.string_dec v defined then None else Some v
  | UEWrap inner => first_undefined_load defined follow_phi inner
  | UEPair lhs rhs =>
      match first_undefined_load defined follow_phi lhs with
      | Some v => Some v
      | None => first_undefined_load defined follow_phi rhs
      end
  | UEPhi inner =>
      if follow_phi then first_undefined_load defined follow_phi inner else None
  end.

Fixpoint loads_defined (defined : def_set) (follow_phi : bool)
    (e : use_expr) : Prop :=
  match e with
  | UEConst => True
  | UELoad v => In v defined
  | UEWrap inner => loads_defined defined follow_phi inner
  | UEPair lhs rhs =>
      (loads_defined defined follow_phi lhs /\
       loads_defined defined follow_phi rhs)%type
  | UEPhi inner =>
      if follow_phi then loads_defined defined follow_phi inner else True
  end.

Lemma first_undefined_load_none_of_loads_defined :
  forall defined follow_phi e,
    loads_defined defined follow_phi e ->
    first_undefined_load defined follow_phi e = None.
Proof.
  intros defined follow_phi e.
  induction e as [|v|inner IH|lhs IHl rhs IHr|inner IH]; intros H.
  - reflexivity.
  - simpl in H. simpl.
    destruct (in_dec String.string_dec v defined).
    + reflexivity.
    + contradiction.
  - simpl in H. exact (IH H).
  - simpl in H. destruct H as [Hl Hr].
    simpl. rewrite (IHl Hl). exact (IHr Hr).
  - simpl.
    destruct follow_phi.
    + exact (IH H).
    + reflexivity.
Qed.

Definition example_traversal_expr : use_expr :=
  UEPair (UEWrap (UELoad "x")) (UEPair (UELoad "tmp") UEConst).

Definition example_phi_traversal_expr : use_expr :=
  UEPhi (UELoad "missing").

Lemma example_stable_traversal_loads_defined :
  loads_defined
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    true
    example_traversal_expr.
Proof.
  rewrite example_stable_seed_iterate_five_block3.
  simpl. auto.
Qed.

Lemma example_stable_traversal_scan_clean :
  first_undefined_load
    (iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
      example_stable_assign_map 5%nat example_stable_seed 3%nat)
    true
    example_traversal_expr = None.
Proof.
  apply first_undefined_load_none_of_loads_defined.
  exact example_stable_traversal_loads_defined.
Qed.

Lemma example_phi_traversal_ignored_when_not_following :
  first_undefined_load [] false example_phi_traversal_expr = None.
Proof.
  reflexivity.
Qed.

End RRVerifyIrUseTraversalSubset.
