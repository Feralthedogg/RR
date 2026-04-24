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

Module RRPipelineFnCfgReentrySubset.

Record src_fn_cfg_reentry_program : Type := {
  src_reentry_join_exec_prog : src_fn_cfg_join_exec_program;
  src_reentry_table : list src_post_join_step;
  src_reentry_trace : list nat;
}.

Record mir_fn_cfg_reentry_program : Type := {
  mir_reentry_join_exec_prog : mir_fn_cfg_join_exec_program;
  mir_reentry_table : list mir_post_join_step;
  mir_reentry_trace : list nat;
}.

Record r_fn_cfg_reentry_program : Type := {
  r_reentry_join_exec_prog : r_fn_cfg_join_exec_program;
  r_reentry_table : list r_post_join_step;
  r_reentry_trace : list nat;
}.

Fixpoint eval_src_reentry_trace
    (table : list src_post_join_step) (trace : list nat) (current : rvalue) : option rvalue :=
  match trace with
  | [] => Some current
  | idx :: rest =>
      match nth_error table idx with
      | Some step =>
          match eval_src_post_join_steps [step] current with
          | Some next => eval_src_reentry_trace table rest next
          | None => None
          end
      | None => None
      end
  end.

Fixpoint eval_r_reentry_trace
    (table : list r_post_join_step) (trace : list nat) (current : rvalue) : option rvalue :=
  match trace with
  | [] => Some current
  | idx :: rest =>
      match nth_error table idx with
      | Some step =>
          match eval_r_post_join_steps [step] current with
          | Some next => eval_r_reentry_trace table rest next
          | None => None
          end
      | None => None
      end
  end.

Definition lower_fn_cfg_reentry_program
    (p : src_fn_cfg_reentry_program) : mir_fn_cfg_reentry_program :=
  {| mir_reentry_join_exec_prog := lower_fn_cfg_join_exec_program (src_reentry_join_exec_prog p);
     mir_reentry_table :=
       map (fun step =>
              {| mir_step_bind_name := src_step_bind_name step;
                 mir_step_block := lower_block_env_program (src_step_block step) |})
           (src_reentry_table p);
     mir_reentry_trace := src_reentry_trace p |}.

Definition emit_r_fn_cfg_reentry_program
    (p : mir_fn_cfg_reentry_program) : r_fn_cfg_reentry_program :=
  {| r_reentry_join_exec_prog := emit_r_fn_cfg_join_exec_program (mir_reentry_join_exec_prog p);
     r_reentry_table :=
       map (fun step =>
              {| r_step_bind_name := mir_step_bind_name step;
                 r_step_block := emit_r_block_env_program (mir_step_block step) |})
           (mir_reentry_table p);
     r_reentry_trace := mir_reentry_trace p |}.

Definition eval_src_fn_cfg_reentry_program
    (p : src_fn_cfg_reentry_program) (choice : branch_choice) : option rvalue :=
  match eval_src_fn_cfg_join_exec_program (src_reentry_join_exec_prog p) choice with
  | Some joined => eval_src_reentry_trace (src_reentry_table p) (src_reentry_trace p) joined
  | None => None
  end.

Definition eval_r_fn_cfg_reentry_program
    (p : r_fn_cfg_reentry_program) (choice : branch_choice) : option rvalue :=
  match eval_r_fn_cfg_join_exec_program (r_reentry_join_exec_prog p) choice with
  | Some joined => eval_r_reentry_trace (r_reentry_table p) (r_reentry_trace p) joined
  | None => None
  end.

Definition branching_fn_cfg_reentry_program : src_fn_cfg_reentry_program :=
  {| src_reentry_join_exec_prog := branching_fn_cfg_join_exec_program;
     src_reentry_table := src_control_steps branching_fn_cfg_control_program;
     src_reentry_trace := [0%nat; 1%nat; 0%nat] |}.

Lemma branching_fn_cfg_reentry_program_meta_preserved :
  mir_phi_join_bid
      (mir_join_exec_phi_prog
        (mir_reentry_join_exec_prog (lower_fn_cfg_reentry_program branching_fn_cfg_reentry_program))) = 17%nat /\
  List.length (mir_reentry_table (lower_fn_cfg_reentry_program branching_fn_cfg_reentry_program)) = 2%nat /\
  mir_reentry_trace (lower_fn_cfg_reentry_program branching_fn_cfg_reentry_program) = [0%nat; 1%nat; 0%nat].
Proof.
  repeat split; reflexivity.
Qed.

Lemma branching_fn_cfg_reentry_program_then_preserved :
  eval_r_fn_cfg_reentry_program
    (emit_r_fn_cfg_reentry_program (lower_fn_cfg_reentry_program branching_fn_cfg_reentry_program))
    ThenBranch =
    Some (RVInt 35).
Proof.
  reflexivity.
Qed.

Lemma branching_fn_cfg_reentry_program_else_preserved :
  eval_r_fn_cfg_reentry_program
    (emit_r_fn_cfg_reentry_program (lower_fn_cfg_reentry_program branching_fn_cfg_reentry_program))
    ElseBranch =
    Some (RVInt 48).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgReentrySubset.
