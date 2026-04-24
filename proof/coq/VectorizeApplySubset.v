Require Import VectorizeSubset.
From Stdlib Require Import List.
From Stdlib Require Import Bool.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope Z_scope.
Import RRVectorizeSubset.

Module RRVectorizeApplySubset.

Inductive reduced_vector_plan : Type :=
| RVPExprMap : list loop_instr -> Z -> Z -> reduced_vector_plan
| RVPCondMap : list loop_instr -> list loop_instr -> Z -> Z -> reduced_vector_plan.

Definition scalar_result (plan : reduced_vector_plan) : Z :=
  match plan with
  | RVPExprMap _ scalar _ => scalar
  | RVPCondMap _ _ scalar _ => scalar
  end.

Definition vector_result (plan : reduced_vector_plan) : Z :=
  match plan with
  | RVPExprMap _ _ vec => vec
  | RVPCondMap _ _ _ vec => vec
  end.

Definition certify_plan (plan : reduced_vector_plan) : bool :=
  match plan with
  | RVPExprMap body _ _ => certify_expr_map body
  | RVPCondMap then_branch else_branch _ _ => certify_cond_map then_branch else_branch
  end.

Definition transactional_apply (plan : reduced_vector_plan) : Z :=
  if certify_plan plan then vector_result plan else scalar_result plan.

Definition result_preserving (plan : reduced_vector_plan) : Prop :=
  vector_result plan = scalar_result plan.

Lemma transactional_apply_rolls_back_on_reject :
  forall plan,
    certify_plan plan = false ->
    transactional_apply plan = scalar_result plan.
Proof.
  intros plan Hreject.
  unfold transactional_apply.
  rewrite Hreject.
  reflexivity.
Qed.

Lemma transactional_apply_commits_preserving_plan :
  forall plan,
    certify_plan plan = true ->
    result_preserving plan ->
    transactional_apply plan = scalar_result plan.
Proof.
  intros plan Hcert Hpres.
  unfold transactional_apply, result_preserving.
  rewrite Hcert.
  exact Hpres.
Qed.

Definition pure_expr_map_case : reduced_vector_plan :=
  RVPExprMap [LPureAssign; LPureAssign] 7 7.

Definition reject_expr_map_case : reduced_vector_plan :=
  RVPExprMap [LPureAssign; LEval] 7 99.

Definition store_only_cond_case : reduced_vector_plan :=
  RVPCondMap [LStore] [LStore] 4 4.

Definition reject_cond_branch_case : reduced_vector_plan :=
  RVPCondMap [LStore; LPureAssign] [LStore] 4 99.

Lemma pure_expr_map_case_preserved :
  transactional_apply pure_expr_map_case = 7.
Proof.
  apply transactional_apply_commits_preserving_plan.
  - reflexivity.
  - reflexivity.
Qed.

Lemma reject_expr_map_case_rolls_back :
  transactional_apply reject_expr_map_case = 7.
Proof.
  apply transactional_apply_rolls_back_on_reject.
  reflexivity.
Qed.

Lemma store_only_cond_case_preserved :
  transactional_apply store_only_cond_case = 4.
Proof.
  apply transactional_apply_commits_preserving_plan.
  - reflexivity.
  - reflexivity.
Qed.

Lemma reject_cond_branch_case_rolls_back :
  transactional_apply reject_cond_branch_case = 4.
Proof.
  apply transactional_apply_rolls_back_on_reject.
  reflexivity.
Qed.

End RRVectorizeApplySubset.
