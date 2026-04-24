Require Import VerifyIrConsumerMetaSubset.
Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrChildDepsSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import PeanoNat.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerMetaSubset.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrChildDepsSubset.

Module RRVerifyIrValueDepsWalkSubset.

Inductive value_deps_kind : Type :=
| VDConstLike
| VDParamLike
| VDLoadLike
| VDRSymbolLike
| VDLen : consumer_node_id -> value_deps_kind
| VDIndices : consumer_node_id -> value_deps_kind
| VDUnary : consumer_node_id -> value_deps_kind
| VDFieldGet : consumer_node_id -> value_deps_kind
| VDRange : consumer_node_id -> consumer_node_id -> value_deps_kind
| VDBinary : consumer_node_id -> consumer_node_id -> value_deps_kind
| VDPhi : list consumer_node_id -> value_deps_kind
| VDCall : list consumer_node_id -> value_deps_kind
| VDIntrinsic : list consumer_node_id -> value_deps_kind
| VDRecordLit : list (string * consumer_node_id) -> value_deps_kind
| VDFieldSet : consumer_node_id -> consumer_node_id -> value_deps_kind
| VDIndex1D : consumer_node_id -> consumer_node_id -> value_deps_kind
| VDIndex2D : consumer_node_id -> consumer_node_id -> consumer_node_id -> value_deps_kind
| VDIndex3D : consumer_node_id -> consumer_node_id -> consumer_node_id -> consumer_node_id ->
    value_deps_kind.

Definition value_deps (kind : value_deps_kind) : list consumer_node_id :=
  match kind with
  | VDConstLike | VDParamLike | VDLoadLike | VDRSymbolLike => []
  | VDLen base | VDIndices base | VDUnary base | VDFieldGet base => [base]
  | VDRange start finish | VDBinary start finish | VDFieldSet start finish
  | VDIndex1D start finish => [start; finish]
  | VDPhi args | VDCall args | VDIntrinsic args => args
  | VDRecordLit fields => map snd fields
  | VDIndex2D base r c => [base; r; c]
  | VDIndex3D base i j k => [base; i; j; k]
  end.

Definition to_child_deps_kind (kind : value_deps_kind) : option child_deps_kind :=
  match kind with
  | VDConstLike => Some CDConstLike
  | VDParamLike => Some CDParamLike
  | VDLoadLike => Some CDLoadLike
  | VDRSymbolLike => Some CDRSymbolLike
  | VDLen base => Some (CDLen base)
  | VDIndices base => Some (CDIndices base)
  | VDUnary base => Some (CDUnary base)
  | VDFieldGet base => Some (CDFieldGet base)
  | VDRange start finish => Some (CDRange start finish)
  | VDBinary lhs rhs => Some (CDBinary lhs rhs)
  | VDPhi _ => None
  | VDCall args => Some (CDCall args)
  | VDIntrinsic args => Some (CDIntrinsic args)
  | VDRecordLit fields => Some (CDRecordLit fields)
  | VDFieldSet base value => Some (CDFieldSet base value)
  | VDIndex1D base idx => Some (CDIndex1D base idx)
  | VDIndex2D base r c => Some (CDIndex2D base r c)
  | VDIndex3D base i j k => Some (CDIndex3D base i j k)
  end.

Lemma value_deps_eq_non_phi_deps_of_to_child :
  forall kind child_kind,
    to_child_deps_kind kind = Some child_kind ->
    value_deps kind = non_phi_deps child_kind.
Proof.
  intros kind child_kind H.
  destruct kind; inversion H; reflexivity.
Qed.

Record phi_walk_node : Type := {
  walk_phi_block : option nat;
  walk_deps_kind : value_deps_kind;
}.

Definition phi_walk_graph := consumer_node_id -> option phi_walk_node.

Fixpoint depends_on_phi_in_block_except_fuel
    (fuel : nat) (graph : phi_walk_graph) (seen : list consumer_node_id)
    (root phi_block exempt : nat) : Prop :=
  match fuel with
  | O => False
  | S fuel' =>
      if in_dec Nat.eq_dec root seen then
        False
      else
        match graph root with
        | None => False
        | Some node =>
            ((root <> exempt) /\ walk_phi_block node = Some phi_block) \/
            dep_list_depends_on_phi_fuel fuel' graph (root :: seen)
              (value_deps (walk_deps_kind node)) phi_block exempt
        end
  end
with dep_list_depends_on_phi_fuel
    (fuel : nat) (graph : phi_walk_graph) (seen deps : list consumer_node_id)
    (phi_block exempt : nat) : Prop :=
  match fuel with
  | O => False
  | S fuel' =>
      match deps with
      | [] => False
      | root :: rest =>
          depends_on_phi_in_block_except_fuel fuel' graph seen root phi_block exempt \/
          dep_list_depends_on_phi_fuel fuel' graph seen rest phi_block exempt
      end
  end.

Lemma depends_on_phi_in_block_except_fuel_here :
  forall fuel graph seen root phi_block exempt deps_kind,
    graph root = Some {| walk_phi_block := Some phi_block; walk_deps_kind := deps_kind |} ->
    ~ In root seen ->
    root <> exempt ->
    depends_on_phi_in_block_except_fuel (S fuel) graph seen root phi_block exempt.
Proof.
  intros fuel graph seen root phi_block exempt deps_kind HNode HFresh HNe.
  simpl.
  destruct (in_dec Nat.eq_dec root seen); simpl.
  - contradiction.
  - rewrite HNode. left. split.
    + exact HNe.
    + reflexivity.
Qed.

