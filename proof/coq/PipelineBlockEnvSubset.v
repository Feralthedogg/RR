Require Import PipelineStmtSubset.
Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRPipelineStmtSubset.

Module RRPipelineBlockEnvSubset.

Record src_block_env_program : Type := {
  src_block_id : nat;
  src_block_in_env : env;
  src_block_stmts : list src_stmt;
  src_block_ret : src_local_expr;
}.

Record mir_block_env_program : Type := {
  mir_block_id : nat;
  mir_block_in_env : env;
  mir_block_stmts : list mir_stmt;
  mir_block_ret : mir_local_expr;
}.

Record r_block_env_program : Type := {
  r_block_id : nat;
  r_block_in_env : env;
  r_block_stmts : list r_stmt;
  r_block_ret : r_local_expr;
}.

Definition lower_block_env_program (p : src_block_env_program) : mir_block_env_program :=
  {| mir_block_id := src_block_id p;
     mir_block_in_env := src_block_in_env p;
     mir_block_stmts := lower_stmts (src_block_stmts p);
     mir_block_ret := lower_local_expr (src_block_ret p) |}.

Definition emit_r_block_env_program (p : mir_block_env_program) : r_block_env_program :=
  {| r_block_id := mir_block_id p;
     r_block_in_env := mir_block_in_env p;
     r_block_stmts := emit_r_stmts (mir_block_stmts p);
     r_block_ret := emit_r_local (mir_block_ret p) |}.

Definition eval_src_block_env_program (p : src_block_env_program) : option rvalue :=
  match exec_src_stmts (src_block_in_env p) (src_block_stmts p) with
  | Some rho => eval_src_local rho (src_block_ret p)
  | None => None
  end.

Definition eval_r_block_env_program (p : r_block_env_program) : option rvalue :=
  match exec_r_stmts (r_block_in_env p) (r_block_stmts p) with
  | Some rho => eval_r_local rho (r_block_ret p)
  | None => None
  end.

Definition incoming_field_block_program : src_block_env_program :=
  {| src_block_id := 7%nat;
     src_block_in_env := [("arg", RVRecord [("x", RVInt 4)])];
     src_block_stmts := [SAssign "tmp" (SConstInt 3)];
     src_block_ret := SLAdd (SLField (SLVar "arg") "x") (SLVar "tmp") |}.

Lemma incoming_field_block_program_block_id_preserved :
  mir_block_id (lower_block_env_program incoming_field_block_program) = 7%nat /\
  r_block_id (emit_r_block_env_program (lower_block_env_program incoming_field_block_program)) = 7%nat.
Proof.
  split; reflexivity.
Qed.

Lemma incoming_field_block_program_preserved :
  eval_r_block_env_program
    (emit_r_block_env_program (lower_block_env_program incoming_field_block_program)) =
    Some (RVInt 7).
Proof.
  reflexivity.
Qed.

Definition incoming_branch_block_program : src_block_env_program :=
  {| src_block_id := 11%nat;
     src_block_in_env := [("arg", RVRecord [("base", RVInt 10)])];
     src_block_stmts :=
       [SIfAssign "tmp" (SConstBool true) (SConstInt 2) (SConstInt 5)];
     src_block_ret := SLAdd (SLField (SLVar "arg") "base") (SLVar "tmp") |}.

Lemma incoming_branch_block_program_preserved :
  eval_r_block_env_program
    (emit_r_block_env_program (lower_block_env_program incoming_branch_block_program)) =
    Some (RVInt 12).
Proof.
  reflexivity.
Qed.

End RRPipelineBlockEnvSubset.
