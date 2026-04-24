Require Import VerifyIrCfgExecutableSubset.
Require Import VerifyIrJoinExecutableSubset.
Require Import VerifyIrBlockExecutableSubset.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrBlockAssignChainSubset.
Require Import VerifyIrBlockAssignBranchSubset.
Require Import VerifyIrBlockMustDefComposeSubset.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrMustDefFixedPointSubset.
Require Import VerifyIrStructLite.
Require Import VerifyIrFlowLite.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrCfgExecutableSubset.
Import RRVerifyIrMustDefFixedPointSubset.
Import RRVerifyIrCfgExecutableSubset.
Import RRVerifyIrJoinExecutableSubset.
Import RRVerifyIrBlockExecutableSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrBlockAssignChainSubset.
Import RRVerifyIrBlockAssignBranchSubset.
Import RRVerifyIrBlockMustDefComposeSubset.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrStructLite.
Import RRVerifyIrFlowLite.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrCfgReachabilitySubset.

Record join_cfg_reachability_witness_lite : Type := {
  reach_cfg : join_cfg_witness_lite;
  reach_reachable : reachable_map;
  reach_preds : pred_map;
  reach_out_defs : def_map;
  reach_entry : nat;
  reach_entry_defs : def_set;
}.

Definition join_cfg_reach_join_reachable_ok (w : join_cfg_reachability_witness_lite) : Prop :=
  reach_reachable w (actual_block_id (cfg_join (reach_cfg w))) = true.

Definition join_cfg_reach_join_preds_ok (w : join_cfg_reachability_witness_lite) : Prop :=
  reachable_preds (reach_reachable w) (reach_preds w) (actual_block_id (cfg_join (reach_cfg w))) =
    cfg_join_preds (reach_cfg w).

Definition join_cfg_reach_join_step_in_defs_ok (w : join_cfg_reachability_witness_lite) : Prop :=
  step_in_defs (reach_entry w) (reach_entry_defs w)
    (reach_reachable w) (reach_preds w) (reach_out_defs w)
    (actual_block_id (cfg_join (reach_cfg w))) =
    cfg_defs_join (reach_cfg w).

Lemma join_cfg_reach_accepts_of_join_step_in_defs :
  forall w,
    join_cfg_preds_ok (reach_cfg w) ->
    join_cfg_order_ok (reach_cfg w) ->
    join_cfg_reach_join_reachable_ok w ->
    join_cfg_reach_join_preds_ok w ->
    join_cfg_reach_join_step_in_defs_ok w ->
    verify_ir_struct_lite (cfg_base (reach_cfg w)) = None ->
    verify_flow_block (raw_flow_case_of_actual_block
      (cfg_table (reach_cfg w)) (cfg_defs_left (reach_cfg w)) (cfg_left (reach_cfg w))) = None ->
    verify_flow_block (raw_flow_case_of_actual_block
      (cfg_table (reach_cfg w)) (cfg_defs_right (reach_cfg w)) (cfg_right (reach_cfg w))) = None ->
    (forall v, In v (raw_required_vars_of_block
      (cfg_table (reach_cfg w)) (cfg_join (reach_cfg w))) ->
      In v (step_in_defs (reach_entry w) (reach_entry_defs w)
        (reach_reachable w) (reach_preds w) (reach_out_defs w)
        (actual_block_id (cfg_join (reach_cfg w))))) ->
    verify_ir_flow_lite (join_cfg_to_flow_case (reach_cfg w)) = None.
Proof.
  intros w HCfgPreds HOrder HReach HPreds HJoinDefs HBase HLeft HRight HJoinReq.
  assert (HJoin :
    verify_flow_block (raw_flow_case_of_actual_block
      (cfg_table (reach_cfg w)) (cfg_defs_join (reach_cfg w)) (cfg_join (reach_cfg w))) = None).
  {
    rewrite <- HJoinDefs.
    apply raw_block_flow_none_of_required_subset.
    exact HJoinReq.
  }
  apply join_cfg_accepts_of_raw_blocks_none; assumption.
Qed.

Definition example_cfg_reachable : reachable_map :=
  fun bid =>
    match bid with
    | 40%nat => true
    | 50%nat => true
    | 30%nat => true
    | _ => false
    end.

Definition example_cfg_pred_map : pred_map :=
  fun bid =>
    match bid with
    | 30%nat => [40%nat; 50%nat]
    | _ => []
    end.

Definition example_cfg_out_defs : def_map :=
  fun bid =>
    match bid with
    | 40%nat => ["x"; "y"]
    | 50%nat => ["x"; "y"]
    | _ => []
    end.

