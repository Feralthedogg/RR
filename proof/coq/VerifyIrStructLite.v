Require Import ReducedFnIR.
Require Import CfgSmallStep.
Require Import GraphLicmSound.
Require Import RRWellFormed.
Require Import VerifyIrLite.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRReducedFnIR.
Import RRCfgSmallStep.
Import RRGraphLicmSound.
Import RRVerifyIrLite.

Module RRVerifyIrStructLite.

Inductive verify_error_struct_lite : Type :=
| EBase : verify_error_lite -> verify_error_struct_lite
| EInvalidBodyHead : verify_error_struct_lite
| EInvalidBodyHeadEntryEdge : verify_error_struct_lite
| EInvalidEntryPrologue : verify_error_struct_lite
| EInvalidBodyHeadTerminator : verify_error_struct_lite
| EInvalidEntryPredecessor : verify_error_struct_lite
| EInvalidEntryTerminator : verify_error_struct_lite
| EInvalidBranchTargets : verify_error_struct_lite
| EInvalidLoopHeaderSplit : verify_error_struct_lite
| EInvalidLoopHeaderPredecessors : verify_error_struct_lite
| EInvalidPhiPlacement : verify_error_struct_lite
| EInvalidPhiPredecessorAliases : verify_error_struct_lite
| EInvalidPhiEdgeValue : verify_error_struct_lite
| EMissingPhiBlock : verify_error_struct_lite
| ENonPhiCarriesPhiBlock : verify_error_struct_lite
| EInvalidPhiOwnerBlock : verify_error_struct_lite
| EInvalidParamIndex : verify_error_struct_lite
| EInvalidCallArgNames : verify_error_struct_lite
| ESelfReferentialValue : verify_error_struct_lite
| ENonPhiValueCycle : verify_error_struct_lite.

Record value_struct_case : Type := {
  svc_is_phi : bool;
  svc_phi_block : option block_id;
  svc_owner_block_valid : bool;
  svc_owner_block_has_preds : bool;
  svc_owner_block_has_distinct_preds : bool;
  svc_phi_args_edge_available : bool;
  svc_param_index_valid : bool;
  svc_call_names_valid : bool;
  svc_self_reference_free : bool;
  svc_non_phi_acyclic : bool;
}.

Definition verify_value_struct (v : value_struct_case) : option verify_error_struct_lite :=
  if negb (svc_param_index_valid v) then
    Some EInvalidParamIndex
  else if negb (svc_call_names_valid v) then
    Some EInvalidCallArgNames
  else if negb (svc_self_reference_free v) then
    Some ESelfReferentialValue
  else if negb (svc_non_phi_acyclic v) then
    Some ENonPhiValueCycle
  else
    match svc_is_phi v, svc_phi_block v with
    | true, None => Some EMissingPhiBlock
    | false, Some _ => Some ENonPhiCarriesPhiBlock
    | true, Some _ =>
        if negb (svc_owner_block_valid v) then Some EInvalidPhiOwnerBlock
        else if negb (svc_owner_block_has_preds v) then Some EInvalidPhiPlacement
        else if negb (svc_owner_block_has_distinct_preds v) then Some EInvalidPhiPredecessorAliases
        else if negb (svc_phi_args_edge_available v) then Some EInvalidPhiEdgeValue
        else None
    | false, None => None
    end.

Fixpoint verify_value_structs (vals : list value_struct_case)
    : option verify_error_struct_lite :=
  match vals with
  | [] => None
  | v :: rest =>
      match verify_value_struct v with
      | Some err => Some err
      | None => verify_value_structs rest
      end
  end.

Record verify_ir_struct_lite_case : Type := {
  struct_base : verify_ir_lite_case;
  struct_body_head_reachable : bool;
  struct_body_head_direct_entry_edge : bool;
  struct_entry_prologue_safe : bool;
  struct_body_head_not_unreachable : bool;
  struct_entry_has_no_preds : bool;
  struct_entry_not_unreachable : bool;
  struct_branch_targets_distinct : bool;
  struct_loop_header_split_valid : bool;
  struct_loop_header_preds_valid : bool;
  struct_values : list value_struct_case;
}.

