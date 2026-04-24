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

Module RRVerifyIrCfgOrderWorklistSubset.

Record join_cfg_order_worklist_witness_lite : Type := {
  order_base : join_cfg_worklist_witness_lite;
}.

Definition join_cfg_order_next_left_out_defs
    (w : join_cfg_order_worklist_witness_lite) : list string :=
  step_out_defs
    (reach_entry (conv_base (work_base (order_base w))))
    (reach_entry_defs (conv_base (work_base (order_base w))))
    (reach_reachable (conv_base (work_base (order_base w))))
    (reach_preds (conv_base (work_base (order_base w))))
    (conv_assigned (work_base (order_base w)))
    (conv_seed (work_base (order_base w)))
    (actual_block_id (cfg_left (reach_cfg (conv_base (work_base (order_base w)))))).

Definition join_cfg_order_next_right_out_defs
    (w : join_cfg_order_worklist_witness_lite) : list string :=
  step_out_defs
    (reach_entry (conv_base (work_base (order_base w))))
    (reach_entry_defs (conv_base (work_base (order_base w))))
    (reach_reachable (conv_base (work_base (order_base w))))
    (reach_preds (conv_base (work_base (order_base w))))
    (conv_assigned (work_base (order_base w)))
    (conv_seed (work_base (order_base w)))
    (actual_block_id (cfg_right (reach_cfg (conv_base (work_base (order_base w)))))).

Definition join_cfg_order_left_changed (w : join_cfg_order_worklist_witness_lite) : bool :=
  if list_eq_dec String.string_dec
      (join_cfg_order_next_left_out_defs w)
      (conv_seed (work_base (order_base w))
        (actual_block_id (cfg_left (reach_cfg (conv_base (work_base (order_base w))))))) then
    false
  else
    true.

Definition join_cfg_order_right_changed (w : join_cfg_order_worklist_witness_lite) : bool :=
  if list_eq_dec String.string_dec
      (join_cfg_order_next_right_out_defs w)
      (conv_seed (work_base (order_base w))
        (actual_block_id (cfg_right (reach_cfg (conv_base (work_base (order_base w))))))) then
    false
  else
    true.

Definition join_cfg_order_changed_flags (w : join_cfg_order_worklist_witness_lite) : list bool :=
  [join_cfg_order_left_changed w;
   join_cfg_order_right_changed w;
   join_cfg_worklist_changed (order_base w)].

Definition join_cfg_order_any_changed (w : join_cfg_order_worklist_witness_lite) : bool :=
  existsb (fun b => b) (join_cfg_order_changed_flags w).

Lemma join_cfg_order_next_left_out_defs_eq_seed_of_stable :
  forall w,
    join_cfg_conv_seed_stable (work_base (order_base w)) ->
    join_cfg_order_next_left_out_defs w =
      conv_seed (work_base (order_base w))
        (actual_block_id (cfg_left (reach_cfg (conv_base (work_base (order_base w)))))).
Proof.
  intros w Hstable.
  unfold join_cfg_conv_seed_stable, out_map_stable in Hstable.
  pose proof (f_equal
    (fun m => m (actual_block_id (cfg_left (reach_cfg (conv_base (work_base (order_base w)))))))
    Hstable) as Hleft.
  unfold join_cfg_order_next_left_out_defs in Hleft.
  simpl in Hleft.
  exact Hleft.
Qed.

Lemma join_cfg_order_next_right_out_defs_eq_seed_of_stable :
  forall w,
    join_cfg_conv_seed_stable (work_base (order_base w)) ->
    join_cfg_order_next_right_out_defs w =
      conv_seed (work_base (order_base w))
        (actual_block_id (cfg_right (reach_cfg (conv_base (work_base (order_base w)))))).
Proof.
  intros w Hstable.
  unfold join_cfg_conv_seed_stable, out_map_stable in Hstable.
  pose proof (f_equal
    (fun m => m (actual_block_id (cfg_right (reach_cfg (conv_base (work_base (order_base w)))))))
    Hstable) as Hright.
  unfold join_cfg_order_next_right_out_defs in Hright.
  simpl in Hright.
  exact Hright.
Qed.

Lemma join_cfg_order_left_changed_eq_false_of_stable :
  forall w,
    join_cfg_conv_seed_stable (work_base (order_base w)) ->
    join_cfg_order_left_changed w = false.
Proof.
  intros w Hstable.
  unfold join_cfg_order_left_changed.
  rewrite join_cfg_order_next_left_out_defs_eq_seed_of_stable by exact Hstable.
  destruct (list_eq_dec String.string_dec
    (conv_seed (work_base (order_base w))
      (actual_block_id (cfg_left (reach_cfg (conv_base (work_base (order_base w)))))))
    (conv_seed (work_base (order_base w))
      (actual_block_id (cfg_left (reach_cfg (conv_base (work_base (order_base w)))))))).
  - reflexivity.
  - contradiction n. reflexivity.
Qed.

Lemma join_cfg_order_right_changed_eq_false_of_stable :
  forall w,
    join_cfg_conv_seed_stable (work_base (order_base w)) ->
    join_cfg_order_right_changed w = false.
Proof.
  intros w Hstable.
  unfold join_cfg_order_right_changed.
  rewrite join_cfg_order_next_right_out_defs_eq_seed_of_stable by exact Hstable.
  destruct (list_eq_dec String.string_dec
    (conv_seed (work_base (order_base w))
      (actual_block_id (cfg_right (reach_cfg (conv_base (work_base (order_base w)))))))
    (conv_seed (work_base (order_base w))
      (actual_block_id (cfg_right (reach_cfg (conv_base (work_base (order_base w)))))))).
  - reflexivity.
  - contradiction n. reflexivity.
Qed.

Lemma join_cfg_order_any_changed_eq_false_of_stable :
  forall w,
    join_cfg_conv_seed_stable (work_base (order_base w)) ->
    join_cfg_order_any_changed w = false.
Proof.
  intros w Hstable.
  unfold join_cfg_order_any_changed, join_cfg_order_changed_flags.
  rewrite join_cfg_order_left_changed_eq_false_of_stable by exact Hstable.
  rewrite join_cfg_order_right_changed_eq_false_of_stable by exact Hstable.
  rewrite join_cfg_worklist_changed_eq_false_of_stable by exact Hstable.
  reflexivity.
Qed.

Lemma join_cfg_order_accepts_and_reports_no_change_of_stable_seed_step_in_defs :
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
    verify_ir_flow_lite (join_cfg_to_flow_case (reach_cfg (conv_base (work_base (order_base w))))) = None /\
    join_cfg_order_any_changed w = false.
Proof.
  intros w HCfgPreds HOrder HReach HPredMap HStable HSeedJoinDefs HBase HLeft HRight HJoinReq.
  split.
  - apply join_cfg_worklist_accepts_and_reports_no_join_change_of_stable_seed_step_in_defs;
      assumption.
  - apply join_cfg_order_any_changed_eq_false_of_stable.
    exact HStable.
Qed.

End RRVerifyIrCfgOrderWorklistSubset.
