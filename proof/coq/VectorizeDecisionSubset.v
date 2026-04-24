Require Import VectorizeValueRewriteSubset.
Require Import VectorizeUseRewriteSubset.
Require Import VectorizeOriginMemoSubset.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Bool.

Open Scope string_scope.
Open Scope Z_scope.
Import RRVectorizeValueRewriteSubset.
Import RRVectorizeUseRewriteSubset.
Import RRVectorizeOriginMemoSubset.

Module RRVectorizeDecisionSubset.

Record rewrite_decision_state : Type := {
  rds_node : tiny_node;
  rds_use : reachable_use;
  rds_memo : tiny_memo;
  rds_next_id : tiny_value_id;
  rds_changed : bool;
}.

Definition decision_base_id (target : string) (replacement_id : tiny_value_id)
    (s : rewrite_decision_state) : tiny_value_id :=
  boundary_rewrite target replacement_id (rds_node s).

Definition decision_chosen_id (target : string) (replacement_id : tiny_value_id)
    (s : rewrite_decision_state) : tiny_value_id :=
  memoized_result (rds_memo s)
    (decision_base_id target replacement_id s)
    (allocate_rewrite_id (rds_next_id s) (tn_id (rds_node s)) (rds_changed s)).

Definition rewrite_use_with_decision
    (target : var_name)
    (replacement_expr : rewrite_expr)
    (replacement_id : tiny_value_id)
    (s : rewrite_decision_state) : tiny_value_id * reachable_use :=
  (decision_chosen_id target replacement_id s,
   rewrite_reachable_use target replacement_expr (rds_use s)).

Lemma decision_chosen_id_memo_hit :
  forall target replacement_id mapped s,
    rds_memo s (decision_base_id target replacement_id s) = Some mapped ->
    decision_chosen_id target replacement_id s = mapped.
Proof.
  intros. unfold decision_chosen_id.
  apply memoized_result_hit_reuses.
  exact H.
Qed.

Lemma decision_chosen_id_unchanged_root :
  forall target replacement_id s,
    decision_base_id target replacement_id s = tn_id (rds_node s) ->
    rds_memo s (tn_id (rds_node s)) = None ->
    rds_changed s = false ->
    decision_chosen_id target replacement_id s = tn_id (rds_node s).
Proof.
  intros target replacement_id s Hboundary Hmiss Hchanged.
  unfold decision_chosen_id.
  rewrite Hboundary.
  rewrite memoized_result_miss_uses_computed by exact Hmiss.
  now rewrite Hchanged, allocate_rewrite_id_unchanged_reuses_root.
Qed.

Lemma rewrite_use_with_decision_preserves_eval :
  forall ρ target replacement_expr replacement_id s,
    eval_rewrite_expr ρ replacement_expr = eval_rewrite_expr ρ (RLoad target) ->
    eval_reachable_use ρ (snd (rewrite_use_with_decision target replacement_expr replacement_id s)) =
    eval_reachable_use ρ (rds_use s).
Proof.
  intros ρ target replacement_expr replacement_id s Hpres.
  unfold rewrite_use_with_decision.
  simpl.
  apply rewrite_reachable_use_preserves_eval.
  exact Hpres.
Qed.

Definition sample_decision_state : rewrite_decision_state :=
  {| rds_node := {| tn_id := 4%nat; tn_origin_var := Some "dest"; tn_kind := TKOther |};
     rds_use := {| ru_id := 0%nat; ru_expr := RAdd (RLoad "dest") (RConstInt 3) |};
     rds_memo := fun _ => None;
     rds_next_id := 9%nat;
     rds_changed := true |}.

Lemma sample_decision_chosen_fresh :
  decision_chosen_id "dest" 7%nat sample_decision_state = 9%nat.
Proof.
  reflexivity.
Qed.

Lemma sample_decision_use_preserved :
  eval_reachable_use sample_use_env
    (snd (rewrite_use_with_decision "dest" sample_replacement_use 7%nat sample_decision_state))
  = 10.
Proof.
  apply rewrite_use_with_decision_preserves_eval.
  reflexivity.
Qed.

End RRVectorizeDecisionSubset.
