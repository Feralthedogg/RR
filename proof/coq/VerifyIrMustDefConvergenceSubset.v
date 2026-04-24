Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrMustDefFixedPointSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import Bool.
From Stdlib Require Import PeanoNat.
From Stdlib Require Import FunctionalExtensionality.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrMustDefFixedPointSubset.

Module RRVerifyIrMustDefConvergenceSubset.

Definition out_map_stable (entry : nat) (entry_defs : def_set)
    (reachable : reachable_map) (preds : pred_map) (assigned : assign_map)
    (seed : def_map) : Prop :=
  step_out_map entry entry_defs reachable preds assigned seed = seed.

Lemma iterate_out_map_of_stable :
  forall entry entry_defs reachable preds assigned seed fuel,
    out_map_stable entry entry_defs reachable preds assigned seed ->
    iterate_out_map entry entry_defs reachable preds assigned fuel seed = seed.
Proof.
  intros entry entry_defs reachable preds assigned seed fuel Hstable.
  induction fuel as [|fuel IH].
  - reflexivity.
  - simpl. rewrite Hstable. exact IH.
Qed.

Definition example_stable_reachable : reachable_map := example_reachable.

Definition example_stable_pred_map (bid : nat) : list nat :=
  match bid with
  | 1%nat => [0%nat]
  | 2%nat => [0%nat]
  | 3%nat => [1%nat; 2%nat]
  | _ => []
  end.

Definition example_stable_assign_map (bid : nat) : def_set :=
  match bid with
  | 1%nat => ["x"]
  | 2%nat => ["x"]
  | 3%nat => ["tmp"]
  | _ => []
  end.

Definition example_stable_seed (bid : nat) : def_set :=
  match bid with
  | 0%nat => []
  | 1%nat => ["x"]
  | 2%nat => ["x"]
  | 3%nat => ["tmp"; "x"]
  | _ => []
  end.

Lemma example_stable_seed_is_stable :
  out_map_stable 0%nat [] example_stable_reachable example_stable_pred_map
    example_stable_assign_map example_stable_seed.
Proof.
  unfold out_map_stable, step_out_map.
  extensionality bid.
  destruct bid as [|[|[|[|bid]]]]; reflexivity.
Qed.

Lemma example_stable_seed_iterate_five_block3 :
  iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
    example_stable_assign_map 5%nat example_stable_seed 3%nat = ["tmp"; "x"].
Proof.
  rewrite iterate_out_map_of_stable by exact example_stable_seed_is_stable.
  reflexivity.
Qed.

Lemma example_stable_seed_required_x_is_flow_clean_after_five :
  verify_flow_block
    {| flow_defined :=
         iterate_out_map 0%nat [] example_stable_reachable example_stable_pred_map
           example_stable_assign_map 5%nat example_stable_seed 3%nat;
       flow_required := ["x"] |} = None.
Proof.
  apply verify_flow_block_singleton_none_of_must_def.
  rewrite example_stable_seed_iterate_five_block3.
  simpl. auto.
Qed.

End RRVerifyIrMustDefConvergenceSubset.
