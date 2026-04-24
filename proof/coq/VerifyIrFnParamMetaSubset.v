Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrFnRecordSubset.
Require Import VerifyIrFnMetaSubset.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrFnRecordSubset.
Import RRVerifyIrFnMetaSubset.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrFnParamMetaSubset.

Record fn_param_meta_record_lite : Type := {
  fn_param_meta_shell : fn_meta_record_lite;
  fn_param_meta_default_r_exprs : list (option string);
  fn_param_meta_spans : list span_tag;
  fn_param_meta_ty_hints : list value_ty_tag;
  fn_param_meta_term_hints : list value_term_tag;
  fn_param_meta_hint_spans : list (option span_tag);
}.

Definition fn_param_meta_to_fn_meta
    (fn_param_meta : fn_param_meta_record_lite) : fn_meta_record_lite :=
  fn_param_meta_shell fn_param_meta.

Definition fn_param_meta_to_fn_record
    (fn_param_meta : fn_param_meta_record_lite) : fn_record_lite :=
  fn_meta_to_fn_record (fn_param_meta_shell fn_param_meta).

Definition fn_param_meta_lookup_value_deps
    (fn_param_meta : fn_param_meta_record_lite) (root : consumer_node_id)
    : option (list consumer_node_id) :=
  fn_meta_lookup_value_deps (fn_param_meta_shell fn_param_meta) root.

Definition fn_param_meta_depends_on_phi_in_block_except_fuel
    (fuel : nat) (fn_param_meta : fn_param_meta_record_lite)
    (seen : list consumer_node_id) (root phi_block exempt : nat) : Prop :=
  fn_meta_depends_on_phi_in_block_except_fuel fuel
    (fn_param_meta_shell fn_param_meta) seen root phi_block exempt.

Lemma fn_param_meta_lookup_value_deps_eq_fn_meta :
  forall fn_param_meta root,
    fn_param_meta_lookup_value_deps fn_param_meta root =
    fn_meta_lookup_value_deps (fn_param_meta_to_fn_meta fn_param_meta) root.
Proof.
  intros fn_param_meta root. reflexivity.
Qed.

Lemma fn_param_meta_depends_on_phi_in_block_except_eq_fn_meta :
  forall fuel fn_param_meta seen root phi_block exempt,
    fn_param_meta_depends_on_phi_in_block_except_fuel fuel fn_param_meta seen root phi_block exempt =
    fn_meta_depends_on_phi_in_block_except_fuel fuel
      (fn_param_meta_to_fn_meta fn_param_meta) seen root phi_block exempt.
Proof.
  intros fuel fn_param_meta seen root phi_block exempt. reflexivity.
Qed.

Lemma fn_param_meta_lookup_value_deps_eq_fn_record :
  forall fn_param_meta root,
    fn_param_meta_lookup_value_deps fn_param_meta root =
    fn_record_lookup_value_deps (fn_param_meta_to_fn_record fn_param_meta) root.
Proof.
  intros fn_param_meta root.
  exact (fn_meta_lookup_value_deps_eq_fn_record (fn_param_meta_shell fn_param_meta) root).
Qed.

Lemma fn_param_meta_depends_on_phi_in_block_except_eq_fn_record :
  forall fuel fn_param_meta seen root phi_block exempt,
    fn_param_meta_depends_on_phi_in_block_except_fuel fuel fn_param_meta seen root phi_block exempt =
    fn_record_depends_on_phi_in_block_except_fuel fuel
      (fn_param_meta_to_fn_record fn_param_meta) seen root phi_block exempt.
Proof.
  intros fuel fn_param_meta seen root phi_block exempt.
  exact (fn_meta_depends_on_phi_in_block_except_eq_fn_record
    fuel (fn_param_meta_shell fn_param_meta) seen root phi_block exempt).
Qed.

Definition example_fn_param_meta_record : fn_param_meta_record_lite :=
  {| fn_param_meta_shell := example_fn_meta_record;
     fn_param_meta_default_r_exprs := [None; Some "1L"];
     fn_param_meta_spans := [SSource; SDummy];
     fn_param_meta_ty_hints := [TYIntLike; TYRecordLike];
     fn_param_meta_term_hints := [TScalar; TRecord];
     fn_param_meta_hint_spans := [Some SSource; None] |}.

Lemma example_fn_param_meta_record_to_shell :
  fn_param_meta_to_fn_meta example_fn_param_meta_record = example_fn_meta_record.
Proof.
  reflexivity.
Qed.

Lemma example_fn_param_meta_record_param_defaults :
  fn_param_meta_default_r_exprs example_fn_param_meta_record = [None; Some "1L"].
Proof.
  reflexivity.
Qed.

Lemma example_fn_param_meta_record_param_hints :
  fn_param_meta_ty_hints example_fn_param_meta_record = [TYIntLike; TYRecordLike] /\
  fn_param_meta_term_hints example_fn_param_meta_record = [TScalar; TRecord].
Proof.
  split; reflexivity.
Qed.

Lemma example_fn_param_meta_record_param_spans :
  fn_param_meta_spans example_fn_param_meta_record = [SSource; SDummy] /\
  fn_param_meta_hint_spans example_fn_param_meta_record = [Some SSource; None].
Proof.
  split; reflexivity.
Qed.

Lemma example_fn_param_meta_lookup_phi :
  fn_param_meta_lookup_value_deps example_fn_param_meta_record 1%nat = Some [3%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_param_meta_lookup_binary :
  fn_param_meta_lookup_value_deps example_fn_param_meta_record 6%nat = Some [6%nat; 1%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_param_meta_depends_direct_phi :
  fn_param_meta_depends_on_phi_in_block_except_fuel 3%nat example_fn_param_meta_record [] 0%nat 7%nat 99%nat.
Proof.
  exact example_fn_meta_depends_direct_phi.
Qed.

Lemma example_fn_param_meta_depends_exempt_phi_through_arg :
  fn_param_meta_depends_on_phi_in_block_except_fuel 3%nat example_fn_param_meta_record [] 1%nat 7%nat 1%nat.
Proof.
  exact example_fn_meta_depends_exempt_phi_through_arg.
Qed.

Lemma example_fn_param_meta_depends_other_block_ignored :
  ~ fn_param_meta_depends_on_phi_in_block_except_fuel 3%nat example_fn_param_meta_record [] 2%nat 7%nat 99%nat.
Proof.
  exact example_fn_meta_depends_other_block_ignored.
Qed.

End RRVerifyIrFnParamMetaSubset.
