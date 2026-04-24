Require Import PipelineBlockEnvSubset.
Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRPipelineBlockEnvSubset.

Module RRPipelineFnEnvSubset.

Record src_fn_env_program : Type := {
  src_fn_name : string;
  src_fn_entry : nat;
  src_fn_body_head : nat;
  src_fn_blocks : list src_block_env_program;
}.

Record mir_fn_env_program : Type := {
  mir_fn_name : string;
  mir_fn_entry : nat;
  mir_fn_body_head : nat;
  mir_fn_blocks : list mir_block_env_program;
}.

Record r_fn_env_program : Type := {
  r_fn_name : string;
  r_fn_entry : nat;
  r_fn_body_head : nat;
  r_fn_blocks : list r_block_env_program;
}.

Definition lower_fn_blocks (blocks : list src_block_env_program) : list mir_block_env_program :=
  map lower_block_env_program blocks.

Definition emit_r_fn_blocks (blocks : list mir_block_env_program) : list r_block_env_program :=
  map emit_r_block_env_program blocks.

Definition lower_fn_env_program (p : src_fn_env_program) : mir_fn_env_program :=
  {| mir_fn_name := src_fn_name p;
     mir_fn_entry := src_fn_entry p;
     mir_fn_body_head := src_fn_body_head p;
     mir_fn_blocks := lower_fn_blocks (src_fn_blocks p) |}.

Definition emit_r_fn_env_program (p : mir_fn_env_program) : r_fn_env_program :=
  {| r_fn_name := mir_fn_name p;
     r_fn_entry := mir_fn_entry p;
     r_fn_body_head := mir_fn_body_head p;
     r_fn_blocks := emit_r_fn_blocks (mir_fn_blocks p) |}.

Definition eval_src_fn_env_program (p : src_fn_env_program) : list (nat * option rvalue) :=
  map (fun bb => (src_block_id bb, eval_src_block_env_program bb)) (src_fn_blocks p).

Definition eval_r_fn_env_program (p : r_fn_env_program) : list (nat * option rvalue) :=
  map (fun bb => (r_block_id bb, eval_r_block_env_program bb)) (r_fn_blocks p).

Definition two_block_fn_env_program : src_fn_env_program :=
  {| src_fn_name := "toy_fn";
     src_fn_entry := 7%nat;
     src_fn_body_head := 11%nat;
     src_fn_blocks := [incoming_field_block_program; incoming_branch_block_program] |}.

Lemma two_block_fn_env_program_meta_preserved :
  mir_fn_name (lower_fn_env_program two_block_fn_env_program) = "toy_fn" /\
  mir_fn_entry (lower_fn_env_program two_block_fn_env_program) = 7%nat /\
  mir_fn_body_head (lower_fn_env_program two_block_fn_env_program) = 11%nat /\
  r_fn_name (emit_r_fn_env_program (lower_fn_env_program two_block_fn_env_program)) = "toy_fn".
Proof.
  repeat split; reflexivity.
Qed.

Lemma two_block_fn_env_program_preserved :
  eval_r_fn_env_program
    (emit_r_fn_env_program (lower_fn_env_program two_block_fn_env_program)) =
    [(7%nat, Some (RVInt 7)); (11%nat, Some (RVInt 12))].
Proof.
  reflexivity.
Qed.

End RRPipelineFnEnvSubset.