Lemma dep_list_depends_on_phi_fuel_head :
  forall fuel graph seen root rest phi_block exempt,
    depends_on_phi_in_block_except_fuel fuel graph seen root phi_block exempt ->
    dep_list_depends_on_phi_fuel (S fuel) graph seen (root :: rest) phi_block exempt.
Proof.
  intros fuel graph seen root rest phi_block exempt HRoot.
  simpl. auto.
Qed.

Lemma dep_list_depends_on_phi_fuel_tail :
  forall fuel graph seen root rest phi_block exempt,
    dep_list_depends_on_phi_fuel fuel graph seen rest phi_block exempt ->
    dep_list_depends_on_phi_fuel (S fuel) graph seen (root :: rest) phi_block exempt.
Proof.
  intros fuel graph seen root rest phi_block exempt HRest.
  simpl. auto.
Qed.

Lemma depends_on_phi_in_block_except_fuel_of_dep_list :
  forall fuel graph seen root phi_block exempt node,
    graph root = Some node ->
    ~ In root seen ->
    dep_list_depends_on_phi_fuel fuel graph (root :: seen)
      (value_deps (walk_deps_kind node)) phi_block exempt ->
    depends_on_phi_in_block_except_fuel (S fuel) graph seen root phi_block exempt.
Proof.
  intros fuel graph seen root phi_block exempt node HNode HFresh HDeps.
  simpl.
  destruct (in_dec Nat.eq_dec root seen); simpl.
  - contradiction.
  - rewrite HNode. right. exact HDeps.
Qed.

Definition example_phi_walk_graph : phi_walk_graph :=
  fun root =>
    match root with
    | 1%nat =>
        Some {| walk_phi_block := None; walk_deps_kind := VDBinary 2%nat 3%nat |}
    | 2%nat =>
        Some {| walk_phi_block := Some 7%nat; walk_deps_kind := VDPhi [4%nat] |}
    | 3%nat =>
        Some {| walk_phi_block := None; walk_deps_kind := VDCall [5%nat; 6%nat] |}
    | 4%nat =>
        Some {| walk_phi_block := Some 7%nat; walk_deps_kind := VDConstLike |}
    | 5%nat =>
        Some {| walk_phi_block := None; walk_deps_kind := VDConstLike |}
    | 6%nat =>
        Some {| walk_phi_block := Some 8%nat; walk_deps_kind := VDConstLike |}
    | 8%nat =>
        Some {| walk_phi_block := None; walk_deps_kind := VDBinary 8%nat 2%nat |}
    | _ => None
    end.

Lemma example_value_deps_phi_shape :
  value_deps (VDPhi [4%nat; 6%nat; 1%nat]) = [4%nat; 6%nat; 1%nat].
Proof.
  reflexivity.
Qed.

Lemma example_value_deps_non_phi_matches_child :
  value_deps (VDIndex3D 4%nat 5%nat 6%nat 1%nat) =
  non_phi_deps (CDIndex3D 4%nat 5%nat 6%nat 1%nat).
Proof.
  exact (value_deps_eq_non_phi_deps_of_to_child
    (VDIndex3D 4%nat 5%nat 6%nat 1%nat)
    (CDIndex3D 4%nat 5%nat 6%nat 1%nat) eq_refl).
Qed.

Lemma example_depends_direct_phi :
  depends_on_phi_in_block_except_fuel 3%nat example_phi_walk_graph [] 1%nat 7%nat 99%nat.
Proof.
  apply depends_on_phi_in_block_except_fuel_of_dep_list with
    (node := {| walk_phi_block := None; walk_deps_kind := VDBinary 2%nat 3%nat |}).
  - reflexivity.
  - simpl. tauto.
  - apply dep_list_depends_on_phi_fuel_head.
    apply depends_on_phi_in_block_except_fuel_here with (deps_kind := VDPhi [4%nat]).
    + reflexivity.
    + simpl. intros [H | []]. discriminate H.
    + discriminate.
Qed.

Lemma example_depends_exempt_phi_through_arg :
  depends_on_phi_in_block_except_fuel 3%nat example_phi_walk_graph [] 2%nat 7%nat 2%nat.
Proof.
  apply depends_on_phi_in_block_except_fuel_of_dep_list with
    (node := {| walk_phi_block := Some 7%nat; walk_deps_kind := VDPhi [4%nat] |}).
  - reflexivity.
  - simpl. tauto.
  - apply dep_list_depends_on_phi_fuel_head.
    apply depends_on_phi_in_block_except_fuel_here with (deps_kind := VDConstLike).
    + reflexivity.
    + simpl. intros [H | []]. discriminate H.
    + discriminate.
Qed.

Lemma example_depends_other_block_ignored :
  ~ depends_on_phi_in_block_except_fuel 3%nat example_phi_walk_graph [] 3%nat 7%nat 99%nat.
Proof.
  simpl. intro H.
  repeat match type of H with
  | _ /\ _ => destruct H
  | _ \/ _ => destruct H
  end;
  congruence.
Qed.

Lemma example_depends_self_loop_skips_seen_but_finds_phi :
  depends_on_phi_in_block_except_fuel 4%nat example_phi_walk_graph [] 8%nat 7%nat 99%nat.
Proof.
  apply depends_on_phi_in_block_except_fuel_of_dep_list with
    (node := {| walk_phi_block := None; walk_deps_kind := VDBinary 8%nat 2%nat |}).
  - reflexivity.
  - simpl. tauto.
  - apply dep_list_depends_on_phi_fuel_tail.
    apply dep_list_depends_on_phi_fuel_head.
    apply depends_on_phi_in_block_except_fuel_here with (deps_kind := VDPhi [4%nat]).
    + reflexivity.
    + simpl. intros [H | []]. discriminate H.
    + discriminate.
Qed.

End RRVerifyIrValueDepsWalkSubset.
