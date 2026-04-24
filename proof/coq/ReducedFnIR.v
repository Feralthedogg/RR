Require Import MirSubsetHoist.
Require Import CfgHoist.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRMirSubsetHoist.
Import RRCfgHoist.

Module RRReducedFnIR.

Inductive block_id : Type :=
| BPreheader
| BHeader
| BBody
| BExit.

Inductive terminator : Type :=
| TGoto : block_id -> terminator
| TBranchOnEntered : block_id -> block_id -> terminator
| TRetCand : terminator
| TRetTmp : var -> terminator.

Record block : Type := {
  block_name : block_id;
  instrs : list mir_instr;
  term : terminator;
}.

Record reduced_fn_ir : Type := {
  tmp_name : var;
  cand_val : mir_value;
  body_instrs : list mir_instr;
}.

Definition to_cfg (f : reduced_fn_ir) : loop_cfg :=
  {| tmp := tmp_name f; cand := cand_val f; body := body_instrs f |}.

Definition preheader (f : reduced_fn_ir) : block :=
  {| block_name := BPreheader; instrs := []; term := TGoto BHeader |}.

Definition header (f : reduced_fn_ir) : block :=
  {| block_name := BHeader; instrs := []; term := TBranchOnEntered BBody BExit |}.

Definition body_original (f : reduced_fn_ir) : block :=
  {| block_name := BBody; instrs := body_instrs f; term := TRetCand |}.

Definition body_hoisted (f : reduced_fn_ir) (entry locals : state) : block :=
  {| block_name := BBody;
     instrs := MAssign (tmp_name f) (MVConst (pre_val (to_cfg f) entry locals)) :: body_instrs f;
     term := TRetTmp (tmp_name f) |}.

Definition exit_block (f : reduced_fn_ir) : block :=
  {| block_name := BExit; instrs := []; term := TRetCand |}.

Definition blocks_original (f : reduced_fn_ir) : list block :=
  [preheader f; header f; body_original f; exit_block f].

Definition blocks_hoisted (f : reduced_fn_ir) (entry locals : state) : list block :=
  [preheader f; header f; body_hoisted f entry locals; exit_block f].

Definition run_original_fn (f : reduced_fn_ir) (entered : bool) (entry locals : state) : Z :=
  run_original (to_cfg f) entered entry locals.

Definition run_hoisted_fn (f : reduced_fn_ir) (entered : bool) (entry locals : state) : Z :=
  run_hoisted (to_cfg f) entered entry locals.

Lemma reduced_fn_ir_zero_trip_sound :
  forall f entry locals,
    run_original_fn f false entry locals = run_hoisted_fn f false entry locals.
Proof.
  intros f entry locals.
  unfold run_original_fn, run_hoisted_fn.
  apply run_original_false_eq_run_hoisted_false.
Qed.

Lemma reduced_fn_ir_one_trip_sound :
  forall f entry locals,
    safe_to_hoist_cfg (to_cfg f) ->
    run_original_fn f true entry locals = run_hoisted_fn f true entry locals.
Proof.
  intros f entry locals Hsafe.
  unfold run_original_fn, run_hoisted_fn.
  apply run_original_true_eq_run_hoisted_true.
  exact Hsafe.
Qed.

Definition reduced_phi_time_fn : reduced_fn_ir :=
  {| tmp_name := tmp phi_time_cfg;
     cand_val := cand phi_time_cfg;
     body_instrs := body phi_time_cfg |}.

Lemma reduced_phi_time_fn_not_safe :
  ~ safe_to_hoist_cfg (to_cfg reduced_phi_time_fn).
Proof.
  exact phi_time_cfg_not_safe.
Qed.

Lemma reduced_phi_time_fn_unsound :
  forall entry locals,
    locals "time" + 1 <> entry "time0" ->
    run_original_fn reduced_phi_time_fn true entry locals <>
    run_hoisted_fn reduced_phi_time_fn true entry locals.
Proof.
  intros entry locals Hneq.
  unfold run_original_fn, run_hoisted_fn.
  exact (phi_time_cfg_true_trip_unsound entry locals Hneq).
Qed.

End RRReducedFnIR.
