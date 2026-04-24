Require Import MirSubsetHoist.
Require Import CfgHoist.
Require Import ReducedFnIR.
Require Import CfgSmallStep.
Require Import LoopPredGraph.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Open Scope string_scope.
Open Scope Z_scope.

Import RRMirSubsetHoist.
Import RRCfgHoist.
Import RRReducedFnIR.
Import RRCfgSmallStep.
Import RRLoopPredGraph.

Module RRGraphLicmSound.

Record licm_graph_case : Type := {
  case_fnir : reduced_fn_ir;
  case_graph : pred_graph;
  case_shape : loop_shape;
  case_phi : header_phi_graph;
}.

Definition graph_wf (c : licm_graph_case) : Prop :=
  wf_loop (case_graph c) (case_shape c).

Definition safe_candidate (c : licm_graph_case) : Prop :=
  safe_to_hoist_cfg (to_cfg (case_fnir c)).

Definition self_backedge_phi (c : licm_graph_case) : Prop :=
  self_backedge (case_phi c).

Lemma graph_level_zero_trip_sound :
  forall c entry locals,
    safe_candidate c ->
    result_of (run_original_machine (case_fnir c) false entry locals) =
    result_of (run_hoisted_machine (case_fnir c) false entry locals).
Proof.
  intros c entry locals _.
  apply small_step_zero_trip_sound.
Qed.

Lemma graph_level_one_trip_sound :
  forall c entry locals,
    graph_wf c ->
    safe_candidate c ->
    result_of (run_original_machine (case_fnir c) true entry locals) =
    result_of (run_hoisted_machine (case_fnir c) true entry locals).
Proof.
  intros c entry locals _ Hsafe.
  apply small_step_one_trip_sound.
  exact Hsafe.
Qed.

Lemma graph_level_self_backedge_phi_not_invariant :
  forall c ρ,
    graph_wf c ->
    self_backedge_phi c ->
    ρ (phi_entry_val_of (case_phi c)) <> ρ (phi_self_of (case_phi c)) ->
    ~ pred_invariant (case_graph c) (case_shape c) (case_phi c) ρ.
Proof.
  intros c ρ Hwf Hback Hvals.
  apply self_backedge_header_phi_not_pred_invariant.
  - exact Hwf.
  - exact Hback.
  - exact Hvals.
Qed.

Definition example_licm_graph_case : licm_graph_case :=
  {| case_fnir := reduced_phi_time_fn;
     case_graph := example_graph;
     case_shape := example_loop_shape;
     case_phi := example_header_phi |}.

Lemma example_licm_graph_case_graph_wf :
  graph_wf example_licm_graph_case.
Proof.
  exact example_loop_shape_wf.
Qed.

Lemma example_licm_graph_case_self_backedge_phi :
  self_backedge_phi example_licm_graph_case.
Proof.
  exact example_header_phi_self_backedge.
Qed.

Lemma example_licm_graph_case_phi_not_invariant :
  forall ρ,
    ρ 3%nat <> ρ 7%nat ->
    ~ pred_invariant
        (case_graph example_licm_graph_case)
        (case_shape example_licm_graph_case)
        (case_phi example_licm_graph_case)
        ρ.
Proof.
  intros ρ Hneq.
  apply graph_level_self_backedge_phi_not_invariant.
  - exact example_licm_graph_case_graph_wf.
  - exact example_licm_graph_case_self_backedge_phi.
  - exact Hneq.
Qed.

Lemma example_licm_graph_case_unsound_machine :
  forall (entry locals : state),
    locals "time" + 1 <> entry "time0" ->
    result_of (run_original_machine (case_fnir example_licm_graph_case) true entry locals) <>
    result_of (run_hoisted_machine (case_fnir example_licm_graph_case) true entry locals).
Proof.
  intros entry locals Hneq.
  exact (small_step_phi_time_unsound entry locals Hneq).
Qed.

End RRGraphLicmSound.
