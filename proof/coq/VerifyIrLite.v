Require Import MirSubsetHoist.
Require Import CfgHoist.
Require Import ReducedFnIR.
Require Import CfgSmallStep.
Require Import LoopPredGraph.
Require Import GraphLicmSound.
Require Import RRWellFormed.
Require Import VerifyLite.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRMirSubsetHoist.
Import RRCfgHoist.
Import RRReducedFnIR.
Import RRCfgSmallStep.
Import RRLoopPredGraph.
Import RRGraphLicmSound.
Import RRVerifyLite.

Module RRVerifyIrLite.

Inductive verify_error_lite : Type :=
| EUndefinedVar : var -> verify_error_lite
| EInvalidPhiSource : verify_error_lite
| EReachablePhi : verify_error_lite.

Record verify_ir_lite_case : Type := {
  ir_base : RRCompilerWellFormed.rr_wf_case;
  ir_undefined_var : option var;
  ir_phi_sources_valid : bool;
  ir_reachable_phi : bool;
}.

Definition verify_ir_lite (c : verify_ir_lite_case) : option verify_error_lite :=
  match ir_undefined_var c with
  | Some x => Some (EUndefinedVar x)
  | None =>
      if negb (ir_phi_sources_valid c) then
        Some EInvalidPhiSource
      else if ir_reachable_phi c then
        Some EReachablePhi
      else
        None
  end.

Lemma verify_ir_lite_none_implies_clean :
  forall c,
    verify_ir_lite c = None ->
    ir_undefined_var c = None /\
    ir_phi_sources_valid c = true /\
    ir_reachable_phi c = false.
Proof.
  intros c H.
  destruct (ir_undefined_var c) as [x|] eqn:HU.
  - unfold verify_ir_lite in H.
    rewrite HU in H.
    simpl in H.
    discriminate H.
  - unfold verify_ir_lite in H.
    rewrite HU in H.
    destruct (ir_phi_sources_valid c) eqn:HP;
    destruct (ir_reachable_phi c) eqn:HR;
    simpl in H;
    try discriminate H;
    inversion H; subst; auto.
Qed.

Lemma verify_ir_lite_ok_zero_trip_sound :
  forall c entry locals,
    verify_ir_lite c = None ->
    RRCompilerWellFormed.wf (ir_base c) ->
    safe_candidate (RRCompilerWellFormed.rr_licm (ir_base c)) ->
    result_of (run_original_machine (case_fnir (RRCompilerWellFormed.rr_licm (ir_base c))) false entry locals) =
    result_of (run_hoisted_machine (case_fnir (RRCompilerWellFormed.rr_licm (ir_base c))) false entry locals).
Proof.
  intros c entry locals _ Hwf Hsafe.
  exact (RRCompilerWellFormed.rrwf_safe_candidate_zero_trip (ir_base c) entry locals Hwf Hsafe).
Qed.

Lemma verify_ir_lite_ok_one_trip_sound :
  forall c entry locals,
    verify_ir_lite c = None ->
    RRCompilerWellFormed.wf (ir_base c) ->
    safe_candidate (RRCompilerWellFormed.rr_licm (ir_base c)) ->
    result_of (run_original_machine (case_fnir (RRCompilerWellFormed.rr_licm (ir_base c))) true entry locals) =
    result_of (run_hoisted_machine (case_fnir (RRCompilerWellFormed.rr_licm (ir_base c))) true entry locals).
Proof.
  intros c entry locals _ Hwf Hsafe.
  exact (RRCompilerWellFormed.rrwf_safe_candidate_one_trip (ir_base c) entry locals Hwf Hsafe).
Qed.

Definition example_reachable_phi_case : verify_ir_lite_case :=
  {| ir_base := RRCompilerWellFormed.example_rr_wf_case;
     ir_undefined_var := None;
     ir_phi_sources_valid := true;
     ir_reachable_phi := true |}.

Definition example_invalid_phi_source_case : verify_ir_lite_case :=
  {| ir_base := RRCompilerWellFormed.example_rr_wf_case;
     ir_undefined_var := None;
     ir_phi_sources_valid := false;
     ir_reachable_phi := false |}.

Definition example_undefined_var_case : verify_ir_lite_case :=
  {| ir_base := RRCompilerWellFormed.example_rr_wf_case;
     ir_undefined_var := Some "time";
     ir_phi_sources_valid := true;
     ir_reachable_phi := false |}.

Lemma example_reachable_phi_case_rejects :
  verify_ir_lite example_reachable_phi_case = Some EReachablePhi.
Proof.
  reflexivity.
Qed.

Lemma example_invalid_phi_source_case_rejects :
  verify_ir_lite example_invalid_phi_source_case = Some EInvalidPhiSource.
Proof.
  reflexivity.
Qed.

Lemma example_undefined_var_case_rejects :
  verify_ir_lite example_undefined_var_case = Some (EUndefinedVar "time").
Proof.
  reflexivity.
Qed.

End RRVerifyIrLite.
