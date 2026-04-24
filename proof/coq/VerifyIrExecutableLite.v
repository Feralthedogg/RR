Require Import CfgSmallStep.
Require Import GraphLicmSound.
Require Import RRWellFormed.
Require Import VerifyIrLite.
Require Import VerifyIrStructLite.
Require Import VerifyIrFlowLite.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRCfgSmallStep.
Import RRGraphLicmSound.
Import RRVerifyIrLite.
Import RRVerifyIrStructLite.
Import RRVerifyIrFlowLite.

Module RRVerifyIrExecutableLite.

Inductive verify_error_executable_lite : Type :=
| EBadBlock : verify_error_executable_lite
| EBadValue : verify_error_executable_lite
| EInvalidIntrinsicArity : verify_error_executable_lite
| EBadTerminator : verify_error_executable_lite
| EExecBase : verify_error_flow_lite -> verify_error_executable_lite
| EReachablePhiExec : verify_error_executable_lite.

Record verify_ir_executable_lite_case : Type := {
  exec_base : verify_ir_flow_lite_case;
  exec_entry_block_valid : bool;
  exec_body_head_block_valid : bool;
  exec_value_ids_valid : bool;
  exec_intrinsic_arities_valid : bool;
  exec_block_ids_valid : bool;
  exec_block_targets_valid : bool;
  exec_bad_terminator_free : bool;
  exec_emittable_reachable_phi : bool;
}.

Definition verify_ir_executable_lite
    (c : verify_ir_executable_lite_case)
    : option verify_error_executable_lite :=
  if negb (exec_entry_block_valid c) then
    Some EBadBlock
  else if negb (exec_body_head_block_valid c) then
    Some EBadBlock
  else if negb (exec_value_ids_valid c) then
    Some EBadValue
  else if negb (exec_intrinsic_arities_valid c) then
    Some EInvalidIntrinsicArity
  else if negb (exec_block_ids_valid c) then
    Some EBadBlock
  else if negb (exec_block_targets_valid c) then
    Some EBadBlock
  else if negb (exec_bad_terminator_free c) then
    Some EBadTerminator
  else
    option_map EExecBase (verify_ir_flow_lite (exec_base c)).

Definition verify_emittable_executable_lite
    (c : verify_ir_executable_lite_case)
    : option verify_error_executable_lite :=
  match verify_ir_executable_lite c with
  | Some err => Some err
  | None =>
      if exec_emittable_reachable_phi c then
        Some EReachablePhiExec
      else
        None
  end.

Lemma verify_ir_executable_lite_none_implies_flow_clean :
  forall c,
    verify_ir_executable_lite c = None ->
    verify_ir_flow_lite (exec_base c) = None.
Proof.
  intros c H.
  unfold verify_ir_executable_lite in H.
  destruct (exec_entry_block_valid c) eqn:HE1; try discriminate H.
  destruct (exec_body_head_block_valid c) eqn:HE2; try discriminate H.
  destruct (exec_value_ids_valid c) eqn:HV; try discriminate H.
  destruct (exec_intrinsic_arities_valid c) eqn:HI; try discriminate H.
  destruct (exec_block_ids_valid c) eqn:HB1; try discriminate H.
  destruct (exec_block_targets_valid c) eqn:HB2; try discriminate H.
  destruct (exec_bad_terminator_free c) eqn:HT; try discriminate H.
  destruct (verify_ir_flow_lite (exec_base c)) eqn:HF; try discriminate H.
  reflexivity.
Qed.

Lemma verify_ir_executable_lite_ok_zero_trip_sound :
  forall c entry locals,
    verify_ir_executable_lite c = None ->
    RRCompilerWellFormed.wf (ir_base (struct_base (flow_base (exec_base c)))) ->
    safe_candidate (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base (exec_base c))))) ->
    result_of
      (run_original_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base (exec_base c))))))
        false
        entry
        locals) =
    result_of
      (run_hoisted_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base (exec_base c))))))
        false
        entry
        locals).
Proof.
  intros c entry locals Hverify Hwf Hsafe.
  eapply verify_ir_flow_lite_ok_zero_trip_sound; eauto.
  exact (verify_ir_executable_lite_none_implies_flow_clean c Hverify).
