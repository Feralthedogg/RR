Require Import VerifyIrConsumerMetaSubset.
Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrChildDepsSubset.
Require Import VerifyIrValueDepsWalkSubset.
Require Import VerifyIrValueTableWalkSubset.
Require Import VerifyIrValueKindTableSubset.
Require Import VerifyIrValueRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerMetaSubset.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrChildDepsSubset.
Import RRVerifyIrValueDepsWalkSubset.
Import RRVerifyIrValueTableWalkSubset.
Import RRVerifyIrValueKindTableSubset.
Import RRVerifyIrValueRecordSubset.

Module RRVerifyIrValueFullRecordSubset.

Inductive span_tag : Type :=
| SDummy
| SSource.

Inductive facts_tag : Type :=
| FUnknown
| FNonneg
| FBounded.

Inductive value_ty_tag : Type :=
| TYUnknown
| TYIntLike
| TYRecordLike.

Inductive value_term_tag : Type :=
| TAny
| TScalar
| TRecord.

Record actual_value_full_record_lite : Type := {
  actual_full_id : consumer_node_id;
  actual_full_kind : value_table_kind;
  actual_full_span : span_tag;
  actual_full_facts : facts_tag;
  actual_full_value_ty : value_ty_tag;
  actual_full_value_term : value_term_tag;
  actual_full_origin_var : option string;
  actual_full_phi_block : option nat;
  actual_full_escape : escape_tag;
}.

Definition actual_value_full_table_lite := list actual_value_full_record_lite.

Definition actual_value_full_record_to_record (row : actual_value_full_record_lite)
    : actual_value_record_lite :=
  {| actual_value_id := actual_full_id row;
     actual_value_kind := actual_full_kind row;
     actual_origin_var := actual_full_origin_var row;
     actual_phi_block := actual_full_phi_block row;
     actual_escape := actual_full_escape row |}.

Definition actual_value_full_table_to_record_table (table : actual_value_full_table_lite)
    : actual_value_table_lite :=
  map actual_value_full_record_to_record table.

Fixpoint lookup_actual_value_full_row (table : actual_value_full_table_lite)
    (root : consumer_node_id) : option actual_value_full_record_lite :=
  match table, root with
  | [], _ => None
  | row :: _, O => Some row
  | _ :: rest, S n => lookup_actual_value_full_row rest n
  end.

Definition lookup_actual_value_full_deps
    (table : actual_value_full_table_lite) (root : consumer_node_id)
    : option (list consumer_node_id) :=
  match lookup_actual_value_full_row table root with
  | Some row => Some (value_table_kind_deps (actual_full_kind row))
  | None => None
  end.

Lemma lookup_actual_value_full_deps_eq_lookup_actual_value_deps :
  forall table root,
    lookup_actual_value_full_deps table root =
    lookup_actual_value_deps (actual_value_full_table_to_record_table table) root.
Proof.
  intros table.
  induction table as [|row rest IH]; intros root.
  - destruct root; reflexivity.
  - destruct root as [|n].
    + reflexivity.
    + simpl. exact (IH n).
Qed.

Definition depends_on_phi_in_block_except_actual_value_full_table_fuel
    (fuel : nat) (table : actual_value_full_table_lite) (seen : list consumer_node_id)
    (root phi_block exempt : nat) : Prop :=
  depends_on_phi_in_block_except_actual_value_table_fuel fuel
    (actual_value_full_table_to_record_table table) seen root phi_block exempt.

