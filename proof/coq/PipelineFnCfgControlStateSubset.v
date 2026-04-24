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

Module RRPipelineFnCfgControlStateSubset.

Record src_cfg_control_state : Type := {
  src_control_current : rvalue;
  src_control_remaining : list src_post_join_step;
}.

Record mir_cfg_control_state : Type := {
  mir_control_current : rvalue;
  mir_control_remaining : list mir_post_join_step;
}.

Record r_cfg_control_state : Type := {
  r_control_current : rvalue;
  r_control_remaining : list r_post_join_step;
}.

Record src_fn_cfg_control_program : Type := {
  src_control_join_exec_prog : src_fn_cfg_join_exec_program;
  src_control_steps : list src_post_join_step;
}.

Record mir_fn_cfg_control_program : Type := {
  mir_control_join_exec_prog : mir_fn_cfg_join_exec_program;
  mir_control_steps : list mir_post_join_step;
}.

Record r_fn_cfg_control_program : Type := {
  r_control_join_exec_prog : r_fn_cfg_join_exec_program;
  r_control_steps : list r_post_join_step;
}.

Fixpoint run_src_cfg_control (fuel : nat) (st : src_cfg_control_state) : option rvalue :=
  match fuel with
  | O =>
      match src_control_remaining st with
      | [] => Some (src_control_current st)
      | _ => None
      end
  | S fuel' =>
      match src_control_remaining st with
      | [] => Some (src_control_current st)
      | step :: rest =>
          match eval_src_post_join_steps [step] (src_control_current st) with
          | Some next =>
              run_src_cfg_control fuel'
                {| src_control_current := next; src_control_remaining := rest |}
          | None => None
          end
      end
  end.

Fixpoint run_r_cfg_control (fuel : nat) (st : r_cfg_control_state) : option rvalue :=
  match fuel with
  | O =>
      match r_control_remaining st with
      | [] => Some (r_control_current st)
      | _ => None
      end
  | S fuel' =>
      match r_control_remaining st with
      | [] => Some (r_control_current st)
      | step :: rest =>
          match eval_r_post_join_steps [step] (r_control_current st) with
          | Some next =>
              run_r_cfg_control fuel'
                {| r_control_current := next; r_control_remaining := rest |}
          | None => None
          end
      end
  end.

Definition lower_fn_cfg_control_program
    (p : src_fn_cfg_control_program) : mir_fn_cfg_control_program :=
  {| mir_control_join_exec_prog := lower_fn_cfg_join_exec_program (src_control_join_exec_prog p);
     mir_control_steps :=
       map (fun step =>
              {| mir_step_bind_name := src_step_bind_name step;
                 mir_step_block := lower_block_env_program (src_step_block step) |})
           (src_control_steps p) |}.

Definition emit_r_fn_cfg_control_program
    (p : mir_fn_cfg_control_program) : r_fn_cfg_control_program :=
  {| r_control_join_exec_prog := emit_r_fn_cfg_join_exec_program (mir_control_join_exec_prog p);
     r_control_steps :=
       map (fun step =>
              {| r_step_bind_name := mir_step_bind_name step;
                 r_step_block := emit_r_block_env_program (mir_step_block step) |})
           (mir_control_steps p) |}.

Definition eval_src_fn_cfg_control_program
    (p : src_fn_cfg_control_program) (choice : branch_choice) : option rvalue :=
  match eval_src_fn_cfg_join_exec_program (src_control_join_exec_prog p) choice with
  | Some joined =>
      run_src_cfg_control (List.length (src_control_steps p))
        {| src_control_current := joined; src_control_remaining := src_control_steps p |}
  | None => None
  end.

Definition eval_r_fn_cfg_control_program
    (p : r_fn_cfg_control_program) (choice : branch_choice) : option rvalue :=
  match eval_r_fn_cfg_join_exec_program (r_control_join_exec_prog p) choice with
  | Some joined =>
      run_r_cfg_control (List.length (r_control_steps p))
        {| r_control_current := joined; r_control_remaining := r_control_steps p |}
  | None => None
  end.

Definition branching_fn_cfg_control_program : src_fn_cfg_control_program :=
  {| src_control_join_exec_prog := branching_fn_cfg_join_exec_program;
     src_control_steps := src_iter_steps branching_fn_cfg_iter_exec_program |}.

Lemma branching_fn_cfg_control_program_meta_preserved :
  mir_phi_join_bid
      (mir_join_exec_phi_prog
        (mir_control_join_exec_prog (lower_fn_cfg_control_program branching_fn_cfg_control_program))) = 17%nat /\
  List.length (mir_control_steps (lower_fn_cfg_control_program branching_fn_cfg_control_program)) = 2%nat.
Proof.
  split; reflexivity.
Qed.

Lemma branching_fn_cfg_control_program_then_preserved :
  eval_r_fn_cfg_control_program
    (emit_r_fn_cfg_control_program (lower_fn_cfg_control_program branching_fn_cfg_control_program))
    ThenBranch =
    Some (RVInt 28).
Proof.
  reflexivity.
Qed.

Lemma branching_fn_cfg_control_program_else_preserved :
  eval_r_fn_cfg_control_program
    (emit_r_fn_cfg_control_program (lower_fn_cfg_control_program branching_fn_cfg_control_program))
    ElseBranch =
    Some (RVInt 41).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgControlStateSubset.
