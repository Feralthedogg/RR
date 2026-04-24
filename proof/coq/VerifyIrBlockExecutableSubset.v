Require Import VerifyIrBlockDefinedHereSubset.
Require Import VerifyIrBlockAssignChainSubset.
Require Import VerifyIrBlockAssignBranchSubset.
Require Import VerifyIrBlockAssignStoreSubset.
Require Import VerifyIrBlockMustDefComposeSubset.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrStructLite.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrBlockDefinedHereSubset.
Import RRVerifyIrBlockAssignChainSubset.
Import RRVerifyIrBlockAssignBranchSubset.
Import RRVerifyIrBlockAssignStoreSubset.
Import RRVerifyIrBlockMustDefComposeSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrStructLite.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrBlockExecutableSubset.

Definition flow_lite_single_block_case
    (base : verify_ir_struct_lite_case)
    (table : actual_value_full_table_lite)
    (defs : def_set)
    (bb : actual_block_record_lite) : verify_ir_flow_lite_case :=
  {| flow_base := base;
     flow_blocks_case := [raw_flow_case_of_actual_block table defs bb] |}.

Lemma flow_lite_single_block_case_accepts_of_raw_block_flow_none :
  forall base table defs bb,
    verify_ir_struct_lite base = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs bb) = None ->
    verify_ir_flow_lite (flow_lite_single_block_case base table defs bb) = None.
Proof.
  intros base table defs bb HBase HBlock.
  unfold verify_ir_flow_lite, flow_lite_single_block_case.
  simpl.
  rewrite HBlock, HBase.
  reflexivity.
Qed.

Lemma flow_lite_single_block_case_accepts_of_required_subset :
  forall base table defs bb,
    verify_ir_struct_lite base = None ->
    (forall v, In v (raw_required_vars_of_block table bb) -> In v defs) ->
    verify_ir_flow_lite (flow_lite_single_block_case base table defs bb) = None.
Proof.
  intros base table defs bb HBase HReq.
  apply flow_lite_single_block_case_accepts_of_raw_block_flow_none; auto.
  apply raw_block_flow_none_of_required_subset.
  exact HReq.
Qed.

Lemma flow_lite_single_block_case_accepts_and_preserves_init :
  forall base table defs bb v,
    verify_ir_struct_lite base = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs bb) = None ->
    In v defs ->
    verify_ir_flow_lite (flow_lite_single_block_case base table defs bb) = None /\
    In v (final_defined_vars defs bb).
Proof.
  intros base table defs bb v HBase HBlock HMem.
  split.
  - apply flow_lite_single_block_case_accepts_of_raw_block_flow_none; auto.
  - apply in_final_defined_vars_of_in_init. exact HMem.
Qed.

Lemma example_flow_base_struct_clean :
  verify_ir_struct_lite example_flow_base = None.
Proof.
  reflexivity.
Qed.

Lemma example_assign_chain_executable_accepts :
  verify_ir_flow_lite
    (flow_lite_single_block_case example_flow_base example_actual_value_full_table
      (in_defs_from_preds 0%nat [] example_preds example_assign_chain_out_defs 3%nat)
      example_assign_chain_block) = None.
Proof.
  apply flow_lite_single_block_case_accepts_of_raw_block_flow_none.
  - exact example_flow_base_struct_clean.
  - exact example_assign_chain_block_clean_from_join.
Qed.

Lemma example_assign_branch_executable_accepts :
  verify_ir_flow_lite
    (flow_lite_single_block_case example_flow_base example_actual_value_full_table
      (in_defs_from_preds 0%nat [] example_preds example_assign_branch_out_defs 3%nat)
      example_assign_branch_block) = None.
Proof.
  apply flow_lite_single_block_case_accepts_of_raw_block_flow_none.
  - exact example_flow_base_struct_clean.
  - exact example_assign_branch_block_clean_from_join.
Qed.

Lemma example_assign_store3d_executable_accepts :
  verify_ir_flow_lite
    (flow_lite_single_block_case example_flow_base example_actual_value_full_table
      (in_defs_from_preds 0%nat [] example_preds example_assign_store_out_defs 3%nat)
      example_assign_store3d_block) = None.
Proof.
  apply flow_lite_single_block_case_accepts_of_raw_block_flow_none.
  - exact example_flow_base_struct_clean.
  - exact example_assign_store3d_block_clean_from_join.
Qed.

Lemma example_assign_chain_executable_preserves_incoming_y :
  let defs := in_defs_from_preds 0%nat [] example_preds example_assign_chain_out_defs 3%nat in
  verify_ir_flow_lite
    (flow_lite_single_block_case example_flow_base example_actual_value_full_table defs
      example_assign_chain_block) = None /\
  In "y" (final_defined_vars defs example_assign_chain_block).
Proof.
  simpl.
  split.
  - apply flow_lite_single_block_case_accepts_of_raw_block_flow_none.
    + exact example_flow_base_struct_clean.
    + exact example_assign_chain_block_clean_from_join.
  - simpl. auto.
Qed.

End RRVerifyIrBlockExecutableSubset.
