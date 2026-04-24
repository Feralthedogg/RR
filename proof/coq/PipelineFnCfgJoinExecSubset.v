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
Import RRPipelineFnCfgJoinStateSubset.
Import RRPipelineFnCfgPhiExecSubset.
Import RRPipelineFnCfgBranchExecSubset.
Import RRPipelineFnCfgExecSubset.
Import RRPipelineFnCfgSubset.
Import RRPipelineFnEnvSubset.
Import RRPipelineBlockEnvSubset.
Import RRPipelineStmtSubset.

Module RRPipelineFnCfgJoinExecSubset.

Record src_fn_cfg_join_exec_program : Type := {
  src_join_exec_phi_prog : src_fn_cfg_phi_program;
  src_join_exec_block : src_block_env_program;
}.

Record mir_fn_cfg_join_exec_program : Type := {
  mir_join_exec_phi_prog : mir_fn_cfg_phi_program;
  mir_join_exec_block : mir_block_env_program;
}.

Record r_fn_cfg_join_exec_program : Type := {
  r_join_exec_phi_prog : r_fn_cfg_phi_program;
  r_join_exec_block : r_block_env_program;
}.

Definition lower_fn_cfg_join_exec_program
    (p : src_fn_cfg_join_exec_program) : mir_fn_cfg_join_exec_program :=
  {| mir_join_exec_phi_prog := lower_fn_cfg_phi_program (src_join_exec_phi_prog p);
     mir_join_exec_block := lower_block_env_program (src_join_exec_block p) |}.

Definition emit_r_fn_cfg_join_exec_program
    (p : mir_fn_cfg_join_exec_program) : r_fn_cfg_join_exec_program :=
  {| r_join_exec_phi_prog := emit_r_fn_cfg_phi_program (mir_join_exec_phi_prog p);
     r_join_exec_block := emit_r_block_env_program (mir_join_exec_block p) |}.

Definition eval_src_fn_cfg_join_exec_program
    (p : src_fn_cfg_join_exec_program) (choice : branch_choice) : option rvalue :=
  match eval_src_fn_cfg_phi_program (src_join_exec_phi_prog p) choice with
  | Some merged =>
      eval_src_block_env_program
        {| src_block_id := src_block_id (src_join_exec_block p);
           src_block_in_env :=
             (src_phi_name (src_join_exec_phi_prog p), merged) :: src_block_in_env (src_join_exec_block p);
           src_block_stmts := src_block_stmts (src_join_exec_block p);
           src_block_ret := src_block_ret (src_join_exec_block p) |}
  | None => None
  end.

Definition eval_r_fn_cfg_join_exec_program
    (p : r_fn_cfg_join_exec_program) (choice : branch_choice) : option rvalue :=
  match eval_r_fn_cfg_phi_program (r_join_exec_phi_prog p) choice with
  | Some merged =>
      eval_r_block_env_program
        {| r_block_id := r_block_id (r_join_exec_block p);
           r_block_in_env :=
             (r_phi_name (r_join_exec_phi_prog p), merged) :: r_block_in_env (r_join_exec_block p);
           r_block_stmts := r_block_stmts (r_join_exec_block p);
           r_block_ret := r_block_ret (r_join_exec_block p) |}
  | None => None
  end.

Definition branching_fn_cfg_join_exec_program : src_fn_cfg_join_exec_program :=
  {| src_join_exec_phi_prog := branching_fn_cfg_phi_program;
     src_join_exec_block :=
       {| src_block_id := 17%nat;
          src_block_in_env := [("bonus", RVInt 1)];
          src_block_stmts := [SAssign "tmp2" (SConstInt 4)];
          src_block_ret := SLAdd (SLVar "out") (SLAdd (SLVar "tmp2") (SLVar "bonus")) |} |}.

Lemma branching_fn_cfg_join_exec_program_meta_preserved :
  mir_phi_join_bid (mir_join_exec_phi_prog (lower_fn_cfg_join_exec_program branching_fn_cfg_join_exec_program)) = 17%nat /\
  mir_phi_name (mir_join_exec_phi_prog (lower_fn_cfg_join_exec_program branching_fn_cfg_join_exec_program)) = "out" /\
  mir_block_id (mir_join_exec_block (lower_fn_cfg_join_exec_program branching_fn_cfg_join_exec_program)) = 17%nat.
Proof.
  repeat split; reflexivity.
Qed.

Lemma branching_fn_cfg_join_exec_program_then_preserved :
  eval_r_fn_cfg_join_exec_program
    (emit_r_fn_cfg_join_exec_program (lower_fn_cfg_join_exec_program branching_fn_cfg_join_exec_program))
    ThenBranch =
    Some (RVInt 17).
Proof.
  reflexivity.
Qed.

Lemma branching_fn_cfg_join_exec_program_else_preserved :
  eval_r_fn_cfg_join_exec_program
    (emit_r_fn_cfg_join_exec_program (lower_fn_cfg_join_exec_program branching_fn_cfg_join_exec_program))
    ElseBranch =
    Some (RVInt 30).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgJoinExecSubset.