Definition example_actual_value_full_table : actual_value_full_table_lite :=
  [ {| actual_full_id := 0%nat; actual_full_kind := VTKBinary 1%nat 2%nat;
       actual_full_span := SSource; actual_full_facts := FBounded;
       actual_full_value_ty := TYIntLike; actual_full_value_term := TScalar;
       actual_full_origin_var := Some "tmp0"; actual_full_phi_block := None;
       actual_full_escape := EUnknown |}
  ; {| actual_full_id := 1%nat; actual_full_kind := VTKPhi [3%nat];
       actual_full_span := SSource; actual_full_facts := FUnknown;
       actual_full_value_ty := TYIntLike; actual_full_value_term := TScalar;
       actual_full_origin_var := Some "phi1"; actual_full_phi_block := Some 7%nat;
       actual_full_escape := EUnknown |}
  ; {| actual_full_id := 2%nat; actual_full_kind := VTKCall [4%nat; 5%nat];
       actual_full_span := SSource; actual_full_facts := FUnknown;
       actual_full_value_ty := TYUnknown; actual_full_value_term := TAny;
       actual_full_origin_var := None; actual_full_phi_block := None;
       actual_full_escape := EEscaped |}
  ; {| actual_full_id := 3%nat; actual_full_kind := VTKConstLike;
       actual_full_span := SDummy; actual_full_facts := FNonneg;
       actual_full_value_ty := TYIntLike; actual_full_value_term := TScalar;
       actual_full_origin_var := Some "x"; actual_full_phi_block := Some 7%nat;
       actual_full_escape := ELocal |}
  ; {| actual_full_id := 4%nat; actual_full_kind := VTKConstLike;
       actual_full_span := SDummy; actual_full_facts := FNonneg;
       actual_full_value_ty := TYIntLike; actual_full_value_term := TScalar;
       actual_full_origin_var := Some "y"; actual_full_phi_block := None;
       actual_full_escape := ELocal |}
  ; {| actual_full_id := 5%nat; actual_full_kind := VTKConstLike;
       actual_full_span := SDummy; actual_full_facts := FUnknown;
       actual_full_value_ty := TYIntLike; actual_full_value_term := TScalar;
       actual_full_origin_var := None; actual_full_phi_block := Some 8%nat;
       actual_full_escape := EUnknown |}
  ; {| actual_full_id := 6%nat; actual_full_kind := VTKBinary 6%nat 1%nat;
       actual_full_span := SSource; actual_full_facts := FUnknown;
       actual_full_value_ty := TYIntLike; actual_full_value_term := TScalar;
       actual_full_origin_var := Some "loop"; actual_full_phi_block := None;
       actual_full_escape := EEscaped |}
  ].

Lemma example_lookup_actual_value_full_deps_phi :
  lookup_actual_value_full_deps example_actual_value_full_table 1%nat = Some [3%nat].
Proof.
  reflexivity.
Qed.

Lemma example_lookup_actual_value_full_deps_binary :
  lookup_actual_value_full_deps example_actual_value_full_table 6%nat = Some [6%nat; 1%nat].
Proof.
  reflexivity.
Qed.

Lemma example_lookup_actual_value_full_deps_oob :
  lookup_actual_value_full_deps example_actual_value_full_table 99%nat = None.
Proof.
  reflexivity.
Qed.

Lemma example_lookup_actual_value_full_deps_matches_record_lookup :
  lookup_actual_value_full_deps example_actual_value_full_table 2%nat =
  lookup_actual_value_deps (actual_value_full_table_to_record_table example_actual_value_full_table) 2%nat.
Proof.
  exact (lookup_actual_value_full_deps_eq_lookup_actual_value_deps
    example_actual_value_full_table 2%nat).
Qed.

Lemma example_actual_value_full_table_depends_direct_phi :
  depends_on_phi_in_block_except_actual_value_full_table_fuel 3%nat example_actual_value_full_table [] 0%nat 7%nat 99%nat.
Proof.
  exact example_actual_value_table_depends_direct_phi.
Qed.

Lemma example_actual_value_full_table_depends_exempt_phi_through_arg :
  depends_on_phi_in_block_except_actual_value_full_table_fuel 3%nat example_actual_value_full_table [] 1%nat 7%nat 1%nat.
Proof.
  exact example_actual_value_table_depends_exempt_phi_through_arg.
Qed.

Lemma example_actual_value_full_table_depends_other_block_ignored :
  ~ depends_on_phi_in_block_except_actual_value_full_table_fuel 3%nat example_actual_value_full_table [] 2%nat 7%nat 99%nat.
Proof.
  exact example_actual_value_table_depends_other_block_ignored.
Qed.

Lemma example_actual_value_full_table_depends_self_loop_skips_seen_but_finds_phi :
  depends_on_phi_in_block_except_actual_value_full_table_fuel 4%nat example_actual_value_full_table [] 6%nat 7%nat 99%nat.
Proof.
  exact example_actual_value_table_depends_self_loop_skips_seen_but_finds_phi.
Qed.

End RRVerifyIrValueFullRecordSubset.
