Require Import VectorizeApplySubset.
From Stdlib Require Import Bool.
From Stdlib Require Import ZArith.

Open Scope Z_scope.
Import RRVectorizeApplySubset.

Module RRVectorizeRewriteSubset.

Record reduced_rewrite_site : Type := {
  rrs_plan : reduced_vector_plan;
  rrs_apply_taken : bool;
}.

Definition original_exit_value (site : reduced_rewrite_site) : Z :=
  scalar_result (rrs_plan site).

Definition rewritten_exit_phi_value (site : reduced_rewrite_site) : Z :=
  if rrs_apply_taken site then vector_result (rrs_plan site) else scalar_result (rrs_plan site).

Lemma rewritten_exit_phi_value_fallback_eq_original :
  forall site,
    rrs_apply_taken site = false ->
    rewritten_exit_phi_value site = original_exit_value site.
Proof.
  intros site H.
  unfold rewritten_exit_phi_value, original_exit_value.
  rewrite H.
  reflexivity.
Qed.

Lemma rewritten_exit_phi_value_apply_eq_original :
  forall site,
    rrs_apply_taken site = true ->
    result_preserving (rrs_plan site) ->
    rewritten_exit_phi_value site = original_exit_value site.
Proof.
  intros site Happly Hpres.
  unfold rewritten_exit_phi_value, original_exit_value, result_preserving.
  rewrite Happly.
  exact Hpres.
Qed.

Lemma rewritten_exit_phi_value_preserved_if_result_preserving :
  forall site,
    result_preserving (rrs_plan site) ->
    rewritten_exit_phi_value site = original_exit_value site.
Proof.
  intros site Hpres.
  destruct (rrs_apply_taken site) eqn:Happly.
  - apply rewritten_exit_phi_value_apply_eq_original; assumption.
  - apply rewritten_exit_phi_value_fallback_eq_original; assumption.
Qed.

Definition fallback_rewrite_case : reduced_rewrite_site :=
  {| rrs_plan := reject_expr_map_case; rrs_apply_taken := false |}.

Definition apply_rewrite_case : reduced_rewrite_site :=
  {| rrs_plan := pure_expr_map_case; rrs_apply_taken := true |}.

Definition cond_fallback_rewrite_case : reduced_rewrite_site :=
  {| rrs_plan := reject_cond_branch_case; rrs_apply_taken := false |}.

Definition cond_apply_rewrite_case : reduced_rewrite_site :=
  {| rrs_plan := store_only_cond_case; rrs_apply_taken := true |}.

Lemma fallback_rewrite_case_preserved :
  rewritten_exit_phi_value fallback_rewrite_case = original_exit_value fallback_rewrite_case.
Proof.
  apply rewritten_exit_phi_value_fallback_eq_original.
  reflexivity.
Qed.

Lemma apply_rewrite_case_preserved :
  rewritten_exit_phi_value apply_rewrite_case = original_exit_value apply_rewrite_case.
Proof.
  apply rewritten_exit_phi_value_apply_eq_original.
  - reflexivity.
  - reflexivity.
Qed.

Lemma cond_fallback_rewrite_case_preserved :
  rewritten_exit_phi_value cond_fallback_rewrite_case =
  original_exit_value cond_fallback_rewrite_case.
Proof.
  apply rewritten_exit_phi_value_fallback_eq_original.
  reflexivity.
Qed.

Lemma cond_apply_rewrite_case_preserved :
  rewritten_exit_phi_value cond_apply_rewrite_case =
  original_exit_value cond_apply_rewrite_case.
Proof.
  apply rewritten_exit_phi_value_apply_eq_original.
  - reflexivity.
  - reflexivity.
Qed.

End RRVectorizeRewriteSubset.
