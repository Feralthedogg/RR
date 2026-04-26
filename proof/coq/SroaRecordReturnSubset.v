From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Open Scope string_scope.
Open Scope Z_scope.

Module RRSroaRecordReturnSubset.

Definition var_name := string.
Definition field_name := string.

Record sroa_env := {
  scalar : var_name -> Z;
  record : var_name -> field_name -> Z
}.

Inductive sroa_expr : Type :=
| SConstInt : Z -> sroa_expr
| SLoad : var_name -> sroa_expr
| SField : var_name -> field_name -> sroa_expr
| SCallField : var_name -> field_name -> sroa_expr
| SAdd : sroa_expr -> sroa_expr -> sroa_expr.

Fixpoint eval_sroa_expr (ρ : sroa_env) (e : sroa_expr) : Z :=
  match e with
  | SConstInt z => z
  | SLoad v => scalar ρ v
  | SField alias field => record ρ alias field
  | SCallField call field => record ρ call field
  | SAdd lhs rhs => eval_sroa_expr ρ lhs + eval_sroa_expr ρ rhs
  end.

Fixpoint rewrite_alias_field_to_temp
    (alias : var_name)
    (field : field_name)
    (temp : var_name)
    (e : sroa_expr) : sroa_expr :=
  match e with
  | SConstInt z => SConstInt z
  | SLoad v => SLoad v
  | SField current_alias current_field =>
      if String.eqb current_alias alias
      then if String.eqb current_field field
           then SLoad temp
           else SField current_alias current_field
      else SField current_alias current_field
  | SCallField call field => SCallField call field
  | SAdd lhs rhs =>
      SAdd (rewrite_alias_field_to_temp alias field temp lhs)
           (rewrite_alias_field_to_temp alias field temp rhs)
  end.

Fixpoint rewrite_alias_field_to_value
    (alias : var_name)
    (field : field_name)
    (replacement : sroa_expr)
    (e : sroa_expr) : sroa_expr :=
  match e with
  | SConstInt z => SConstInt z
  | SLoad v => SLoad v
  | SField current_alias current_field =>
      if String.eqb current_alias alias
      then if String.eqb current_field field
           then replacement
           else SField current_alias current_field
      else SField current_alias current_field
  | SCallField call field => SCallField call field
  | SAdd lhs rhs =>
      SAdd (rewrite_alias_field_to_value alias field replacement lhs)
           (rewrite_alias_field_to_value alias field replacement rhs)
  end.

Fixpoint rewrite_direct_call_field_to_value
    (call : var_name)
    (field : field_name)
    (replacement : sroa_expr)
    (e : sroa_expr) : sroa_expr :=
  match e with
  | SConstInt z => SConstInt z
  | SLoad v => SLoad v
  | SField alias field => SField alias field
  | SCallField current_call current_field =>
      if String.eqb current_call call
      then if String.eqb current_field field
           then replacement
           else SCallField current_call current_field
      else SCallField current_call current_field
  | SAdd lhs rhs =>
      SAdd (rewrite_direct_call_field_to_value call field replacement lhs)
           (rewrite_direct_call_field_to_value call field replacement rhs)
  end.

Lemma rewrite_alias_field_to_temp_preserves_eval :
  forall ρ alias field temp expr,
    scalar ρ temp = record ρ alias field ->
    eval_sroa_expr ρ
      (rewrite_alias_field_to_temp alias field temp expr) =
    eval_sroa_expr ρ expr.
Proof.
  intros ρ alias field temp expr Htemp.
  induction expr as
    [z|v|current_alias current_field|call current_field|lhs IHlhs rhs IHrhs]; simpl.
  - reflexivity.
  - reflexivity.
  - destruct (String.eqb current_alias alias) eqn:Halias.
    + apply String.eqb_eq in Halias. subst current_alias.
      destruct (String.eqb current_field field) eqn:Hfield.
      * apply String.eqb_eq in Hfield. subst current_field. exact Htemp.
      * reflexivity.
    + reflexivity.
  - reflexivity.
  - rewrite IHlhs by exact Htemp.
    rewrite IHrhs by exact Htemp.
    reflexivity.
Qed.

Lemma rewrite_alias_field_to_value_preserves_eval :
  forall ρ alias field replacement expr,
    eval_sroa_expr ρ replacement = record ρ alias field ->
    eval_sroa_expr ρ
      (rewrite_alias_field_to_value alias field replacement expr) =
    eval_sroa_expr ρ expr.
Proof.
  intros ρ alias field replacement expr Hreplacement.
  induction expr as
    [z|v|current_alias current_field|call current_field|lhs IHlhs rhs IHrhs]; simpl.
  - reflexivity.
  - reflexivity.
  - destruct (String.eqb current_alias alias) eqn:Halias.
    + apply String.eqb_eq in Halias. subst current_alias.
      destruct (String.eqb current_field field) eqn:Hfield.
      * apply String.eqb_eq in Hfield. subst current_field. exact Hreplacement.
      * reflexivity.
    + reflexivity.
  - reflexivity.
  - rewrite IHlhs by exact Hreplacement.
    rewrite IHrhs by exact Hreplacement.
    reflexivity.
