Require Import PipelineFnCfgExecSubset.
Require Import PipelineFnCfgSubset.
Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRPipelineFnCfgExecSubset.
Import RRPipelineFnCfgSubset.

Module RRPipelineFnCfgSmallStepSubset.

Record trace_machine : Type := {
  tm_cursor : nat;
  tm_trace : list (nat * option rvalue);
}.

Definition initial_trace_machine : trace_machine :=
  {| tm_cursor := 0%nat; tm_trace := [] |}.

Definition step_trace_machine
    (results : list (nat * option rvalue)) (path : list nat)
    (m : trace_machine) : trace_machine :=
  match nth_error path (tm_cursor m) with
  | Some bid =>
      {| tm_cursor := S (tm_cursor m);
         tm_trace := tm_trace m ++ [(bid, lookup_fn_block_result results bid)] |}
  | None => m
  end.

Fixpoint run_trace_machine
    (results : list (nat * option rvalue)) (path : list nat)
    (fuel : nat) (m : trace_machine) : trace_machine :=
  match fuel with
  | O => m
  | S fuel' => run_trace_machine results path fuel' (step_trace_machine results path m)
  end.

Definition run_src_fn_cfg_machine (p : src_fn_cfg_exec_program) : trace_machine :=
  run_trace_machine (eval_src_fn_cfg_exec_program p)
    (src_exec_path p) (List.length (src_exec_path p)) initial_trace_machine.

Definition run_r_fn_cfg_machine (p : r_fn_cfg_exec_program) : trace_machine :=
  run_trace_machine (eval_r_fn_cfg_exec_program p)
    (r_exec_path p) (List.length (r_exec_path p)) initial_trace_machine.

Lemma two_block_fn_cfg_exec_program_small_step_preserved :
  tm_trace
    (run_r_fn_cfg_machine (emit_r_fn_cfg_exec_program (lower_fn_cfg_exec_program two_block_fn_cfg_exec_program))) =
    [(7%nat, Some (RVInt 7)); (11%nat, Some (RVInt 12))].
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgSmallStepSubset.
