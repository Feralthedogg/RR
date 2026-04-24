Require Import VectorizeValueRewriteSubset.
Require Import VectorizeUseRewriteSubset.
Require Import VectorizeOriginMemoSubset.
Require Import VectorizeTreeRewriteSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRVectorizeValueRewriteSubset.
Import RRVectorizeUseRewriteSubset.
Import RRVectorizeOriginMemoSubset.
Import RRVectorizeTreeRewriteSubset.

Module RRVectorizeAllocStateSubset.

Definition tree_rewrite_out := list (tiny_value_id * rewrite_expr).

Fixpoint rewrite_tree_list
    (target : string)
    (replacement_expr : rewrite_expr)
    (replacement_id : tiny_value_id)
    (next_id : tiny_value_id)
    (trees : list tiny_tree) : tree_rewrite_out * tiny_value_id :=
  match trees with
  | [] => ([], next_id)
  | tree :: rest =>
      let '(chosen, expr, next1, changed) :=
        rewrite_tree target replacement_expr replacement_id next_id tree in
      let '(tail_out, next2) :=
        rewrite_tree_list target replacement_expr replacement_id next1 rest in
      ((chosen, expr) :: tail_out, next2)
  end.

Definition eval_tree_list (ρ : val_env) (trees : list tiny_tree) : list Z :=
  map (eval_tree ρ) trees.

Definition eval_rewrite_out (ρ : val_env) (out : tree_rewrite_out) : list Z :=
  map (fun '(_, expr) => eval_rewrite_expr ρ expr) out.

Definition sample_tree_list : list tiny_tree :=
  [sample_tree_unchanged; sample_tree_changed].

Definition sample_changed_tree_list : list tiny_tree :=
  [sample_tree_changed; sample_tree_changed].

Lemma sample_tree_list_preserved :
  eval_rewrite_out sample_use_env
    (fst (rewrite_tree_list "dest" sample_replacement_use 9%nat 20%nat sample_tree_list))
  = eval_tree_list sample_use_env sample_tree_list.
Proof.
  reflexivity.
Qed.

Lemma sample_tree_list_final_next_id :
  snd (rewrite_tree_list "dest" sample_replacement_use 9%nat 20%nat sample_tree_list) = 21%nat.
Proof.
  reflexivity.
Qed.

Lemma sample_changed_tree_list_final_next_id :
  snd (rewrite_tree_list "dest" sample_replacement_use 9%nat 20%nat sample_changed_tree_list) = 22%nat.
Proof.
  reflexivity.
Qed.

Lemma sample_changed_tree_list_fresh_ids :
  map fst (fst (rewrite_tree_list "dest" sample_replacement_use 9%nat 20%nat sample_changed_tree_list))
  = [20%nat; 21%nat].
Proof.
  reflexivity.
Qed.

End RRVectorizeAllocStateSubset.