Definition example_join_cfg_reachability_witness : join_cfg_reachability_witness_lite :=
  {| reach_cfg := example_join_cfg_witness;
     reach_reachable := example_cfg_reachable;
     reach_preds := example_cfg_pred_map;
     reach_out_defs := example_cfg_out_defs;
     reach_entry := 0%nat;
     reach_entry_defs := [] |}.

Lemma example_join_cfg_reachability_witness_join_reachable_ok :
  join_cfg_reach_join_reachable_ok example_join_cfg_reachability_witness.
Proof.
  reflexivity.
Qed.

Lemma example_join_cfg_reachability_witness_join_preds_ok :
  join_cfg_reach_join_preds_ok example_join_cfg_reachability_witness.
Proof.
  reflexivity.
Qed.

Lemma example_join_cfg_reachability_witness_join_step_in_defs_ok :
  join_cfg_reach_join_step_in_defs_ok example_join_cfg_reachability_witness.
Proof.
  reflexivity.
Qed.

Lemma example_join_cfg_reachability_witness_join_reachable_preds_nonempty :
  reachable_preds
    (reach_reachable example_join_cfg_reachability_witness)
    (reach_preds example_join_cfg_reachability_witness)
    (actual_block_id (cfg_join (reach_cfg example_join_cfg_reachability_witness))) <> [].
Proof.
  discriminate.
Qed.

Lemma example_join_cfg_reachability_witness_join_req :
  forall v,
    In v (raw_required_vars_of_block
      (cfg_table (reach_cfg example_join_cfg_reachability_witness))
      (cfg_join (reach_cfg example_join_cfg_reachability_witness))) ->
    In v (step_in_defs
      (reach_entry example_join_cfg_reachability_witness)
      (reach_entry_defs example_join_cfg_reachability_witness)
      (reach_reachable example_join_cfg_reachability_witness)
      (reach_preds example_join_cfg_reachability_witness)
      (reach_out_defs example_join_cfg_reachability_witness)
      (actual_block_id (cfg_join (reach_cfg example_join_cfg_reachability_witness)))).
Proof.
  intros v Hv.
  change (In v (raw_required_vars_of_block example_actual_value_full_table example_multi_read_block)) in Hv.
  rewrite example_multi_read_block_raw_required in Hv.
  simpl in Hv.
  destruct Hv as [Hv|[Hv|[Hv|[]]]]; subst v.
  - apply in_step_in_defs_of_forall_reachable_pred.
    + exact example_join_cfg_reachability_witness_join_reachable_ok.
    + discriminate.
    + exact example_join_cfg_reachability_witness_join_reachable_preds_nonempty.
    + intros pred Hpred.
      simpl in Hpred.
      destruct Hpred as [Hpred|[Hpred|[]]]; subst pred; simpl; auto.
  - apply in_step_in_defs_of_forall_reachable_pred.
    + exact example_join_cfg_reachability_witness_join_reachable_ok.
    + discriminate.
    + exact example_join_cfg_reachability_witness_join_reachable_preds_nonempty.
    + intros pred Hpred.
      simpl in Hpred.
      destruct Hpred as [Hpred|[Hpred|[]]]; subst pred; simpl; auto.
  - apply in_step_in_defs_of_forall_reachable_pred.
    + exact example_join_cfg_reachability_witness_join_reachable_ok.
    + discriminate.
    + exact example_join_cfg_reachability_witness_join_reachable_preds_nonempty.
    + intros pred Hpred.
      simpl in Hpred.
      destruct Hpred as [Hpred|[Hpred|[]]]; subst pred; simpl; auto.
Qed.

Lemma example_join_cfg_reachability_witness_accepts :
  verify_ir_flow_lite (join_cfg_to_flow_case (reach_cfg example_join_cfg_reachability_witness)) = None.
Proof.
  apply join_cfg_reach_accepts_of_join_step_in_defs.
  - exact example_join_cfg_witness_preds_ok.
  - exact example_join_cfg_witness_order_ok.
  - exact example_join_cfg_reachability_witness_join_reachable_ok.
  - exact example_join_cfg_reachability_witness_join_preds_ok.
  - exact example_join_cfg_reachability_witness_join_step_in_defs_ok.
  - exact example_flow_base_struct_clean.
  - exact example_assign_chain_block_clean_from_join.
  - exact example_assign_branch_block_clean_from_join.
  - exact example_join_cfg_reachability_witness_join_req.
Qed.

End RRVerifyIrCfgReachabilitySubset.
