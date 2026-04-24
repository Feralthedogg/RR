Require Import MirSubsetHoist.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Lia.
From Stdlib Require Import FunctionalExtensionality.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRMirSubsetHoist.

Module RRMiniFnHoist.

Record mini_hoist_case : Type := {
  tmp : var;
  cand : mir_value;
  body : list mir_instr;
}.

Definition safe_to_hoist_case (c : mini_hoist_case) : Prop :=
  carried_deps (cand c) = [] /\
  Forall (fun instr => ~ In (instr_write instr) (local_deps (cand c)) /\ instr_write instr <> tmp c) (body c) /\
  ~ In (tmp c) (local_deps (cand c)).

Definition original_next_header_value (c : mini_hoist_case) (entry locals : state) : Z :=
  let post := exec_instrs 1 entry locals locals (body c) in
  eval 1 entry post post (cand c).

Definition hoisted_value_after_body (c : mini_hoist_case) (entry locals : state) : Z :=
  let hoisted := eval 1 entry locals locals (cand c) in
  let post := exec_instrs 1 entry locals (update locals (tmp c) hoisted) (body c) in
  post (tmp c).

Lemma forall_fst_proj :
  forall (P Q : mir_instr -> Prop) xs,
    Forall (fun x => P x /\ Q x) xs ->
    Forall P xs.
Proof.
  intros P Q xs H.
  induction H.
  - constructor.
  - constructor.
    + exact (proj1 H).
    + exact IHForall.
Qed.

Lemma forall_snd_proj :
  forall (P Q : mir_instr -> Prop) xs,
    Forall (fun x => P x /\ Q x) xs ->
    Forall Q xs.
Proof.
  intros P Q xs H.
  induction H.
  - constructor.
  - constructor.
    + exact (proj2 H).
    + exact IHForall.
Qed.

Lemma eval_irrelevant_carried :
  forall e iter entry carried1 carried2 locals,
    carried_deps e = [] ->
    eval iter entry carried1 locals e = eval iter entry carried2 locals e.
Proof.
  induction e as [n|x|seed carried|lhs IHL rhs IHR];
    intros iter entry carried1 carried2 locals Hdeps; simpl in *.
  - reflexivity.
  - reflexivity.
  - discriminate Hdeps.
  - apply app_eq_nil in Hdeps as [Hlhs Hrhs].
    simpl.
    rewrite (IHL iter entry carried1 carried2 locals Hlhs).
    rewrite (IHR iter entry carried1 carried2 locals Hrhs).
    reflexivity.
Qed.

Lemma exec_instrs_preserve_unwritten_var :
  forall iter entry carried locals body x,
    Forall (fun instr => instr_write instr <> x) body ->
    exec_instrs iter entry carried locals body x = locals x.
Proof.
  intros iter entry carried locals body x Hall.
  induction Hall in locals |- *.
  - reflexivity.
  - destruct x0 as [dst rhs].
    simpl in *.
    rewrite IHHall.
    unfold exec_instr, update.
    destruct (String.eqb_spec x dst).
    + subst. contradiction.
    + reflexivity.
Qed.

Lemma hoist_safe_case_sound :
  forall c entry locals,
    safe_to_hoist_case c ->
    original_next_header_value c entry locals = hoisted_value_after_body c entry locals.
Proof.
  intros c entry locals [Hcarried [Hwrites HtmpFresh]].
  unfold original_next_header_value, hoisted_value_after_body.
  set (post := exec_instrs 1 entry locals locals (body c)).
  set (pre_val := eval 1 entry locals locals (cand c)).
  assert (Hcarry :
      eval 1 entry post post (cand c) = eval 1 entry locals post (cand c)).
  {
    apply eval_irrelevant_carried.
    exact Hcarried.
  }
  assert (HlocalWrites :
      Forall (fun instr => ~ In (instr_write instr) (local_deps (cand c))) (body c)).
  {
    apply forall_fst_proj with
      (Q := fun instr => instr_write instr <> tmp c).
    exact Hwrites.
  }
  assert (Hlocals :
      eval 1 entry locals post (cand c) = eval 1 entry locals locals (cand c)).
  {
    subst post.
    apply eval_exec_irrelevant_body.
    exact HlocalWrites.
  }
  assert (HtmpWrites :
      Forall (fun instr => instr_write instr <> tmp c) (body c)).
  {
    apply forall_snd_proj with
      (P := fun instr => ~ In (instr_write instr) (local_deps (cand c))).
    exact Hwrites.
  }
  assert (Htmp :
      exec_instrs 1 entry locals (update locals (tmp c) pre_val) (body c) (tmp c) = pre_val).
  {
    subst pre_val.
    rewrite exec_instrs_preserve_unwritten_var by exact HtmpWrites.
    unfold update.
    rewrite String.eqb_refl.
    reflexivity.
  }
  rewrite Hcarry.
  rewrite Hlocals.
  exact (eq_sym Htmp).
Qed.

Definition time_bump_body : list mir_instr :=
  [MAssign "time" (MVAdd (MVLocal "time") (MVConst 1))].

Definition phi_time_case : mini_hoist_case :=
  {| tmp := "licm_time"; cand := MVPhi "time0" "time"; body := time_bump_body |}.

Lemma exec_time_bump_time :
  forall entry locals,
    exec_instrs 1 entry locals locals time_bump_body "time" = locals "time" + 1.
Proof.
  intros entry locals.
  unfold time_bump_body.
  vm_compute.
  reflexivity.
Qed.

Lemma exec_time_bump_tmp :
  forall entry locals tmp_val,
    exec_instrs 1 entry locals (update locals "licm_time" tmp_val) time_bump_body "licm_time" =
    tmp_val.
Proof.
  intros entry locals tmp_val.
  unfold time_bump_body.
  vm_compute.
  reflexivity.
Qed.

Lemma phi_time_case_not_safe :
  ~ safe_to_hoist_case phi_time_case.
Proof.
  intros [Hdeps _].
  discriminate Hdeps.
Qed.

Lemma phi_time_case_unsound :
  forall entry locals,
    locals "time" + 1 <> locals "time" ->
    original_next_header_value phi_time_case entry locals <>
    hoisted_value_after_body phi_time_case entry locals.
Proof.
  intros entry locals Hneq Heq.
  assert (Hlhs : original_next_header_value phi_time_case entry locals = locals "time" + 1).
  {
    unfold original_next_header_value, phi_time_case.
    simpl.
    exact (exec_time_bump_time entry locals).
  }
  assert (Hrhs : hoisted_value_after_body phi_time_case entry locals = locals "time").
  {
    unfold hoisted_value_after_body, phi_time_case.
    simpl.
    exact (exec_time_bump_tmp entry locals (locals "time")).
  }
  rewrite Hlhs in Heq.
  rewrite Hrhs in Heq.
  exact (Hneq Heq).
Qed.

End RRMiniFnHoist.
