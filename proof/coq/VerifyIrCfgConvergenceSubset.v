Require Import VerifyIrCfgReachabilitySubset.
Require Import VerifyIrCfgExecutableSubset.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrBlockMustDefComposeSubset.
Require Import VerifyIrStructLite.
Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefConvergenceSubset.
Require Import VerifyIrMustDefFixedPointSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrCfgReachabilitySubset.
Import RRVerifyIrCfgExecutableSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrBlockMustDefComposeSubset.
Import RRVerifyIrStructLite.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefConvergenceSubset.
Import RRVerifyIrMustDefFixedPointSubset.

Module RRVerifyIrCfgConvergenceSubset.

Record join_cfg_convergence_witness_lite : Type := {
  conv_base : join_cfg_reachability_witness_lite;
  conv_assigned : assign_map;
  conv_seed : def_map;
  conv_fuel : nat;
}.

Definition join_cfg_conv_seed_stable (w : join_cfg_convergence_witness_lite) : Prop :=
  out_map_stable
    (reach_entry (conv_base w))
    (reach_entry_defs (conv_base w))
    (reach_reachable (conv_base w))
    (reach_preds (conv_base w))
    (conv_assigned w)
    (conv_seed w).

Definition join_cfg_conv_iterated_out_defs (w : join_cfg_convergence_witness_lite) : def_map :=
  iterate_out_map
    (reach_entry (conv_base w))
    (reach_entry_defs (conv_base w))
    (reach_reachable (conv_base w))
    (reach_preds (conv_base w))
    (conv_assigned w)
    (conv_fuel w)
    (conv_seed w).

Definition join_cfg_conv_to_reachability_witness
    (w : join_cfg_convergence_witness_lite) : join_cfg_reachability_witness_lite :=
  {| reach_cfg := reach_cfg (conv_base w);
     reach_reachable := reach_reachable (conv_base w);
     reach_preds := reach_preds (conv_base w);
     reach_out_defs := join_cfg_conv_iterated_out_defs w;
     reach_entry := reach_entry (conv_base w);
     reach_entry_defs := reach_entry_defs (conv_base w) |}.

Lemma join_cfg_conv_iterated_out_defs_eq_seed_of_stable :
  forall w,
    join_cfg_conv_seed_stable w ->
    join_cfg_conv_iterated_out_defs w = conv_seed w.
Proof.
  intros w Hstable.
  unfold join_cfg_conv_iterated_out_defs.
  apply iterate_out_map_of_stable.
  exact Hstable.
Qed.

Lemma join_cfg_conv_accepts_of_stable_seed_step_in_defs :
  forall w,
    join_cfg_preds_ok (reach_cfg (conv_base w)) ->
    join_cfg_order_ok (reach_cfg (conv_base w)) ->
    join_cfg_reach_join_reachable_ok (conv_base w) ->
    join_cfg_reach_join_preds_ok (conv_base w) ->
    join_cfg_conv_seed_stable w ->
    step_in_defs
      (reach_entry (conv_base w))
      (reach_entry_defs (conv_base w))
      (reach_reachable (conv_base w))
      (reach_preds (conv_base w))
      (conv_seed w)
      (actual_block_id (cfg_join (reach_cfg (conv_base w)))) =
      cfg_defs_join (reach_cfg (conv_base w)) ->
    verify_ir_struct_lite (cfg_base (reach_cfg (conv_base w))) = None ->
    verify_flow_block (raw_flow_case_of_actual_block
      (cfg_table (reach_cfg (conv_base w)))
      (cfg_defs_left (reach_cfg (conv_base w)))
      (cfg_left (reach_cfg (conv_base w)))) = None ->
    verify_flow_block (raw_flow_case_of_actual_block
      (cfg_table (reach_cfg (conv_base w)))
      (cfg_defs_right (reach_cfg (conv_base w)))
      (cfg_right (reach_cfg (conv_base w)))) = None ->
    (forall v, In v (raw_required_vars_of_block
      (cfg_table (reach_cfg (conv_base w)))
      (cfg_join (reach_cfg (conv_base w)))) ->
      In v (step_in_defs
        (reach_entry (conv_base w))
        (reach_entry_defs (conv_base w))
        (reach_reachable (conv_base w))
        (reach_preds (conv_base w))
        (conv_seed w)
        (actual_block_id (cfg_join (reach_cfg (conv_base w)))))) ->
    verify_ir_flow_lite (join_cfg_to_flow_case (reach_cfg (conv_base w))) = None.
Proof.
  intros w HCfgPreds HOrder HReach HPredMap HStable HSeedJoinDefs HBase HLeft HRight HJoinReq.
  assert (HIterJoinDefs :
    step_in_defs
      (reach_entry (conv_base w))
      (reach_entry_defs (conv_base w))
      (reach_reachable (conv_base w))
      (reach_preds (conv_base w))
      (join_cfg_conv_iterated_out_defs w)
      (actual_block_id (cfg_join (reach_cfg (conv_base w)))) =
      cfg_defs_join (reach_cfg (conv_base w))).
  {
    rewrite join_cfg_conv_iterated_out_defs_eq_seed_of_stable by exact HStable.
    exact HSeedJoinDefs.
  }
  assert (HIterJoinReq :
    forall v, In v (raw_required_vars_of_block
      (cfg_table (reach_cfg (conv_base w)))
      (cfg_join (reach_cfg (conv_base w)))) ->
      In v (step_in_defs
        (reach_entry (conv_base w))
        (reach_entry_defs (conv_base w))
        (reach_reachable (conv_base w))
        (reach_preds (conv_base w))
        (join_cfg_conv_iterated_out_defs w)
        (actual_block_id (cfg_join (reach_cfg (conv_base w)))))).
  {
    intros v Hv.
    rewrite join_cfg_conv_iterated_out_defs_eq_seed_of_stable by exact HStable.
    exact (HJoinReq v Hv).
  }
  change (verify_ir_flow_lite
    (join_cfg_to_flow_case
      (reach_cfg (join_cfg_conv_to_reachability_witness w))) = None).
  apply join_cfg_reach_accepts_of_join_step_in_defs.
  - exact HCfgPreds.
  - exact HOrder.
  - exact HReach.
  - exact HPredMap.
  - exact HIterJoinDefs.
  - exact HBase.
  - exact HLeft.
  - exact HRight.
  - exact HIterJoinReq.
Qed.

End RRVerifyIrCfgConvergenceSubset.
