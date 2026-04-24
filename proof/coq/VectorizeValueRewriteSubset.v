From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Bool.

Open Scope string_scope.
Open Scope Z_scope.

Module RRVectorizeValueRewriteSubset.

Definition var_name := string.
Definition val_env := var_name -> Z.

Inductive rewrite_expr : Type :=
| RConstInt : Z -> rewrite_expr
| RLoad : var_name -> rewrite_expr
| RAdd : rewrite_expr -> rewrite_expr -> rewrite_expr.

Fixpoint eval_rewrite_expr (ρ : val_env) (e : rewrite_expr) : Z :=
  match e with
  | RConstInt z => z
  | RLoad v => ρ v
  | RAdd lhs rhs => eval_rewrite_expr ρ lhs + eval_rewrite_expr ρ rhs
  end.

Fixpoint rewrite_loads_for_var (target : var_name) (replacement : rewrite_expr)
    (e : rewrite_expr) : rewrite_expr :=
  match e with
  | RConstInt z => RConstInt z
  | RLoad v => if String.eqb v target then replacement else RLoad v
  | RAdd lhs rhs =>
      RAdd (rewrite_loads_for_var target replacement lhs)
           (rewrite_loads_for_var target replacement rhs)
  end.

Lemma rewrite_loads_for_var_preserves_eval :
  forall ρ target replacement expr,
    eval_rewrite_expr ρ replacement = eval_rewrite_expr ρ (RLoad target) ->
    eval_rewrite_expr ρ (rewrite_loads_for_var target replacement expr) =
    eval_rewrite_expr ρ expr.
Proof.
  intros ρ target replacement expr Hpres.
  induction expr as [z|v|lhs IHlhs rhs IHrhs]; simpl.
  - reflexivity.
  - destruct (String.eqb v target) eqn:Heq.
    + apply String.eqb_eq in Heq. subst v. exact Hpres.
    + reflexivity.
  - rewrite IHlhs, IHrhs by exact Hpres. reflexivity.
Qed.

Definition original_return (ρ : val_env) (ret : rewrite_expr) : Z :=
  eval_rewrite_expr ρ ret.

Definition rewritten_return (ρ : val_env) (target : var_name)
    (replacement ret : rewrite_expr) : Z :=
  eval_rewrite_expr ρ (rewrite_loads_for_var target replacement ret).

Lemma rewritten_return_preserves_original :
  forall ρ target replacement ret,
    eval_rewrite_expr ρ replacement = eval_rewrite_expr ρ (RLoad target) ->
    rewritten_return ρ target replacement ret = original_return ρ ret.
Proof.
  intros ρ target replacement ret Hpres.
  unfold rewritten_return, original_return.
  apply rewrite_loads_for_var_preserves_eval.
  exact Hpres.
Qed.

Definition sample_ret : rewrite_expr := RAdd (RLoad "x") (RConstInt 3).
Definition sample_replacement : rewrite_expr := RConstInt 7.
Definition sample_env : val_env :=
  fun v => if String.eqb v "x" then 7 else 0.

Lemma sample_ret_preserved :
  rewritten_return sample_env "x" sample_replacement sample_ret = 10.
Proof.
  apply rewritten_return_preserves_original.
  reflexivity.
Qed.

End RRVectorizeValueRewriteSubset.
