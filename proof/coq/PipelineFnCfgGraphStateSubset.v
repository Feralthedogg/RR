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

Module RRPipelineFnCfgGraphStateSubset.

Record src_cfg_graph_state : Type := {
  src_graph_current : rvalue;
  src_graph_pc : nat;
  src_graph_table : list src_post_join_step;
}.

Record mir_cfg_graph_state : Type := {
  mir_graph_current : rvalue;
  mir_graph_pc : nat;
  mir_graph_table : list mir_post_join_step;
}.

Record r_cfg_graph_state : Type := {
  r_graph_current : rvalue;
  r_graph_pc : nat;
  r_graph_table : list r_post_join_step;
}.

Record src_fn_cfg_graph_program : Type := {
  src_graph_join_exec_prog : src_fn_cfg_join_exec_program;
  src_graph_table_prog : list src_post_join_step;
}.

Record mir_fn_cfg_graph_program : Type := {
  mir_graph_join_exec_prog : mir_fn_cfg_join_exec_program;
  mir_graph_table_prog : list mir_post_join_step;
}.

Record r_fn_cfg_graph_program : Type := {
  r_graph_join_exec_prog : r_fn_cfg_join_exec_program;
  r_graph_table_prog : list r_post_join_step;
}.

Definition lower_fn_cfg_graph_program
    (p : src_fn_cfg_graph_program) : mir_fn_cfg_graph_program :=
  {| mir_graph_join_exec_prog := lower_fn_cfg_join_exec_program (src_graph_join_exec_prog p);
     mir_graph_table_prog :=
       map (fun step =>
              {| mir_step_bind_name := src_step_bind_name step;
                 mir_step_block := lower_block_env_program (src_step_block step) |})
           (src_graph_table_prog p) |}.

Definition emit_r_fn_cfg_graph_program
    (p : mir_fn_cfg_graph_program) : r_fn_cfg_graph_program :=
  {| r_graph_join_exec_prog := emit_r_fn_cfg_join_exec_program (mir_graph_join_exec_prog p);
     r_graph_table_prog :=
       map (fun step =>
              {| r_step_bind_name := mir_step_bind_name step;
                 r_step_block := emit_r_block_env_program (mir_step_block step) |})
           (mir_graph_table_prog p) |}.

Definition eval_src_fn_cfg_graph_program
    (p : src_fn_cfg_graph_program) (choice : branch_choice) : option rvalue :=
  eval_src_fn_cfg_control_program
    {| src_control_join_exec_prog := src_graph_join_exec_prog p;
       src_control_steps := src_graph_table_prog p |}
    choice.

Definition eval_r_fn_cfg_graph_program
    (p : r_fn_cfg_graph_program) (choice : branch_choice) : option rvalue :=
  eval_r_fn_cfg_control_program
    {| r_control_join_exec_prog := r_graph_join_exec_prog p;
       r_control_steps := r_graph_table_prog p |}
    choice.

Definition branching_fn_cfg_graph_program : src_fn_cfg_graph_program :=
  {| src_graph_join_exec_prog := branching_fn_cfg_join_exec_program;
     src_graph_table_prog := src_control_steps branching_fn_cfg_control_program |}.

Lemma branching_fn_cfg_graph_program_meta_preserved :
  mir_phi_join_bid
      (mir_join_exec_phi_prog
        (mir_graph_join_exec_prog (lower_fn_cfg_graph_program branching_fn_cfg_graph_program))) = 17%nat /\
  List.length (mir_graph_table_prog (lower_fn_cfg_graph_program branching_fn_cfg_graph_program)) = 2%nat.
Proof.
  split; reflexivity.
Qed.

Lemma branching_fn_cfg_graph_program_then_preserved :
  eval_r_fn_cfg_graph_program
    (emit_r_fn_cfg_graph_program (lower_fn_cfg_graph_program branching_fn_cfg_graph_program))
    ThenBranch =
    Some (RVInt 28).
Proof.
  reflexivity.
Qed.

Lemma branching_fn_cfg_graph_program_else_preserved :
  eval_r_fn_cfg_graph_program
    (emit_r_fn_cfg_graph_program (lower_fn_cfg_graph_program branching_fn_cfg_graph_program))
    ElseBranch =
    Some (RVInt 41).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgGraphStateSubset.
