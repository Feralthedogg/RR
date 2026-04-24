Require Import PipelineFnCfgBranchExecSubset.
Require Import PipelineFnCfgExecSubset.
Require Import PipelineFnCfgSubset.
Require Import PipelineFnEnvSubset.
Require Import PipelineBlockEnvSubset.
Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRPipelineFnCfgBranchExecSubset.
Import RRPipelineFnCfgExecSubset.
Import RRPipelineFnCfgSubset.
Import RRPipelineFnEnvSubset.
Import RRPipelineBlockEnvSubset.

Module RRPipelineFnCfgPhiExecSubset.

Definition branch_exit_result (results : list (nat * option rvalue)) : option rvalue :=
  match rev results with
  | (_, value) :: _ => value
  | [] => None
  end.

Definition phi_merge_result (choice : branch_choice)
    (then_results else_results : list (nat * option rvalue)) : option rvalue :=
  match choice with
  | ThenBranch => branch_exit_result then_results
  | ElseBranch => branch_exit_result else_results
  end.

Record src_fn_cfg_phi_program : Type := {
  src_phi_branch_prog : src_fn_cfg_branch_program;
  src_phi_join_bid : nat;
  src_phi_name : string;
}.

Record mir_fn_cfg_phi_program : Type := {
  mir_phi_branch_prog : mir_fn_cfg_branch_program;
  mir_phi_join_bid : nat;
  mir_phi_name : string;
}.

Record r_fn_cfg_phi_program : Type := {
  r_phi_branch_prog : r_fn_cfg_branch_program;
  r_phi_join_bid : nat;
  r_phi_name : string;
}.

Definition lower_fn_cfg_phi_program (p : src_fn_cfg_phi_program) : mir_fn_cfg_phi_program :=
  {| mir_phi_branch_prog := lower_fn_cfg_branch_program (src_phi_branch_prog p);
     mir_phi_join_bid := src_phi_join_bid p;
     mir_phi_name := src_phi_name p |}.

Definition emit_r_fn_cfg_phi_program (p : mir_fn_cfg_phi_program) : r_fn_cfg_phi_program :=
  {| r_phi_branch_prog := emit_r_fn_cfg_branch_program (mir_phi_branch_prog p);
     r_phi_join_bid := mir_phi_join_bid p;
     r_phi_name := mir_phi_name p |}.

Definition eval_src_fn_cfg_phi_program
    (p : src_fn_cfg_phi_program) (choice : branch_choice) : option rvalue :=
  phi_merge_result choice
    (eval_src_fn_cfg_branch_program (src_phi_branch_prog p) ThenBranch)
    (eval_src_fn_cfg_branch_program (src_phi_branch_prog p) ElseBranch).

Definition eval_r_fn_cfg_phi_program
    (p : r_fn_cfg_phi_program) (choice : branch_choice) : option rvalue :=
  phi_merge_result choice
    (eval_r_fn_cfg_branch_program (r_phi_branch_prog p) ThenBranch)
    (eval_r_fn_cfg_branch_program (r_phi_branch_prog p) ElseBranch).

Definition branching_fn_cfg_phi_program : src_fn_cfg_phi_program :=
  {| src_phi_branch_prog := branching_fn_cfg_program;
     src_phi_join_bid := 17%nat;
     src_phi_name := "out" |}.

Lemma branching_fn_cfg_phi_program_meta_preserved :
  mir_phi_join_bid (lower_fn_cfg_phi_program branching_fn_cfg_phi_program) = 17%nat /\
  mir_phi_name (lower_fn_cfg_phi_program branching_fn_cfg_phi_program) = "out" /\
  r_phi_join_bid (emit_r_fn_cfg_phi_program (lower_fn_cfg_phi_program branching_fn_cfg_phi_program)) = 17%nat.
Proof.
  repeat split; reflexivity.
Qed.

Lemma branching_fn_cfg_phi_program_then_preserved :
  eval_r_fn_cfg_phi_program
    (emit_r_fn_cfg_phi_program (lower_fn_cfg_phi_program branching_fn_cfg_phi_program))
    ThenBranch =
    Some (RVInt 12).
Proof.
  reflexivity.
Qed.

Lemma branching_fn_cfg_phi_program_else_preserved :
  eval_r_fn_cfg_phi_program
    (emit_r_fn_cfg_phi_program (lower_fn_cfg_phi_program branching_fn_cfg_phi_program))
    ElseBranch =
    Some (RVInt 25).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgPhiExecSubset.
