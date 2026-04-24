Require Import MirSubsetHoist.
Require Import CfgHoist.
Require Import ReducedFnIR.
Require Import CfgSmallStep.
Require Import LoopPredGraph.
Require Import GraphLicmSound.
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

Module RRCompilerWellFormed.

Definition unique_preheader_latch_pred (c : licm_graph_case) : Prop :=
  preheader_of (case_shape c) <> latch_of (case_shape c).

Definition header_preds_exact_pred (c : licm_graph_case) : Prop :=
  preds_of (case_graph c) (header_of (case_shape c)) =
    preheader_of (case_shape c) :: latch_of (case_shape c) :: nil.

Record rr_wf_case : Type := {
  rr_licm : licm_graph_case;
  rr_unique_preheader_latch : unique_preheader_latch_pred rr_licm;
  rr_header_preds_exact : header_preds_exact_pred rr_licm;
  rr_tmp_fresh_in_body :
    Forall (fun instr => instr_write instr <> tmp_name (case_fnir rr_licm))
      (body_instrs (case_fnir rr_licm));
}.

Definition wf (c : rr_wf_case) : Prop :=
  graph_wf (rr_licm c) /\
  self_backedge_phi (rr_licm c) =
    (phi_latch_val_of (case_phi (rr_licm c)) = phi_self_of (case_phi (rr_licm c))) /\
  body_instrs (case_fnir (rr_licm c)) = body (to_cfg (case_fnir (rr_licm c))).

Lemma rrwf_implies_graph_wf :
  forall c,
    wf c ->
    graph_wf (rr_licm c).
Proof.
  intros c H.
  exact (proj1 H).
Qed.

Lemma rrwf_safe_candidate_zero_trip :
  forall c entry locals,
    wf c ->
    safe_candidate (rr_licm c) ->
    result_of (run_original_machine (case_fnir (rr_licm c)) false entry locals) =
    result_of (run_hoisted_machine (case_fnir (rr_licm c)) false entry locals).
Proof.
  intros c entry locals _ Hsafe.
  apply graph_level_zero_trip_sound.
  exact Hsafe.
Qed.

Lemma rrwf_safe_candidate_one_trip :
  forall c entry locals,
    wf c ->
    safe_candidate (rr_licm c) ->
    result_of (run_original_machine (case_fnir (rr_licm c)) true entry locals) =
    result_of (run_hoisted_machine (case_fnir (rr_licm c)) true entry locals).
Proof.
  intros c entry locals Hwf Hsafe.
  apply graph_level_one_trip_sound.
  - exact (rrwf_implies_graph_wf c Hwf).
  - exact Hsafe.
Qed.

Lemma rrwf_self_backedge_phi_not_invariant :
  forall c ρ,
    wf c ->
    self_backedge_phi (rr_licm c) ->
    ρ (phi_entry_val_of (case_phi (rr_licm c))) <> ρ (phi_self_of (case_phi (rr_licm c))) ->
    ~ pred_invariant
        (case_graph (rr_licm c))
        (case_shape (rr_licm c))
        (case_phi (rr_licm c))
        ρ.
Proof.
  intros c ρ Hwf Hback Hvals.
  apply graph_level_self_backedge_phi_not_invariant.
  - exact (rrwf_implies_graph_wf c Hwf).
  - exact Hback.
  - exact Hvals.
Qed.

Lemma example_rr_tmp_fresh_in_body :
  Forall (fun instr => instr_write instr <> tmp_name (case_fnir example_licm_graph_case))
    (body_instrs (case_fnir example_licm_graph_case)).
Proof.
  simpl.
  constructor.
  - discriminate.
  - constructor.
Qed.

Definition example_rr_wf_case : rr_wf_case :=
  {| rr_licm := example_licm_graph_case;
     rr_unique_preheader_latch := (proj2 example_loop_shape_wf);
     rr_header_preds_exact := (proj1 example_loop_shape_wf);
     rr_tmp_fresh_in_body := example_rr_tmp_fresh_in_body |}.

Lemma example_rr_wf_case_wf :
  wf example_rr_wf_case.
Proof.
  unfold wf, example_rr_wf_case.
  simpl.
  split.
  - exact example_licm_graph_case_graph_wf.
  - split; reflexivity.
Qed.

Lemma example_rr_wf_case_unsound :
  forall entry locals,
    locals "time" + 1 <> entry "time0" ->
    result_of (run_original_machine (case_fnir (rr_licm example_rr_wf_case)) true entry locals) <>
    result_of (run_hoisted_machine (case_fnir (rr_licm example_rr_wf_case)) true entry locals).
Proof.
  intros entry locals Hneq.
  exact (example_licm_graph_case_unsound_machine entry locals Hneq).
Qed.

End RRCompilerWellFormed.
