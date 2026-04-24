Require Import VerifyIrTwoBlockExecutableSubset.
Require Import VerifyIrBlockExecutableSubset.
Require Import VerifyIrBlockAssignChainSubset.
Require Import VerifyIrBlockAssignBranchSubset.
Require Import VerifyIrBlockDefinedHereSubset.
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
Import RRVerifyIrTwoBlockExecutableSubset.
Import RRVerifyIrBlockExecutableSubset.
Import RRVerifyIrBlockAssignChainSubset.
Import RRVerifyIrBlockAssignBranchSubset.
Import RRVerifyIrBlockDefinedHereSubset.
Import RRVerifyIrBlockMustDefComposeSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrStructLite.
Import RRVerifyIrValueFullRecordSubset.
Import RRVerifyIrBlockDefinedHereSubset.

Module RRVerifyIrJoinExecutableSubset.

Definition flow_lite_join_case
    (base : verify_ir_struct_lite_case)
    (table : actual_value_full_table_lite)
    (defs_left : def_set) (left : actual_block_record_lite)
    (defs_right : def_set) (right : actual_block_record_lite)
    (defs_join : def_set) (join : actual_block_record_lite)
    : verify_ir_flow_lite_case :=
  {| flow_base := base;
     flow_blocks_case :=
       [ raw_flow_case_of_actual_block table defs_left left
       ; raw_flow_case_of_actual_block table defs_right right
       ; raw_flow_case_of_actual_block table defs_join join
       ] |}.

Lemma flow_lite_join_case_accepts_of_raw_blocks_none :
  forall base table defs_left left defs_right right defs_join join,
    verify_ir_struct_lite base = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs_left left) = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs_right right) = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs_join join) = None ->
    verify_ir_flow_lite
      (flow_lite_join_case base table defs_left left defs_right right defs_join join) = None.
Proof.
  intros base table defs_left left defs_right right defs_join join HBase HLeft HRight HJoin.
  unfold verify_ir_flow_lite, flow_lite_join_case.
  simpl.
  rewrite HLeft, HRight, HJoin, HBase.
  reflexivity.
Qed.

Lemma flow_lite_join_case_accepts_of_required_subset :
  forall base table defs_left left defs_right right defs_join join,
    verify_ir_struct_lite base = None ->
    (forall v, In v (raw_required_vars_of_block table left) -> In v defs_left) ->
    (forall v, In v (raw_required_vars_of_block table right) -> In v defs_right) ->
    (forall v, In v (raw_required_vars_of_block table join) -> In v defs_join) ->
    verify_ir_flow_lite
      (flow_lite_join_case base table defs_left left defs_right right defs_join join) = None.
Proof.
  intros base table defs_left left defs_right right defs_join join HBase HLeft HRight HJoin.
  apply flow_lite_join_case_accepts_of_raw_blocks_none; auto.
  - apply raw_block_flow_none_of_required_subset. exact HLeft.
  - apply raw_block_flow_none_of_required_subset. exact HRight.
  - apply raw_block_flow_none_of_required_subset. exact HJoin.
Qed.

Lemma flow_lite_join_case_accepts_and_preserves_init :
  forall base table defs_left left defs_right right defs_join join v_left v_right v_join,
    verify_ir_struct_lite base = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs_left left) = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs_right right) = None ->
    verify_flow_block (raw_flow_case_of_actual_block table defs_join join) = None ->
    In v_left defs_left ->
    In v_right defs_right ->
    In v_join defs_join ->
    verify_ir_flow_lite
      (flow_lite_join_case base table defs_left left defs_right right defs_join join) = None /\
    In v_left (final_defined_vars defs_left left) /\
    In v_right (final_defined_vars defs_right right) /\
    In v_join (final_defined_vars defs_join join).
Proof.
  intros base table defs_left left defs_right right defs_join join
    v_left v_right v_join HBase HLeft HRight HJoin HMemLeft HMemRight HMemJoin.
  split.
  - apply flow_lite_join_case_accepts_of_raw_blocks_none; auto.
  - split.
    + apply in_final_defined_vars_of_in_init. exact HMemLeft.
    + split.
      * apply in_final_defined_vars_of_in_init. exact HMemRight.
      * apply in_final_defined_vars_of_in_init. exact HMemJoin.
Qed.

Lemma example_join_executable_accepts :
  verify_ir_flow_lite
    (flow_lite_join_case example_flow_base example_actual_value_full_table
      (in_defs_from_preds 0%nat [] example_preds example_assign_chain_out_defs 3%nat)
      example_assign_chain_block
      (in_defs_from_preds 0%nat [] example_preds example_assign_branch_out_defs 3%nat)
      example_assign_branch_block
      (in_defs_from_preds 0%nat [] example_preds example_two_read_out_defs 3%nat)
      example_multi_read_block) = None.
Proof.
  apply flow_lite_join_case_accepts_of_raw_blocks_none.
  - exact example_flow_base_struct_clean.
  - exact example_assign_chain_block_clean_from_join.
  - exact example_assign_branch_block_clean_from_join.
  - exact example_multi_read_block_clean_from_join.
Qed.

Lemma example_join_executable_preserves_incoming_y :
  let defs_left := in_defs_from_preds 0%nat [] example_preds example_assign_chain_out_defs 3%nat in
  let defs_right := in_defs_from_preds 0%nat [] example_preds example_assign_branch_out_defs 3%nat in
  let defs_join := in_defs_from_preds 0%nat [] example_preds example_two_read_out_defs 3%nat in
  verify_ir_flow_lite
    (flow_lite_join_case example_flow_base example_actual_value_full_table
      defs_left example_assign_chain_block
      defs_right example_assign_branch_block
      defs_join example_multi_read_block) = None /\
  In "y" (final_defined_vars defs_left example_assign_chain_block) /\
  In "y" (final_defined_vars defs_right example_assign_branch_block) /\
  In "y" (final_defined_vars defs_join example_multi_read_block).
Proof.
  simpl.
  split.
  - apply flow_lite_join_case_accepts_of_raw_blocks_none.
    + exact example_flow_base_struct_clean.
    + exact example_assign_chain_block_clean_from_join.
    + exact example_assign_branch_block_clean_from_join.
    + exact example_multi_read_block_clean_from_join.
  - split; [simpl; auto | split; simpl; auto].
Qed.

End RRVerifyIrJoinExecutableSubset.
