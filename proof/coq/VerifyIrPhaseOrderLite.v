Require Import VerifyIrRustErrorLite.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRVerifyIrRustErrorLite.

Module RRVerifyIrPhaseOrderLite.

Record verify_ir_phase_order_lite_case : Type := {
  phase_entry_block_valid : bool;
  phase_body_head_block_valid : bool;
  phase_body_head_reachable : bool;
  phase_entry_not_unreachable : bool;
  phase_body_head_not_unreachable : bool;
  phase_value_error : option verify_error_rust_lite;
  phase_non_phi_cycle_free : bool;
  phase_block_error : option verify_error_rust_lite;
  phase_entry_has_no_preds : bool;
  phase_phi_error : option verify_error_rust_lite;
  phase_flow_error : option verify_error_rust_lite;
  phase_reachable_phi : bool;
}.

Definition verify_ir_phase_order_lite
    (c : verify_ir_phase_order_lite_case)
    : option verify_error_rust_lite :=
  if negb (phase_entry_block_valid c) then
    Some RBadBlock
  else if negb (phase_body_head_block_valid c) then
    Some RBadBlock
  else if negb (phase_body_head_reachable c) then
    Some RInvalidBodyHead
  else if negb (phase_entry_not_unreachable c) then
    Some RInvalidEntryTerminator
  else if negb (phase_body_head_not_unreachable c) then
    Some RInvalidBodyHeadTerminator
  else
    match phase_value_error c with
    | Some err => Some err
    | None =>
        if negb (phase_non_phi_cycle_free c) then
          Some RNonPhiValueCycle
        else
          match phase_block_error c with
          | Some err => Some err
          | None =>
              if negb (phase_entry_has_no_preds c) then
                Some RInvalidEntryPredecessor
              else
                match phase_phi_error c with
                | Some err => Some err
                | None => phase_flow_error c
                end
          end
    end.

Definition verify_emittable_phase_order_lite
    (c : verify_ir_phase_order_lite_case)
    : option verify_error_rust_lite :=
  match verify_ir_phase_order_lite c with
  | Some err => Some err
  | None =>
      if phase_reachable_phi c then
        Some RReachablePhi
      else
        None
  end.

Definition example_entry_dominates_value_phase : verify_ir_phase_order_lite_case :=
  {| phase_entry_block_valid := false;
     phase_body_head_block_valid := true;
     phase_body_head_reachable := true;
     phase_entry_not_unreachable := true;
     phase_body_head_not_unreachable := true;
     phase_value_error := Some RInvalidParamIndex;
     phase_non_phi_cycle_free := true;
     phase_block_error := None;
     phase_entry_has_no_preds := true;
     phase_phi_error := None;
     phase_flow_error := None;
     phase_reachable_phi := false |}.

Definition example_value_phase_dominates_block_phase : verify_ir_phase_order_lite_case :=
  {| phase_entry_block_valid := true;
     phase_body_head_block_valid := true;
     phase_body_head_reachable := true;
     phase_entry_not_unreachable := true;
     phase_body_head_not_unreachable := true;
     phase_value_error := Some RInvalidIntrinsicArity;
     phase_non_phi_cycle_free := true;
     phase_block_error := Some RBadBlock;
     phase_entry_has_no_preds := true;
     phase_phi_error := None;
     phase_flow_error := None;
     phase_reachable_phi := false |}.

Definition example_block_phase_dominates_phi_phase : verify_ir_phase_order_lite_case :=
  {| phase_entry_block_valid := true;
     phase_body_head_block_valid := true;
     phase_body_head_reachable := true;
     phase_entry_not_unreachable := true;
     phase_body_head_not_unreachable := true;
     phase_value_error := None;
     phase_non_phi_cycle_free := true;
     phase_block_error := Some RBadBlock;
     phase_entry_has_no_preds := true;
     phase_phi_error := Some RInvalidPhiArgs;
     phase_flow_error := None;
     phase_reachable_phi := false |}.

Definition example_phi_phase_dominates_flow_phase : verify_ir_phase_order_lite_case :=
  {| phase_entry_block_valid := true;
     phase_body_head_block_valid := true;
     phase_body_head_reachable := true;
     phase_entry_not_unreachable := true;
     phase_body_head_not_unreachable := true;
     phase_value_error := None;
     phase_non_phi_cycle_free := true;
     phase_block_error := None;
     phase_entry_has_no_preds := true;
     phase_phi_error := Some RInvalidPhiEdgeValue;
     phase_flow_error := Some RUseBeforeDef;
     phase_reachable_phi := false |}.

Definition example_flow_phase_dominates_reachable_phi : verify_ir_phase_order_lite_case :=
  {| phase_entry_block_valid := true;
     phase_body_head_block_valid := true;
     phase_body_head_reachable := true;
     phase_entry_not_unreachable := true;
     phase_body_head_not_unreachable := true;
     phase_value_error := None;
     phase_non_phi_cycle_free := true;
     phase_block_error := None;
     phase_entry_has_no_preds := true;
     phase_phi_error := None;
     phase_flow_error := Some RUndefinedVar;
     phase_reachable_phi := true |}.

Definition example_phase_order_clean : verify_ir_phase_order_lite_case :=
  {| phase_entry_block_valid := true;
     phase_body_head_block_valid := true;
     phase_body_head_reachable := true;
     phase_entry_not_unreachable := true;
     phase_body_head_not_unreachable := true;
     phase_value_error := None;
     phase_non_phi_cycle_free := true;
     phase_block_error := None;
     phase_entry_has_no_preds := true;
     phase_phi_error := None;
     phase_flow_error := None;
     phase_reachable_phi := false |}.

Lemma example_entry_dominates_value_phase_rejects :
  verify_ir_phase_order_lite example_entry_dominates_value_phase = Some RBadBlock.
Proof.
  reflexivity.
Qed.

Lemma example_value_phase_dominates_block_phase_rejects :
  verify_ir_phase_order_lite example_value_phase_dominates_block_phase =
    Some RInvalidIntrinsicArity.
Proof.
  reflexivity.
Qed.

Lemma example_block_phase_dominates_phi_phase_rejects :
  verify_ir_phase_order_lite example_block_phase_dominates_phi_phase = Some RBadBlock.
Proof.
  reflexivity.
Qed.

Lemma example_phi_phase_dominates_flow_phase_rejects :
  verify_ir_phase_order_lite example_phi_phase_dominates_flow_phase =
    Some RInvalidPhiEdgeValue.
Proof.
  reflexivity.
Qed.

Lemma example_flow_phase_dominates_reachable_phi_rejects :
  verify_emittable_phase_order_lite example_flow_phase_dominates_reachable_phi =
    Some RUndefinedVar.
Proof.
  reflexivity.
Qed.

Lemma example_phase_order_clean_accepts :
  verify_emittable_phase_order_lite example_phase_order_clean = None.
Proof.
  reflexivity.
Qed.

End RRVerifyIrPhaseOrderLite.
