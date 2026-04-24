Require Import VerifyIrConsumerMetaSubset.
Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrChildDepsSubset.
Require Import VerifyIrValueDepsWalkSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import PeanoNat.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerMetaSubset.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrChildDepsSubset.
Import RRVerifyIrValueDepsWalkSubset.

Module RRVerifyIrValueTableWalkSubset.

Record table_value : Type := {
  table_phi_block : option nat;
  table_deps_kind : value_deps_kind;
}.

Definition value_table := list table_value.

Definition lookup_table_value (table : value_table) (root : consumer_node_id)
    : option table_value :=
  nth_error table root.

Definition lookup_value_deps (table : value_table) (root : consumer_node_id)
    : option (list consumer_node_id) :=
  match lookup_table_value table root with
  | Some node => Some (value_deps (table_deps_kind node))
  | None => None
  end.

Fixpoint depends_on_phi_in_block_except_table_fuel
    (fuel : nat) (table : value_table) (seen : list consumer_node_id)
    (root phi_block exempt : nat) : Prop :=
  match fuel with
  | O => False
  | S fuel' =>
      if in_dec Nat.eq_dec root seen then
        False
      else
        match lookup_table_value table root with
        | None => False
        | Some node =>
            ((root <> exempt) /\ table_phi_block node = Some phi_block) \/
            dep_list_depends_on_phi_table_fuel fuel' table (root :: seen)
              (value_deps (table_deps_kind node)) phi_block exempt
        end
  end
with dep_list_depends_on_phi_table_fuel
    (fuel : nat) (table : value_table) (seen deps : list consumer_node_id)
    (phi_block exempt : nat) : Prop :=
  match fuel with
  | O => False
  | S fuel' =>
      match deps with
      | [] => False
      | root :: rest =>
          depends_on_phi_in_block_except_table_fuel fuel' table seen root phi_block exempt \/
          dep_list_depends_on_phi_table_fuel fuel' table seen rest phi_block exempt
      end
  end.

Lemma lookup_value_deps_some :
  forall table root node,
    lookup_table_value table root = Some node ->
    lookup_value_deps table root = Some (value_deps (table_deps_kind node)).
Proof.
  intros table root node H.
  unfold lookup_value_deps. rewrite H. reflexivity.
Qed.

Lemma lookup_value_deps_none :
  forall table root,
    lookup_table_value table root = None ->
    lookup_value_deps table root = None.
Proof.
  intros table root H.
  unfold lookup_value_deps. rewrite H. reflexivity.
Qed.

Lemma depends_on_phi_in_block_except_table_fuel_here :
  forall fuel table seen root phi_block exempt node,
    lookup_table_value table root = Some node ->
    table_phi_block node = Some phi_block ->
    ~ In root seen ->
    root <> exempt ->
    depends_on_phi_in_block_except_table_fuel (S fuel) table seen root phi_block exempt.
Proof.
  intros fuel table seen root phi_block exempt node HLookup HPhi HFresh HNe.
  simpl.
  destruct (in_dec Nat.eq_dec root seen); simpl.
  - contradiction.
  - rewrite HLookup. left. split.
    + exact HNe.
    + exact HPhi.
Qed.

Lemma dep_list_depends_on_phi_table_fuel_head :
  forall fuel table seen root rest phi_block exempt,
    depends_on_phi_in_block_except_table_fuel fuel table seen root phi_block exempt ->
    dep_list_depends_on_phi_table_fuel (S fuel) table seen (root :: rest) phi_block exempt.
Proof.
  intros fuel table seen root rest phi_block exempt HRoot.
  simpl. auto.
Qed.

Lemma dep_list_depends_on_phi_table_fuel_tail :
  forall fuel table seen root rest phi_block exempt,
    dep_list_depends_on_phi_table_fuel fuel table seen rest phi_block exempt ->
    dep_list_depends_on_phi_table_fuel (S fuel) table seen (root :: rest) phi_block exempt.
Proof.
  intros fuel table seen root rest phi_block exempt HRest.
  simpl. auto.
Qed.

Lemma depends_on_phi_in_block_except_table_fuel_of_dep_list :
  forall fuel table seen root phi_block exempt node,
    lookup_table_value table root = Some node ->
    ~ In root seen ->
    dep_list_depends_on_phi_table_fuel fuel table (root :: seen)
      (value_deps (table_deps_kind node)) phi_block exempt ->
    depends_on_phi_in_block_except_table_fuel (S fuel) table seen root phi_block exempt.
