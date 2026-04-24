Require Import VerifyIrFlowLite.
From Stdlib Require Import List.
From Stdlib Require Import Bool.
From Stdlib Require Import Arith.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRVerifyIrClosureLite.

Inductive wrapper_kind : Type :=
| WKLeaf
| WKPhi
| WKIntrinsic
| WKRecord
| WKFieldGet
| WKFieldSet.

Record wrapper_node : Type := {
  wk_kind : wrapper_kind;
  wk_deps : list nat;
}.

Definition wrapper_graph : Type := list wrapper_node.

Inductive used_from (g : wrapper_graph) : nat -> nat -> Prop :=
| UFself : forall root, used_from g root root
| UFstep : forall root from to node,
    used_from g root from ->
    nth_error g from = Some node ->
    In to (wk_deps node) ->
    used_from g root to.

Definition node_is_phi (g : wrapper_graph) (idx : nat) : Prop :=
  exists node, nth_error g idx = Some node /\ wk_kind node = WKPhi.

Definition intrinsic_phi_graph : wrapper_graph :=
  [ {| wk_kind := WKIntrinsic; wk_deps := [1%nat] |}
  ; {| wk_kind := WKPhi; wk_deps := [] |}
  ].

Definition record_phi_graph : wrapper_graph :=
  [ {| wk_kind := WKRecord; wk_deps := [1%nat] |}
  ; {| wk_kind := WKPhi; wk_deps := [] |}
  ].

Definition record_intrinsic_phi_graph : wrapper_graph :=
  [ {| wk_kind := WKRecord; wk_deps := [1%nat] |}
  ; {| wk_kind := WKIntrinsic; wk_deps := [2%nat] |}
  ; {| wk_kind := WKPhi; wk_deps := [] |}
  ].

Definition field_get_record_phi_graph : wrapper_graph :=
  [ {| wk_kind := WKFieldGet; wk_deps := [1%nat] |}
  ; {| wk_kind := WKRecord; wk_deps := [2%nat] |}
  ; {| wk_kind := WKPhi; wk_deps := [] |}
  ].

Definition nested_field_get_record_phi_graph : wrapper_graph :=
  [ {| wk_kind := WKFieldGet; wk_deps := [1%nat] |}
  ; {| wk_kind := WKFieldGet; wk_deps := [2%nat] |}
  ; {| wk_kind := WKRecord; wk_deps := [3%nat] |}
  ; {| wk_kind := WKPhi; wk_deps := [] |}
  ].

Lemma intrinsic_phi_graph_reaches_nested_phi :
  used_from intrinsic_phi_graph 0%nat 1%nat.
Proof.
  eapply UFstep with (from := 0%nat) (node := {| wk_kind := WKIntrinsic; wk_deps := [1%nat] |}).
  - apply UFself.
  - reflexivity.
  - simpl. auto.
Qed.

Lemma record_phi_graph_reaches_nested_phi :
  used_from record_phi_graph 0%nat 1%nat.
Proof.
  eapply UFstep with (from := 0%nat) (node := {| wk_kind := WKRecord; wk_deps := [1%nat] |}).
  - apply UFself.
  - reflexivity.
  - simpl. auto.
Qed.

Lemma intrinsic_phi_graph_nested_target_is_phi :
  node_is_phi intrinsic_phi_graph 1%nat.
Proof.
  exists {| wk_kind := WKPhi; wk_deps := [] |}.
  split; reflexivity.
Qed.

Lemma record_phi_graph_nested_target_is_phi :
  node_is_phi record_phi_graph 1%nat.
Proof.
  exists {| wk_kind := WKPhi; wk_deps := [] |}.
  split; reflexivity.
Qed.

Lemma record_intrinsic_phi_graph_reaches_nested_phi :
  used_from record_intrinsic_phi_graph 0%nat 2%nat.
Proof.
  eapply UFstep with (from := 1%nat) (node := {| wk_kind := WKIntrinsic; wk_deps := [2%nat] |}).
  - eapply UFstep with (from := 0%nat) (node := {| wk_kind := WKRecord; wk_deps := [1%nat] |}).
    + apply UFself.
    + reflexivity.
    + simpl. auto.
  - reflexivity.
  - simpl. auto.
Qed.

Lemma record_intrinsic_phi_graph_nested_target_is_phi :
  node_is_phi record_intrinsic_phi_graph 2%nat.
Proof.
  exists {| wk_kind := WKPhi; wk_deps := [] |}.
  split; reflexivity.
Qed.

Lemma field_get_record_phi_graph_reaches_nested_phi :
  used_from field_get_record_phi_graph 0%nat 2%nat.
Proof.
  eapply UFstep with (from := 1%nat) (node := {| wk_kind := WKRecord; wk_deps := [2%nat] |}).
  - eapply UFstep with (from := 0%nat) (node := {| wk_kind := WKFieldGet; wk_deps := [1%nat] |}).
    + apply UFself.
    + reflexivity.
    + simpl. auto.
  - reflexivity.
  - simpl. auto.
Qed.

Lemma field_get_record_phi_graph_nested_target_is_phi :
  node_is_phi field_get_record_phi_graph 2%nat.
Proof.
  exists {| wk_kind := WKPhi; wk_deps := [] |}.
  split; reflexivity.
Qed.

Lemma nested_field_get_record_phi_graph_reaches_nested_phi :
  used_from nested_field_get_record_phi_graph 0%nat 3%nat.
Proof.
  eapply UFstep with (from := 2%nat) (node := {| wk_kind := WKRecord; wk_deps := [3%nat] |}).
  - eapply UFstep with (from := 1%nat) (node := {| wk_kind := WKFieldGet; wk_deps := [2%nat] |}).
    + eapply UFstep with (from := 0%nat) (node := {| wk_kind := WKFieldGet; wk_deps := [1%nat] |}).
      * apply UFself.
      * reflexivity.
      * simpl. auto.
    + reflexivity.
    + simpl. auto.
  - reflexivity.
  - simpl. auto.
Qed.

Lemma nested_field_get_record_phi_graph_nested_target_is_phi :
  node_is_phi nested_field_get_record_phi_graph 3%nat.
Proof.
  exists {| wk_kind := WKPhi; wk_deps := [] |}.
  split; reflexivity.
Qed.

End RRVerifyIrClosureLite.
