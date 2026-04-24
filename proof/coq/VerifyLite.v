Require Import MirSubsetHoist.
Require Import CfgHoist.
Require Import ReducedFnIR.
Require Import CfgSmallStep.
Require Import LoopPredGraph.
Require Import GraphLicmSound.
Require Import RRWellFormed.
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

Module RRVerifyLite.

Record verify_lite_case : Type := {
  verify_rr : RRCompilerWellFormed.rr_wf_case;
}.

Definition verify_lite (c : verify_lite_case) : Prop :=
  RRCompilerWellFormed.wf (verify_rr c) /\
  (forall (entry locals : state),
      safe_candidate (RRCompilerWellFormed.rr_licm (verify_rr c)) ->
      result_of (run_original_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr c))) false entry locals) =
      result_of (run_hoisted_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr c))) false entry locals)) /\
  (forall (entry locals : state),
      safe_candidate (RRCompilerWellFormed.rr_licm (verify_rr c)) ->
      result_of (run_original_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr c))) true entry locals) =
      result_of (run_hoisted_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr c))) true entry locals)) /\
  (forall ρ,
      self_backedge_phi (RRCompilerWellFormed.rr_licm (verify_rr c)) ->
      ρ (phi_entry_val_of (case_phi (RRCompilerWellFormed.rr_licm (verify_rr c)))) <>
        ρ (phi_self_of (case_phi (RRCompilerWellFormed.rr_licm (verify_rr c)))) ->
      ~ pred_invariant
          (case_graph (RRCompilerWellFormed.rr_licm (verify_rr c)))
          (case_shape (RRCompilerWellFormed.rr_licm (verify_rr c)))
          (case_phi (RRCompilerWellFormed.rr_licm (verify_rr c)))
          ρ).

Lemma verify_lite_zero_trip_sound :
  forall c entry locals,
    verify_lite c ->
    safe_candidate (RRCompilerWellFormed.rr_licm (verify_rr c)) ->
    result_of (run_original_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr c))) false entry locals) =
    result_of (run_hoisted_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr c))) false entry locals).
Proof.
  intros c entry locals Hverify Hsafe.
  exact ((proj1 (proj2 Hverify)) entry locals Hsafe).
Qed.

Lemma verify_lite_one_trip_sound :
  forall c entry locals,
    verify_lite c ->
    safe_candidate (RRCompilerWellFormed.rr_licm (verify_rr c)) ->
    result_of (run_original_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr c))) true entry locals) =
    result_of (run_hoisted_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr c))) true entry locals).
Proof.
  intros c entry locals Hverify Hsafe.
  exact ((proj1 (proj2 (proj2 Hverify))) entry locals Hsafe).
Qed.

Lemma verify_lite_rejects_self_backedge_phi :
  forall c ρ,
    verify_lite c ->
    self_backedge_phi (RRCompilerWellFormed.rr_licm (verify_rr c)) ->
    ρ (phi_entry_val_of (case_phi (RRCompilerWellFormed.rr_licm (verify_rr c)))) <>
      ρ (phi_self_of (case_phi (RRCompilerWellFormed.rr_licm (verify_rr c)))) ->
    ~ pred_invariant
        (case_graph (RRCompilerWellFormed.rr_licm (verify_rr c)))
        (case_shape (RRCompilerWellFormed.rr_licm (verify_rr c)))
        (case_phi (RRCompilerWellFormed.rr_licm (verify_rr c)))
        ρ.
Proof.
  intros c ρ Hverify Hback Hvals.
  exact ((proj2 (proj2 (proj2 Hverify))) ρ Hback Hvals).
Qed.

Definition example_verify_lite_case : verify_lite_case :=
  {| verify_rr := RRCompilerWellFormed.example_rr_wf_case |}.

Lemma example_verify_lite_case_holds :
  verify_lite example_verify_lite_case.
Proof.
  unfold verify_lite, example_verify_lite_case.
  split.
  - exact RRCompilerWellFormed.example_rr_wf_case_wf.
  - split.
    + intros entry locals Hsafe.
      exact (RRCompilerWellFormed.rrwf_safe_candidate_zero_trip
        RRCompilerWellFormed.example_rr_wf_case entry locals RRCompilerWellFormed.example_rr_wf_case_wf Hsafe).
    + split.
      * intros entry locals Hsafe.
        exact (RRCompilerWellFormed.rrwf_safe_candidate_one_trip
          RRCompilerWellFormed.example_rr_wf_case entry locals RRCompilerWellFormed.example_rr_wf_case_wf Hsafe).
      * intros ρ Hback Hvals.
        exact (RRCompilerWellFormed.rrwf_self_backedge_phi_not_invariant
          RRCompilerWellFormed.example_rr_wf_case ρ RRCompilerWellFormed.example_rr_wf_case_wf Hback Hvals).
Qed.

Lemma example_verify_lite_case_unsound_if_phi_forced :
  forall entry locals,
    locals "time" + 1 <> entry "time0" ->
    result_of (run_original_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr example_verify_lite_case))) true entry locals) <>
    result_of (run_hoisted_machine (case_fnir (RRCompilerWellFormed.rr_licm (verify_rr example_verify_lite_case))) true entry locals).
Proof.
  intros entry locals Hneq.
  exact (RRCompilerWellFormed.example_rr_wf_case_unsound entry locals Hneq).
Qed.

End RRVerifyLite.
