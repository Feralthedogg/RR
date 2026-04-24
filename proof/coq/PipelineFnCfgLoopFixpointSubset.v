Require Import PipelineFnCfgLoopCycleSubset.
Require Import PipelineFnCfgReentrySubset.
Require Import PipelineFnCfgGraphStateSubset.
Require Import PipelineFnCfgControlStateSubset.
Require Import PipelineFnCfgIterExecSubset.
Require Import PipelineFnCfgPostJoinSubset.
Require Import PipelineFnCfgJoinExecSubset.
Require Import PipelineFnCfgJoinStateSubset.
Require Import PipelineFnCfgPhiExecSubset.
Require Import PipelineFnCfgBranchExecSubset.
Require Import PipelineFnCfgExecSubset.
Require Import PipelineFnCfgSubset.
Require Import PipelineFnEnvSubset.
Require Import PipelineBlockEnvSubset.
Require Import PipelineStmtSubset.
Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRPipelineFnCfgLoopCycleSubset.
Import RRPipelineFnCfgReentrySubset.
Import RRPipelineFnCfgGraphStateSubset.
Import RRPipelineFnCfgControlStateSubset.
Import RRPipelineFnCfgIterExecSubset.
Import RRPipelineFnCfgPostJoinSubset.
Import RRPipelineFnCfgJoinExecSubset.
Import RRPipelineFnCfgJoinStateSubset.
Import RRPipelineFnCfgPhiExecSubset.
Import RRPipelineFnCfgBranchExecSubset.
Import RRPipelineFnCfgExecSubset.
Import RRPipelineFnCfgSubset.
Import RRPipelineFnEnvSubset.
Import RRPipelineBlockEnvSubset.
Import RRPipelineStmtSubset.

Module RRPipelineFnCfgLoopFixpointSubset.

Record src_fn_cfg_loop_fixpoint_program : Type := {
  src_fix_cycle_prog : src_fn_cfg_loop_cycle_program;
  src_fix_choice : branch_choice;
  src_fix_stable : rvalue;
}.

Record mir_fn_cfg_loop_fixpoint_program : Type := {
  mir_fix_cycle_prog : mir_fn_cfg_loop_cycle_program;
  mir_fix_choice : branch_choice;
  mir_fix_stable : rvalue;
}.

Record r_fn_cfg_loop_fixpoint_program : Type := {
  r_fix_cycle_prog : r_fn_cfg_loop_cycle_program;
  r_fix_choice : branch_choice;
  r_fix_stable : rvalue;
}.

Definition src_loop_fixpoint_witness (p : src_fn_cfg_loop_fixpoint_program) : Prop :=
  eval_src_fn_cfg_loop_cycle_program (src_fix_cycle_prog p) = Some (src_fix_stable p) /\
  eval_src_loop_choices (src_fix_cycle_prog p) [src_fix_choice p] (src_fix_stable p) = Some (src_fix_stable p).

Definition r_loop_fixpoint_witness (p : r_fn_cfg_loop_fixpoint_program) : Prop :=
  eval_r_fn_cfg_loop_cycle_program (r_fix_cycle_prog p) = Some (r_fix_stable p) /\
  eval_r_loop_choices (r_fix_cycle_prog p) [r_fix_choice p] (r_fix_stable p) = Some (r_fix_stable p).

Definition lower_fn_cfg_loop_fixpoint_program
    (p : src_fn_cfg_loop_fixpoint_program) : mir_fn_cfg_loop_fixpoint_program :=
  {| mir_fix_cycle_prog := lower_fn_cfg_loop_cycle_program (src_fix_cycle_prog p);
     mir_fix_choice := src_fix_choice p;
     mir_fix_stable := src_fix_stable p |}.

Definition emit_r_fn_cfg_loop_fixpoint_program
    (p : mir_fn_cfg_loop_fixpoint_program) : r_fn_cfg_loop_fixpoint_program :=
  {| r_fix_cycle_prog := emit_r_fn_cfg_loop_cycle_program (mir_fix_cycle_prog p);
     r_fix_choice := mir_fix_choice p;
     r_fix_stable := mir_fix_stable p |}.

Definition stable_fn_cfg_loop_cycle_program : src_fn_cfg_loop_cycle_program :=
  {| src_loop_reentry_prog := branching_fn_cfg_reentry_program;
     src_loop_acc_name := "acc";
     src_loop_cycle_name := "cycle";
     src_loop_init := RVInt 10;
     src_loop_choices := [ThenBranch; ElseBranch];
     src_loop_block :=
       {| src_block_id := 37%nat;
          src_block_in_env := [];
          src_block_stmts := [];
          src_block_ret := SLVar "acc" |} |}.

Definition stable_fn_cfg_loop_fixpoint_program : src_fn_cfg_loop_fixpoint_program :=
  {| src_fix_cycle_prog := stable_fn_cfg_loop_cycle_program;
     src_fix_choice := ThenBranch;
     src_fix_stable := RVInt 10 |}.

Lemma stable_fn_cfg_loop_fixpoint_program_meta_preserved :
  mir_phi_join_bid
      (mir_join_exec_phi_prog
        (mir_reentry_join_exec_prog
          (mir_loop_reentry_prog
            (mir_fix_cycle_prog (lower_fn_cfg_loop_fixpoint_program stable_fn_cfg_loop_fixpoint_program))))) = 17%nat /\
  mir_fix_choice (lower_fn_cfg_loop_fixpoint_program stable_fn_cfg_loop_fixpoint_program) = ThenBranch /\
  mir_fix_stable (lower_fn_cfg_loop_fixpoint_program stable_fn_cfg_loop_fixpoint_program) = RVInt 10.
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_loop_fixpoint_program_preserved :
  r_loop_fixpoint_witness
    (emit_r_fn_cfg_loop_fixpoint_program (lower_fn_cfg_loop_fixpoint_program stable_fn_cfg_loop_fixpoint_program)).
Proof.
  split; reflexivity.
Qed.

End RRPipelineFnCfgLoopFixpointSubset.
