Require Import CfgSmallStep.
Require Import GraphLicmSound.
Require Import RRWellFormed.
Require Import VerifyIrLite.
Require Import VerifyIrStructLite.
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

Module RRVerifyIrFlowLite.

Inductive verify_error_flow_lite : Type :=
| EFlowBase : verify_error_struct_lite -> verify_error_flow_lite
| EUseBeforeDef : string -> verify_error_flow_lite.

Record flow_block_case : Type := {
  flow_defined : list string;
  flow_required : list string;
}.

Fixpoint first_missing_var (defined required : list string) : option string :=
  match required with
  | [] => None
  | v :: rest =>
      if in_dec String.string_dec v defined then
        first_missing_var defined rest
      else
        Some v
  end.

Definition verify_flow_block (b : flow_block_case) : option verify_error_flow_lite :=
  option_map EUseBeforeDef (first_missing_var (flow_defined b) (flow_required b)).

Fixpoint verify_flow_blocks (blocks : list flow_block_case)
    : option verify_error_flow_lite :=
  match blocks with
  | [] => None
  | b :: rest =>
      match verify_flow_block b with
      | Some err => Some err
      | None => verify_flow_blocks rest
      end
  end.

Record verify_ir_flow_lite_case : Type := {
  flow_base : verify_ir_struct_lite_case;
  flow_blocks_case : list flow_block_case;
}.

Definition verify_ir_flow_lite
    (c : verify_ir_flow_lite_case)
    : option verify_error_flow_lite :=
  match verify_flow_blocks (flow_blocks_case c) with
  | Some err => Some err
  | None => option_map EFlowBase (verify_ir_struct_lite (flow_base c))
  end.

Lemma verify_ir_flow_lite_none_implies_struct_clean :
  forall c,
    verify_ir_flow_lite c = None ->
    verify_ir_struct_lite (flow_base c) = None.
Proof.
  intros c H.
  unfold verify_ir_flow_lite in H.
  destruct (verify_flow_blocks (flow_blocks_case c)) as [err|] eqn:HF.
  - discriminate H.
  - destruct (verify_ir_struct_lite (flow_base c)) as [base_err|] eqn:HB.
    + discriminate H.
    + reflexivity.
Qed.

Lemma verify_ir_flow_lite_ok_zero_trip_sound :
  forall c entry locals,
    verify_ir_flow_lite c = None ->
    RRCompilerWellFormed.wf (ir_base (struct_base (flow_base c))) ->
    safe_candidate (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base c)))) ->
    result_of
      (run_original_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base c)))))
        false
        entry
        locals) =
    result_of
      (run_hoisted_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base c)))))
        false
        entry
        locals).
Proof.
  intros c entry locals Hverify Hwf Hsafe.
  eapply verify_ir_struct_lite_ok_zero_trip_sound; eauto.
  exact (verify_ir_flow_lite_none_implies_struct_clean c Hverify).
Qed.

Lemma verify_ir_flow_lite_ok_one_trip_sound :
  forall c entry locals,
    verify_ir_flow_lite c = None ->
    RRCompilerWellFormed.wf (ir_base (struct_base (flow_base c))) ->
    safe_candidate (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base c)))) ->
    result_of
      (run_original_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base c)))))
        true
        entry
        locals) =
    result_of
      (run_hoisted_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base (flow_base c)))))
        true
        entry
        locals).
Proof.
  intros c entry locals Hverify Hwf Hsafe.
  eapply verify_ir_struct_lite_ok_one_trip_sound; eauto.
  exact (verify_ir_flow_lite_none_implies_struct_clean c Hverify).
Qed.

Definition example_flow_base : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := true;
     struct_body_head_direct_entry_edge := true;
     struct_entry_prologue_safe := true;
     struct_body_head_not_unreachable := true;
     struct_entry_has_no_preds := true;
     struct_entry_not_unreachable := true;
     struct_branch_targets_distinct := true;
     struct_loop_header_split_valid := true;
     struct_loop_header_preds_valid := true;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_use_before_def_case : verify_ir_flow_lite_case :=
  {| flow_base := example_flow_base;
     flow_blocks_case := [{| flow_defined := ["y"]; flow_required := ["x"; "y"] |}] |}.

Definition example_flow_clean_case : verify_ir_flow_lite_case :=
  {| flow_base := example_flow_base;
     flow_blocks_case :=
       [ {| flow_defined := ["x"; "y"]; flow_required := ["x"] |}
       ; {| flow_defined := ["x"; "y"]; flow_required := ["y"] |}
       ] |}.

Lemma example_use_before_def_case_rejects :
  verify_ir_flow_lite example_use_before_def_case = Some (EUseBeforeDef "x").
Proof.
  reflexivity.
Qed.

Lemma example_flow_clean_case_accepts :
  verify_ir_flow_lite example_flow_clean_case = None.
Proof.
  reflexivity.
Qed.

End RRVerifyIrFlowLite.
