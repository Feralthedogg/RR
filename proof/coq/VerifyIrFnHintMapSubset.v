Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrFnRecordSubset.
Require Import VerifyIrFnMetaSubset.
Require Import VerifyIrFnParamMetaSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrFnRecordSubset.
Import RRVerifyIrFnMetaSubset.
Import RRVerifyIrFnParamMetaSubset.

Module RRVerifyIrFnHintMapSubset.

Inductive call_semantics_tag : Type :=
| CSBuiltin
| CSRuntimeHelper
| CSClosureDispatch
| CSUserDefined.

Inductive memory_layout_hint_tag : Type :=
| MLDense1D
| MLColumnMajor2D
| MLColumnMajorND.

Record fn_hint_map_record_lite : Type := {
  fn_hint_map_shell : fn_param_meta_record_lite;
  fn_hint_map_call_semantics : list (consumer_node_id * call_semantics_tag);
  fn_hint_map_memory_layout_hints : list (consumer_node_id * memory_layout_hint_tag);
}.

Definition fn_hint_map_to_fn_param_meta
    (fn_hint_map : fn_hint_map_record_lite) : fn_param_meta_record_lite :=
  fn_hint_map_shell fn_hint_map.

Definition fn_hint_map_to_fn_meta
    (fn_hint_map : fn_hint_map_record_lite) : fn_meta_record_lite :=
  fn_param_meta_to_fn_meta (fn_hint_map_shell fn_hint_map).

Definition fn_hint_map_to_fn_record
    (fn_hint_map : fn_hint_map_record_lite) : fn_record_lite :=
  fn_param_meta_to_fn_record (fn_hint_map_shell fn_hint_map).

Definition fn_hint_map_lookup_value_deps
    (fn_hint_map : fn_hint_map_record_lite) (root : consumer_node_id)
    : option (list consumer_node_id) :=
  fn_param_meta_lookup_value_deps (fn_hint_map_shell fn_hint_map) root.

Definition fn_hint_map_depends_on_phi_in_block_except_fuel
    (fuel : nat) (fn_hint_map : fn_hint_map_record_lite)
    (seen : list consumer_node_id) (root phi_block exempt : nat) : Prop :=
  fn_param_meta_depends_on_phi_in_block_except_fuel fuel
    (fn_hint_map_shell fn_hint_map) seen root phi_block exempt.

Lemma fn_hint_map_lookup_value_deps_eq_fn_param_meta :
  forall fn_hint_map root,
    fn_hint_map_lookup_value_deps fn_hint_map root =
    fn_param_meta_lookup_value_deps (fn_hint_map_to_fn_param_meta fn_hint_map) root.
Proof.
  intros fn_hint_map root. reflexivity.
Qed.

Lemma fn_hint_map_depends_on_phi_in_block_except_eq_fn_param_meta :
  forall fuel fn_hint_map seen root phi_block exempt,
    fn_hint_map_depends_on_phi_in_block_except_fuel fuel fn_hint_map seen root phi_block exempt =
    fn_param_meta_depends_on_phi_in_block_except_fuel fuel
      (fn_hint_map_to_fn_param_meta fn_hint_map) seen root phi_block exempt.
Proof.
  intros fuel fn_hint_map seen root phi_block exempt. reflexivity.
Qed.

Lemma fn_hint_map_lookup_value_deps_eq_fn_record :
  forall fn_hint_map root,
    fn_hint_map_lookup_value_deps fn_hint_map root =
    fn_record_lookup_value_deps (fn_hint_map_to_fn_record fn_hint_map) root.
Proof.
  intros fn_hint_map root.
  exact (fn_param_meta_lookup_value_deps_eq_fn_record (fn_hint_map_shell fn_hint_map) root).
Qed.

Lemma fn_hint_map_depends_on_phi_in_block_except_eq_fn_record :
  forall fuel fn_hint_map seen root phi_block exempt,
    fn_hint_map_depends_on_phi_in_block_except_fuel fuel fn_hint_map seen root phi_block exempt =
    fn_record_depends_on_phi_in_block_except_fuel fuel
      (fn_hint_map_to_fn_record fn_hint_map) seen root phi_block exempt.
Proof.
  intros fuel fn_hint_map seen root phi_block exempt.
  exact (fn_param_meta_depends_on_phi_in_block_except_eq_fn_record
    fuel (fn_hint_map_shell fn_hint_map) seen root phi_block exempt).
Qed.

Definition example_fn_hint_map_record : fn_hint_map_record_lite :=
  {| fn_hint_map_shell := example_fn_param_meta_record;
     fn_hint_map_call_semantics := [(2%nat, CSBuiltin); (6%nat, CSUserDefined)];
     fn_hint_map_memory_layout_hints := [(2%nat, MLDense1D); (6%nat, MLColumnMajorND)] |}.

Lemma example_fn_hint_map_record_to_shell :
  fn_hint_map_to_fn_param_meta example_fn_hint_map_record = example_fn_param_meta_record.
Proof.
  reflexivity.
Qed.

Lemma example_fn_hint_map_record_call_semantics :
  fn_hint_map_call_semantics example_fn_hint_map_record =
    [(2%nat, CSBuiltin); (6%nat, CSUserDefined)].
Proof.
  reflexivity.
Qed.

Lemma example_fn_hint_map_record_memory_layout_hints :
  fn_hint_map_memory_layout_hints example_fn_hint_map_record =
    [(2%nat, MLDense1D); (6%nat, MLColumnMajorND)].
Proof.
  reflexivity.
Qed.

Lemma example_fn_hint_map_lookup_phi :
  fn_hint_map_lookup_value_deps example_fn_hint_map_record 1%nat = Some [3%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_hint_map_lookup_binary :
  fn_hint_map_lookup_value_deps example_fn_hint_map_record 6%nat = Some [6%nat; 1%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_hint_map_depends_direct_phi :
  fn_hint_map_depends_on_phi_in_block_except_fuel 3%nat example_fn_hint_map_record [] 0%nat 7%nat 99%nat.
Proof.
  exact example_fn_param_meta_depends_direct_phi.
Qed.

Lemma example_fn_hint_map_depends_exempt_phi_through_arg :
  fn_hint_map_depends_on_phi_in_block_except_fuel 3%nat example_fn_hint_map_record [] 1%nat 7%nat 1%nat.
Proof.
  exact example_fn_param_meta_depends_exempt_phi_through_arg.
Qed.

Lemma example_fn_hint_map_depends_other_block_ignored :
  ~ fn_hint_map_depends_on_phi_in_block_except_fuel 3%nat example_fn_hint_map_record [] 2%nat 7%nat 99%nat.
Proof.
  exact example_fn_param_meta_depends_other_block_ignored.
Qed.

End RRVerifyIrFnHintMapSubset.
