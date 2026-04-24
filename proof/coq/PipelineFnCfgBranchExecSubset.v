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
Import RRPipelineFnCfgExecSubset.
Import RRPipelineFnCfgSubset.
Import RRPipelineFnEnvSubset.
Import RRPipelineBlockEnvSubset.
Import RRPipelineStmtSubset.

Module RRPipelineFnCfgBranchExecSubset.

Inductive branch_choice : Type :=
| ThenBranch
| ElseBranch.

Record src_fn_cfg_branch_program : Type := {
  src_branch_fn_cfg : src_fn_cfg_program;
  src_branch_block_order : list nat;
  src_branch_then_path : list nat;
  src_branch_else_path : list nat;
}.

Record mir_fn_cfg_branch_program : Type := {
  mir_branch_fn_cfg : mir_fn_cfg_program;
  mir_branch_block_order : list nat;
  mir_branch_then_path : list nat;
  mir_branch_else_path : list nat;
}.

Record r_fn_cfg_branch_program : Type := {
  r_branch_fn_cfg : r_fn_cfg_program;
  r_branch_block_order : list nat;
  r_branch_then_path : list nat;
  r_branch_else_path : list nat;
}.

Definition path_for_choice (choice : branch_choice)
    (then_path else_path : list nat) : list nat :=
  match choice with
  | ThenBranch => then_path
  | ElseBranch => else_path
  end.

Definition to_src_fn_cfg_exec_program
    (p : src_fn_cfg_branch_program) (choice : branch_choice) : src_fn_cfg_exec_program :=
  {| src_exec_fn_cfg := src_branch_fn_cfg p;
     src_exec_block_order := src_branch_block_order p;
     src_exec_path := path_for_choice choice (src_branch_then_path p) (src_branch_else_path p) |}.

Definition to_r_fn_cfg_exec_program
    (p : r_fn_cfg_branch_program) (choice : branch_choice) : r_fn_cfg_exec_program :=
  {| r_exec_fn_cfg := r_branch_fn_cfg p;
     r_exec_block_order := r_branch_block_order p;
     r_exec_path := path_for_choice choice (r_branch_then_path p) (r_branch_else_path p) |}.

Definition lower_fn_cfg_branch_program (p : src_fn_cfg_branch_program) : mir_fn_cfg_branch_program :=
  {| mir_branch_fn_cfg := lower_fn_cfg_program (src_branch_fn_cfg p);
     mir_branch_block_order := src_branch_block_order p;
     mir_branch_then_path := src_branch_then_path p;
     mir_branch_else_path := src_branch_else_path p |}.

Definition emit_r_fn_cfg_branch_program (p : mir_fn_cfg_branch_program) : r_fn_cfg_branch_program :=
  {| r_branch_fn_cfg := emit_r_fn_cfg_program (mir_branch_fn_cfg p);
     r_branch_block_order := mir_branch_block_order p;
     r_branch_then_path := mir_branch_then_path p;
     r_branch_else_path := mir_branch_else_path p |}.

Definition eval_src_fn_cfg_branch_program
    (p : src_fn_cfg_branch_program) (choice : branch_choice) : list (nat * option rvalue) :=
  eval_src_fn_cfg_exec_program (to_src_fn_cfg_exec_program p choice).

Definition eval_r_fn_cfg_branch_program
    (p : r_fn_cfg_branch_program) (choice : branch_choice) : list (nat * option rvalue) :=
  eval_r_fn_cfg_exec_program (to_r_fn_cfg_exec_program p choice).

Definition incoming_else_block_program : src_block_env_program :=
  {| src_block_id := 13%nat;
     src_block_in_env := [("arg", RVRecord [("base", RVInt 20)])];
     src_block_stmts := [SAssign "tmp" (SConstInt 5)];
     src_block_ret := SLAdd (SLField (SLVar "arg") "base") (SLVar "tmp") |}.

Definition branching_fn_cfg_program : src_fn_cfg_branch_program :=
  {| src_branch_fn_cfg :=
       {| src_cfg_name := "toy_branch_fn";
          src_cfg_entry := 7%nat;
          src_cfg_body_head := 11%nat;
          src_cfg_preds := fun bid =>
            match bid with
            | 7%nat => []
            | 11%nat => [7%nat]
            | 13%nat => [7%nat]
            | _ => []
            end;
          src_cfg_blocks :=
            [incoming_field_block_program; incoming_branch_block_program; incoming_else_block_program] |};
     src_branch_block_order := [7%nat; 11%nat; 13%nat];
     src_branch_then_path := [7%nat; 11%nat];
     src_branch_else_path := [7%nat; 13%nat] |}.

Lemma branching_fn_cfg_program_then_preserved :
  eval_r_fn_cfg_branch_program
    (emit_r_fn_cfg_branch_program (lower_fn_cfg_branch_program branching_fn_cfg_program))
    ThenBranch =
    [(7%nat, Some (RVInt 7)); (11%nat, Some (RVInt 12))].
Proof.
  reflexivity.
Qed.

Lemma branching_fn_cfg_program_else_preserved :
  eval_r_fn_cfg_branch_program
    (emit_r_fn_cfg_branch_program (lower_fn_cfg_branch_program branching_fn_cfg_program))
    ElseBranch =
    [(7%nat, Some (RVInt 7)); (13%nat, Some (RVInt 25))].
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgBranchExecSubset.
