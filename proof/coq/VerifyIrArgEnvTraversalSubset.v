Require Import VerifyIrArgEnvSubset.
Require Import VerifyIrValueEnvSubset.
Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import Bool.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRVerifyIrValueEnvSubset.
Import RRVerifyIrArgEnvSubset.

Module RRVerifyIrArgEnvTraversalSubset.

Fixpoint first_missing_arg_env_expr (env : value_env) (e : env_expr)
    : option value_id :=
  match e with
  | EConst _ => None
  | EUse vid =>
      match env vid with
      | Some _ => None
      | None => Some vid
      end
  | EAdd lhs rhs =>
      match first_missing_arg_env_expr env lhs with
      | Some v => Some v
      | None => first_missing_arg_env_expr env rhs
      end
  | EField base _ => first_missing_arg_env_expr env base
  end.

Fixpoint first_missing_arg_env_expr_list (env : value_env) (es : list env_expr)
    : option value_id :=
  match es with
  | [] => None
  | e :: rest =>
      match first_missing_arg_env_expr env e with
      | Some v => Some v
      | None => first_missing_arg_env_expr_list env rest
      end
  end.

Fixpoint first_missing_arg_env_fields (env : value_env) (fs : list env_field_arg)
    : option value_id :=
  match fs with
  | [] => None
  | (_, e) :: rest =>
      match first_missing_arg_env_expr env e with
      | Some v => Some v
      | None => first_missing_arg_env_fields env rest
      end
  end.

Lemma missing_choice_clean_iff :
  forall head tail : option value_id,
    (match head with
     | Some v => Some v
     | None => tail
     end) = None <->
    head = None /\ tail = None.
Proof.
  intros head tail.
  destruct head as [v|]; simpl.
  - split.
    + intro H. discriminate H.
    + intro H. destruct H as [H _]. discriminate H.
  - tauto.
Qed.

Lemma first_missing_arg_env_expr_clean_rewrite_phi_use :
  forall env phi arg e,
    first_missing_arg_env_expr (merged_env env phi arg) e = None <->
    first_missing_arg_env_expr env (rewrite_phi_use phi arg e) = None.
Proof.
  intros env phi arg e.
  induction e as [rv|vid|lhs IHl rhs IHr|base IH name].
  - reflexivity.
  - simpl.
    unfold merged_env.
    destruct (Nat.eqb vid phi) eqn:Heq.
    + apply Nat.eqb_eq in Heq. subst vid.
      destruct (env arg); simpl.
      * split; intro H; reflexivity.
      * split; intro H; discriminate H.
    + destruct (env vid); simpl; split; intro H; assumption || reflexivity.
  - change
      ((match first_missing_arg_env_expr (merged_env env phi arg) lhs with
        | Some v => Some v
        | None => first_missing_arg_env_expr (merged_env env phi arg) rhs
        end) = None <->
       (match first_missing_arg_env_expr env (rewrite_phi_use phi arg lhs) with
        | Some v => Some v
        | None => first_missing_arg_env_expr env (rewrite_phi_use phi arg rhs)
        end) = None).
    rewrite missing_choice_clean_iff, missing_choice_clean_iff, IHl, IHr.
    tauto.
  - change
      (first_missing_arg_env_expr (merged_env env phi arg) base = None <->
       first_missing_arg_env_expr env (rewrite_phi_use phi arg base) = None).
    exact IH.
Qed.

Lemma first_missing_arg_env_expr_list_clean_rewrite_phi_use :
  forall env phi arg es,
    first_missing_arg_env_expr_list (merged_env env phi arg) es = None <->
    first_missing_arg_env_expr_list env (rewrite_phi_use_list phi arg es) = None.
Proof.
  intros env phi arg es.
  induction es as [|e rest IH].
  - reflexivity.
  - change
      ((match first_missing_arg_env_expr (merged_env env phi arg) e with
        | Some v => Some v
        | None => first_missing_arg_env_expr_list (merged_env env phi arg) rest
        end) = None <->
       (match first_missing_arg_env_expr env (rewrite_phi_use phi arg e) with
        | Some v => Some v
        | None => first_missing_arg_env_expr_list env (rewrite_phi_use_list phi arg rest)
        end) = None).
    rewrite missing_choice_clean_iff, missing_choice_clean_iff,
      first_missing_arg_env_expr_clean_rewrite_phi_use, IH.
    tauto.
Qed.

Lemma first_missing_arg_env_fields_clean_rewrite_phi_use :
  forall env phi arg fs,
    first_missing_arg_env_fields (merged_env env phi arg) fs = None <->
    first_missing_arg_env_fields env (rewrite_phi_use_field_args phi arg fs) = None.
Proof.
  intros env phi arg fs.
  induction fs as [|(name, e) rest IH].
  - reflexivity.
  - change
      ((match first_missing_arg_env_expr (merged_env env phi arg) e with
        | Some v => Some v
        | None => first_missing_arg_env_fields (merged_env env phi arg) rest
        end) = None <->
       (match first_missing_arg_env_expr env (rewrite_phi_use phi arg e) with
        | Some v => Some v
        | None => first_missing_arg_env_fields env (rewrite_phi_use_field_args phi arg rest)
        end) = None).
    rewrite missing_choice_clean_iff, missing_choice_clean_iff,
      first_missing_arg_env_expr_clean_rewrite_phi_use, IH.
    tauto.
Qed.

Lemma example_call_env_args_scan_preserved_on_selected_edge :
  first_missing_arg_env_expr_list (merged_env example_env 9%nat 1%nat) example_call_env_args = None <->
  first_missing_arg_env_expr_list example_env
    (rewrite_phi_use_list 9%nat 1%nat example_call_env_args) = None.
Proof.
  apply first_missing_arg_env_expr_list_clean_rewrite_phi_use.
Qed.

Lemma example_call_env_args_scan_clean_from_selected_eval :
  first_missing_arg_env_expr_list (merged_env example_env 9%nat 1%nat) example_call_env_args =
  None.
Proof.
  reflexivity.
Qed.

Lemma example_record_env_fields_scan_preserved_on_selected_edge :
  first_missing_arg_env_fields (merged_env example_env 12%nat 7%nat) example_record_env_fields = None <->
  first_missing_arg_env_fields example_env
    (rewrite_phi_use_field_args 12%nat 7%nat example_record_env_fields) = None.
Proof.
  apply first_missing_arg_env_fields_clean_rewrite_phi_use.
Qed.

Lemma example_record_env_fields_scan_clean_from_selected_eval :
  first_missing_arg_env_fields (merged_env example_env 12%nat 7%nat) example_record_env_fields =
  None.
Proof.
  reflexivity.
Qed.

End RRVerifyIrArgEnvTraversalSubset.
