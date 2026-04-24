Require Import VerifyIrConsumerMetaSubset.
Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrChildDepsSubset.
Require Import VerifyIrValueDepsWalkSubset.
Require Import VerifyIrValueTableWalkSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerMetaSubset.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrChildDepsSubset.
Import RRVerifyIrValueDepsWalkSubset.
Import RRVerifyIrValueTableWalkSubset.

Module RRVerifyIrValueKindTableSubset.

Inductive value_table_kind : Type :=
| VTKConstLike
| VTKParamLike
| VTKLoadLike
| VTKRSymbolLike
| VTKLen : consumer_node_id -> value_table_kind
| VTKIndices : consumer_node_id -> value_table_kind
| VTKUnary : consumer_node_id -> value_table_kind
| VTKFieldGet : consumer_node_id -> value_table_kind
| VTKRange : consumer_node_id -> consumer_node_id -> value_table_kind
| VTKBinary : consumer_node_id -> consumer_node_id -> value_table_kind
| VTKPhi : list consumer_node_id -> value_table_kind
| VTKCall : list consumer_node_id -> value_table_kind
| VTKIntrinsic : list consumer_node_id -> value_table_kind
| VTKRecordLit : list (string * consumer_node_id) -> value_table_kind
| VTKFieldSet : consumer_node_id -> consumer_node_id -> value_table_kind
| VTKIndex1D : consumer_node_id -> consumer_node_id -> value_table_kind
| VTKIndex2D : consumer_node_id -> consumer_node_id -> consumer_node_id -> value_table_kind
| VTKIndex3D : consumer_node_id -> consumer_node_id -> consumer_node_id -> consumer_node_id ->
    value_table_kind.

Definition value_table_kind_to_deps_kind (kind : value_table_kind) : value_deps_kind :=
  match kind with
  | VTKConstLike => VDConstLike
  | VTKParamLike => VDParamLike
  | VTKLoadLike => VDLoadLike
  | VTKRSymbolLike => VDRSymbolLike
  | VTKLen base => VDLen base
  | VTKIndices base => VDIndices base
  | VTKUnary base => VDUnary base
  | VTKFieldGet base => VDFieldGet base
  | VTKRange start finish => VDRange start finish
  | VTKBinary lhs rhs => VDBinary lhs rhs
  | VTKPhi args => VDPhi args
  | VTKCall args => VDCall args
  | VTKIntrinsic args => VDIntrinsic args
  | VTKRecordLit fields => VDRecordLit fields
  | VTKFieldSet base value => VDFieldSet base value
  | VTKIndex1D base idx => VDIndex1D base idx
  | VTKIndex2D base r c => VDIndex2D base r c
  | VTKIndex3D base i j k => VDIndex3D base i j k
  end.

Definition value_table_kind_deps (kind : value_table_kind) : list consumer_node_id :=
  value_deps (value_table_kind_to_deps_kind kind).

Lemma value_table_kind_deps_eq_value_deps :
  forall kind,
    value_table_kind_deps kind = value_deps (value_table_kind_to_deps_kind kind).
Proof.
  intros kind. reflexivity.
Qed.

Record fn_ir_value_row_lite : Type := {
  fn_ir_row_phi_block : option nat;
  fn_ir_row_kind : value_table_kind;
}.

Definition fn_ir_value_table_lite := list fn_ir_value_row_lite.

Definition fn_ir_row_to_table_value (row : fn_ir_value_row_lite) : table_value :=
  {| table_phi_block := fn_ir_row_phi_block row;
     table_deps_kind := value_table_kind_to_deps_kind (fn_ir_row_kind row) |}.

Definition fn_ir_value_table_to_value_table (table : fn_ir_value_table_lite) : value_table :=
  map fn_ir_row_to_table_value table.

Fixpoint lookup_fn_ir_value_row (table : fn_ir_value_table_lite) (root : consumer_node_id)
    : option fn_ir_value_row_lite :=
  match table, root with
  | [], _ => None
  | row :: _, O => Some row
  | _ :: rest, S n => lookup_fn_ir_value_row rest n
  end.

Definition lookup_fn_ir_value_deps
    (table : fn_ir_value_table_lite) (root : consumer_node_id)
    : option (list consumer_node_id) :=
  match lookup_fn_ir_value_row table root with
  | Some row => Some (value_table_kind_deps (fn_ir_row_kind row))
  | None => None
  end.

