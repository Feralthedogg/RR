Require Import VerifyIrConsumerMetaSubset.
Require Import VerifyIrConsumerGraphSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerMetaSubset.
Import RRVerifyIrConsumerGraphSubset.

Module RRVerifyIrChildDepsSubset.

Inductive child_deps_kind : Type :=
| CDConstLike
| CDParamLike
| CDLoadLike
| CDRSymbolLike
| CDLen : consumer_node_id -> child_deps_kind
| CDIndices : consumer_node_id -> child_deps_kind
| CDUnary : consumer_node_id -> child_deps_kind
| CDFieldGet : consumer_node_id -> child_deps_kind
| CDRange : consumer_node_id -> consumer_node_id -> child_deps_kind
| CDBinary : consumer_node_id -> consumer_node_id -> child_deps_kind
| CDPhi
| CDCall : list consumer_node_id -> child_deps_kind
| CDIntrinsic : list consumer_node_id -> child_deps_kind
| CDRecordLit : list (string * consumer_node_id) -> child_deps_kind
| CDFieldSet : consumer_node_id -> consumer_node_id -> child_deps_kind
| CDIndex1D : consumer_node_id -> consumer_node_id -> child_deps_kind
| CDIndex2D : consumer_node_id -> consumer_node_id -> consumer_node_id -> child_deps_kind
| CDIndex3D : consumer_node_id -> consumer_node_id -> consumer_node_id -> consumer_node_id ->
    child_deps_kind.

Definition non_phi_deps (kind : child_deps_kind) : list consumer_node_id :=
  match kind with
  | CDConstLike | CDParamLike | CDLoadLike | CDRSymbolLike | CDPhi => []
  | CDLen base | CDIndices base | CDUnary base | CDFieldGet base => [base]
  | CDRange start finish | CDBinary start finish | CDFieldSet start finish
  | CDIndex1D start finish => [start; finish]
  | CDCall args | CDIntrinsic args => args
  | CDRecordLit fields => map snd fields
  | CDIndex2D base r c => [base; r; c]
  | CDIndex3D base i j k => [base; i; j; k]
  end.

Definition dep_traversal_clean_fuel
    (fuel : nat) (graph : consumer_graph) (seen : list consumer_node_id)
    (kind : child_deps_kind) : Prop :=
  root_list_scan_clean_fuel fuel graph seen (non_phi_deps kind).

Lemma dep_traversal_clean_fuel_of_root_list :
  forall fuel graph seen kind,
    root_list_scan_clean_fuel fuel graph seen (non_phi_deps kind) ->
    dep_traversal_clean_fuel fuel graph seen kind.
Proof.
  intros fuel graph seen kind H.
  exact H.
Qed.

Lemma dep_traversal_clean_fuel_const_like :
  forall fuel graph seen,
    dep_traversal_clean_fuel fuel graph seen CDConstLike.
Proof.
  intros fuel graph seen.
  unfold dep_traversal_clean_fuel, non_phi_deps.
  destruct fuel; simpl; exact I.
Qed.

Lemma dep_traversal_clean_fuel_phi :
  forall fuel graph seen,
    dep_traversal_clean_fuel fuel graph seen CDPhi.
Proof.
  intros fuel graph seen.
  unfold dep_traversal_clean_fuel, non_phi_deps.
  destruct fuel; simpl; exact I.
Qed.

Lemma example_unary_dep_traversal_clean :
  dep_traversal_clean_fuel 2%nat example_consumer_graph [] (CDUnary 6%nat).
Proof.
  unfold dep_traversal_clean_fuel, non_phi_deps.
  apply root_list_scan_clean_fuel_cons.
  - exact example_wrapped_record_node_clean.
  - simpl. exact I.
Qed.

Lemma example_binary_dep_traversal_clean :
  dep_traversal_clean_fuel 3%nat example_consumer_graph [] (CDBinary 4%nat 5%nat).
Proof.
  unfold dep_traversal_clean_fuel, non_phi_deps.
  apply root_list_scan_clean_fuel_cons.
  - exact example_shared_call_node_clean.
  - apply root_list_scan_clean_fuel_cons.
    + exact example_call_intrinsic_node_clean.
    + simpl. exact I.
Qed.

Lemma example_call_deps_clean :
  dep_traversal_clean_fuel 2%nat example_consumer_graph [] (CDCall [1%nat; 2%nat]).
Proof.
  unfold dep_traversal_clean_fuel, non_phi_deps.
  apply root_list_scan_clean_fuel_cons.
  - apply node_scan_clean_fuel_meta_of_clean with (c := example_call_consumer).
    + reflexivity.
    + exact example_call_consumer_clean.
  - apply root_list_scan_clean_fuel_cons.
    + apply node_scan_clean_fuel_meta_of_clean with (c := example_intrinsic_consumer).
      * reflexivity.
      * exact example_intrinsic_consumer_clean.
    + simpl. exact I.
Qed.

Lemma example_intrinsic_deps_clean :
  dep_traversal_clean_fuel 2%nat example_consumer_graph [] (CDIntrinsic [2%nat; 1%nat]).
Proof.
  unfold dep_traversal_clean_fuel, non_phi_deps.
  apply root_list_scan_clean_fuel_cons.
  - apply node_scan_clean_fuel_meta_of_clean with (c := example_intrinsic_consumer).
    + reflexivity.
    + exact example_intrinsic_consumer_clean.
  - apply root_list_scan_clean_fuel_cons.
    + apply node_scan_clean_fuel_meta_of_clean with (c := example_call_consumer).
      * reflexivity.
      * exact example_call_consumer_clean.
    + simpl. exact I.
Qed.

Lemma example_record_lit_deps_clean :
  dep_traversal_clean_fuel 2%nat example_consumer_graph [] (CDRecordLit [("a", 3%nat); ("b", 1%nat)]).
Proof.
  unfold dep_traversal_clean_fuel, non_phi_deps.
  apply root_list_scan_clean_fuel_cons.
  - apply node_scan_clean_fuel_meta_of_clean with (c := example_record_consumer).
    + reflexivity.
    + exact example_record_consumer_clean.
  - apply root_list_scan_clean_fuel_cons.
    + apply node_scan_clean_fuel_meta_of_clean with (c := example_call_consumer).
      * reflexivity.
      * exact example_call_consumer_clean.
    + simpl. exact I.
Qed.

Lemma example_index3d_deps_clean :
  dep_traversal_clean_fuel 4%nat example_consumer_graph [] (CDIndex3D 4%nat 5%nat 6%nat 1%nat).
Proof.
  unfold dep_traversal_clean_fuel, non_phi_deps.
  apply root_list_scan_clean_fuel_cons.
  - exact example_shared_call_node_clean.
  - apply root_list_scan_clean_fuel_cons.
    + exact example_call_intrinsic_node_clean.
    + apply root_list_scan_clean_fuel_cons.
      * exact example_wrapped_record_node_clean.
      * apply root_list_scan_clean_fuel_cons.
        -- apply node_scan_clean_fuel_meta_of_clean with (c := example_call_consumer).
           ++ reflexivity.
           ++ exact example_call_consumer_clean.
        -- simpl. exact I.
Qed.

End RRVerifyIrChildDepsSubset.
