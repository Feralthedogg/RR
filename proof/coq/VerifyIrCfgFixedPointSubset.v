Require Import VerifyIrCfgOrderWorklistSubset.
Require Import VerifyIrCfgWorklistSubset.
Require Import VerifyIrCfgConvergenceSubset.
Require Import VerifyIrCfgReachabilitySubset.
Require Import VerifyIrCfgExecutableSubset.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrStructLite.
Require Import VerifyIrFlowLite.
Require Import VerifyIrBlockMustDefComposeSubset.
Require Import VerifyIrMustDefConvergenceSubset.
Require Import VerifyIrMustDefFixedPointSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrCfgOrderWorklistSubset.
Import RRVerifyIrCfgWorklistSubset.
Import RRVerifyIrCfgConvergenceSubset.
Import RRVerifyIrCfgReachabilitySubset.
Import RRVerifyIrCfgExecutableSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrStructLite.
Import RRVerifyIrFlowLite.
Import RRVerifyIrBlockMustDefComposeSubset.
Import RRVerifyIrMustDefConvergenceSubset.
Import RRVerifyIrMustDefFixedPointSubset.

Module RRVerifyIrCfgFixedPointSubset.

Inductive cfg_fixed_point_error_lite : Type :=
| cfp_struct
| cfp_changed
| cfp_flow.

Definition verify_ir_cfg_fixed_point_lite
    (w : join_cfg_order_worklist_witness_lite) :
    option cfg_fixed_point_error_lite :=
  match verify_ir_struct_lite (cfg_base (reach_cfg (conv_base (work_base (order_base w))))) with
  | Some _ => Some cfp_struct
  | None =>
      if join_cfg_order_any_changed w then
        Some cfp_changed
      else
        match verify_ir_flow_lite
          (join_cfg_to_flow_case (reach_cfg (conv_base (work_base (order_base w))))) with
        | None => None
        | Some _ => Some cfp_flow
        end
  end.

Lemma verify_ir_cfg_fixed_point_lite_none_of_stable_seed_step_in_defs :
  forall w,
    join_cfg_preds_ok (reach_cfg (conv_base (work_base (order_base w)))) ->
    join_cfg_order_ok (reach_cfg (conv_base (work_base (order_base w)))) ->
    join_cfg_reach_join_reachable_ok (conv_base (work_base (order_base w))) ->
    join_cfg_reach_join_preds_ok (conv_base (work_base (order_base w))) ->
    join_cfg_conv_seed_stable (work_base (order_base w)) ->
    step_in_defs
      (reach_entry (conv_base (work_base (order_base w))))
      (reach_entry_defs (conv_base (work_base (order_base w))))
      (reach_reachable (conv_base (work_base (order_base w))))
      (reach_preds (conv_base (work_base (order_base w))))
      (conv_seed (work_base (order_base w)))
      (actual_block_id (cfg_join (reach_cfg (conv_base (work_base (order_base w)))))) =
      cfg_defs_join (reach_cfg (conv_base (work_base (order_base w)))) ->
    verify_ir_struct_lite (cfg_base (reach_cfg (conv_base (work_base (order_base w))))) = None ->
    verify_flow_block (raw_flow_case_of_actual_block
      (cfg_table (reach_cfg (conv_base (work_base (order_base w)))))
      (cfg_defs_left (reach_cfg (conv_base (work_base (order_base w)))))
      (cfg_left (reach_cfg (conv_base (work_base (order_base w)))))) = None ->
    verify_flow_block (raw_flow_case_of_actual_block
      (cfg_table (reach_cfg (conv_base (work_base (order_base w)))))
      (cfg_defs_right (reach_cfg (conv_base (work_base (order_base w)))))
      (cfg_right (reach_cfg (conv_base (work_base (order_base w)))))) = None ->
    (forall v, In v (raw_required_vars_of_block
      (cfg_table (reach_cfg (conv_base (work_base (order_base w)))))
      (cfg_join (reach_cfg (conv_base (work_base (order_base w)))))) ->
      In v (step_in_defs
        (reach_entry (conv_base (work_base (order_base w))))
        (reach_entry_defs (conv_base (work_base (order_base w))))
        (reach_reachable (conv_base (work_base (order_base w))))
        (reach_preds (conv_base (work_base (order_base w))))
        (conv_seed (work_base (order_base w)))
        (actual_block_id (cfg_join (reach_cfg (conv_base (work_base (order_base w)))))))) ->
    verify_ir_cfg_fixed_point_lite w = None.
Proof.
  intros w HCfgPreds HOrder HReach HPredMap HStable HSeedJoinDefs HBase HLeft HRight HJoinReq.
  pose proof (join_cfg_order_accepts_and_reports_no_change_of_stable_seed_step_in_defs
    w HCfgPreds HOrder HReach HPredMap HStable HSeedJoinDefs HBase HLeft HRight HJoinReq)
    as Hacc.
  destruct Hacc as [HFlow HNoChange].
  unfold verify_ir_cfg_fixed_point_lite.
  rewrite HBase.
  rewrite HNoChange.
  rewrite HFlow.
  reflexivity.
Qed.

End RRVerifyIrCfgFixedPointSubset.
