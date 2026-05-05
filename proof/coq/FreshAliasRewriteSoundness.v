Require Import MirSemanticsLite.
From Stdlib Require Import String Bool.
Open Scope string_scope.
Open Scope bool_scope.

Module RRFreshAliasRewriteSoundness.

Import RRMirSemanticsLite.

Fixpoint rename_alias_expr (alias source : string) (expr : mir_expr) : mir_expr :=
  match expr with
  | MEConst v => MEConst v
  | MELoad name => if String.eqb name alias then MELoad source else MELoad name
  | MEAdd lhs rhs => MEAdd (rename_alias_expr alias source lhs) (rename_alias_expr alias source rhs)
  | MEMul lhs rhs => MEMul (rename_alias_expr alias source lhs) (rename_alias_expr alias source rhs)
  | MENeg arg => MENeg (rename_alias_expr alias source arg)
  | MELt lhs rhs => MELt (rename_alias_expr alias source lhs) (rename_alias_expr alias source rhs)
  end.

Definition env_alias_agrees (ρ : env) (alias source : string) : Prop :=
  lookup_env ρ alias = lookup_env ρ source.

Lemma rename_alias_expr_preserves_eval :
  forall ρ alias source expr,
    env_alias_agrees ρ alias source ->
    eval_expr ρ (rename_alias_expr alias source expr) = eval_expr ρ expr.
Proof.
  intros ρ alias source expr Hagree.
  induction expr; simpl.
  - reflexivity.
  - destruct (String.eqb s alias) eqn:Heq.
    + apply String.eqb_eq in Heq. subst s. simpl. exact (eq_sym Hagree).
    + reflexivity.
  - rewrite IHexpr1, IHexpr2; reflexivity; exact Hagree.
  - rewrite IHexpr1, IHexpr2; reflexivity; exact Hagree.
  - rewrite IHexpr; reflexivity; exact Hagree.
  - rewrite IHexpr1, IHexpr2; reflexivity; exact Hagree.
Qed.

End RRFreshAliasRewriteSoundness.
