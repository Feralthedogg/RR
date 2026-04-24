From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Lia.
From Stdlib Require Import FunctionalExtensionality.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRMirSubsetHoist.

Definition var := string.
Definition state := var -> Z.

Inductive mir_value : Type :=
| MVConst : Z -> mir_value
| MVLocal : var -> mir_value
| MVPhi : var -> var -> mir_value
| MVAdd : mir_value -> mir_value -> mir_value.

Fixpoint local_deps (e : mir_value) : list var :=
  match e with
  | MVConst _ => []
  | MVLocal x => [x]
  | MVPhi _ _ => []
  | MVAdd lhs rhs => local_deps lhs ++ local_deps rhs
  end.

Fixpoint carried_deps (e : mir_value) : list var :=
  match e with
  | MVConst _ => []
  | MVLocal _ => []
  | MVPhi _ carried => [carried]
  | MVAdd lhs rhs => carried_deps lhs ++ carried_deps rhs
  end.

Fixpoint eval (iter : nat) (entry carried locals : state) (e : mir_value) : Z :=
  match e with
  | MVConst n => n
  | MVLocal x => locals x
  | MVPhi seed loop_var =>
      match iter with
      | O => entry seed
      | S _ => carried loop_var
      end
  | MVAdd lhs rhs => eval iter entry carried locals lhs + eval iter entry carried locals rhs
  end.

Definition update (st : state) (x : var) (v : Z) : state :=
  fun y => if String.eqb y x then v else st y.

Inductive mir_instr : Type :=
| MAssign : var -> mir_value -> mir_instr.

Definition instr_write (i : mir_instr) : var :=
  match i with
  | MAssign dst _ => dst
  end.

Definition exec_instr (iter : nat) (entry carried locals : state) (i : mir_instr) : state :=
  match i with
  | MAssign dst rhs => update locals dst (eval iter entry carried locals rhs)
  end.

Fixpoint exec_instrs (iter : nat) (entry carried locals : state) (body : list mir_instr) : state :=
  match body with
  | [] => locals
  | instr :: rest => exec_instrs iter entry carried (exec_instr iter entry carried locals instr) rest
  end.

Definition hoist_safe_over (body : list mir_instr) (e : mir_value) : Prop :=
  carried_deps e = [] /\ Forall (fun instr => ~ In (instr_write instr) (local_deps e)) body.

Lemma eval_update_irrelevant_local :
  forall e iter entry carried locals x val,
    ~ In x (local_deps e) ->
    eval iter entry carried (update locals x val) e = eval iter entry carried locals e.
Proof.
  induction e as [n|y|seed loop_var|lhs IHL rhs IHR];
    intros iter entry carried locals x val Hnotin; simpl in *.
  - reflexivity.
  - unfold update.
    destruct (String.eqb_spec y x).
    + subst.
      exfalso.
      apply Hnotin.
      simpl; auto.
    + reflexivity.
  - reflexivity.
  - assert (Hlhs : ~ In x (local_deps lhs)).
    {
      intro Hin.
      apply Hnotin.
      apply in_or_app.
      left; exact Hin.
    }
    assert (Hrhs : ~ In x (local_deps rhs)).
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

Lemma eval_exec_irrelevant_body :
  forall e iter entry carried locals body,
    Forall (fun instr => ~ In (instr_write instr) (local_deps e)) body ->
    eval iter entry carried (exec_instrs iter entry carried locals body) e =
    eval iter entry carried locals e.
Proof.
  intros e iter entry carried locals body Hall.
  revert locals.
  induction Hall as [| instr rest hHead hTail IH]; intros locals.
  - reflexivity.
  - destruct instr as [dst rhs].
    simpl in *.
    rewrite (IH (update locals dst (eval iter entry carried locals rhs))).
    now apply eval_update_irrelevant_local.
Qed.

Lemma phi_has_carried_dep :
  forall seed carried,
    carried_deps (MVPhi seed carried) = [carried].
Proof.
  reflexivity.
Qed.

Lemma phi_not_safe_to_hoist_over_any_body :
  forall seed carried body,
    ~ hoist_safe_over body (MVPhi seed carried).
Proof.
  intros seed carried body [Hdeps _].
  discriminate Hdeps.
Qed.

Lemma hoist_sound_over_body :
  forall e iter entry carried locals body,
    hoist_safe_over body e ->
    eval iter entry carried (exec_instrs iter entry carried locals body) e =
    eval iter entry carried locals e.
Proof.
  intros e iter entry carried locals body [_ Hwrites].
  exact (eval_exec_irrelevant_body e iter entry carried locals body Hwrites).
Qed.

Lemma phi_plus_local_not_hoistable :
  forall seed carried x body,
    ~ hoist_safe_over body (MVAdd (MVPhi seed carried) (MVLocal x)).
Proof.
  intros seed carried x body [Hdeps _].
  discriminate Hdeps.
Qed.

End RRMirSubsetHoist.
