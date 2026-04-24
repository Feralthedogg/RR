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

Module RRPipelineFnCfgIterExecSubset.

Record src_post_join_step : Type := {
  src_step_bind_name : string;
  src_step_block : src_block_env_program;
}.

Record mir_post_join_step : Type := {
  mir_step_bind_name : string;
  mir_step_block : mir_block_env_program;
}.

Record r_post_join_step : Type := {
  r_step_bind_name : string;
  r_step_block : r_block_env_program;
}.

Record src_fn_cfg_iter_exec_program : Type := {
  src_iter_join_exec_prog : src_fn_cfg_join_exec_program;
  src_iter_steps : list src_post_join_step;
}.

Record mir_fn_cfg_iter_exec_program : Type := {
  mir_iter_join_exec_prog : mir_fn_cfg_join_exec_program;
  mir_iter_steps : list mir_post_join_step;
}.

Record r_fn_cfg_iter_exec_program : Type := {
  r_iter_join_exec_prog : r_fn_cfg_join_exec_program;
  r_iter_steps : list r_post_join_step;
}.

Fixpoint eval_src_post_join_steps (steps : list src_post_join_step) (current : rvalue) : option rvalue :=
  match steps with
  | [] => Some current
  | step :: rest =>
      match eval_src_block_env_program
              {| src_block_id := src_block_id (src_step_block step);
                 src_block_in_env := (src_step_bind_name step, current) :: src_block_in_env (src_step_block step);
                 src_block_stmts := src_block_stmts (src_step_block step);
                 src_block_ret := src_block_ret (src_step_block step) |} with
      | Some next => eval_src_post_join_steps rest next
      | None => None
      end
  end.

Fixpoint eval_r_post_join_steps (steps : list r_post_join_step) (current : rvalue) : option rvalue :=
  match steps with
  | [] => Some current
  | step :: rest =>
      match eval_r_block_env_program
              {| r_block_id := r_block_id (r_step_block step);
                 r_block_in_env := (r_step_bind_name step, current) :: r_block_in_env (r_step_block step);
                 r_block_stmts := r_block_stmts (r_step_block step);
                 r_block_ret := r_block_ret (r_step_block step) |} with
      | Some next => eval_r_post_join_steps rest next
      | None => None
      end
  end.

Definition eval_src_fn_cfg_iter_exec_program
    (p : src_fn_cfg_iter_exec_program) (choice : branch_choice) : option rvalue :=
  match eval_src_fn_cfg_join_exec_program (src_iter_join_exec_prog p) choice with
  | Some joined => eval_src_post_join_steps (src_iter_steps p) joined
  | None => None
  end.

Definition eval_r_fn_cfg_iter_exec_program
    (p : r_fn_cfg_iter_exec_program) (choice : branch_choice) : option rvalue :=
  match eval_r_fn_cfg_join_exec_program (r_iter_join_exec_prog p) choice with
  | Some joined => eval_r_post_join_steps (r_iter_steps p) joined
  | None => None
  end.

Definition branching_fn_cfg_iter_exec_program : src_fn_cfg_iter_exec_program :=
  {| src_iter_join_exec_prog := branching_fn_cfg_join_exec_program;
     src_iter_steps :=
       [ {| src_step_bind_name := "joined";
            src_step_block :=
              {| src_block_id := 19%nat;
                 src_block_in_env := [("tail", RVInt 2)];
                 src_block_stmts := [SAssign "tmp3" (SConstInt 5)];
                 src_block_ret := SLAdd (SLVar "joined") (SLAdd (SLVar "tmp3") (SLVar "tail")) |} |};
         {| src_step_bind_name := "after";
            src_step_block :=
              {| src_block_id := 23%nat;
                 src_block_in_env := [("delta", RVInt 3)];
                 src_block_stmts := [SAssign "tmp4" (SConstInt 1)];
                 src_block_ret := SLAdd (SLVar "after") (SLAdd (SLVar "tmp4") (SLVar "delta")) |} |}
       ] |}.

Definition lower_fn_cfg_iter_exec_program
    (p : src_fn_cfg_iter_exec_program) : mir_fn_cfg_iter_exec_program :=
  {| mir_iter_join_exec_prog := lower_fn_cfg_join_exec_program (src_iter_join_exec_prog p);
     mir_iter_steps :=
       map (fun step =>
              {| mir_step_bind_name := src_step_bind_name step;
                 mir_step_block := lower_block_env_program (src_step_block step) |})
           (src_iter_steps p) |}.

Definition emit_r_fn_cfg_iter_exec_program
    (p : mir_fn_cfg_iter_exec_program) : r_fn_cfg_iter_exec_program :=
  {| r_iter_join_exec_prog := emit_r_fn_cfg_join_exec_program (mir_iter_join_exec_prog p);
     r_iter_steps :=
       map (fun step =>
              {| r_step_bind_name := mir_step_bind_name step;
                 r_step_block := emit_r_block_env_program (mir_step_block step) |})
           (mir_iter_steps p) |}.

Lemma branching_fn_cfg_iter_exec_program_meta_preserved :
  mir_phi_join_bid
      (mir_join_exec_phi_prog
        (mir_iter_join_exec_prog (lower_fn_cfg_iter_exec_program branching_fn_cfg_iter_exec_program))) = 17%nat /\
  List.length (mir_iter_steps (lower_fn_cfg_iter_exec_program branching_fn_cfg_iter_exec_program)) = 2%nat.
Proof.
  split; reflexivity.
Qed.

Lemma branching_fn_cfg_iter_exec_program_then_preserved :
  eval_r_fn_cfg_iter_exec_program
    (emit_r_fn_cfg_iter_exec_program (lower_fn_cfg_iter_exec_program branching_fn_cfg_iter_exec_program))
    ThenBranch =
    Some (RVInt 28).
Proof.
  reflexivity.
Qed.

Lemma branching_fn_cfg_iter_exec_program_else_preserved :
  eval_r_fn_cfg_iter_exec_program
    (emit_r_fn_cfg_iter_exec_program (lower_fn_cfg_iter_exec_program branching_fn_cfg_iter_exec_program))
    ElseBranch =
    Some (RVInt 41).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgIterExecSubset.
