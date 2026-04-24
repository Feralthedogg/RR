Require Import VerifyIrConsumerMetaSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import PeanoNat.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerMetaSubset.

Module RRVerifyIrConsumerGraphSubset.

Definition consumer_node_id := nat.

Inductive consumer_graph_node : Type :=
| CGMeta : consumer_meta -> consumer_graph_node
| CGWrap1 : consumer_node_id -> consumer_graph_node
| CGWrap2 : consumer_node_id -> consumer_node_id -> consumer_graph_node.

Definition consumer_graph := consumer_node_id -> option consumer_graph_node.

Fixpoint node_scan_clean_fuel
    (fuel : nat) (graph : consumer_graph) (seen : list consumer_node_id)
    (root : consumer_node_id) : Prop :=
  match fuel with
  | O => True
  | S fuel' =>
      if in_dec Nat.eq_dec root seen then
        True
      else
        match graph root with
        | None => True
        | Some (CGMeta c) => consumer_meta_clean c
        | Some (CGWrap1 child) => node_scan_clean_fuel fuel' graph (root :: seen) child
        | Some (CGWrap2 lhs rhs) =>
            node_scan_clean_fuel fuel' graph (root :: seen) lhs /\
            node_scan_clean_fuel fuel' graph (root :: seen) rhs
        end
  end.

Fixpoint root_list_scan_clean_fuel
    (fuel : nat) (graph : consumer_graph) (seen roots : list consumer_node_id) : Prop :=
  match fuel with
  | O => True
  | S fuel' =>
      match roots with
      | [] => True
      | root :: rest =>
          node_scan_clean_fuel (S fuel') graph seen root /\
          root_list_scan_clean_fuel fuel' graph (root :: seen) rest
      end
  end.

Lemma node_scan_clean_fuel_meta_of_clean :
  forall fuel graph seen root c,
    graph root = Some (CGMeta c) ->
    consumer_meta_clean c ->
    node_scan_clean_fuel (S fuel) graph seen root.
Proof.
  intros fuel graph seen root c HNode HClean.
  simpl.
  destruct (in_dec Nat.eq_dec root seen); simpl; auto.
  rewrite HNode. exact HClean.
Qed.

Lemma node_scan_clean_fuel_wrap1_of_child :
  forall fuel graph seen root child,
    graph root = Some (CGWrap1 child) ->
    node_scan_clean_fuel fuel graph (root :: seen) child ->
    node_scan_clean_fuel (S fuel) graph seen root.
Proof.
  intros fuel graph seen root child HNode HChild.
  simpl.
  destruct (in_dec Nat.eq_dec root seen); simpl; auto.
  rewrite HNode. exact HChild.
Qed.

Lemma node_scan_clean_fuel_wrap2_of_children :
  forall fuel graph seen root lhs rhs,
    graph root = Some (CGWrap2 lhs rhs) ->
    node_scan_clean_fuel fuel graph (root :: seen) lhs ->
    node_scan_clean_fuel fuel graph (root :: seen) rhs ->
    node_scan_clean_fuel (S fuel) graph seen root.
Proof.
  intros fuel graph seen root lhs rhs HNode HL HR.
  simpl.
  destruct (in_dec Nat.eq_dec root seen); simpl; auto.
  rewrite HNode. split; assumption.
Qed.

Lemma root_list_scan_clean_fuel_cons :
  forall fuel graph seen root rest,
    node_scan_clean_fuel (S fuel) graph seen root ->
    root_list_scan_clean_fuel fuel graph (root :: seen) rest ->
    root_list_scan_clean_fuel (S fuel) graph seen (root :: rest).
Proof.
  intros fuel graph seen root rest HRoot HRest.
  simpl. exact (conj HRoot HRest).
Qed.

Definition example_consumer_graph : consumer_graph :=
  fun root =>
    match root with
    | 1%nat => Some (CGMeta example_call_consumer)
    | 2%nat => Some (CGMeta example_intrinsic_consumer)
    | 3%nat => Some (CGMeta example_record_consumer)
    | 4%nat => Some (CGWrap2 1%nat 1%nat)
    | 5%nat => Some (CGWrap2 1%nat 2%nat)
    | 6%nat => Some (CGWrap1 3%nat)
    | _ => None
    end.

Lemma example_shared_call_node_clean :
  node_scan_clean_fuel 2%nat example_consumer_graph [] 4%nat.
Proof.
  apply node_scan_clean_fuel_wrap2_of_children with (lhs := 1%nat) (rhs := 1%nat).
  - reflexivity.
  - apply node_scan_clean_fuel_meta_of_clean with (c := example_call_consumer).
    + reflexivity.
    + exact example_call_consumer_clean.
  - apply node_scan_clean_fuel_meta_of_clean with (c := example_call_consumer).
    + reflexivity.
    + exact example_call_consumer_clean.
Qed.

Lemma example_call_intrinsic_node_clean :
  node_scan_clean_fuel 2%nat example_consumer_graph [] 5%nat.
Proof.
  apply node_scan_clean_fuel_wrap2_of_children with (lhs := 1%nat) (rhs := 2%nat).
  - reflexivity.
  - apply node_scan_clean_fuel_meta_of_clean with (c := example_call_consumer).
    + reflexivity.
    + exact example_call_consumer_clean.
  - apply node_scan_clean_fuel_meta_of_clean with (c := example_intrinsic_consumer).
    + reflexivity.
    + exact example_intrinsic_consumer_clean.
Qed.

Lemma example_wrapped_record_node_clean :
  node_scan_clean_fuel 2%nat example_consumer_graph [] 6%nat.
Proof.
  apply node_scan_clean_fuel_wrap1_of_child with (child := 3%nat).
  - reflexivity.
  - apply node_scan_clean_fuel_meta_of_clean with (c := example_record_consumer).
    + reflexivity.
    + exact example_record_consumer_clean.
Qed.

Lemma example_consumer_graph_root_list_clean :
  root_list_scan_clean_fuel 4%nat example_consumer_graph [] [4%nat; 5%nat; 6%nat].
Proof.
  apply root_list_scan_clean_fuel_cons.
  - exact example_shared_call_node_clean.
  - apply root_list_scan_clean_fuel_cons.
    + exact example_call_intrinsic_node_clean.
    + apply root_list_scan_clean_fuel_cons.
      * exact example_wrapped_record_node_clean.
      * simpl. exact I.
Qed.

End RRVerifyIrConsumerGraphSubset.