Definition verify_ir_struct_lite
    (c : verify_ir_struct_lite_case)
    : option verify_error_struct_lite :=
  if negb (struct_body_head_reachable c) then
    Some EInvalidBodyHead
  else if negb (struct_body_head_direct_entry_edge c) then
    Some EInvalidBodyHeadEntryEdge
  else if negb (struct_entry_prologue_safe c) then
    Some EInvalidEntryPrologue
  else if negb (struct_body_head_not_unreachable c) then
    Some EInvalidBodyHeadTerminator
  else if negb (struct_entry_has_no_preds c) then
    Some EInvalidEntryPredecessor
  else if negb (struct_entry_not_unreachable c) then
    Some EInvalidEntryTerminator
  else if negb (struct_branch_targets_distinct c) then
    Some EInvalidBranchTargets
  else if negb (struct_loop_header_split_valid c) then
    Some EInvalidLoopHeaderSplit
  else if negb (struct_loop_header_preds_valid c) then
    Some EInvalidLoopHeaderPredecessors
  else
    match verify_value_structs (struct_values c) with
    | Some err => Some err
    | None =>
        option_map EBase (verify_ir_lite (struct_base c))
    end.

Lemma verify_ir_struct_lite_none_implies_base_clean :
  forall c,
    verify_ir_struct_lite c = None ->
    verify_ir_lite (struct_base c) = None.
Proof.
  intros c H.
  unfold verify_ir_struct_lite in H.
  destruct (struct_body_head_reachable c) eqn:HBH; try discriminate H.
  destruct (struct_body_head_direct_entry_edge c) eqn:HBHE; try discriminate H.
  destruct (struct_entry_prologue_safe c) eqn:HPRO; try discriminate H.
  destruct (struct_body_head_not_unreachable c) eqn:HBHU; try discriminate H.
  destruct (struct_entry_has_no_preds c) eqn:HE; try discriminate H.
  destruct (struct_entry_not_unreachable c) eqn:HET; try discriminate H.
  destruct (struct_branch_targets_distinct c) eqn:HBT; try discriminate H.
  destruct (struct_loop_header_split_valid c) eqn:HLS; try discriminate H.
  destruct (struct_loop_header_preds_valid c) eqn:HLP; try discriminate H.
  destruct (verify_value_structs (struct_values c)) as [err|] eqn:HS; try discriminate H.
  destruct (verify_ir_lite (struct_base c)) as [base_err|] eqn:HB; try discriminate H.
  reflexivity.
Qed.

Lemma verify_ir_struct_lite_ok_zero_trip_sound :
  forall c entry locals,
    verify_ir_struct_lite c = None ->
    RRCompilerWellFormed.wf (ir_base (struct_base c)) ->
    safe_candidate (RRCompilerWellFormed.rr_licm (ir_base (struct_base c))) ->
    result_of
      (run_original_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base c))))
        false
        entry
        locals) =
    result_of
      (run_hoisted_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base c))))
        false
        entry
        locals).
Proof.
  intros c entry locals Hverify Hwf Hsafe.
  eapply verify_ir_lite_ok_zero_trip_sound; eauto.
  exact (verify_ir_struct_lite_none_implies_base_clean c Hverify).
Qed.

Lemma verify_ir_struct_lite_ok_one_trip_sound :
  forall c entry locals,
    verify_ir_struct_lite c = None ->
    RRCompilerWellFormed.wf (ir_base (struct_base c)) ->
    safe_candidate (RRCompilerWellFormed.rr_licm (ir_base (struct_base c))) ->
    result_of
      (run_original_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base c))))
        true
        entry
        locals) =
    result_of
      (run_hoisted_machine
        (case_fnir (RRCompilerWellFormed.rr_licm (ir_base (struct_base c))))
        true
        entry
        locals).
Proof.
  intros c entry locals Hverify Hwf Hsafe.
  eapply verify_ir_lite_ok_one_trip_sound; eauto.
  exact (verify_ir_struct_lite_none_implies_base_clean c Hverify).
Qed.

Definition example_clean_struct_base : verify_ir_lite_case :=
  {| ir_base := RRCompilerWellFormed.example_rr_wf_case;
     ir_undefined_var := None;
     ir_phi_sources_valid := true;
     ir_reachable_phi := false |}.

