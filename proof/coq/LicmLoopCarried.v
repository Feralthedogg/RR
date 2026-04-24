From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Lia.
From Stdlib Require Import FunctionalExtensionality.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRLicmLoopCarried.

Definition var := string.
Definition state := var -> Z.

Inductive expr : Type :=
| EConst : Z -> expr
| EVar : var -> expr
| EAdd : expr -> expr -> expr.

Fixpoint free_vars (e : expr) : list var :=
  match e with
  | EConst _ => []
  | EVar x => [x]
  | EAdd lhs rhs => free_vars lhs ++ free_vars rhs
  end.

Fixpoint eval (st : state) (e : expr) : Z :=
  match e with
  | EConst n => n
  | EVar x => st x
  | EAdd lhs rhs => eval st lhs + eval st rhs
  end.

Definition update (st : state) (x : var) (v : Z) : state :=
  fun y => if String.eqb y x then v else st y.

Fixpoint updates (st : state) (us : list (var * Z)) : state :=
  match us with
  | [] => st
  | (x, v) :: rest => updates (update st x v) rest
  end.

Definition licm_hoistable (written : list var) (e : expr) : Prop :=
  forall x, In x (free_vars e) -> ~ In x written.

Definition writes (us : list (var * Z)) : list var :=
  map fst us.

Lemma eval_update_irrelevant :
  forall e st x val,
    ~ In x (free_vars e) ->
    eval (update st x val) e = eval st e.
Proof.
  induction e as [n|y|lhs IHL rhs IHR]; intros st x val Hnotin; simpl in *.
  - reflexivity.
  - cbn in Hnotin.
    unfold update.
    destruct (String.eqb_spec y x).
    + subst.
      exfalso.
      apply Hnotin.
      simpl; auto.
    + reflexivity.
  - assert (Hlhs : ~ In x (free_vars lhs)).
    {
      intro Hin.
      apply Hnotin.
      apply in_or_app.
      left; exact Hin.
    }
    assert (Hrhs : ~ In x (free_vars rhs)).
    {
      intro Hin.
      apply Hnotin.
      apply in_or_app.
      right; exact Hin.
    }
    simpl.
    rewrite IHL by exact Hlhs.
    rewrite IHR by exact Hrhs.
    reflexivity.
Qed.

Lemma eval_updates_irrelevant :
  forall e st us,
    Forall (fun uv => ~ In (fst uv) (free_vars e)) us ->
    eval (updates st us) e = eval st e.
Proof.
  intros e st us Hall.
  revert st.
  induction Hall as [| [x v] rest hHead hTail IH]; intros st.
  - reflexivity.
  - simpl.
    rewrite (IH (update st x v)).
    simpl.
    now apply eval_update_irrelevant.
Qed.

Lemma time_plus_dt_not_hoistable :
  forall dt,
    ~ licm_hoistable ["time"] (EAdd (EVar "time") (EConst dt)).
Proof.
  intros dt Hhoist.
  specialize (Hhoist "time").
  simpl in Hhoist.
  assert (Hin : In "time" ("time" :: [] ++ [])) by (simpl; auto).
  specialize (Hhoist Hin).
  apply Hhoist.
  simpl; auto.
Qed.

Lemma time_plus_dt_not_invariant :
  forall st new_time dt,
    new_time <> st "time" ->
    eval (update st "time" new_time) (EAdd (EVar "time") (EConst dt)) <>
    eval st (EAdd (EVar "time") (EConst dt)).
Proof.
  intros st new_time dt Hneq Heq.
  simpl in Heq.
  unfold update in Heq.
  rewrite String.eqb_refl in Heq.
  lia.
Qed.

Lemma disjoint_updates_make_loop_invariant :
  forall e st us,
    Forall (fun uv => ~ In (fst uv) (free_vars e)) us ->
    eval (updates st us) e = eval st e.
Proof.
  exact eval_updates_irrelevant.
Qed.

Lemma updates_preserve_unwritten :
  forall st us x,
    ~ In x (writes us) ->
    updates st us x = st x.
Proof.
  intros st us x Hnotin.
  induction us as [| [y v] rest IH] in st, Hnotin |- *.
  - reflexivity.
  - simpl in Hnotin.
    apply not_in_cons in Hnotin as [Hneq Htail].
    simpl.
    rewrite IH by exact Htail.
    unfold update.
    destruct (String.eqb_spec x y).
    + subst. contradiction.
    + reflexivity.
Qed.

Lemma update_commute_distinct :
  forall st x vx y vy,
    x <> y ->
    update (update st x vx) y vy = update (update st y vy) x vx.
Proof.
  intros st x vx y vy Hneq.
  extensionality z.
  unfold update.
  destruct (String.eqb z y) eqn:HzY;
  destruct (String.eqb z x) eqn:HzX.
  - apply String.eqb_eq in HzY.
    apply String.eqb_eq in HzX.
    subst.
    contradiction.
  - reflexivity.
  - reflexivity.
  - reflexivity.
Qed.

Lemma updates_ignore_fresh_seed :
  forall st tmp v us y,
    y <> tmp ->
    ~ In tmp (writes us) ->
    updates (update st tmp v) us y = updates st us y.
Proof.
  intros st tmp v us y HyTmp Hfresh.
  induction us as [| [x val] rest IH] in st, Hfresh |- *.
  - simpl.
    unfold update.
    destruct (String.eqb_spec y tmp).
    + contradiction.
    + reflexivity.
  - simpl in Hfresh.
    apply not_in_cons in Hfresh as [HtmpNeq Htail].
    simpl.
    rewrite <- update_commute_distinct by (intro Heq; subst; contradiction).
    apply IH; exact Htail.
Qed.

Lemma updates_commute_fresh_temp :
  forall st tmp v us,
    ~ In tmp (writes us) ->
    updates (update st tmp v) us = update (updates st us) tmp v.
Proof.
  intros st tmp v us Hfresh.
  extensionality y.
  destruct (String.eqb_spec y tmp) as [Heq | Hneq].
  - subst.
    rewrite updates_preserve_unwritten by exact Hfresh.
    unfold update.
    rewrite String.eqb_refl.
    reflexivity.
  - unfold update.
    destruct (String.eqb_spec y tmp).
    + contradiction.
    + clear n.
    apply updates_ignore_fresh_seed; assumption.
Qed.

Lemma licm_hoistable_forall_irrelevant :
  forall us e,
    licm_hoistable (writes us) e ->
    Forall (fun uv => ~ In (fst uv) (free_vars e)) us.
Proof.
  intros us e Hhoist.
  induction us as [| [x v] rest IH].
  - constructor.
  - simpl in Hhoist.
    constructor.
    + intro Hin.
      specialize (Hhoist x Hin).
      exact (Hhoist (or_introl eq_refl)).
    + apply IH.
      intros y Hy.
      specialize (Hhoist y Hy).
      intro HinRest.
      apply Hhoist.
      right; exact HinRest.
Qed.

Lemma licm_hoist_sound_concrete_updates :
  forall st tmp e us,
    ~ In tmp (writes us) ->
    licm_hoistable (writes us) e ->
    updates (update st tmp (eval st e)) us =
    update (updates st us) tmp (eval (updates st us) e).
Proof.
  intros st tmp e us Hfresh Hhoist.
  pose proof (updates_commute_fresh_temp st tmp (eval st e) us Hfresh) as Hcommute.
  rewrite Hcommute.
  f_equal.
  symmetry.
  apply eval_updates_irrelevant.
  exact (licm_hoistable_forall_irrelevant us e Hhoist).
Qed.

End RRLicmLoopCarried.
