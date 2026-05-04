From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Bool.
From Stdlib Require Import Arith.

Require Import MirSemanticsLite.
Require Import MirInvariantBundle.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRDataflowOptSoundness.

Import RRMirSemanticsLite.
Import RRMirInvariantBundle.

Definition value_fingerprint (value : mir_value) : Z :=
  match value with
  | MVInt z => z
  | MVBool true => 1
  | MVBool false => 0
  | MVNull => -1
  | MVArray items => 1000 + Z.of_nat (List.length items)
  | MVRecord fields => 2000 + Z.of_nat (List.length fields)
  end.

Fixpoint expr_size (expr : mir_expr) : nat :=
  match expr with
  | MEConst _ => 1
  | MELoad _ => 1
  | MEAdd lhs rhs => S (expr_size lhs + expr_size rhs)
  | MEMul lhs rhs => S (expr_size lhs + expr_size rhs)
  | MENeg arg => S (expr_size arg)
  | MELt lhs rhs => S (expr_size lhs + expr_size rhs)
  end.

Fixpoint expr_fingerprint (expr : mir_expr) : Z :=
  match expr with
  | MEConst value => 100 + value_fingerprint value
  | MELoad name => 300 + Z.of_nat (String.length name)
  | MEAdd lhs rhs => 500 + 31 * expr_fingerprint lhs + 37 * expr_fingerprint rhs
  | MEMul lhs rhs => 700 + 41 * expr_fingerprint lhs + 43 * expr_fingerprint rhs
  | MENeg arg => 900 + 47 * expr_fingerprint arg
  | MELt lhs rhs => 1100 + 53 * expr_fingerprint lhs + 59 * expr_fingerprint rhs
  end.

Definition expr_lt (lhs rhs : mir_expr) : bool :=
  Z.ltb (expr_fingerprint lhs) (expr_fingerprint rhs)
    || (Z.eqb (expr_fingerprint lhs) (expr_fingerprint rhs)
          && Nat.ltb (expr_size lhs) (expr_size rhs)).

Fixpoint canonicalize_expr (expr : mir_expr) : mir_expr :=
  match expr with
  | MEConst value => MEConst value
  | MELoad name => MELoad name
  | MEAdd lhs rhs =>
      let lhs' := canonicalize_expr lhs in
      let rhs' := canonicalize_expr rhs in
      if expr_lt rhs' lhs' then MEAdd rhs' lhs' else MEAdd lhs' rhs'
  | MEMul lhs rhs =>
      let lhs' := canonicalize_expr lhs in
      let rhs' := canonicalize_expr rhs in
      if expr_lt rhs' lhs' then MEMul rhs' lhs' else MEMul lhs' rhs'
  | MENeg arg => MENeg (canonicalize_expr arg)
  | MELt lhs rhs => MELt (canonicalize_expr lhs) (canonicalize_expr rhs)
  end.

Lemma eval_add_comm :
  forall ρ lhs rhs,
    eval_expr ρ (MEAdd lhs rhs) = eval_expr ρ (MEAdd rhs lhs).
Proof.
  intros ρ lhs rhs.
  simpl.
  destruct (eval_expr ρ lhs) as [lv|] eqn:HL.
  - destruct (eval_expr ρ rhs) as [rv|] eqn:HR.
    + destruct lv; destruct rv; simpl; try reflexivity.
      rewrite Z.add_comm. reflexivity.
    + destruct lv; simpl; reflexivity.
  - destruct (eval_expr ρ rhs) as [rv|] eqn:HR.
    + destruct rv; simpl; reflexivity.
    + reflexivity.
Qed.

Lemma eval_mul_comm :
  forall ρ lhs rhs,
    eval_expr ρ (MEMul lhs rhs) = eval_expr ρ (MEMul rhs lhs).
Proof.
  intros ρ lhs rhs.
  simpl.
  destruct (eval_expr ρ lhs) as [lv|] eqn:HL.
  - destruct (eval_expr ρ rhs) as [rv|] eqn:HR.
    + destruct lv; destruct rv; simpl; try reflexivity.
      rewrite Z.mul_comm. reflexivity.
    + destruct lv; simpl; reflexivity.
  - destruct (eval_expr ρ rhs) as [rv|] eqn:HR.
    + destruct rv; simpl; reflexivity.
    + reflexivity.
Qed.

Fixpoint canonicalize_expr_preserves_eval (ρ : env) (expr : mir_expr)
    : eval_expr ρ (canonicalize_expr expr) = eval_expr ρ expr.
