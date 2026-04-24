Require Import PipelineFnCfgPhiExecSubset.
Require Import PipelineAssignPhiSubset.
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
Import RRPipelineFnCfgPhiExecSubset.
Import RRPipelineAssignPhiSubset.
Import RRPipelineFnCfgBranchExecSubset.
Import RRPipelineFnCfgExecSubset.
Import RRPipelineFnCfgSubset.
Import RRPipelineFnEnvSubset.
Import RRPipelineBlockEnvSubset.
Import RRPipelineStmtSubset.

Module RRPipelineFnCfgJoinStateSubset.

Record src_fn_cfg_join_state_program : Type := {
  src_join_phi_prog : src_fn_cfg_phi_program;
  src_join_env : env;
  src_join_ret : src_local_expr;
}.

Record mir_fn_cfg_join_state_program : Type := {
  mir_join_phi_prog : mir_fn_cfg_phi_program;
  mir_join_env : env;
  mir_join_ret : mir_local_expr;
}.

Record r_fn_cfg_join_state_program : Type := {
  r_join_phi_prog : r_fn_cfg_phi_program;
  r_join_env : env;
  r_join_ret : r_local_expr;
}.

Definition lower_fn_cfg_join_state_program
    (p : src_fn_cfg_join_state_program) : mir_fn_cfg_join_state_program :=
  {| mir_join_phi_prog := lower_fn_cfg_phi_program (src_join_phi_prog p);
     mir_join_env := src_join_env p;
     mir_join_ret := lower_local_expr (src_join_ret p) |}.

Definition emit_r_fn_cfg_join_state_program
    (p : mir_fn_cfg_join_state_program) : r_fn_cfg_join_state_program :=
  {| r_join_phi_prog := emit_r_fn_cfg_phi_program (mir_join_phi_prog p);
     r_join_env := mir_join_env p;
     r_join_ret := emit_r_local (mir_join_ret p) |}.

Definition eval_src_fn_cfg_join_state_program
    (p : src_fn_cfg_join_state_program) (choice : branch_choice) : option rvalue :=
  match eval_src_fn_cfg_phi_program (src_join_phi_prog p) choice with
  | Some merged =>
      eval_src_local ((src_phi_name (src_join_phi_prog p), merged) :: src_join_env p) (src_join_ret p)
  | None => None
  end.

Definition eval_r_fn_cfg_join_state_program
    (p : r_fn_cfg_join_state_program) (choice : branch_choice) : option rvalue :=
  match eval_r_fn_cfg_phi_program (r_join_phi_prog p) choice with
  | Some merged =>
      eval_r_local ((r_phi_name (r_join_phi_prog p), merged) :: r_join_env p) (r_join_ret p)
  | None => None
  end.

Definition branching_fn_cfg_join_state_program : src_fn_cfg_join_state_program :=
  {| src_join_phi_prog := branching_fn_cfg_phi_program;
     src_join_env := [("bonus", RVInt 1)];
     src_join_ret := SLAdd (SLVar "out") (SLVar "bonus") |}.

Lemma branching_fn_cfg_join_state_program_meta_preserved :
  mir_phi_join_bid (mir_join_phi_prog (lower_fn_cfg_join_state_program branching_fn_cfg_join_state_program)) = 17%nat /\
  mir_phi_name (mir_join_phi_prog (lower_fn_cfg_join_state_program branching_fn_cfg_join_state_program)) = "out" /\
  r_phi_join_bid
    (r_join_phi_prog
      (emit_r_fn_cfg_join_state_program
        (lower_fn_cfg_join_state_program branching_fn_cfg_join_state_program))) = 17%nat.
Proof.
  repeat split; reflexivity.
Qed.

Lemma branching_fn_cfg_join_state_program_then_preserved :
  eval_r_fn_cfg_join_state_program
    (emit_r_fn_cfg_join_state_program (lower_fn_cfg_join_state_program branching_fn_cfg_join_state_program))
    ThenBranch =
    Some (RVInt 13).
Proof.
  reflexivity.
Qed.

Lemma branching_fn_cfg_join_state_program_else_preserved :
  eval_r_fn_cfg_join_state_program
    (emit_r_fn_cfg_join_state_program (lower_fn_cfg_join_state_program branching_fn_cfg_join_state_program))
    ElseBranch =
    Some (RVInt 26).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgJoinStateSubset.
