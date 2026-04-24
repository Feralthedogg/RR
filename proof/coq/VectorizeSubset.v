From Stdlib Require Import List.
From Stdlib Require Import Bool.

Import ListNotations.

Module RRVectorizeSubset.

Inductive loop_instr : Type :=
| LPureAssign
| LEval
| LStore.

Definition is_effectful (instr : loop_instr) : bool :=
  match instr with
  | LPureAssign => false
  | LEval => true
  | LStore => true
  end.

Fixpoint loop_has_effect (body : list loop_instr) : bool :=
  match body with
  | [] => false
  | instr :: rest => is_effectful instr || loop_has_effect rest
  end.

Definition certify_expr_map (body : list loop_instr) : bool :=
  negb (loop_has_effect body).

Definition certify_cond_store_branch (branch : list loop_instr) : bool :=
  match branch with
  | [LStore] => true
  | _ => false
  end.

Definition certify_cond_map (then_branch else_branch : list loop_instr) : bool :=
  certify_cond_store_branch then_branch && certify_cond_store_branch else_branch.

Lemma certify_expr_map_rejects_eval :
  certify_expr_map [LPureAssign; LEval] = false.
Proof.
  reflexivity.
Qed.

Lemma certify_expr_map_rejects_store :
  certify_expr_map [LPureAssign; LStore] = false.
Proof.
  reflexivity.
Qed.

Lemma certify_expr_map_accepts_pure_assigns :
  certify_expr_map [LPureAssign; LPureAssign] = true.
Proof.
  reflexivity.
Qed.

Lemma certify_cond_map_rejects_branch_eval :
  certify_cond_map [LStore; LEval] [LStore] = false.
Proof.
  reflexivity.
Qed.

Lemma certify_cond_map_rejects_branch_assign :
  certify_cond_map [LStore; LPureAssign] [LStore] = false.
Proof.
  reflexivity.
Qed.

Lemma certify_cond_map_accepts_store_only :
  certify_cond_map [LStore] [LStore] = true.
Proof.
  reflexivity.
Qed.

End RRVectorizeSubset.
