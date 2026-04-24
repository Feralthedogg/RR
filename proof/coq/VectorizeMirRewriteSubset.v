Require Import VectorizeApplySubset.
Require Import VectorizeRewriteSubset.
From Stdlib Require Import Bool.
From Stdlib Require Import ZArith.

Open Scope Z_scope.
Import RRVectorizeApplySubset.
Import RRVectorizeRewriteSubset.

Module RRVectorizeMirRewriteSubset.

Inductive tiny_pc : Type :=
| PCPreheader
| PCApply
| PCFallback
| PCExit
| PCDone.

Record tiny_rewrite_state : Type := {
  trs_site : reduced_rewrite_site;
  trs_scalar_slot : Z;
  trs_vector_slot : Z;
  trs_incoming_apply : option bool;
  trs_exit_value : option Z;
  trs_pc : tiny_pc;
}.

Definition initial_rewrite_state (site : reduced_rewrite_site) : tiny_rewrite_state :=
  {| trs_site := site;
     trs_scalar_slot := original_exit_value site;
     trs_vector_slot := vector_result (rrs_plan site);
     trs_incoming_apply := None;
     trs_exit_value := None;
     trs_pc := PCPreheader |}.

Definition step_rewrite (s : tiny_rewrite_state) : tiny_rewrite_state :=
  match trs_pc s with
  | PCPreheader =>
      if rrs_apply_taken (trs_site s)
      then {| trs_site := trs_site s;
              trs_scalar_slot := trs_scalar_slot s;
              trs_vector_slot := trs_vector_slot s;
              trs_incoming_apply := trs_incoming_apply s;
              trs_exit_value := trs_exit_value s;
              trs_pc := PCApply |}
      else {| trs_site := trs_site s;
              trs_scalar_slot := trs_scalar_slot s;
              trs_vector_slot := trs_vector_slot s;
              trs_incoming_apply := trs_incoming_apply s;
              trs_exit_value := trs_exit_value s;
              trs_pc := PCFallback |}
  | PCApply =>
      {| trs_site := trs_site s;
         trs_scalar_slot := trs_scalar_slot s;
         trs_vector_slot := trs_vector_slot s;
         trs_incoming_apply := Some true;
         trs_exit_value := trs_exit_value s;
         trs_pc := PCExit |}
  | PCFallback =>
      {| trs_site := trs_site s;
         trs_scalar_slot := trs_scalar_slot s;
         trs_vector_slot := trs_vector_slot s;
         trs_incoming_apply := Some false;
         trs_exit_value := trs_exit_value s;
         trs_pc := PCExit |}
  | PCExit =>
      let merged :=
        match trs_incoming_apply s with
        | Some true => trs_vector_slot s
        | _ => trs_scalar_slot s
        end in
      {| trs_site := trs_site s;
         trs_scalar_slot := trs_scalar_slot s;
         trs_vector_slot := trs_vector_slot s;
         trs_incoming_apply := trs_incoming_apply s;
         trs_exit_value := Some merged;
         trs_pc := PCDone |}
  | PCDone => s
  end.

Definition run_rewrite (site : reduced_rewrite_site) : tiny_rewrite_state :=
  step_rewrite (step_rewrite (step_rewrite (initial_rewrite_state site))).

Definition run_original (site : reduced_rewrite_site) : Z :=
  original_exit_value site.

Lemma run_rewrite_fallback_preserves_original :
  forall site,
    rrs_apply_taken site = false ->
    trs_exit_value (run_rewrite site) = Some (run_original site).
Proof.
  intros [plan apply_taken] H.
  simpl in *.
  unfold run_rewrite, initial_rewrite_state, run_original, original_exit_value.
  simpl.
  destruct apply_taken eqn:Happly.
  - discriminate.
  - reflexivity.
Qed.

Lemma run_rewrite_apply_preserves_original :
  forall site,
    rrs_apply_taken site = true ->
    result_preserving (rrs_plan site) ->
    trs_exit_value (run_rewrite site) = Some (run_original site).
Proof.
  intros [plan apply_taken] Happly Hpres.
  simpl in *.
  unfold run_rewrite, initial_rewrite_state, run_original, original_exit_value.
  simpl.
  rewrite Happly.
  simpl.
  unfold result_preserving in Hpres.
  now rewrite Hpres.
Qed.

Lemma run_rewrite_preserves_original_if_result_preserving :
  forall site,
    rrs_apply_taken site = false \/ result_preserving (rrs_plan site) ->
    trs_exit_value (run_rewrite site) = Some (run_original site).
Proof.
  intros site [Hfallback | Hpres].
  - apply run_rewrite_fallback_preserves_original; assumption.
  - destruct (rrs_apply_taken site) eqn:Happly.
    + apply run_rewrite_apply_preserves_original; assumption.
    + apply run_rewrite_fallback_preserves_original; assumption.
Qed.

Definition mir_fallback_case : reduced_rewrite_site := fallback_rewrite_case.
Definition mir_apply_case : reduced_rewrite_site := apply_rewrite_case.
Definition mir_cond_fallback_case : reduced_rewrite_site := cond_fallback_rewrite_case.
Definition mir_cond_apply_case : reduced_rewrite_site := cond_apply_rewrite_case.

Lemma mir_fallback_case_preserved :
  trs_exit_value (run_rewrite mir_fallback_case) = Some 7.
Proof.
  apply run_rewrite_fallback_preserves_original.
  reflexivity.
Qed.

Lemma mir_apply_case_preserved :
  trs_exit_value (run_rewrite mir_apply_case) = Some 7.
Proof.
  apply run_rewrite_apply_preserves_original.
  - reflexivity.
  - reflexivity.
Qed.

Lemma mir_cond_fallback_case_preserved :
  trs_exit_value (run_rewrite mir_cond_fallback_case) = Some 4.
Proof.
  apply run_rewrite_fallback_preserves_original.
  reflexivity.
Qed.

Lemma mir_cond_apply_case_preserved :
  trs_exit_value (run_rewrite mir_cond_apply_case) = Some 4.
Proof.
  apply run_rewrite_apply_preserves_original.
  - reflexivity.
  - reflexivity.
Qed.

End RRVectorizeMirRewriteSubset.