Lemma lookup_fn_ir_value_deps_eq_lookup_value_deps :
  forall table root,
    lookup_fn_ir_value_deps table root =
    lookup_value_deps (fn_ir_value_table_to_value_table table) root.
Proof.
  intros table.
  induction table as [|row rest IH]; intros root.
  - destruct root; reflexivity.
  - destruct root as [|n].
    + reflexivity.
    + simpl. exact (IH n).
Qed.

Definition depends_on_phi_in_block_except_fn_ir_table_fuel
    (fuel : nat) (table : fn_ir_value_table_lite) (seen : list consumer_node_id)
    (root phi_block exempt : nat) : Prop :=
  depends_on_phi_in_block_except_table_fuel fuel
    (fn_ir_value_table_to_value_table table) seen root phi_block exempt.

Definition example_fn_ir_value_table : fn_ir_value_table_lite :=
  [ {| fn_ir_row_phi_block := None; fn_ir_row_kind := VTKBinary 1%nat 2%nat |}
  ; {| fn_ir_row_phi_block := Some 7%nat; fn_ir_row_kind := VTKPhi [3%nat] |}
  ; {| fn_ir_row_phi_block := None; fn_ir_row_kind := VTKCall [4%nat; 5%nat] |}
  ; {| fn_ir_row_phi_block := Some 7%nat; fn_ir_row_kind := VTKConstLike |}
  ; {| fn_ir_row_phi_block := None; fn_ir_row_kind := VTKConstLike |}
  ; {| fn_ir_row_phi_block := Some 8%nat; fn_ir_row_kind := VTKConstLike |}
  ; {| fn_ir_row_phi_block := None; fn_ir_row_kind := VTKBinary 6%nat 1%nat |}
  ].

Lemma example_lookup_fn_ir_value_deps_phi :
  lookup_fn_ir_value_deps example_fn_ir_value_table 1%nat = Some [3%nat].
Proof.
  reflexivity.
Qed.

Lemma example_lookup_fn_ir_value_deps_binary :
  lookup_fn_ir_value_deps example_fn_ir_value_table 6%nat = Some [6%nat; 1%nat].
Proof.
  reflexivity.
Qed.

Lemma example_lookup_fn_ir_value_deps_oob :
  lookup_fn_ir_value_deps example_fn_ir_value_table 99%nat = None.
Proof.
  reflexivity.
Qed.

Lemma example_lookup_fn_ir_value_deps_matches_table_lookup :
  lookup_fn_ir_value_deps example_fn_ir_value_table 2%nat =
  lookup_value_deps (fn_ir_value_table_to_value_table example_fn_ir_value_table) 2%nat.
Proof.
  exact (lookup_fn_ir_value_deps_eq_lookup_value_deps example_fn_ir_value_table 2%nat).
Qed.

Lemma example_fn_ir_table_depends_direct_phi :
  depends_on_phi_in_block_except_fn_ir_table_fuel 3%nat example_fn_ir_value_table [] 0%nat 7%nat 99%nat.
Proof.
  exact example_table_depends_direct_phi.
Qed.

Lemma example_fn_ir_table_depends_exempt_phi_through_arg :
  depends_on_phi_in_block_except_fn_ir_table_fuel 3%nat example_fn_ir_value_table [] 1%nat 7%nat 1%nat.
Proof.
  exact example_table_depends_exempt_phi_through_arg.
Qed.

Lemma example_fn_ir_table_depends_other_block_ignored :
  ~ depends_on_phi_in_block_except_fn_ir_table_fuel 3%nat example_fn_ir_value_table [] 2%nat 7%nat 99%nat.
Proof.
  exact example_table_depends_other_block_ignored.
Qed.

Lemma example_fn_ir_table_depends_self_loop_skips_seen_but_finds_phi :
  depends_on_phi_in_block_except_fn_ir_table_fuel 4%nat example_fn_ir_value_table [] 6%nat 7%nat 99%nat.
Proof.
  exact example_table_depends_self_loop_skips_seen_but_finds_phi.
Qed.

End RRVerifyIrValueKindTableSubset.