Proof.
  destruct expr as [value|name|lhs rhs|lhs rhs|arg|lhs rhs]; simpl.
  - reflexivity.
  - reflexivity.
  - remember (canonicalize_expr lhs) as lhs'.
    remember (canonicalize_expr rhs) as rhs'.
    destruct (expr_lt rhs' lhs') eqn:Hlt.
    + subst lhs' rhs'. rewrite eval_add_comm. simpl.
      now rewrite canonicalize_expr_preserves_eval, canonicalize_expr_preserves_eval.
    + subst lhs' rhs'. simpl.
      now rewrite canonicalize_expr_preserves_eval, canonicalize_expr_preserves_eval.
  - remember (canonicalize_expr lhs) as lhs'.
    remember (canonicalize_expr rhs) as rhs'.
    destruct (expr_lt rhs' lhs') eqn:Hlt.
    + subst lhs' rhs'. rewrite eval_mul_comm. simpl.
      now rewrite canonicalize_expr_preserves_eval, canonicalize_expr_preserves_eval.
    + subst lhs' rhs'. simpl.
      now rewrite canonicalize_expr_preserves_eval, canonicalize_expr_preserves_eval.
  - simpl. now rewrite canonicalize_expr_preserves_eval.
  - simpl. now rewrite canonicalize_expr_preserves_eval, canonicalize_expr_preserves_eval.
Defined.

Definition const_env : Type := env.

Definition env_agrees_on_consts (ρ : env) (consts : const_env) : Prop :=
  forall name value,
    lookup_env consts name = Some value ->
    lookup_env ρ name = Some value.

Fixpoint const_prop_expr (consts : const_env) (expr : mir_expr) : mir_expr :=
  match expr with
  | MEConst value => MEConst value
  | MELoad name =>
      match lookup_env consts name with
      | Some value => MEConst value
      | None => MELoad name
      end
  | MEAdd lhs rhs => MEAdd (const_prop_expr consts lhs) (const_prop_expr consts rhs)
  | MEMul lhs rhs => MEMul (const_prop_expr consts lhs) (const_prop_expr consts rhs)
  | MENeg arg => MENeg (const_prop_expr consts arg)
  | MELt lhs rhs => MELt (const_prop_expr consts lhs) (const_prop_expr consts rhs)
  end.

Fixpoint const_prop_expr_preserves_eval
    (ρ : env) (consts : const_env) (expr : mir_expr)
    (Hagree : env_agrees_on_consts ρ consts)
    : eval_expr ρ (const_prop_expr consts expr) = eval_expr ρ expr.
Proof.
  destruct expr as [value|name|lhs rhs|lhs rhs|arg|lhs rhs]; simpl.
  - reflexivity.
  - destruct (lookup_env consts name) as [value|] eqn:Hconst.
    + specialize (Hagree name value Hconst). now rewrite Hagree.
    + reflexivity.
  - now rewrite const_prop_expr_preserves_eval, const_prop_expr_preserves_eval.
  - now rewrite const_prop_expr_preserves_eval, const_prop_expr_preserves_eval.
  - now rewrite const_prop_expr_preserves_eval.
  - now rewrite const_prop_expr_preserves_eval, const_prop_expr_preserves_eval.
Qed.

Definition rewrite_expr (consts : const_env) (expr : mir_expr) : mir_expr :=
  canonicalize_expr (const_prop_expr consts expr).

Lemma rewrite_expr_preserves_eval :
  forall ρ consts expr,
    env_agrees_on_consts ρ consts ->
    eval_expr ρ (rewrite_expr consts expr) = eval_expr ρ expr.
Proof.
  intros ρ consts expr Hagree.
  unfold rewrite_expr.
  rewrite canonicalize_expr_preserves_eval.
  apply const_prop_expr_preserves_eval.
  exact Hagree.
Qed.

Fixpoint expr_depends_on (target : string) (expr : mir_expr) : Prop :=
  match expr with
  | MEConst _ => False
  | MELoad name => name = target
  | MEAdd lhs rhs => expr_depends_on target lhs \/ expr_depends_on target rhs
  | MEMul lhs rhs => expr_depends_on target lhs \/ expr_depends_on target rhs
  | MENeg arg => expr_depends_on target arg
  | MELt lhs rhs => expr_depends_on target lhs \/ expr_depends_on target rhs
  end.

Lemma lookup_env_update_env_ne :
  forall ρ target name value,
    name <> target ->
    lookup_env (update_env ρ target value) name = lookup_env ρ name.
