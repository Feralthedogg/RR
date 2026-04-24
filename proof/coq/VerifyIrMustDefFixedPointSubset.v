Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import Bool.
From Stdlib Require Import PeanoNat.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.

Module RRVerifyIrMustDefFixedPointSubset.

Definition reachable_map := nat -> bool.
Definition pred_map := nat -> list nat.
Definition assign_map := nat -> def_set.
Definition def_map := nat -> def_set.

Definition reachable_preds (reachable : reachable_map) (preds : pred_map)
    (bid : nat) : list nat :=
  filter reachable (preds bid).

Definition step_in_defs (entry : nat) (entry_defs : def_set)
    (reachable : reachable_map) (preds : pred_map) (out_defs : def_map)
    (bid : nat) : def_set :=
  if negb (reachable bid) then []
  else if Nat.eqb bid entry then entry_defs
  else intersect_pred_out_defs (reachable_preds reachable preds bid) out_defs.

Definition step_out_defs (entry : nat) (entry_defs : def_set)
    (reachable : reachable_map) (preds : pred_map) (assigned : assign_map)
    (out_defs : def_map) (bid : nat) : def_set :=
  out_defs_of_block (step_in_defs entry entry_defs reachable preds out_defs bid) (assigned bid).

Definition step_out_map (entry : nat) (entry_defs : def_set)
    (reachable : reachable_map) (preds : pred_map) (assigned : assign_map)
    (out_defs : def_map) : def_map :=
  fun bid => step_out_defs entry entry_defs reachable preds assigned out_defs bid.

Fixpoint iterate_out_map (entry : nat) (entry_defs : def_set)
    (reachable : reachable_map) (preds : pred_map) (assigned : assign_map)
    (fuel : nat) (seed : def_map) : def_map :=
  match fuel with
  | O => seed
  | S fuel' =>
      iterate_out_map entry entry_defs reachable preds assigned fuel'
        (step_out_map entry entry_defs reachable preds assigned seed)
  end.

Lemma in_step_in_defs_of_forall_reachable_pred :
  forall v entry entry_defs reachable preds out_defs bid,
    reachable bid = true ->
    bid <> entry ->
    reachable_preds reachable preds bid <> [] ->
    (forall pred, In pred (reachable_preds reachable preds bid) -> In v (out_defs pred)) ->
    In v (step_in_defs entry entry_defs reachable preds out_defs bid).
Proof.
  intros v entry entry_defs reachable preds out_defs bid Hreach Hbid Hpreds Hall.
  unfold step_in_defs.
  rewrite Hreach.
  destruct (Nat.eqb bid entry) eqn:Heq.
  - apply Nat.eqb_eq in Heq. contradiction.
  - apply in_intersect_pred_out_defs_of_forall_pred; assumption.
Qed.

Lemma in_step_out_defs_of_forall_reachable_pred :
  forall v entry entry_defs reachable preds assigned out_defs bid,
    reachable bid = true ->
    bid <> entry ->
    reachable_preds reachable preds bid <> [] ->
    (forall pred, In pred (reachable_preds reachable preds bid) -> In v (out_defs pred)) ->
    In v (step_out_defs entry entry_defs reachable preds assigned out_defs bid).
Proof.
  intros v entry entry_defs reachable preds assigned out_defs bid Hreach Hbid Hpreds Hall.
  unfold step_out_defs.
  apply in_or_app.
  right.
  apply in_step_in_defs_of_forall_reachable_pred; assumption.
Qed.

Lemma in_step_out_defs_entry_of_in_entry_defs :
  forall v entry entry_defs reachable preds assigned out_defs,
    reachable entry = true ->
    In v entry_defs ->
    In v (step_out_defs entry entry_defs reachable preds assigned out_defs entry).
Proof.
  intros v entry entry_defs reachable preds assigned out_defs Hreach Hentry.
  unfold step_out_defs, step_in_defs.
  rewrite Hreach.
  rewrite Nat.eqb_refl.
  apply in_or_app.
  right.
  exact Hentry.
Qed.

Lemma iterate_out_map_one_apply :
  forall entry entry_defs reachable preds assigned seed bid,
    iterate_out_map entry entry_defs reachable preds assigned 1 seed bid =
    step_out_defs entry entry_defs reachable preds assigned seed bid.
Proof.
  reflexivity.
Qed.

Definition example_reachable (bid : nat) : bool :=
  match bid with
  | 0%nat => true
  | 1%nat => true
  | 2%nat => true
  | 3%nat => true
  | _ => false
  end.

Definition example_pred_map (bid : nat) : list nat :=
  match bid with
  | 3%nat => [1%nat; 2%nat]
  | _ => []
  end.

Definition example_assign_map (bid : nat) : def_set :=
  match bid with
  | 3%nat => ["tmp"]
  | _ => []
  end.

Definition example_seed_out_defs : def_map := example_out_defs.

Lemma example_join_reachable_preds_nonempty :
  reachable_preds example_reachable example_pred_map 3%nat <> [].
Proof.
  discriminate.
Qed.

Lemma example_join_step_out_contains_x :
  In "x"
    (step_out_defs 0%nat [] example_reachable example_pred_map example_assign_map
      example_seed_out_defs 3%nat).
Proof.
  apply in_step_out_defs_of_forall_reachable_pred.
  - reflexivity.
  - discriminate.
  - exact example_join_reachable_preds_nonempty.
  - intros pred Hpred.
    simpl in Hpred.
    destruct Hpred as [Hpred|[Hpred|[]]]; subst pred; simpl; auto.
Qed.

Lemma example_join_after_one_iteration_contains_x :
  In "x"
    (iterate_out_map 0%nat [] example_reachable example_pred_map example_assign_map
      1%nat example_seed_out_defs 3%nat).
Proof.
  rewrite iterate_out_map_one_apply.
  exact example_join_step_out_contains_x.
Qed.

Lemma example_join_required_x_is_flow_clean_after_one_iteration :
  verify_flow_block
    {| flow_defined :=
         iterate_out_map 0%nat [] example_reachable example_pred_map example_assign_map
           1%nat example_seed_out_defs 3%nat;
       flow_required := ["x"] |} = None.
Proof.
  apply verify_flow_block_singleton_none_of_must_def.
  exact example_join_after_one_iteration_contains_x.
Qed.

End RRVerifyIrMustDefFixedPointSubset.