Definition example_missing_phi_block_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := true; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_tagged_non_phi_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := false; svc_phi_block := Some BHeader; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_phi_owner_block_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := true; svc_phi_block := Some BHeader; svc_owner_block_valid := false; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_param_index_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := false; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_call_arg_names_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := false; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_self_referential_value_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := false; svc_non_phi_acyclic := true |}] |}.

Definition example_non_phi_value_cycle_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := false |}] |}.

Definition example_invalid_body_head_case : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := false;
     struct_body_head_direct_entry_edge := true;
     struct_entry_prologue_safe := true;
     struct_body_head_not_unreachable := true;
     struct_entry_has_no_preds := true;
     struct_entry_not_unreachable := true;
     struct_branch_targets_distinct := true;
     struct_loop_header_split_valid := true;
     struct_loop_header_preds_valid := true;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_body_head_entry_edge_case : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := true;
     struct_body_head_direct_entry_edge := false;
     struct_entry_prologue_safe := true;
     struct_body_head_not_unreachable := true;
     struct_entry_has_no_preds := true;
     struct_entry_not_unreachable := true;
     struct_branch_targets_distinct := true;
     struct_loop_header_split_valid := true;
     struct_loop_header_preds_valid := true;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_entry_prologue_case : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := true;
     struct_body_head_direct_entry_edge := true;
     struct_entry_prologue_safe := false;
     struct_body_head_not_unreachable := true;
     struct_entry_has_no_preds := true;
     struct_entry_not_unreachable := true;
     struct_branch_targets_distinct := true;
     struct_loop_header_split_valid := true;
     struct_loop_header_preds_valid := true;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_entry_pred_case : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := true;
     struct_body_head_direct_entry_edge := true;
     struct_entry_prologue_safe := true;
     struct_body_head_not_unreachable := true;
     struct_entry_has_no_preds := false;
     struct_entry_not_unreachable := true;
     struct_branch_targets_distinct := true;
     struct_loop_header_split_valid := true;
     struct_loop_header_preds_valid := true;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_phi_placement_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := true; svc_phi_block := Some BHeader; svc_owner_block_valid := true; svc_owner_block_has_preds := false; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_phi_predecessor_aliases_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := true; svc_phi_block := Some BHeader; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := false; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_phi_edge_value_case : verify_ir_struct_lite_case :=
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
     struct_values := [{| svc_is_phi := true; svc_phi_block := Some BHeader; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := false; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_entry_terminator_case : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := true;
     struct_body_head_direct_entry_edge := true;
     struct_entry_prologue_safe := true;
     struct_body_head_not_unreachable := true;
     struct_entry_has_no_preds := true;
     struct_entry_not_unreachable := false;
     struct_branch_targets_distinct := true;
     struct_loop_header_split_valid := true;
     struct_loop_header_preds_valid := true;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_branch_targets_case : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := true;
     struct_body_head_direct_entry_edge := true;
     struct_entry_prologue_safe := true;
     struct_body_head_not_unreachable := true;
     struct_entry_has_no_preds := true;
     struct_entry_not_unreachable := true;
     struct_branch_targets_distinct := false;
     struct_loop_header_split_valid := true;
     struct_loop_header_preds_valid := true;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_loop_header_split_case : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := true;
     struct_body_head_direct_entry_edge := true;
     struct_entry_prologue_safe := true;
     struct_body_head_not_unreachable := true;
     struct_entry_has_no_preds := true;
     struct_entry_not_unreachable := true;
     struct_branch_targets_distinct := true;
     struct_loop_header_split_valid := false;
     struct_loop_header_preds_valid := true;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_loop_header_preds_case : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := true;
     struct_body_head_direct_entry_edge := true;
     struct_entry_prologue_safe := true;
     struct_body_head_not_unreachable := true;
     struct_entry_has_no_preds := true;
     struct_entry_not_unreachable := true;
     struct_branch_targets_distinct := true;
     struct_loop_header_split_valid := true;
     struct_loop_header_preds_valid := false;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_invalid_body_head_terminator_case : verify_ir_struct_lite_case :=
  {| struct_base := example_clean_struct_base;
     struct_body_head_reachable := true;
     struct_body_head_direct_entry_edge := true;
     struct_entry_prologue_safe := true;
     struct_body_head_not_unreachable := false;
     struct_entry_has_no_preds := true;
     struct_entry_not_unreachable := true;
     struct_branch_targets_distinct := true;
     struct_loop_header_split_valid := true;
     struct_loop_header_preds_valid := true;
     struct_values := [{| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}] |}.

Definition example_struct_clean_case : verify_ir_struct_lite_case :=
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
     struct_values :=
       [ {| svc_is_phi := true; svc_phi_block := Some BHeader; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}
       ; {| svc_is_phi := false; svc_phi_block := None; svc_owner_block_valid := true; svc_owner_block_has_preds := true; svc_owner_block_has_distinct_preds := true; svc_phi_args_edge_available := true; svc_param_index_valid := true; svc_call_names_valid := true; svc_self_reference_free := true; svc_non_phi_acyclic := true |}
       ] |}.

Lemma example_missing_phi_block_case_rejects :
  verify_ir_struct_lite example_missing_phi_block_case = Some EMissingPhiBlock.
Proof.
  reflexivity.
Qed.

Lemma example_tagged_non_phi_case_rejects :
  verify_ir_struct_lite example_tagged_non_phi_case = Some ENonPhiCarriesPhiBlock.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_phi_owner_block_case_rejects :
  verify_ir_struct_lite example_invalid_phi_owner_block_case = Some EInvalidPhiOwnerBlock.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_param_index_case_rejects :
  verify_ir_struct_lite example_invalid_param_index_case = Some EInvalidParamIndex.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_call_arg_names_case_rejects :
  verify_ir_struct_lite example_invalid_call_arg_names_case = Some EInvalidCallArgNames.
Proof.
  reflexivity.
Qed.

Lemma example_self_referential_value_case_rejects :
  verify_ir_struct_lite example_self_referential_value_case = Some ESelfReferentialValue.
Proof.
  reflexivity.
Qed.

Lemma example_non_phi_value_cycle_case_rejects :
  verify_ir_struct_lite example_non_phi_value_cycle_case = Some ENonPhiValueCycle.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_body_head_case_rejects :
  verify_ir_struct_lite example_invalid_body_head_case = Some EInvalidBodyHead.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_body_head_entry_edge_case_rejects :
  verify_ir_struct_lite example_invalid_body_head_entry_edge_case = Some EInvalidBodyHeadEntryEdge.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_entry_prologue_case_rejects :
  verify_ir_struct_lite example_invalid_entry_prologue_case = Some EInvalidEntryPrologue.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_entry_pred_case_rejects :
  verify_ir_struct_lite example_invalid_entry_pred_case = Some EInvalidEntryPredecessor.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_phi_placement_case_rejects :
  verify_ir_struct_lite example_invalid_phi_placement_case = Some EInvalidPhiPlacement.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_phi_predecessor_aliases_case_rejects :
  verify_ir_struct_lite example_invalid_phi_predecessor_aliases_case = Some EInvalidPhiPredecessorAliases.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_phi_edge_value_case_rejects :
  verify_ir_struct_lite example_invalid_phi_edge_value_case = Some EInvalidPhiEdgeValue.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_entry_terminator_case_rejects :
  verify_ir_struct_lite example_invalid_entry_terminator_case = Some EInvalidEntryTerminator.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_branch_targets_case_rejects :
  verify_ir_struct_lite example_invalid_branch_targets_case = Some EInvalidBranchTargets.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_loop_header_split_case_rejects :
  verify_ir_struct_lite example_invalid_loop_header_split_case = Some EInvalidLoopHeaderSplit.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_loop_header_preds_case_rejects :
  verify_ir_struct_lite example_invalid_loop_header_preds_case = Some EInvalidLoopHeaderPredecessors.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_body_head_terminator_case_rejects :
  verify_ir_struct_lite example_invalid_body_head_terminator_case = Some EInvalidBodyHeadTerminator.
Proof.
  reflexivity.
Qed.

Lemma example_struct_clean_case_accepts :
  verify_ir_struct_lite example_struct_clean_case = None.
Proof.
  reflexivity.
Qed.

End RRVerifyIrStructLite.
