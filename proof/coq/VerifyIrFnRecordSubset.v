Require Import VerifyIrConsumerMetaSubset.
Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrChildDepsSubset.
Require Import VerifyIrValueDepsWalkSubset.
Require Import VerifyIrValueTableWalkSubset.
Require Import VerifyIrValueKindTableSubset.
Require Import VerifyIrValueRecordSubset.
Require Import VerifyIrValueFullRecordSubset.
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
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrFnRecordSubset.

Inductive term_tag : Type :=
| TGoto : nat -> term_tag
| TBranch : nat -> nat -> term_tag
| TRet
| TUnreachable.

Record block_lite : Type := {
  block_id_lite : nat;
  block_term_lite : term_tag;
}.

Record fn_record_lite : Type := {
  fn_record_name : string;
  fn_record_params : list string;
  fn_record_values : actual_value_full_table_lite;
  fn_record_blocks : list block_lite;
  fn_record_entry : nat;
  fn_record_body_head : nat;
}.

Definition fn_record_value_table (fn_rec : fn_record_lite) : actual_value_full_table_lite :=
  fn_record_values fn_rec.

Definition fn_record_lookup_value_deps
    (fn_rec : fn_record_lite) (root : consumer_node_id)
    : option (list consumer_node_id) :=
  lookup_actual_value_full_deps (fn_record_values fn_rec) root.

Definition fn_record_depends_on_phi_in_block_except_fuel
    (fuel : nat) (fn_rec : fn_record_lite) (seen : list consumer_node_id)
    (root phi_block exempt : nat) : Prop :=
  depends_on_phi_in_block_except_actual_value_full_table_fuel fuel
    (fn_record_values fn_rec) seen root phi_block exempt.

Lemma fn_record_lookup_value_deps_eq_value_table :
  forall fn_rec root,
    fn_record_lookup_value_deps fn_rec root =
    lookup_actual_value_full_deps (fn_record_value_table fn_rec) root.
Proof.
  intros fn_rec root. reflexivity.
Qed.

Lemma fn_record_depends_on_phi_in_block_except_eq_value_table :
  forall fuel fn_rec seen root phi_block exempt,
    fn_record_depends_on_phi_in_block_except_fuel fuel fn_rec seen root phi_block exempt =
    depends_on_phi_in_block_except_actual_value_full_table_fuel fuel
      (fn_record_value_table fn_rec) seen root phi_block exempt.
Proof.
  intros fuel fn_rec seen root phi_block exempt. reflexivity.
Qed.

Definition example_fn_record : fn_record_lite :=
  {| fn_record_name := "example";
     fn_record_params := ["p0"; "p1"];
     fn_record_values := example_actual_value_full_table;
     fn_record_blocks :=
       [ {| block_id_lite := 0%nat; block_term_lite := TGoto 1%nat |}
       ; {| block_id_lite := 1%nat; block_term_lite := TBranch 2%nat 3%nat |}
       ; {| block_id_lite := 2%nat; block_term_lite := TRet |}
       ; {| block_id_lite := 3%nat; block_term_lite := TUnreachable |}
       ];
     fn_record_entry := 0%nat;
     fn_record_body_head := 1%nat |}.

Lemma example_fn_record_lookup_phi :
  fn_record_lookup_value_deps example_fn_record 1%nat = Some [3%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_record_lookup_binary :
  fn_record_lookup_value_deps example_fn_record 6%nat = Some [6%nat; 1%nat].
Proof.
  reflexivity.
Qed.

Lemma example_fn_record_lookup_oob :
  fn_record_lookup_value_deps example_fn_record 99%nat = None.
Proof.
  reflexivity.
Qed.

Lemma example_fn_record_depends_direct_phi :
  fn_record_depends_on_phi_in_block_except_fuel 3%nat example_fn_record [] 0%nat 7%nat 99%nat.
Proof.
  exact example_actual_value_full_table_depends_direct_phi.
Qed.

Lemma example_fn_record_depends_exempt_phi_through_arg :
  fn_record_depends_on_phi_in_block_except_fuel 3%nat example_fn_record [] 1%nat 7%nat 1%nat.
Proof.
  exact example_actual_value_full_table_depends_exempt_phi_through_arg.
Qed.

Lemma example_fn_record_depends_other_block_ignored :
  ~ fn_record_depends_on_phi_in_block_except_fuel 3%nat example_fn_record [] 2%nat 7%nat 99%nat.
Proof.
  exact example_actual_value_full_table_depends_other_block_ignored.
Qed.

Lemma example_fn_record_depends_self_loop_skips_seen_but_finds_phi :
  fn_record_depends_on_phi_in_block_except_fuel 4%nat example_fn_record [] 6%nat 7%nat 99%nat.
Proof.
  exact example_actual_value_full_table_depends_self_loop_skips_seen_but_finds_phi.
Qed.

End RRVerifyIrFnRecordSubset.
