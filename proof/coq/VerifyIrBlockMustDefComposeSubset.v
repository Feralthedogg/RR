Require Import VerifyIrBlockFlowSubset.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrBlockFlowSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrBlockMustDefComposeSubset.

Definition raw_required_vars_of_block
    (table : actual_value_full_table_lite) (bb : actual_block_record_lite) : list string :=
  block_required_vars table [] bb.

Definition raw_flow_case_of_actual_block
    (table : actual_value_full_table_lite) (defs : def_set) (bb : actual_block_record_lite)
    : flow_block_case :=
  {| flow_defined := defs;
     flow_required := raw_required_vars_of_block table bb |}.

Lemma first_missing_var_none_of_required_subset :
  forall defs required,
    (forall v, In v required -> In v defs) ->
    first_missing_var defs required = None.
Proof.
  intros defs required Hall.
  induction required as [|v required IH].
  - reflexivity.
  - simpl.
    destruct (in_dec String.string_dec v defs) as [Hv|Hv].
    + apply IH. intros u Hu. apply Hall. simpl. auto.
    + exfalso. apply Hv. apply Hall. simpl. auto.
Qed.

Lemma verify_flow_block_none_of_required_subset :
  forall defs required,
    (forall v, In v required -> In v defs) ->
    verify_flow_block {| flow_defined := defs; flow_required := required |} = None.
Proof.
  intros defs required Hall.
  unfold verify_flow_block.
  rewrite first_missing_var_none_of_required_subset with (required := required); auto.
Qed.

Lemma raw_block_flow_none_of_required_subset :
  forall table defs bb,
    (forall v, In v (raw_required_vars_of_block table bb) -> In v defs) ->
    verify_flow_block (raw_flow_case_of_actual_block table defs bb) = None.
Proof.
  intros table defs bb Hall.
  apply verify_flow_block_none_of_required_subset.
  exact Hall.
Qed.

Definition example_two_read_out_defs (bid : nat) : def_set :=
  match bid with
  | 1%nat => ["x"; "y"]
  | 2%nat => ["x"; "y"; "z"]
  | _ => []
  end.

Lemma example_two_read_join_contains_x :
  In "x" (in_defs_from_preds 0%nat [] example_preds example_two_read_out_defs 3%nat).
Proof.
  apply in_in_defs_from_preds_of_forall_pred.
  - discriminate.
  - discriminate.
  - intros pred Hpred.
    simpl in Hpred.
    destruct Hpred as [Hpred|[Hpred|[]]]; subst pred; simpl; auto.
Qed.

Lemma example_two_read_join_contains_y :
  In "y" (in_defs_from_preds 0%nat [] example_preds example_two_read_out_defs 3%nat).
Proof.
  apply in_in_defs_from_preds_of_forall_pred.
  - discriminate.
  - discriminate.
  - intros pred Hpred.
    simpl in Hpred.
    destruct Hpred as [Hpred|[Hpred|[]]]; subst pred; simpl; auto.
Qed.

Definition example_multi_read_block : actual_block_record_lite :=
  {| actual_block_id := 30%nat;
     actual_block_instrs := [IREval 3%nat SSource; IREval 4%nat SSource];
     actual_block_term := TRLRet (Some 3%nat) |}.

Lemma example_multi_read_block_raw_required :
  raw_required_vars_of_block example_actual_value_full_table example_multi_read_block = ["x"; "y"; "x"].
Proof.
  reflexivity.
Qed.

Lemma example_multi_read_block_clean_from_join :
  verify_flow_block
    (raw_flow_case_of_actual_block example_actual_value_full_table
      (in_defs_from_preds 0%nat [] example_preds example_two_read_out_defs 3%nat)
      example_multi_read_block) = None.
Proof.
  apply raw_block_flow_none_of_required_subset.
  intros v Hv.
  rewrite example_multi_read_block_raw_required in Hv.
  simpl in Hv.
  destruct Hv as [Hv|[Hv|[Hv|[]]]]; subst v.
  - exact example_two_read_join_contains_x.
  - exact example_two_read_join_contains_y.
  - exact example_two_read_join_contains_x.
Qed.

End RRVerifyIrBlockMustDefComposeSubset.
