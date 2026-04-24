Require Import VectorizeValueRewriteSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRVectorizeValueRewriteSubset.

Module RRVectorizeUseRewriteSubset.

Definition use_id := nat.

Record reachable_use : Type := {
  ru_id : use_id;
  ru_expr : rewrite_expr;
}.

Definition eval_reachable_use (ρ : val_env) (u : reachable_use) : Z :=
  eval_rewrite_expr ρ (ru_expr u).

Definition rewrite_reachable_use (target : var_name) (replacement : rewrite_expr)
    (u : reachable_use) : reachable_use :=
  {| ru_id := ru_id u;
     ru_expr := rewrite_loads_for_var target replacement (ru_expr u) |}.

Definition eval_reachable_uses (ρ : val_env) (uses : list reachable_use) : list Z :=
  map (eval_reachable_use ρ) uses.

Definition rewrite_reachable_uses (target : var_name) (replacement : rewrite_expr)
    (uses : list reachable_use) : list reachable_use :=
  map (rewrite_reachable_use target replacement) uses.

Lemma rewrite_reachable_use_preserves_eval :
  forall ρ target replacement u,
    eval_rewrite_expr ρ replacement = eval_rewrite_expr ρ (RLoad target) ->
    eval_reachable_use ρ (rewrite_reachable_use target replacement u) =
    eval_reachable_use ρ u.
Proof.
  intros ρ target replacement u Hpres.
  unfold eval_reachable_use, rewrite_reachable_use.
  apply rewrite_loads_for_var_preserves_eval.
  exact Hpres.
Qed.

Lemma rewrite_reachable_uses_preserves_eval :
  forall ρ target replacement uses,
    eval_rewrite_expr ρ replacement = eval_rewrite_expr ρ (RLoad target) ->
    eval_reachable_uses ρ (rewrite_reachable_uses target replacement uses) =
    eval_reachable_uses ρ uses.
Proof.
  intros ρ target replacement uses Hpres.
  induction uses as [|u rest IH]; simpl.
  - reflexivity.
  - rewrite rewrite_reachable_use_preserves_eval by exact Hpres.
    rewrite IH by exact Hpres.
    reflexivity.
Qed.

Definition sample_use_env : val_env :=
  fun v => if String.eqb v "dest" then 7 else 0.

Definition sample_replacement_use : rewrite_expr := RConstInt 7.

Definition sample_uses : list reachable_use :=
  [ {| ru_id := 0%nat; ru_expr := RLoad "dest" |}
  ; {| ru_id := 1%nat; ru_expr := RAdd (RLoad "dest") (RConstInt 3) |}
  ].

Lemma sample_uses_preserved :
  eval_reachable_uses sample_use_env
    (rewrite_reachable_uses "dest" sample_replacement_use sample_uses)
  = [7; 10].
Proof.
  apply rewrite_reachable_uses_preserves_eval.
  reflexivity.
Qed.

End RRVectorizeUseRewriteSubset.
