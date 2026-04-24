Require Import MirSubsetHoist.
Require Import CfgHoist.
Require Import ReducedFnIR.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Open Scope string_scope.
Open Scope Z_scope.
Import RRMirSubsetHoist.
Import RRCfgHoist.
Import RRReducedFnIR.

Module RRCfgSmallStep.

Inductive pc : Type :=
| PCPreheader
| PCHeader
| PCBody
| PCExit
| PCHalted.

Record machine : Type := {
  pc_of : pc;
  locals_of : state;
  result_of : option Z;
}.

Definition initial_machine (locals : state) : machine :=
  {| pc_of := PCPreheader; locals_of := locals; result_of := None |}.

Definition step_original (f : reduced_fn_ir) (entered : bool) (entry : state) (m : machine) : machine :=
  match pc_of m with
  | PCPreheader =>
      {| pc_of := PCHeader; locals_of := locals_of m; result_of := result_of m |}
  | PCHeader =>
      {| pc_of := if entered then PCBody else PCExit; locals_of := locals_of m; result_of := result_of m |}
  | PCBody =>
      let post := post_original (to_cfg f) entry (locals_of m) in
      {| pc_of := PCHalted;
         locals_of := post;
         result_of := Some (eval 1 entry post post (cand_val f)) |}
  | PCExit =>
      {| pc_of := PCHalted;
         locals_of := locals_of m;
         result_of := Some (pre_val (to_cfg f) entry (locals_of m)) |}
  | PCHalted => m
  end.

Definition step_hoisted (f : reduced_fn_ir) (entered : bool) (entry : state) (m : machine) : machine :=
  match pc_of m with
  | PCPreheader =>
      {| pc_of := PCHeader; locals_of := locals_of m; result_of := result_of m |}
  | PCHeader =>
      {| pc_of := if entered then PCBody else PCExit; locals_of := locals_of m; result_of := result_of m |}
  | PCBody =>
      let post := post_hoisted (to_cfg f) entry (locals_of m) in
      {| pc_of := PCHalted;
         locals_of := post;
         result_of := Some (post (tmp_name f)) |}
  | PCExit =>
      {| pc_of := PCHalted;
         locals_of := locals_of m;
         result_of := Some (pre_val (to_cfg f) entry (locals_of m)) |}
  | PCHalted => m
  end.

Fixpoint run_steps (step : machine -> machine) (fuel : nat) (m : machine) : machine :=
  match fuel with
  | O => m
  | S n => run_steps step n (step m)
  end.

Definition run_original_machine (f : reduced_fn_ir) (entered : bool) (entry locals : state) : machine :=
  run_steps (step_original f entered entry) 3 (initial_machine locals).

Definition run_hoisted_machine (f : reduced_fn_ir) (entered : bool) (entry locals : state) : machine :=
  run_steps (step_hoisted f entered entry) 3 (initial_machine locals).

Lemma zero_trip_machine_original :
  forall f entry locals,
    result_of (run_original_machine f false entry locals) =
    Some (run_original_fn f false entry locals).
Proof.
  intros f entry locals.
  reflexivity.
Qed.

Lemma zero_trip_machine_hoisted :
  forall f entry locals,
    result_of (run_hoisted_machine f false entry locals) =
    Some (run_hoisted_fn f false entry locals).
Proof.
  intros f entry locals.
  reflexivity.
Qed.

Lemma one_trip_machine_original :
  forall f entry locals,
    result_of (run_original_machine f true entry locals) =
    Some (run_original_fn f true entry locals).
Proof.
  intros f entry locals.
  reflexivity.
Qed.

Lemma one_trip_machine_hoisted :
  forall f entry locals,
    result_of (run_hoisted_machine f true entry locals) =
    Some (run_hoisted_fn f true entry locals).
Proof.
  intros f entry locals.
  reflexivity.
Qed.

Lemma small_step_zero_trip_sound :
  forall f entry locals,
    result_of (run_original_machine f false entry locals) =
    result_of (run_hoisted_machine f false entry locals).
Proof.
  intros f entry locals.
  rewrite zero_trip_machine_original.
  rewrite zero_trip_machine_hoisted.
  now rewrite reduced_fn_ir_zero_trip_sound.
Qed.

Lemma small_step_one_trip_sound :
  forall f entry locals,
    safe_to_hoist_cfg (to_cfg f) ->
    result_of (run_original_machine f true entry locals) =
    result_of (run_hoisted_machine f true entry locals).
Proof.
  intros f entry locals Hsafe.
  rewrite one_trip_machine_original.
  rewrite one_trip_machine_hoisted.
  now rewrite reduced_fn_ir_one_trip_sound by exact Hsafe.
Qed.

Lemma small_step_phi_time_unsound :
  forall entry locals,
    locals "time" + 1 <> entry "time0" ->
    result_of (run_original_machine reduced_phi_time_fn true entry locals) <>
    result_of (run_hoisted_machine reduced_phi_time_fn true entry locals).
Proof.
  intros entry locals Hneq.
  rewrite one_trip_machine_original.
  rewrite one_trip_machine_hoisted.
  intro Heq.
  inversion Heq.
  apply (reduced_phi_time_fn_unsound entry locals Hneq).
  assumption.
Qed.

End RRCfgSmallStep.
