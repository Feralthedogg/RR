Require Import VectorizeRewriteSubset.
Require Import VectorizeMirRewriteSubset.
Require Import VectorizeValueRewriteSubset.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Open Scope string_scope.
Open Scope Z_scope.
Import RRVectorizeRewriteSubset.
Import RRVectorizeMirRewriteSubset.
Import RRVectorizeValueRewriteSubset.

Module RRVectorizeGraphSubset.

Definition graph_target_var : var_name := "dest".

Definition graph_env (site : reduced_rewrite_site) : val_env :=
  fun v => if String.eqb v "dest" then run_original site else 0.

Definition graph_replacement (site : reduced_rewrite_site) : rewrite_expr :=
  RConstInt (match trs_exit_value (run_rewrite site) with
             | Some z => z
             | None => 0
             end).

Lemma graph_return_preserved :
  forall site ret,
    trs_exit_value (run_rewrite site) = Some (run_original site) ->
    rewritten_return (graph_env site) graph_target_var (graph_replacement site) ret =
    original_return (graph_env site) ret.
Proof.
  intros site ret Hrun.
  apply rewritten_return_preserves_original.
  unfold graph_env, graph_target_var, graph_replacement.
  simpl.
  rewrite Hrun.
  reflexivity.
Qed.

Definition graph_ret : rewrite_expr := RAdd (RLoad graph_target_var) (RConstInt 3).

Lemma mir_fallback_case_graph_preserved :
  rewritten_return (graph_env mir_fallback_case) graph_target_var
    (graph_replacement mir_fallback_case) graph_ret = 10.
Proof.
  apply graph_return_preserved.
  apply mir_fallback_case_preserved.
Qed.

Lemma mir_apply_case_graph_preserved :
  rewritten_return (graph_env mir_apply_case) graph_target_var
    (graph_replacement mir_apply_case) graph_ret = 10.
Proof.
  apply graph_return_preserved.
  apply mir_apply_case_preserved.
Qed.

Lemma mir_cond_fallback_case_graph_preserved :
  rewritten_return (graph_env mir_cond_fallback_case) graph_target_var
    (graph_replacement mir_cond_fallback_case) graph_ret = 7.
Proof.
  apply graph_return_preserved.
  apply mir_cond_fallback_case_preserved.
Qed.

Lemma mir_cond_apply_case_graph_preserved :
  rewritten_return (graph_env mir_cond_apply_case) graph_target_var
    (graph_replacement mir_cond_apply_case) graph_ret = 7.
Proof.
  apply graph_return_preserved.
  apply mir_cond_apply_case_preserved.
Qed.

End RRVectorizeGraphSubset.
