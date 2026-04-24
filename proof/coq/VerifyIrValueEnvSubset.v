Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import PeanoNat.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.

Module RRVerifyIrValueEnvSubset.

Definition value_id := nat.
Definition block_id := nat.
Definition value_env := value_id -> option rvalue.

Inductive env_expr : Type :=
| EConst : rvalue -> env_expr
| EUse : value_id -> env_expr
| EAdd : env_expr -> env_expr -> env_expr
| EField : env_expr -> string -> env_expr.

Fixpoint eval_env_expr (env : value_env) (e : env_expr) : option rvalue :=
  match e with
  | EConst v => Some v
  | EUse vid => env vid
  | EAdd lhs rhs =>
      match eval_env_expr env lhs, eval_env_expr env rhs with
      | Some (RVInt l), Some (RVInt r) => Some (RVInt (l + r))
      | _, _ => None
      end
  | EField base name =>
      match eval_env_expr env base with
      | Some (RVRecord fields) => lookup_field fields name
      | _ => None
      end
  end.

Fixpoint rewrite_phi_use (phi arg : value_id) (e : env_expr) : env_expr :=
  match e with
  | EConst v => EConst v
  | EUse vid => EUse (if Nat.eqb vid phi then arg else vid)
  | EAdd lhs rhs => EAdd (rewrite_phi_use phi arg lhs) (rewrite_phi_use phi arg rhs)
  | EField base name => EField (rewrite_phi_use phi arg base) name
  end.

Definition merged_env (env : value_env) (phi arg : value_id) : value_env :=
  fun vid => if Nat.eqb vid phi then env arg else env vid.

Definition phi_select (edges : list (value_id * block_id)) (pred : block_id)
    : option value_id :=
  match find (fun entry => Nat.eqb (snd entry) pred) edges with
  | Some (vid, _) => Some vid
  | None => None
  end.

Lemma eval_rewrite_phi_use :
  forall env phi arg expr,
    eval_env_expr (merged_env env phi arg) expr =
    eval_env_expr env (rewrite_phi_use phi arg expr).
Proof.
  intros env phi arg expr.
  induction expr; simpl.
  - reflexivity.
  - unfold merged_env.
    destruct (Nat.eqb v phi); reflexivity.
  - rewrite IHexpr1, IHexpr2. reflexivity.
  - rewrite IHexpr. reflexivity.
Qed.

Lemma eval_after_phi_edge :
  forall env phi arg pred edges expr,
    phi_select edges pred = Some arg ->
    eval_env_expr (merged_env env phi arg) expr =
    eval_env_expr env (rewrite_phi_use phi arg expr).
Proof.
  intros env phi arg pred edges expr _.
  apply eval_rewrite_phi_use.
Qed.

Definition example_env : value_env :=
  fun vid =>
    match vid with
    | 1%nat => Some (RVInt 4)
    | 3%nat => Some (RVInt 5)
    | 7%nat => Some (RVRecord [("x", RVInt 9)])
    | _ => None
    end.

Definition example_phi_args : list (value_id * block_id) :=
  [(1%nat, 0%nat); (3%nat, 1%nat)].

Definition example_consumer : env_expr :=
  EAdd (EUse 9%nat) (EConst (RVInt 3)).

Definition example_field_phi_args : list (value_id * block_id) :=
  [(7%nat, 2%nat)].

Definition example_field_consumer : env_expr :=
  EField (EUse 12%nat) "x".

Lemma example_phi_select_zero :
  phi_select example_phi_args 0%nat = Some 1%nat.
Proof.
  reflexivity.
Qed.

Lemma example_consumer_preserved_on_selected_edge :
  eval_env_expr (merged_env example_env 9%nat 1%nat) example_consumer =
  eval_env_expr example_env (rewrite_phi_use 9%nat 1%nat example_consumer).
Proof.
  exact
    (eval_after_phi_edge example_env 9%nat 1%nat 0%nat example_phi_args
       example_consumer example_phi_select_zero).
Qed.

Lemma example_consumer_value :
  eval_env_expr (merged_env example_env 9%nat 1%nat) example_consumer = Some (RVInt 7).
Proof.
  reflexivity.
Qed.

Lemma example_field_phi_select :
  phi_select example_field_phi_args 2%nat = Some 7%nat.
Proof.
  reflexivity.
Qed.

Lemma example_field_consumer_preserved_on_selected_edge :
  eval_env_expr (merged_env example_env 12%nat 7%nat) example_field_consumer =
  eval_env_expr example_env (rewrite_phi_use 12%nat 7%nat example_field_consumer).
Proof.
  exact
    (eval_after_phi_edge example_env 12%nat 7%nat 2%nat example_field_phi_args
       example_field_consumer example_field_phi_select).
Qed.

Lemma example_field_consumer_value :
  eval_env_expr (merged_env example_env 12%nat 7%nat) example_field_consumer =
  Some (RVInt 9).
Proof.
  reflexivity.
Qed.

End RRVerifyIrValueEnvSubset.