Proof.
  intros fuel table seen root phi_block exempt node HLookup HFresh HDeps.
  simpl.
  destruct (in_dec Nat.eq_dec root seen); simpl.
  - contradiction.
  - rewrite HLookup. right. exact HDeps.
Qed.

Definition example_phi_walk_table : value_table :=
  [ {| table_phi_block := None; table_deps_kind := VDBinary 1%nat 2%nat |}
  ; {| table_phi_block := Some 7%nat; table_deps_kind := VDPhi [3%nat] |}
  ; {| table_phi_block := None; table_deps_kind := VDCall [4%nat; 5%nat] |}
  ; {| table_phi_block := Some 7%nat; table_deps_kind := VDConstLike |}
  ; {| table_phi_block := None; table_deps_kind := VDConstLike |}
  ; {| table_phi_block := Some 8%nat; table_deps_kind := VDConstLike |}
  ; {| table_phi_block := None; table_deps_kind := VDBinary 6%nat 1%nat |}
  ].

Lemma example_lookup_value_deps_phi :
  lookup_value_deps example_phi_walk_table 1%nat = Some [3%nat].
Proof.
  reflexivity.
Qed.

Lemma example_lookup_value_deps_binary :
  lookup_value_deps example_phi_walk_table 6%nat = Some [6%nat; 1%nat].
Proof.
  reflexivity.
Qed.

Lemma example_lookup_value_deps_oob :
  lookup_value_deps example_phi_walk_table 99%nat = None.
Proof.
  reflexivity.
Qed.

Lemma example_table_depends_direct_phi :
  depends_on_phi_in_block_except_table_fuel 3%nat example_phi_walk_table [] 0%nat 7%nat 99%nat.
Proof.
  apply depends_on_phi_in_block_except_table_fuel_of_dep_list with
    (node := {| table_phi_block := None; table_deps_kind := VDBinary 1%nat 2%nat |}).
  - reflexivity.
  - simpl. tauto.
  - apply dep_list_depends_on_phi_table_fuel_head.
    apply depends_on_phi_in_block_except_table_fuel_here with
      (node := {| table_phi_block := Some 7%nat; table_deps_kind := VDPhi [3%nat] |}).
    + reflexivity.
    + reflexivity.
    + simpl. intros [H | []]. discriminate H.
    + discriminate.
Qed.

Lemma example_table_depends_exempt_phi_through_arg :
  depends_on_phi_in_block_except_table_fuel 3%nat example_phi_walk_table [] 1%nat 7%nat 1%nat.
Proof.
  apply depends_on_phi_in_block_except_table_fuel_of_dep_list with
    (node := {| table_phi_block := Some 7%nat; table_deps_kind := VDPhi [3%nat] |}).
  - reflexivity.
  - simpl. tauto.
  - apply dep_list_depends_on_phi_table_fuel_head.
    apply depends_on_phi_in_block_except_table_fuel_here with
      (node := {| table_phi_block := Some 7%nat; table_deps_kind := VDConstLike |}).
    + reflexivity.
    + reflexivity.
    + simpl. intros [H | []]. discriminate H.
    + discriminate.
Qed.

Lemma example_table_depends_other_block_ignored :
  ~ depends_on_phi_in_block_except_table_fuel 3%nat example_phi_walk_table [] 2%nat 7%nat 99%nat.
Proof.
  simpl. intro H.
  repeat match type of H with
  | _ /\ _ => destruct H
  | _ \/ _ => destruct H
  end;
  congruence.
Qed.

Lemma example_table_depends_self_loop_skips_seen_but_finds_phi :
  depends_on_phi_in_block_except_table_fuel 4%nat example_phi_walk_table [] 6%nat 7%nat 99%nat.
Proof.
  apply depends_on_phi_in_block_except_table_fuel_of_dep_list with
    (node := {| table_phi_block := None; table_deps_kind := VDBinary 6%nat 1%nat |}).
  - reflexivity.
  - simpl. tauto.
  - apply dep_list_depends_on_phi_table_fuel_tail.
    apply dep_list_depends_on_phi_table_fuel_head.
    apply depends_on_phi_in_block_except_table_fuel_here with
      (node := {| table_phi_block := Some 7%nat; table_deps_kind := VDPhi [3%nat] |}).
    + reflexivity.
    + reflexivity.
    + simpl. intros [H | []]. discriminate H.
    + discriminate.
Qed.

End RRVerifyIrValueTableWalkSubset.
