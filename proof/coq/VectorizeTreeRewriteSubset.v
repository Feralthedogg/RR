Require Import VectorizeValueRewriteSubset.
Require Import VectorizeUseRewriteSubset.
Require Import VectorizeOriginMemoSubset.
Require Import VectorizeDecisionSubset.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Bool.
From Stdlib Require Import Arith.
From Stdlib Require Import Lia.

Open Scope string_scope.
Open Scope Z_scope.
Import RRVectorizeValueRewriteSubset.
Import RRVectorizeUseRewriteSubset.
Import RRVectorizeOriginMemoSubset.
Import RRVectorizeDecisionSubset.

Module RRVectorizeTreeRewriteSubset.

Inductive tiny_tree : Type :=
| TTConstInt : tiny_value_id -> option string -> Z -> tiny_tree
| TTLoad : tiny_value_id -> option string -> string -> tiny_tree
| TTAdd : tiny_value_id -> option string -> tiny_tree -> tiny_tree -> tiny_tree.

Definition tree_id (t : tiny_tree) : tiny_value_id :=
  match t with
  | TTConstInt id _ _ => id
  | TTLoad id _ _ => id
  | TTAdd id _ _ _ => id
  end.

Definition tree_origin (t : tiny_tree) : option string :=
  match t with
  | TTConstInt _ origin _ => origin
  | TTLoad _ origin _ => origin
  | TTAdd _ origin _ _ => origin
  end.

Definition tree_kind_sig (t : tiny_tree) : tiny_kind :=
  match t with
  | TTLoad _ _ v => TKLoad v
  | _ => TKOther
  end.

Definition tree_node (t : tiny_tree) : tiny_node :=
  {| tn_id := tree_id t; tn_origin_var := tree_origin t; tn_kind := tree_kind_sig t |}.

Fixpoint erase_tree (t : tiny_tree) : rewrite_expr :=
  match t with
  | TTConstInt _ _ i => RConstInt i
  | TTLoad _ _ v => RLoad v
  | TTAdd _ _ lhs rhs => RAdd (erase_tree lhs) (erase_tree rhs)
  end.

Definition eval_tree (ρ : val_env) (t : tiny_tree) : Z :=
  eval_rewrite_expr ρ (erase_tree t).

Definition rewrite_result := (tiny_value_id * rewrite_expr * tiny_value_id * bool)%type.

Fixpoint rewrite_tree
    (target : string)
    (replacement_expr : rewrite_expr)
    (replacement_id : tiny_value_id)
    (next_id : tiny_value_id)
    (t : tiny_tree) : rewrite_result :=
  let base_id := boundary_rewrite target replacement_id (tree_node t) in
  if negb (Nat.eqb base_id (tree_id t)) then
    (base_id, replacement_expr, next_id, true)
  else
    match t with
    | TTConstInt id _ i => (id, RConstInt i, next_id, false)
    | TTLoad id _ v => (id, RLoad v, next_id, false)
    | TTAdd id _ lhs rhs =>
        let '(lid, lhs_expr, next1, changed_lhs) :=
          rewrite_tree target replacement_expr replacement_id next_id lhs in
        let '(rid, rhs_expr, next2, changed_rhs) :=
          rewrite_tree target replacement_expr replacement_id next1 rhs in
        if orb changed_lhs changed_rhs then
          (allocate_rewrite_id next2 id true, RAdd lhs_expr rhs_expr, S next2, true)
        else
          (id, RAdd lhs_expr rhs_expr, next2, false)
    end.

Definition sample_tree_unchanged : tiny_tree :=
  TTConstInt 4%nat None 7.

Definition sample_tree_changed : tiny_tree :=
  TTAdd 4%nat None (TTConstInt 1%nat (Some "dest") 7) (TTConstInt 2%nat None 3).

Lemma sample_tree_unchanged_reuses_root :
  let '(chosen, _, _, _) := rewrite_tree "dest" sample_replacement_use 9%nat 20%nat sample_tree_unchanged in
  chosen = 4%nat.
Proof.
  reflexivity.
Qed.

Lemma sample_tree_changed_allocates_fresh :
  let '(chosen, _, _, _) := rewrite_tree "dest" sample_replacement_use 9%nat 20%nat sample_tree_changed in
  chosen = 20%nat.
Proof.
  reflexivity.
Qed.

Lemma sample_tree_changed_next_id_advanced :
  let '(_, _, next_id, _) := rewrite_tree "dest" sample_replacement_use 9%nat 20%nat sample_tree_changed in
  next_id = 21%nat.
Proof.
  reflexivity.
Qed.

Lemma sample_tree_changed_preserves_eval :
  eval_rewrite_expr sample_use_env
    (let '(_, expr, _, _) := rewrite_tree "dest" sample_replacement_use 9%nat 20%nat sample_tree_changed in expr)
  = 10.
Proof.
  reflexivity.
Qed.

End RRVectorizeTreeRewriteSubset.
