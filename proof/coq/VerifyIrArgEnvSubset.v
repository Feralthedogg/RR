Require Import VerifyIrArgListTraversalSubset.
Require Import VerifyIrValueEnvSubset.
Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRVerifyIrValueEnvSubset.

Module RRVerifyIrArgEnvSubset.

Definition env_field_arg := (string * env_expr)%type.

Fixpoint eval_env_expr_list (env : value_env) (es : list env_expr)
    : option (list rvalue) :=
  match es with
  | [] => Some []
  | e :: rest =>
      match eval_env_expr env e, eval_env_expr_list env rest with
      | Some v, Some vs => Some (v :: vs)
      | _, _ => None
      end
  end.

Definition rewrite_phi_use_list (phi arg : value_id) (es : list env_expr)
    : list env_expr :=
  map (rewrite_phi_use phi arg) es.

Fixpoint eval_env_field_args (env : value_env) (fs : list env_field_arg)
    : option (list (string * rvalue)) :=
  match fs with
  | [] => Some []
  | (name, e) :: rest =>
      match eval_env_expr env e, eval_env_field_args env rest with
      | Some v, Some vs => Some ((name, v) :: vs)
      | _, _ => None
      end
  end.

Definition rewrite_phi_use_field_args (phi arg : value_id)
    (fs : list env_field_arg) : list env_field_arg :=
  map (fun '(name, e) => (name, rewrite_phi_use phi arg e)) fs.

Lemma eval_env_expr_list_rewrite_phi_use :
  forall env phi arg es,
    eval_env_expr_list (merged_env env phi arg) es =
    eval_env_expr_list env (rewrite_phi_use_list phi arg es).
Proof.
  intros env phi arg es.
  induction es as [|e rest IH].
  - reflexivity.
  - simpl.
    rewrite eval_rewrite_phi_use, IH.
    reflexivity.
Qed.

Lemma eval_env_field_args_rewrite_phi_use :
  forall env phi arg fs,
    eval_env_field_args (merged_env env phi arg) fs =
    eval_env_field_args env (rewrite_phi_use_field_args phi arg fs).
Proof.
  intros env phi arg fs.
  induction fs as [|(name, e) rest IH].
  - reflexivity.
  - simpl.
    rewrite eval_rewrite_phi_use, IH.
    reflexivity.
Qed.

Lemma eval_env_expr_list_after_phi_edge :
  forall env phi arg pred edges es,
    phi_select edges pred = Some arg ->
    eval_env_expr_list (merged_env env phi arg) es =
    eval_env_expr_list env (rewrite_phi_use_list phi arg es).
Proof.
  intros env phi arg pred edges es _.
  apply eval_env_expr_list_rewrite_phi_use.
Qed.

Lemma eval_env_field_args_after_phi_edge :
  forall env phi arg pred edges fs,
    phi_select edges pred = Some arg ->
    eval_env_field_args (merged_env env phi arg) fs =
    eval_env_field_args env (rewrite_phi_use_field_args phi arg fs).
Proof.
  intros env phi arg pred edges fs _.
  apply eval_env_field_args_rewrite_phi_use.
Qed.

Definition example_call_env_args : list env_expr :=
  [EUse 9%nat; EAdd (EUse 9%nat) (EConst (RVInt 3))].

Definition example_record_env_fields : list env_field_arg :=
  [("a", EUse 12%nat); ("b", EField (EUse 12%nat) "x")].

Lemma example_call_env_args_preserved_on_selected_edge :
  eval_env_expr_list (merged_env example_env 9%nat 1%nat) example_call_env_args =
  eval_env_expr_list example_env (rewrite_phi_use_list 9%nat 1%nat example_call_env_args).
Proof.
  exact
    (eval_env_expr_list_after_phi_edge example_env 9%nat 1%nat 0%nat example_phi_args
       example_call_env_args example_phi_select_zero).
Qed.

Lemma example_call_env_args_value :
  eval_env_expr_list (merged_env example_env 9%nat 1%nat) example_call_env_args =
  Some [RVInt 4; RVInt 7].
Proof.
  reflexivity.
Qed.

Lemma example_record_env_fields_preserved_on_selected_edge :
  eval_env_field_args (merged_env example_env 12%nat 7%nat) example_record_env_fields =
  eval_env_field_args example_env
    (rewrite_phi_use_field_args 12%nat 7%nat example_record_env_fields).
Proof.
  exact
    (eval_env_field_args_after_phi_edge example_env 12%nat 7%nat 2%nat
       example_field_phi_args example_record_env_fields example_field_phi_select).
Qed.

Lemma example_record_env_fields_value :
  eval_env_field_args (merged_env example_env 12%nat 7%nat) example_record_env_fields =
  Some [("a", RVRecord [("x", RVInt 9)]); ("b", RVInt 9)].
Proof.
  reflexivity.
Qed.

End RRVerifyIrArgEnvSubset.
