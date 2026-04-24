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
Import RRPipelineFnCfgJoinExecSubset.
Import RRPipelineFnCfgJoinStateSubset.
Import RRPipelineFnCfgPhiExecSubset.
Import RRPipelineFnCfgBranchExecSubset.
Import RRPipelineFnCfgExecSubset.
Import RRPipelineFnCfgSubset.
Import RRPipelineFnEnvSubset.
Import RRPipelineBlockEnvSubset.
Import RRPipelineStmtSubset.

Module RRPipelineFnCfgPostJoinSubset.

Record src_fn_cfg_post_join_program : Type := {
  src_post_join_exec_prog : src_fn_cfg_join_exec_program;
  src_post_join_name : string;
  src_post_join_block : src_block_env_program;
}.

Record mir_fn_cfg_post_join_program : Type := {
  mir_post_join_exec_prog : mir_fn_cfg_join_exec_program;
  mir_post_join_name : string;
  mir_post_join_block : mir_block_env_program;
}.

Record r_fn_cfg_post_join_program : Type := {
  r_post_join_exec_prog : r_fn_cfg_join_exec_program;
  r_post_join_name : string;
  r_post_join_block : r_block_env_program;
}.

Definition lower_fn_cfg_post_join_program
    (p : src_fn_cfg_post_join_program) : mir_fn_cfg_post_join_program :=
  {| mir_post_join_exec_prog := lower_fn_cfg_join_exec_program (src_post_join_exec_prog p);
     mir_post_join_name := src_post_join_name p;
     mir_post_join_block := lower_block_env_program (src_post_join_block p) |}.

Definition emit_r_fn_cfg_post_join_program
    (p : mir_fn_cfg_post_join_program) : r_fn_cfg_post_join_program :=
  {| r_post_join_exec_prog := emit_r_fn_cfg_join_exec_program (mir_post_join_exec_prog p);
     r_post_join_name := mir_post_join_name p;
     r_post_join_block := emit_r_block_env_program (mir_post_join_block p) |}.

Definition eval_src_fn_cfg_post_join_program
    (p : src_fn_cfg_post_join_program) (choice : branch_choice) : option rvalue :=
  match eval_src_fn_cfg_join_exec_program (src_post_join_exec_prog p) choice with
  | Some joined =>
      eval_src_block_env_program
        {| src_block_id := src_block_id (src_post_join_block p);
           src_block_in_env :=
             (src_post_join_name p, joined) :: src_block_in_env (src_post_join_block p);
           src_block_stmts := src_block_stmts (src_post_join_block p);
           src_block_ret := src_block_ret (src_post_join_block p) |}
  | None => None
  end.

Definition eval_r_fn_cfg_post_join_program
    (p : r_fn_cfg_post_join_program) (choice : branch_choice) : option rvalue :=
  match eval_r_fn_cfg_join_exec_program (r_post_join_exec_prog p) choice with
  | Some joined =>
      eval_r_block_env_program
        {| r_block_id := r_block_id (r_post_join_block p);
           r_block_in_env :=
             (r_post_join_name p, joined) :: r_block_in_env (r_post_join_block p);
           r_block_stmts := r_block_stmts (r_post_join_block p);
           r_block_ret := r_block_ret (r_post_join_block p) |}
  | None => None
  end.

Definition branching_fn_cfg_post_join_program : src_fn_cfg_post_join_program :=
  {| src_post_join_exec_prog := branching_fn_cfg_join_exec_program;
     src_post_join_name := "joined";
     src_post_join_block :=
       {| src_block_id := 19%nat;
          src_block_in_env := [("tail", RVInt 2)];
          src_block_stmts := [SAssign "tmp3" (SConstInt 5)];
          src_block_ret := SLAdd (SLVar "joined") (SLAdd (SLVar "tmp3") (SLVar "tail")) |} |}.

Lemma branching_fn_cfg_post_join_program_meta_preserved :
  mir_phi_join_bid
      (mir_join_exec_phi_prog
        (mir_post_join_exec_prog (lower_fn_cfg_post_join_program branching_fn_cfg_post_join_program))) = 17%nat /\
  mir_post_join_name (lower_fn_cfg_post_join_program branching_fn_cfg_post_join_program) = "joined" /\
  mir_block_id (mir_post_join_block (lower_fn_cfg_post_join_program branching_fn_cfg_post_join_program)) = 19%nat.
Proof.
  repeat split; reflexivity.
Qed.

Lemma branching_fn_cfg_post_join_program_then_preserved :
  eval_r_fn_cfg_post_join_program
    (emit_r_fn_cfg_post_join_program (lower_fn_cfg_post_join_program branching_fn_cfg_post_join_program))
    ThenBranch =
    Some (RVInt 24).
Proof.
  reflexivity.
Qed.

Lemma branching_fn_cfg_post_join_program_else_preserved :
  eval_r_fn_cfg_post_join_program
    (emit_r_fn_cfg_post_join_program (lower_fn_cfg_post_join_program branching_fn_cfg_post_join_program))
    ElseBranch =
    Some (RVInt 37).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgPostJoinSubset.
