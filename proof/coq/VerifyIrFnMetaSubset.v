Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrFnRecordSubset.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrFnRecordSubset.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrFnMetaSubset.

Record fn_meta_record_lite : Type := {
  fn_meta_shell : fn_record_lite;
  fn_meta_user_name : option string;
  fn_meta_span : span_tag;
  fn_meta_ret_ty_hint : option value_ty_tag;
  fn_meta_ret_term_hint : option value_term_tag;
  fn_meta_ret_hint_span : option span_tag;
  fn_meta_inferred_ret_ty : value_ty_tag;
  fn_meta_inferred_ret_term : value_term_tag;
  fn_meta_unsupported_dynamic : bool;
  fn_meta_fallback_reasons : list string;
  fn_meta_hybrid_interop_reasons : list string;
  fn_meta_opaque_interop : bool;
  fn_meta_opaque_reasons : list string;
  fn_meta_opaque_interop_reasons : list string;
}.

Definition fn_meta_to_fn_record (fn_meta : fn_meta_record_lite) : fn_record_lite :=
  fn_meta_shell fn_meta.

Definition fn_meta_lookup_value_deps
    (fn_meta : fn_meta_record_lite) (root : consumer_node_id)
    : option (list consumer_node_id) :=
  fn_record_lookup_value_deps (fn_meta_shell fn_meta) root.

Definition fn_meta_depends_on_phi_in_block_except_fuel
    (fuel : nat) (fn_meta : fn_meta_record_lite) (seen : list consumer_node_id)
    (root phi_block exempt : nat) : Prop :=
  fn_record_depends_on_phi_in_block_except_fuel fuel
    (fn_meta_shell fn_meta) seen root phi_block exempt.

Lemma fn_meta_lookup_value_deps_eq_fn_record :
  forall fn_meta root,
    fn_meta_lookup_value_deps fn_meta root =
    fn_record_lookup_value_deps (fn_meta_to_fn_record fn_meta) root.
Proof.
  intros fn_meta root. reflexivity.
Qed.

Lemma fn_meta_depends_on_phi_in_block_except_eq_fn_record :
  forall fuel fn_meta seen root phi_block exempt,
    fn_meta_depends_on_phi_in_block_except_fuel fuel fn_meta seen root phi_block exempt =
    fn_record_depends_on_phi_in_block_except_fuel fuel
      (fn_meta_to_fn_record fn_meta) seen root phi_block exempt.
Proof.
  intros fuel fn_meta seen root phi_block exempt. reflexivity.
Qed.

Lemma fn_meta_lookup_value_deps_eq_value_table :
  forall fn_meta root,
    fn_meta_lookup_value_deps fn_meta root =
    lookup_actual_value_full_deps (fn_record_values (fn_meta_shell fn_meta)) root.
Proof.
  intros fn_meta root.
  exact (fn_record_lookup_value_deps_eq_value_table (fn_meta_shell fn_meta) root).
Qed.

Lemma fn_meta_depends_on_phi_in_block_except_eq_value_table :
  forall fuel fn_meta seen root phi_block exempt,
    fn_meta_depends_on_phi_in_block_except_fuel fuel fn_meta seen root phi_block exempt =
    depends_on_phi_in_block_except_actual_value_full_table_fuel fuel
      (fn_record_values (fn_meta_shell fn_meta)) seen root phi_block exempt.
Proof.
  intros fuel fn_meta seen root phi_block exempt.
  exact (fn_record_depends_on_phi_in_block_except_eq_value_table
    fuel (fn_meta_shell fn_meta) seen root phi_block exempt).
Qed.

Definition example_fn_meta_record : fn_meta_record_lite :=
  {| fn_meta_shell := example_fn_record;
     fn_meta_user_name := Some "exampleUser";
     fn_meta_span := SSource;
     fn_meta_ret_ty_hint := Some TYIntLike;
     fn_meta_ret_term_hint := Some TScalar;
     fn_meta_ret_hint_span := Some SSource;
     fn_meta_inferred_ret_ty := TYIntLike;
     fn_meta_inferred_ret_term := TScalar;
     fn_meta_unsupported_dynamic := true;
     fn_meta_fallback_reasons := ["dynamic builtin"];
     fn_meta_hybrid_interop_reasons := ["package::foo"];
     fn_meta_opaque_interop := true;
     fn_meta_opaque_reasons := ["opaque runtime"];
     fn_meta_opaque_interop_reasons := ["ffi::bar"] |}.

Lemma example_fn_meta_record_to_shell :
  fn_meta_to_fn_record example_fn_meta_record = example_fn_record.
Proof.
  reflexivity.
Qed.

Lemma example_fn_meta_record_user_name :
  fn_meta_user_name example_fn_meta_record = Some "exampleUser".
Proof.
  reflexivity.
Qed.

Lemma example_fn_meta_record_ret_hints :
  fn_meta_ret_ty_hint example_fn_meta_record = Some TYIntLike /\
  fn_meta_ret_term_hint example_fn_meta_record = Some TScalar.
Proof.
  split; reflexivity.
Qed.

Lemma example_fn_meta_record_interop_flags :
  fn_meta_unsupported_dynamic example_fn_meta_record = true /\
  fn_meta_opaque_interop example_fn_meta_record = true.
Proof.
  split; reflexivity.
Qed.

Lemma example_fn_meta_lookup_phi :
  fn_meta_lookup_value_deps example_fn_meta_record 1%nat = Some [3%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_meta_lookup_binary :
  fn_meta_lookup_value_deps example_fn_meta_record 6%nat = Some [6%nat; 1%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_meta_depends_direct_phi :
  fn_meta_depends_on_phi_in_block_except_fuel 3%nat example_fn_meta_record [] 0%nat 7%nat 99%nat.
Proof.
  exact example_fn_record_depends_direct_phi.
Qed.

Lemma example_fn_meta_depends_exempt_phi_through_arg :
  fn_meta_depends_on_phi_in_block_except_fuel 3%nat example_fn_meta_record [] 1%nat 7%nat 1%nat.
Proof.
  exact example_fn_record_depends_exempt_phi_through_arg.
Qed.

Lemma example_fn_meta_depends_other_block_ignored :
  ~ fn_meta_depends_on_phi_in_block_except_fuel 3%nat example_fn_meta_record [] 2%nat 7%nat 99%nat.
Proof.
  exact example_fn_record_depends_other_block_ignored.
Qed.

End RRVerifyIrFnMetaSubset.
