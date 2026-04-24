Require Import MirSubsetHoist.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Lia.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRMirSubsetHoist.

Module RRCfgHoist.

Record loop_cfg : Type := {
  tmp : var;
  cand : mir_value;
  body : list mir_instr;
}.

Definition safe_to_hoist_cfg (c : loop_cfg) : Prop :=
  carried_deps (cand c) = [] /\
  Forall (fun instr => ~ In (instr_write instr) (local_deps (cand c)) /\ instr_write instr <> tmp c) (body c) /\
  ~ In (tmp c) (local_deps (cand c)).

Definition pre_val (c : loop_cfg) (entry locals : state) : Z :=
  eval 0 entry locals locals (cand c).

Definition post_original (c : loop_cfg) (entry locals : state) : state :=
  exec_instrs 1 entry locals locals (body c).

Definition post_hoisted (c : loop_cfg) (entry locals : state) : state :=
  exec_instrs 1 entry locals (update locals (tmp c) (pre_val c entry locals)) (body c).

Definition run_original (c : loop_cfg) (entered : bool) (entry locals : state) : Z :=
  if entered then
    let post := post_original c entry locals in
    eval 1 entry post post (cand c)
  else
    pre_val c entry locals.

Definition run_hoisted (c : loop_cfg) (entered : bool) (entry locals : state) : Z :=
  if entered then
    post_hoisted c entry locals (tmp c)
  else
    pre_val c entry locals.

Lemma forall_fst_proj_cfg :
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

Lemma forall_snd_proj_cfg :
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

Lemma eval_iter_irrelevant_no_carried :
  forall e i j entry carried locals,
    carried_deps e = [] ->
    eval i entry carried locals e = eval j entry carried locals e.
Proof.
  induction e as [n|x|seed carriedVar|lhs IHL rhs IHR];
    intros i j entry carried locals Hdeps; simpl in *.
  - reflexivity.
  - reflexivity.
  - discriminate Hdeps.
  - apply app_eq_nil in Hdeps as [Hlhs Hrhs].
    simpl.
    rewrite (IHL i j entry carried locals Hlhs).
    rewrite (IHR i j entry carried locals Hrhs).
    reflexivity.
Qed.

Lemma eval_carried_irrelevant_no_carried :
  forall e iter entry carried1 carried2 locals,
    carried_deps e = [] ->
    eval iter entry carried1 locals e = eval iter entry carried2 locals e.
Proof.
  induction e as [n|x|seed carriedVar|lhs IHL rhs IHR];
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

Lemma exec_instrs_preserve_unwritten_var_cfg :
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

Lemma run_original_false_eq_run_hoisted_false :
  forall c entry locals,
    run_original c false entry locals = run_hoisted c false entry locals.
Proof.
  reflexivity.
Qed.

Lemma run_original_true_eq_run_hoisted_true :
  forall c entry locals,
    safe_to_hoist_cfg c ->
    run_original c true entry locals = run_hoisted c true entry locals.
Proof.
  intros c entry locals [Hcarried [Hwrites HtmpFresh]].
  unfold run_original, run_hoisted.
  simpl.
  set (post := exec_instrs 1 entry locals locals (body c)).
  assert (HlocalWrites :
      Forall (fun instr => ~ In (instr_write instr) (local_deps (cand c))) (body c)).
  {
    apply forall_fst_proj_cfg with
      (Q := fun instr => instr_write instr <> tmp c).
    exact Hwrites.
  }
  assert (HtmpWrites :
      Forall (fun instr => instr_write instr <> tmp c) (body c)).
  {
    apply forall_snd_proj_cfg with
      (P := fun instr => ~ In (instr_write instr) (local_deps (cand c))).
    exact Hwrites.
  }
  assert (Hiter :
      eval 1 entry post post (cand c) = eval 0 entry post post (cand c)).
  {
    apply eval_iter_irrelevant_no_carried.
    exact Hcarried.
  }
  assert (Hcarry :
      eval 0 entry post post (cand c) = eval 0 entry locals post (cand c)).
  {
    apply eval_carried_irrelevant_no_carried.
    exact Hcarried.
  }
  assert (Hlocals_post_iter :
      eval 0 entry locals post (cand c) = eval 1 entry locals post (cand c)).
  {
    symmetry.
    apply eval_iter_irrelevant_no_carried.
    exact Hcarried.
  }
  assert (Hlocals_exec :
      eval 1 entry locals post (cand c) = eval 1 entry locals locals (cand c)).
  {
    apply eval_exec_irrelevant_body.
    exact HlocalWrites.
  }
  assert (Hlocals_base :
      eval 1 entry locals locals (cand c) = eval 0 entry locals locals (cand c)).
  {
    apply eval_iter_irrelevant_no_carried.
    exact Hcarried.
  }
  assert (Htmp :
      exec_instrs 1 entry locals (update locals (tmp c) (pre_val c entry locals)) (body c) (tmp c) =
      pre_val c entry locals).
  {
    unfold pre_val.
    rewrite exec_instrs_preserve_unwritten_var_cfg by exact HtmpWrites.
    unfold update.
    rewrite String.eqb_refl.
    reflexivity.
  }
  unfold run_original, run_hoisted.
  simpl.
  etransitivity.
  - exact Hiter.
  - etransitivity.
    + exact Hcarry.
    + etransitivity.
      * exact Hlocals_post_iter.
      * etransitivity.
        { exact Hlocals_exec. }
        { etransitivity.
          - exact Hlocals_base.
          - exact (eq_sym Htmp). }
Qed.

Definition phi_time_cfg : loop_cfg :=
  {| tmp := "licm_time";
     cand := MVPhi "time0" "time";
     body := [MAssign "time" (MVAdd (MVLocal "time") (MVConst 1))] |}.

Lemma exec_phi_time_body_time :
  forall entry locals,
    post_original phi_time_cfg entry locals "time" = locals "time" + 1.
Proof.
  intros entry locals.
  unfold post_original, phi_time_cfg.
  vm_compute.
  reflexivity.
Qed.

Lemma exec_phi_time_body_tmp :
  forall entry locals,
    post_hoisted phi_time_cfg entry locals "licm_time" = entry "time0".
Proof.
  intros entry locals.
  unfold post_hoisted, phi_time_cfg, pre_val.
  vm_compute.
  reflexivity.
Qed.

Lemma phi_time_cfg_not_safe :
  ~ safe_to_hoist_cfg phi_time_cfg.
Proof.
  intros [Hdeps _].
  discriminate Hdeps.
Qed.

Lemma phi_time_cfg_true_trip_unsound :
  forall entry locals,
    locals "time" + 1 <> entry "time0" ->
    run_original phi_time_cfg true entry locals <>
    run_hoisted phi_time_cfg true entry locals.
Proof.
  intros entry locals Hneq Heq.
  unfold run_original, run_hoisted in Heq.
  simpl in Heq.
  rewrite exec_phi_time_body_time in Heq.
  rewrite exec_phi_time_body_tmp in Heq.
  exact (Hneq Heq).
Qed.

End RRCfgHoist.
