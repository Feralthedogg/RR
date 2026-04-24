Require Import VerifyIrJoinExecutableSubset.
Require Import VerifyIrBlockExecutableSubset.
Require Import VerifyIrBlockAssignChainSubset.
Require Import VerifyIrBlockAssignBranchSubset.
Require Import VerifyIrBlockDefinedHereSubset.
Require Import VerifyIrBlockMustDefComposeSubset.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrFlowLite.
Require Import VerifyIrStructLite.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrJoinExecutableSubset.
Import RRVerifyIrBlockExecutableSubset.
Import RRVerifyIrBlockAssignChainSubset.
Import RRVerifyIrBlockAssignBranchSubset.
Import RRVerifyIrBlockDefinedHereSubset.
Import RRVerifyIrBlockMustDefComposeSubset.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrFlowLite.
Import RRVerifyIrStructLite.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrCfgExecutableSubset.

Record join_cfg_witness_lite : Type := {
  cfg_base : verify_ir_struct_lite_case;
  cfg_table : actual_value_full_table_lite;
  cfg_defs_left : def_set;
  cfg_left : actual_block_record_lite;
  cfg_defs_right : def_set;
  cfg_right : actual_block_record_lite;
  cfg_defs_join : def_set;
  cfg_join : actual_block_record_lite;
  cfg_join_preds : list nat;
  cfg_block_order : list nat;
}.

Definition join_cfg_to_flow_case (w : join_cfg_witness_lite) : verify_ir_flow_lite_case :=
  flow_lite_join_case
    (cfg_base w) (cfg_table w)
    (cfg_defs_left w) (cfg_left w)
    (cfg_defs_right w) (cfg_right w)
    (cfg_defs_join w) (cfg_join w).

Definition join_cfg_preds_ok (w : join_cfg_witness_lite) : Prop :=
  cfg_join_preds w = [actual_block_id (cfg_left w); actual_block_id (cfg_right w)].

Definition join_cfg_order_ok (w : join_cfg_witness_lite) : Prop :=
  cfg_block_order w =
    [actual_block_id (cfg_left w); actual_block_id (cfg_right w); actual_block_id (cfg_join w)].

Lemma join_cfg_accepts_of_raw_blocks_none :
  forall w,
    join_cfg_preds_ok w ->
    join_cfg_order_ok w ->
    verify_ir_struct_lite (cfg_base w) = None ->
    verify_flow_block (raw_flow_case_of_actual_block (cfg_table w) (cfg_defs_left w) (cfg_left w)) = None ->
    verify_flow_block (raw_flow_case_of_actual_block (cfg_table w) (cfg_defs_right w) (cfg_right w)) = None ->
    verify_flow_block (raw_flow_case_of_actual_block (cfg_table w) (cfg_defs_join w) (cfg_join w)) = None ->
    verify_ir_flow_lite (join_cfg_to_flow_case w) = None.
Proof.
  intros w HPreds HOrder HBase HLeft HRight HJoin.
  exact (flow_lite_join_case_accepts_of_raw_blocks_none
    (cfg_base w) (cfg_table w)
    (cfg_defs_left w) (cfg_left w)
    (cfg_defs_right w) (cfg_right w)
    (cfg_defs_join w) (cfg_join w)
    HBase HLeft HRight HJoin).
Qed.

Lemma join_cfg_accepts_and_preserves_init :
  forall w,
    join_cfg_preds_ok w ->
    join_cfg_order_ok w ->
    verify_ir_struct_lite (cfg_base w) = None ->
    verify_flow_block (raw_flow_case_of_actual_block (cfg_table w) (cfg_defs_left w) (cfg_left w)) = None ->
    verify_flow_block (raw_flow_case_of_actual_block (cfg_table w) (cfg_defs_right w) (cfg_right w)) = None ->
    verify_flow_block (raw_flow_case_of_actual_block (cfg_table w) (cfg_defs_join w) (cfg_join w)) = None ->
    In "y" (cfg_defs_left w) ->
    In "y" (cfg_defs_right w) ->
    In "y" (cfg_defs_join w) ->
    verify_ir_flow_lite (join_cfg_to_flow_case w) = None /\
    In "y" (final_defined_vars (cfg_defs_left w) (cfg_left w)) /\
    In "y" (final_defined_vars (cfg_defs_right w) (cfg_right w)) /\
    In "y" (final_defined_vars (cfg_defs_join w) (cfg_join w)).
Proof.
  intros w HPreds HOrder HBase HLeft HRight HJoin HMemLeft HMemRight HMemJoin.
  split.
  - apply join_cfg_accepts_of_raw_blocks_none; assumption.
  - split.
    + apply in_final_defined_vars_of_in_init. exact HMemLeft.
    + split.
      * apply in_final_defined_vars_of_in_init. exact HMemRight.
      * apply in_final_defined_vars_of_in_init. exact HMemJoin.
Qed.

Definition example_join_cfg_witness : join_cfg_witness_lite :=
  {| cfg_base := example_flow_base;
     cfg_table := example_actual_value_full_table;
     cfg_defs_left := in_defs_from_preds 0%nat [] example_preds example_assign_chain_out_defs 3%nat;
     cfg_left := example_assign_chain_block;
     cfg_defs_right := in_defs_from_preds 0%nat [] example_preds example_assign_branch_out_defs 3%nat;
     cfg_right := example_assign_branch_block;
     cfg_defs_join := in_defs_from_preds 0%nat [] example_preds example_two_read_out_defs 3%nat;
     cfg_join := example_multi_read_block;
     cfg_join_preds := [40%nat; 50%nat];
     cfg_block_order := [40%nat; 50%nat; 30%nat] |}.

Lemma example_join_cfg_witness_preds_ok :
  join_cfg_preds_ok example_join_cfg_witness.
Proof.
  reflexivity.
Qed.

Lemma example_join_cfg_witness_order_ok :
  join_cfg_order_ok example_join_cfg_witness.
Proof.
  reflexivity.
Qed.

Lemma example_join_cfg_witness_accepts :
  verify_ir_flow_lite (join_cfg_to_flow_case example_join_cfg_witness) = None.
Proof.
  apply join_cfg_accepts_of_raw_blocks_none.
  - exact example_join_cfg_witness_preds_ok.
  - exact example_join_cfg_witness_order_ok.
  - exact example_flow_base_struct_clean.
  - exact example_assign_chain_block_clean_from_join.
  - exact example_assign_branch_block_clean_from_join.
  - exact example_multi_read_block_clean_from_join.
Qed.

Lemma example_join_cfg_witness_preserves_incoming_y :
  verify_ir_flow_lite (join_cfg_to_flow_case example_join_cfg_witness) = None /\
  In "y" (final_defined_vars (cfg_defs_left example_join_cfg_witness) (cfg_left example_join_cfg_witness)) /\
  In "y" (final_defined_vars (cfg_defs_right example_join_cfg_witness) (cfg_right example_join_cfg_witness)) /\
  In "y" (final_defined_vars (cfg_defs_join example_join_cfg_witness) (cfg_join example_join_cfg_witness)).
Proof.
  apply join_cfg_accepts_and_preserves_init.
  - exact example_join_cfg_witness_preds_ok.
  - exact example_join_cfg_witness_order_ok.
  - exact example_flow_base_struct_clean.
  - exact example_assign_chain_block_clean_from_join.
  - exact example_assign_branch_block_clean_from_join.
  - exact example_multi_read_block_clean_from_join.
  - exact example_assign_chain_join_contains_y.
  - exact example_assign_branch_join_contains_y.
  - exact example_two_read_join_contains_y.
Qed.

End RRVerifyIrCfgExecutableSubset.