Qed.

Lemma verify_ir_executable_lite_ok_one_trip_sound :
  forall c entry locals,
    verify_ir_executable_lite c = None ->
    RRCompilerWellFormed.wf (ir_base (struct_base (flow_base (exec_base c)))) ->
    safe_candidate (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base (exec_base c))))) ->
    result_of
      (run_original_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base (exec_base c))))))
        true
        entry
        locals) =
    result_of
      (run_hoisted_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base (exec_base c))))))
        true
        entry
        locals).
Proof.
  intros c entry locals Hverify Hwf Hsafe.
  eapply verify_ir_flow_lite_ok_one_trip_sound; eauto.
  exact (verify_ir_executable_lite_none_implies_flow_clean c Hverify).
Qed.

Definition example_executable_base : verify_ir_flow_lite_case :=
  {| flow_base := example_flow_base;
     flow_blocks_case := [] |}.

Definition example_bad_entry_case : verify_ir_executable_lite_case :=
  {| exec_base := example_executable_base;
     exec_entry_block_valid := false;
     exec_body_head_block_valid := true;
     exec_value_ids_valid := true;
     exec_intrinsic_arities_valid := true;
     exec_block_ids_valid := true;
     exec_block_targets_valid := true;
     exec_bad_terminator_free := true;
     exec_emittable_reachable_phi := false |}.

Definition example_bad_value_case : verify_ir_executable_lite_case :=
  {| exec_base := example_executable_base;
     exec_entry_block_valid := true;
     exec_body_head_block_valid := true;
     exec_value_ids_valid := false;
     exec_intrinsic_arities_valid := true;
     exec_block_ids_valid := true;
     exec_block_targets_valid := true;
     exec_bad_terminator_free := true;
     exec_emittable_reachable_phi := false |}.

Definition example_intrinsic_arity_case : verify_ir_executable_lite_case :=
  {| exec_base := example_executable_base;
     exec_entry_block_valid := true;
     exec_body_head_block_valid := true;
     exec_value_ids_valid := true;
     exec_intrinsic_arities_valid := false;
     exec_block_ids_valid := true;
     exec_block_targets_valid := true;
     exec_bad_terminator_free := true;
     exec_emittable_reachable_phi := false |}.

Definition example_reachable_phi_executable_case : verify_ir_executable_lite_case :=
  {| exec_base := example_executable_base;
     exec_entry_block_valid := true;
     exec_body_head_block_valid := true;
     exec_value_ids_valid := true;
     exec_intrinsic_arities_valid := true;
     exec_block_ids_valid := true;
     exec_block_targets_valid := true;
     exec_bad_terminator_free := true;
     exec_emittable_reachable_phi := true |}.

Definition example_executable_clean_case : verify_ir_executable_lite_case :=
  {| exec_base := example_executable_base;
     exec_entry_block_valid := true;
     exec_body_head_block_valid := true;
     exec_value_ids_valid := true;
     exec_intrinsic_arities_valid := true;
     exec_block_ids_valid := true;
     exec_block_targets_valid := true;
     exec_bad_terminator_free := true;
     exec_emittable_reachable_phi := false |}.

Lemma example_bad_entry_case_rejects :
  verify_ir_executable_lite example_bad_entry_case = Some EBadBlock.
Proof.
  reflexivity.
Qed.

Lemma example_bad_value_case_rejects :
  verify_ir_executable_lite example_bad_value_case = Some EBadValue.
Proof.
  reflexivity.
Qed.

Lemma example_intrinsic_arity_case_rejects :
  verify_ir_executable_lite example_intrinsic_arity_case = Some EInvalidIntrinsicArity.
Proof.
  reflexivity.
Qed.

Lemma example_reachable_phi_executable_case_rejects :
  verify_emittable_executable_lite example_reachable_phi_executable_case = Some EReachablePhiExec.
Proof.
  reflexivity.
Qed.

Lemma example_executable_clean_case_accepts :
  verify_emittable_executable_lite example_executable_clean_case = None.
Proof.
  reflexivity.
Qed.

End RRVerifyIrExecutableLite.