Qed.

Lemma rewrite_direct_call_field_to_value_preserves_eval :
  forall ρ call field replacement expr,
    eval_sroa_expr ρ replacement = record ρ call field ->
    eval_sroa_expr ρ
      (rewrite_direct_call_field_to_value call field replacement expr) =
    eval_sroa_expr ρ expr.
Proof.
  intros ρ call field replacement expr Hreplacement.
  induction expr as
    [z|v|alias current_field|current_call current_field|lhs IHlhs rhs IHrhs]; simpl.
  - reflexivity.
  - reflexivity.
  - reflexivity.
  - destruct (String.eqb current_call call) eqn:Hcall.
    + apply String.eqb_eq in Hcall. subst current_call.
      destruct (String.eqb current_field field) eqn:Hfield.
      * apply String.eqb_eq in Hfield. subst current_field. exact Hreplacement.
      * reflexivity.
    + reflexivity.
  - rewrite IHlhs by exact Hreplacement.
    rewrite IHrhs by exact Hreplacement.
    reflexivity.
Qed.

Definition repeated_projection_expr : sroa_expr :=
  SAdd (SField "p" "x") (SField "p" "x").

Definition repeated_projection_env : sroa_env :=
  {| scalar := fun v => if String.eqb v "p__rr_sroa_ret_x" then 7 else 0;
     record := fun alias field =>
       if String.eqb alias "p"
       then if String.eqb field "x" then 7 else 0
       else 0 |}.

Lemma repeated_projection_shared_temp_preserved :
  eval_sroa_expr repeated_projection_env
    (rewrite_alias_field_to_temp
      "p" "x" "p__rr_sroa_ret_x" repeated_projection_expr) =
  eval_sroa_expr repeated_projection_env repeated_projection_expr.
Proof.
  apply rewrite_alias_field_to_temp_preserves_eval.
  reflexivity.
Qed.

Definition local_record_projection_expr : sroa_expr :=
  SAdd (SField "moved" "x") (SConstInt 3).

Definition local_record_projection_env : sroa_env :=
  {| scalar := fun v =>
       if String.eqb v "entity_x" then 10
       else if String.eqb v "velocity_x" then 2
       else 0;
     record := fun alias field =>
       if String.eqb alias "moved"
       then if String.eqb field "x" then 12 else 0
       else 0 |}.

Lemma local_record_projection_scalar_value_preserved :
  eval_sroa_expr local_record_projection_env
    (rewrite_alias_field_to_value
      "moved"
      "x"
      (SAdd (SLoad "entity_x") (SLoad "velocity_x"))
      local_record_projection_expr) =
  eval_sroa_expr local_record_projection_env local_record_projection_expr.
Proof.
  apply rewrite_alias_field_to_value_preserves_eval.
  reflexivity.
Qed.

Definition snapshot_record_projection_expr : sroa_expr :=
  SAdd (SField "point" "x") (SField "point" "y").

Definition snapshot_record_projection_env : sroa_env :=
  {| scalar := fun v =>
       if String.eqb v "point__rr_sroa_snap_x" then 4
       else if String.eqb v "point__rr_sroa_snap_y" then 9
       else 0;
     record := fun alias field =>
       if String.eqb alias "point"
       then if String.eqb field "x" then 4
            else if String.eqb field "y" then 9
            else 0
       else 0 |}.

Lemma snapshot_record_projection_temps_preserved :
  eval_sroa_expr snapshot_record_projection_env
    (rewrite_alias_field_to_temp
      "point"
      "y"
      "point__rr_sroa_snap_y"
      (rewrite_alias_field_to_temp
        "point"
        "x"
        "point__rr_sroa_snap_x"
        snapshot_record_projection_expr)) =
  eval_sroa_expr snapshot_record_projection_env snapshot_record_projection_expr.
Proof.
  rewrite rewrite_alias_field_to_temp_preserves_eval.
  - apply rewrite_alias_field_to_temp_preserves_eval.
    reflexivity.
  - reflexivity.
Qed.

Definition direct_projection_expr : sroa_expr :=
  SAdd (SCallField "make_xy()" "x") (SConstInt 5).

Definition direct_projection_env : sroa_env :=
  {| scalar := fun _ => 0;
     record := fun call field =>
       if String.eqb call "make_xy()"
       then if String.eqb field "x" then 11 else 0
       else 0 |}.

Lemma direct_projection_inline_value_preserved :
  eval_sroa_expr direct_projection_env
    (rewrite_direct_call_field_to_value
      "make_xy()" "x" (SConstInt 11) direct_projection_expr) =
  eval_sroa_expr direct_projection_env direct_projection_expr.
Proof.
  apply rewrite_direct_call_field_to_value_preserves_eval.
  reflexivity.
Qed.

End RRSroaRecordReturnSubset.
