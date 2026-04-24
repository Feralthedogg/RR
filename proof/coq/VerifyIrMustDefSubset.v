Require Import VerifyIrFlowLite.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import Bool.
From Stdlib Require Import PeanoNat.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrFlowLite.

Module RRVerifyIrMustDefSubset.

Definition def_set := list string.

Fixpoint intersect_defs (xs ys : def_set) : def_set :=
  match xs with
  | [] => []
  | x :: rest =>
      if in_dec String.string_dec x ys then
        x :: intersect_defs rest ys
      else
        intersect_defs rest ys
  end.

Definition fold_pred_intersections (base : def_set)
    (preds : list nat) (out_defs : nat -> def_set) : def_set :=
  fold_left (fun acc pred => intersect_defs acc (out_defs pred)) preds base.

Definition intersect_pred_out_defs
    (preds : list nat) (out_defs : nat -> def_set) : def_set :=
  match preds with
  | [] => []
  | p :: rest => fold_pred_intersections (out_defs p) rest out_defs
  end.

Definition out_defs_of_block (in_defs assigned : def_set) : def_set :=
  List.app assigned in_defs.

Definition in_defs_from_preds (entry : nat) (entry_defs : def_set)
    (preds : list nat) (out_defs : nat -> def_set) (bid : nat) : def_set :=
  if Nat.eqb bid entry then entry_defs else intersect_pred_out_defs preds out_defs.

Lemma in_intersect_defs_of_in :
  forall v xs ys,
    In v xs ->
    In v ys ->
    In v (intersect_defs xs ys).
Proof.
  intros v xs.
  induction xs as [|x xs IH]; intros ys Hxs Hys.
  - contradiction.
  - simpl in Hxs.
    simpl.
    destruct (in_dec String.string_dec x ys) as [Hxy|Hxy].
    + destruct Hxs as [Hx|Hrest].
      * subst x. simpl. auto.
      * right. apply IH; assumption.
    + destruct Hxs as [Hx|Hrest].
      * subst x. contradiction.
      * apply IH; assumption.
Qed.

Lemma in_fold_pred_intersections_of_forall_pred :
  forall v base preds out_defs,
    In v base ->
    (forall pred, In pred preds -> In v (out_defs pred)) ->
    In v (fold_pred_intersections base preds out_defs).
Proof.
  intros v base preds out_defs Hbase Hall.
  unfold fold_pred_intersections.
  revert base Hbase.
  induction preds as [|pred preds IH]; intros base Hbase.
  - exact Hbase.
  - simpl.
    eapply IH.
    + intros pred' Hpred'.
      apply Hall. simpl. auto.
    + apply in_intersect_defs_of_in.
      * exact Hbase.
      * apply Hall. simpl. auto.
Qed.

Lemma in_intersect_pred_out_defs_of_forall_pred :
  forall v preds out_defs,
    preds <> [] ->
    (forall pred, In pred preds -> In v (out_defs pred)) ->
    In v (intersect_pred_out_defs preds out_defs).
Proof.
  intros v preds out_defs Hpreds Hall.
  destruct preds as [|pred preds].
  - contradiction.
  - simpl.
    apply in_fold_pred_intersections_of_forall_pred.
    + apply Hall. simpl. auto.
    + intros pred' Hpred'.
      apply Hall. simpl. auto.
Qed.

Lemma in_out_defs_of_block_of_in_assigned :
  forall v in_defs assigned,
    In v assigned ->
    In v (out_defs_of_block in_defs assigned).
Proof.
  intros v in_defs assigned H.
  unfold out_defs_of_block.
  apply in_or_app.
  left. exact H.
Qed.

Lemma in_in_defs_from_preds_of_forall_pred :
  forall v entry entry_defs preds out_defs bid,
    bid <> entry ->
    preds <> [] ->
    (forall pred, In pred preds -> In v (out_defs pred)) ->
    In v (in_defs_from_preds entry entry_defs preds out_defs bid).
Proof.
  intros v entry entry_defs preds out_defs bid Hbid Hpreds Hall.
  unfold in_defs_from_preds.
  destruct (Nat.eqb bid entry) eqn:Heq.
  - apply Nat.eqb_eq in Heq. contradiction.
  - apply in_intersect_pred_out_defs_of_forall_pred; assumption.
Qed.

Lemma first_missing_var_singleton_none_of_in :
  forall defs v,
    In v defs ->
    first_missing_var defs [v] = None.
Proof.
  intros defs v H.
  simpl.
  destruct (in_dec String.string_dec v defs).
  - reflexivity.
  - contradiction.
Qed.

Lemma verify_flow_block_singleton_none_of_must_def :
  forall defs v,
    In v defs ->
    verify_flow_block {| flow_defined := defs; flow_required := [v] |} = None.
Proof.
  intros defs v H.
  unfold verify_flow_block.
  simpl.
  destruct (in_dec String.string_dec v defs).
  - reflexivity.
  - contradiction.
Qed.

Definition example_preds : list nat := [1%nat; 2%nat].

Definition example_out_defs (bid : nat) : def_set :=
  match bid with
  | 1%nat => ["x"; "y"]
  | 2%nat => ["x"; "z"]
  | _ => []
  end.

Definition example_assigned : def_set := ["tmp"].

Lemma example_out_defs_contains_tmp :
  In "tmp" (out_defs_of_block ["param"] example_assigned).
Proof.
  apply in_out_defs_of_block_of_in_assigned.
  simpl. auto.
Qed.

Lemma example_join_contains_x :
  In "x" (in_defs_from_preds 0%nat [] example_preds example_out_defs 3%nat).
Proof.
  apply in_in_defs_from_preds_of_forall_pred.
  - discriminate.
  - discriminate.
  - intros pred Hpred.
    simpl in Hpred.
    destruct Hpred as [Hpred|[Hpred|[]]]; subst pred; simpl; auto.
Qed.

Lemma example_join_required_x_is_flow_clean :
  verify_flow_block
    {| flow_defined := in_defs_from_preds 0%nat [] example_preds example_out_defs 3%nat;
       flow_required := ["x"] |} = None.
Proof.
  apply verify_flow_block_singleton_none_of_must_def.
  exact example_join_contains_x.
Qed.

End RRVerifyIrMustDefSubset.
