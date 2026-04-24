Require Import PipelineFnCfgLoopFixpointSubset.
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
Import RRPipelineFnCfgLoopFixpointSubset.
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

Module RRPipelineFnCfgLoopDiscoverSubset.

Record src_fn_cfg_loop_discover_program : Type := {
  src_discover_cycle_prog : src_fn_cfg_loop_cycle_program;
  src_discover_choice : branch_choice;
  src_discover_worklist : list rvalue;
  src_discover_selected : rvalue;
}.

Record mir_fn_cfg_loop_discover_program : Type := {
  mir_discover_cycle_prog : mir_fn_cfg_loop_cycle_program;
  mir_discover_choice : branch_choice;
  mir_discover_worklist : list rvalue;
  mir_discover_selected : rvalue;
}.

Record r_fn_cfg_loop_discover_program : Type := {
  r_discover_cycle_prog : r_fn_cfg_loop_cycle_program;
  r_discover_choice : branch_choice;
  r_discover_worklist : list rvalue;
  r_discover_selected : rvalue;
}.

Definition r_loop_discover_witness (p : r_fn_cfg_loop_discover_program) : Prop :=
  List.In (r_discover_selected p) (r_discover_worklist p) /\
  r_loop_fixpoint_witness
    {| r_fix_cycle_prog := r_discover_cycle_prog p;
       r_fix_choice := r_discover_choice p;
       r_fix_stable := r_discover_selected p |}.

Definition lower_fn_cfg_loop_discover_program
    (p : src_fn_cfg_loop_discover_program) : mir_fn_cfg_loop_discover_program :=
  {| mir_discover_cycle_prog := lower_fn_cfg_loop_cycle_program (src_discover_cycle_prog p);
     mir_discover_choice := src_discover_choice p;
     mir_discover_worklist := src_discover_worklist p;
     mir_discover_selected := src_discover_selected p |}.

Definition emit_r_fn_cfg_loop_discover_program
    (p : mir_fn_cfg_loop_discover_program) : r_fn_cfg_loop_discover_program :=
  {| r_discover_cycle_prog := emit_r_fn_cfg_loop_cycle_program (mir_discover_cycle_prog p);
     r_discover_choice := mir_discover_choice p;
     r_discover_worklist := mir_discover_worklist p;
     r_discover_selected := mir_discover_selected p |}.

Definition stable_fn_cfg_loop_discover_program : src_fn_cfg_loop_discover_program :=
  {| src_discover_cycle_prog := stable_fn_cfg_loop_cycle_program;
     src_discover_choice := ThenBranch;
     src_discover_worklist := [RVInt 7; RVInt 10; RVInt 12];
     src_discover_selected := RVInt 10 |}.

Lemma stable_fn_cfg_loop_discover_program_meta_preserved :
  mir_phi_join_bid
      (mir_join_exec_phi_prog
        (mir_reentry_join_exec_prog
          (mir_loop_reentry_prog
            (mir_discover_cycle_prog (lower_fn_cfg_loop_discover_program stable_fn_cfg_loop_discover_program))))) = 17%nat /\
  mir_discover_choice (lower_fn_cfg_loop_discover_program stable_fn_cfg_loop_discover_program) = ThenBranch /\
  mir_discover_worklist (lower_fn_cfg_loop_discover_program stable_fn_cfg_loop_discover_program) =
    [RVInt 7; RVInt 10; RVInt 12] /\
  mir_discover_selected (lower_fn_cfg_loop_discover_program stable_fn_cfg_loop_discover_program) = RVInt 10.
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_loop_discover_program_preserved :
  r_loop_discover_witness
    (emit_r_fn_cfg_loop_discover_program (lower_fn_cfg_loop_discover_program stable_fn_cfg_loop_discover_program)).
Proof.
  split.
  - simpl. tauto.
  - exact stable_fn_cfg_loop_fixpoint_program_preserved.
Qed.

End RRPipelineFnCfgLoopDiscoverSubset.
