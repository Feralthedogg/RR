Require Import VerifyIrBlockFlowSubset.
Require Import VerifyIrConsumerGraphSubset.
Require Import VerifyIrBlockRecordSubset.
Require Import VerifyIrFlowLite.
Require Import VerifyIrMustDefSubset.
Require Import VerifyIrValueFullRecordSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.

Import ListNotations.
Open Scope string_scope.
Import RRVerifyIrBlockFlowSubset.
Import RRVerifyIrConsumerGraphSubset.
Import RRVerifyIrBlockRecordSubset.
Import RRVerifyIrFlowLite.
Import RRVerifyIrMustDefSubset.
Import RRVerifyIrValueFullRecordSubset.

Module RRVerifyIrBlockMustDefSubset.

Definition singleton_eval_block (bid root : consumer_node_id) : actual_block_record_lite :=
  {| actual_block_id := bid;
     actual_block_instrs := [IREval root SSource];
     actual_block_term := TRLUnreachable |}.

Lemma singleton_eval_block_rejects_without_defs :
  verify_flow_block
    (flow_case_of_actual_block example_actual_value_full_table []
      (singleton_eval_block 20%nat 3%nat)) = Some (EUseBeforeDef "x").
Proof.
  reflexivity.
Qed.

Lemma singleton_eval_block_accepts_x_of_must_def :
  forall defs,
    In "x" defs ->
    verify_flow_block
      (flow_case_of_actual_block example_actual_value_full_table defs
        (singleton_eval_block 20%nat 3%nat)) = None.
Proof.
  intros defs HMem.
  assert (Hreq :
    block_required_vars example_actual_value_full_table defs
      (singleton_eval_block 20%nat 3%nat) = []).
  { unfold block_required_vars, singleton_eval_block, step_instr_flow, missing_vars.
    simpl.
    destruct (in_dec String.string_dec "x" defs).
    - reflexivity.
    - contradiction.
  }
  unfold verify_flow_block, flow_case_of_actual_block.
  simpl.
  rewrite Hreq.
  reflexivity.
Qed.

Lemma example_join_must_def_singleton_eval_clean :
  verify_flow_block
    (flow_case_of_actual_block example_actual_value_full_table
      (in_defs_from_preds 0%nat [] example_preds example_out_defs 3%nat)
      (singleton_eval_block 20%nat 3%nat)) = None.
Proof.
  apply singleton_eval_block_accepts_x_of_must_def.
  exact example_join_contains_x.
Qed.

End RRVerifyIrBlockMustDefSubset.
