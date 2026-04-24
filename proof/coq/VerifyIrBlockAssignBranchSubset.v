Require Import VerifyIrBlockAssignChainSubset.
Require Import VerifyIrBlockMustDefComposeSubset.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrBlockFlowSubset.
Require Import VerifyIrFlowLite.
Require Import VerifyIrFnHintMapSubset.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrBlockAssignChainSubset.
Import RRVerifyIrBlockMustDefComposeSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrBlockFlowSubset.
Import RRVerifyIrFlowLite.
Import RRVerifyIrFnHintMapSubset.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrBlockAssignBranchSubset.

Definition example_assign_branch_out_defs (bid : nat) : def_set :=
  match bid with
  | 1%nat => ["y"]
  | 2%nat => ["y"; "guard"]
  | _ => []
  end.

Lemma example_assign_branch_join_contains_y :
  In "y" (in_defs_from_preds 0%nat [] example_preds example_assign_branch_out_defs 3%nat).
Proof.
  apply in_in_defs_from_preds_of_forall_pred.
  - discriminate.
  - discriminate.
  - intros pred Hpred.
    simpl in Hpred.
    destruct Hpred as [Hpred|[Hpred|[]]]; subst pred; simpl; auto.
Qed.

Definition example_assign_branch_block : actual_block_record_lite :=
  {| actual_block_id := 50%nat;
     actual_block_instrs :=
       [ IRAssign "loop" 4%nat SSource
       ; IRAssign "x" 6%nat SSource
       ];
     actual_block_term := TRLBranch 3%nat 1%nat 2%nat |}.

Lemma example_assign_branch_block_raw_required :
  raw_required_vars_of_block example_actual_value_full_table example_assign_branch_block = ["y"].
Proof.
  reflexivity.
Qed.

Lemma example_assign_branch_block_clean_of_y :
  forall defs,
    In "y" defs ->
    verify_flow_block
      (raw_flow_case_of_actual_block example_actual_value_full_table defs
        example_assign_branch_block) = None.
Proof.
  intros defs HMem.
  apply raw_block_flow_none_of_required_subset.
  intros v Hv.
  rewrite example_assign_branch_block_raw_required in Hv.
  simpl in Hv.
  destruct Hv as [Hv|[]].
  subst v.
  exact HMem.
Qed.

Lemma example_assign_branch_block_clean_from_join :
  verify_flow_block
    (raw_flow_case_of_actual_block example_actual_value_full_table
      (in_defs_from_preds 0%nat [] example_preds example_assign_branch_out_defs 3%nat)
      example_assign_branch_block) = None.
Proof.
  apply example_assign_branch_block_clean_of_y.
  exact example_assign_branch_join_contains_y.
Qed.

Definition example_assign_branch_fn_block_record : fn_block_record_lite :=
  {| fn_block_shell := example_fn_hint_map_record;
     fn_block_blocks := [example_assign_branch_block] |}.

Definition example_assign_branch_flow_lite_case : verify_ir_flow_lite_case :=
  flow_lite_case_of_fn_block example_flow_base example_assign_branch_fn_block_record
    [in_defs_from_preds 0%nat [] example_preds example_assign_branch_out_defs 3%nat].

Lemma example_assign_branch_flow_lite_case_accepts :
  verify_ir_flow_lite example_assign_branch_flow_lite_case = None.
Proof.
  reflexivity.
Qed.

End RRVerifyIrBlockAssignBranchSubset.