Proof.
  intros ρ target name value Hne.
  induction ρ as [|[field current] rest IH]; simpl.
  - destruct (String.eqb target name) eqn:Heq.
    + apply String.eqb_eq in Heq. exfalso. apply Hne. exact (eq_sym Heq).
    + reflexivity.
  - destruct (String.eqb field target) eqn:Hfield.
    + destruct (String.eqb field name) eqn:Hname; simpl.
      * apply String.eqb_eq in Hfield.
        apply String.eqb_eq in Hname.
        subst. exfalso. apply Hne. reflexivity.
      * rewrite Hname. reflexivity.
    + destruct (String.eqb field name) eqn:Hname; simpl.
      * rewrite Hname. reflexivity.
      * rewrite Hname. exact IH.
Qed.

Fixpoint eval_expr_update_irrelevant
    (ρ : env) (target : string) (value : mir_value) (expr : mir_expr)
    : ~ expr_depends_on target expr ->
      eval_expr (update_env ρ target value) expr = eval_expr ρ expr.
Proof.
  destruct expr as [v|name|lhs rhs|lhs rhs|arg|lhs rhs]; intro Hno; simpl in *.
  - reflexivity.
  - apply lookup_env_update_env_ne.
    exact Hno.
  - assert (Hlhs : ~ expr_depends_on target lhs).
    { intro Hc. apply Hno. now left. }
    assert (Hrhs : ~ expr_depends_on target rhs).
    { intro Hc. apply Hno. now right. }
    simpl. now rewrite eval_expr_update_irrelevant, eval_expr_update_irrelevant.
  - assert (Hlhs : ~ expr_depends_on target lhs).
    { intro Hc. apply Hno. now left. }
    assert (Hrhs : ~ expr_depends_on target rhs).
    { intro Hc. apply Hno. now right. }
    simpl. now rewrite eval_expr_update_irrelevant, eval_expr_update_irrelevant.
  - simpl. now rewrite eval_expr_update_irrelevant.
  - assert (Hlhs : ~ expr_depends_on target lhs).
    { intro Hc. apply Hno. now left. }
    assert (Hrhs : ~ expr_depends_on target rhs).
    { intro Hc. apply Hno. now right. }
    simpl. now rewrite eval_expr_update_irrelevant, eval_expr_update_irrelevant.
Qed.

Record straight_line_block : Type := {
  sl_instrs : list (string * mir_expr);
  sl_ret : mir_expr;
}.

Fixpoint exec_assigns (ρ : env) (instrs : list (string * mir_expr)) : option env :=
  match instrs with
  | [] => Some ρ
  | (dst, rhs) :: rest =>
      match eval_expr ρ rhs with
      | Some value => exec_assigns (update_env ρ dst value) rest
      | None => None
      end
  end.

Definition exec_straight_line_block (ρ : env) (blk : straight_line_block)
    : option mir_value :=
  match exec_assigns ρ blk.(sl_instrs) with
  | Some ρ' => eval_expr ρ' blk.(sl_ret)
  | None => None
  end.

Lemma exec_assigns_app :
  forall ρ prefix suffix,
    exec_assigns ρ (prefix ++ suffix) =
      match exec_assigns ρ prefix with
      | Some ρ' => exec_assigns ρ' suffix
      | None => None
      end.
Proof.
  intros ρ prefix.
  revert ρ.
  induction prefix as [|[dst rhs] rest IH]; intros ρ suffix; simpl.
  - reflexivity.
  - destruct (eval_expr ρ rhs) as [value|] eqn:Hrhs; simpl.
    + apply IH.
    + reflexivity.
Qed.

Lemma drop_last_dead_assign_preserves_block :
  forall ρ ρ' prefix dst rhs ret value,
    exec_assigns ρ prefix = Some ρ' ->
    eval_expr ρ' rhs = Some value ->
    ~ expr_depends_on dst ret ->
    exec_straight_line_block ρ
      {| sl_instrs := prefix ++ [(dst, rhs)]; sl_ret := ret |} =
    exec_straight_line_block ρ
      {| sl_instrs := prefix; sl_ret := ret |}.
Proof.
  intros ρ ρ' prefix dst rhs ret value Hprefix Hrhs Hret.
  unfold exec_straight_line_block; simpl.
  rewrite exec_assigns_app.
  rewrite Hprefix. simpl.
  rewrite Hrhs. simpl.
  rewrite (eval_expr_update_irrelevant ρ' dst value ret Hret).
  reflexivity.
Qed.

Lemma identity_dataflow_layer_preserves_verify_ir_bundle :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (identity_pass fn).
Proof.
  intros fn H.
  exact (identity_pass_preserves_verify_ir_bundle fn H).
Qed.

End RRDataflowOptSoundness.
