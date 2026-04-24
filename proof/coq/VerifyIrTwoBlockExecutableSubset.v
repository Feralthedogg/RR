Require Import VerifyIrBlockExecutableSubset.
Require Import VerifyIrBlockAssignChainSubset.
Require Import VerifyIrBlockAssignStoreSubset.
Require Import VerifyIrBlockDefinedHereSubset.
Require Import VerifyIrBlockMustDefComposeSubset.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrFlowLite.
Require Import VerifyIrStructLite.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrBlockExecutableSubset.
Import RRVerifyIrBlockAssignChainSubset.
Import RRVerifyIrBlockAssignStoreSubset.
Import RRVerifyIrBlockDefinedHereSubset.
Import RRVerifyIrBlockMustDefComposeSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrFlowLite.
Import RRVerifyIrStructLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrTwoBlockExecutableSubset.

Definition flow_lite_two_block_case
    (base : verify_ir_struct_lite_case)
    (table : actual_value_full_table_lite)
    (defs1 : def_set) (bb1 : actual_block_record_lite)
    (defs2 : def_set) (bb2 : actual_block_record_lite)
    : verify_ir_flow_lite_case :=
  {| flow_base := base;
     flow_blocks_case :=
       [ raw_flow_case_of_actual_block table defs1 bb1
       ; raw_flow_case_of_actual_block table defs2 bb2
       ] |}.

Lemma flow_lite_two_block_case_accepts_of_raw_blocks_none :
  forall base table defs1 bb1 defs2 bb2,
    verify_ir_struct_lite base = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs1 bb1) = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs2 bb2) = None ->
    verify_ir_flow_lite (flow_lite_two_block_case base table defs1 bb1 defs2 bb2) = None.
Proof.
  intros base table defs1 bb1 defs2 bb2 HBase H1 H2.
  unfold verify_ir_flow_lite, flow_lite_two_block_case.
  simpl.
  rewrite H1, H2, HBase.
  reflexivity.
Qed.

Lemma flow_lite_two_block_case_accepts_of_required_subset :
  forall base table defs1 bb1 defs2 bb2,
    verify_ir_struct_lite base = None ->
    (forall v, In v (raw_required_vars_of_block table bb1) -> In v defs1) ->
    (forall v, In v (raw_required_vars_of_block table bb2) -> In v defs2) ->
    verify_ir_flow_lite (flow_lite_two_block_case base table defs1 bb1 defs2 bb2) = None.
Proof.
  intros base table defs1 bb1 defs2 bb2 HBase HReq1 HReq2.
  apply flow_lite_two_block_case_accepts_of_raw_blocks_none; auto.
  - apply raw_block_flow_none_of_required_subset. exact HReq1.
  - apply raw_block_flow_none_of_required_subset. exact HReq2.
Qed.

Lemma flow_lite_two_block_case_accepts_and_preserves_init :
  forall base table defs1 bb1 defs2 bb2 v1 v2,
    verify_ir_struct_lite base = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs1 bb1) = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs2 bb2) = None ->
    In v1 defs1 ->
    In v2 defs2 ->
    verify_ir_flow_lite (flow_lite_two_block_case base table defs1 bb1 defs2 bb2) = None /\
    In v1 (final_defined_vars defs1 bb1) /\
    In v2 (final_defined_vars defs2 bb2).
Proof.
  intros base table defs1 bb1 defs2 bb2 v1 v2 HBase H1 H2 HMem1 HMem2.
  split.
  - apply flow_lite_two_block_case_accepts_of_raw_blocks_none; auto.
  - split.
    + apply in_final_defined_vars_of_in_init. exact HMem1.
    + apply in_final_defined_vars_of_in_init. exact HMem2.
Qed.

Lemma example_two_block_executable_accepts :
  verify_ir_flow_lite
    (flow_lite_two_block_case example_flow_base example_actual_value_full_table
      (in_defs_from_preds 0%nat [] example_preds example_assign_chain_out_defs 3%nat)
      example_assign_chain_block
      (in_defs_from_preds 0%nat [] example_preds example_assign_store_out_defs 3%nat)
      example_assign_store3d_block) = None.
Proof.
  apply flow_lite_two_block_case_accepts_of_raw_blocks_none.
  - exact example_flow_base_struct_clean.
  - exact example_assign_chain_block_clean_from_join.
  - exact example_assign_store3d_block_clean_from_join.
Qed.

Lemma example_two_block_executable_preserves_incoming_y :
  let defs1 := in_defs_from_preds 0%nat [] example_preds example_assign_chain_out_defs 3%nat in
  let defs2 := in_defs_from_preds 0%nat [] example_preds example_assign_store_out_defs 3%nat in
  verify_ir_flow_lite
    (flow_lite_two_block_case example_flow_base example_actual_value_full_table
      defs1 example_assign_chain_block defs2 example_assign_store3d_block) = None /\
  In "y" (final_defined_vars defs1 example_assign_chain_block) /\
  In "y" (final_defined_vars defs2 example_assign_store3d_block).
Proof.
  simpl.
  split.
  - apply flow_lite_two_block_case_accepts_of_raw_blocks_none.
    + exact example_flow_base_struct_clean.
    + exact example_assign_chain_block_clean_from_join.
    + exact example_assign_store3d_block_clean_from_join.
  - split; simpl; auto.
Qed.

End RRVerifyIrTwoBlockExecutableSubset.
