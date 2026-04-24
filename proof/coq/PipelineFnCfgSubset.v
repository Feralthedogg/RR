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
Import RRPipelineFnEnvSubset.
Import RRPipelineBlockEnvSubset.

Module RRPipelineFnCfgSubset.

Record src_fn_cfg_program : Type := {
  src_cfg_name : string;
  src_cfg_entry : nat;
  src_cfg_body_head : nat;
  src_cfg_preds : nat -> list nat;
  src_cfg_blocks : list src_block_env_program;
}.

Record mir_fn_cfg_program : Type := {
  mir_cfg_name : string;
  mir_cfg_entry : nat;
  mir_cfg_body_head : nat;
  mir_cfg_preds : nat -> list nat;
  mir_cfg_blocks : list mir_block_env_program;
}.

Record r_fn_cfg_program : Type := {
  r_cfg_name : string;
  r_cfg_entry : nat;
  r_cfg_body_head : nat;
  r_cfg_preds : nat -> list nat;
  r_cfg_blocks : list r_block_env_program;
}.

Definition lower_fn_cfg_program (p : src_fn_cfg_program) : mir_fn_cfg_program :=
  {| mir_cfg_name := src_cfg_name p;
     mir_cfg_entry := src_cfg_entry p;
     mir_cfg_body_head := src_cfg_body_head p;
     mir_cfg_preds := src_cfg_preds p;
     mir_cfg_blocks := map lower_block_env_program (src_cfg_blocks p) |}.

Definition emit_r_fn_cfg_program (p : mir_fn_cfg_program) : r_fn_cfg_program :=
  {| r_cfg_name := mir_cfg_name p;
     r_cfg_entry := mir_cfg_entry p;
     r_cfg_body_head := mir_cfg_body_head p;
     r_cfg_preds := mir_cfg_preds p;
     r_cfg_blocks := map emit_r_block_env_program (mir_cfg_blocks p) |}.

Definition eval_src_fn_cfg_program (p : src_fn_cfg_program) : list (nat * option rvalue) :=
  map (fun bb => (src_block_id bb, eval_src_block_env_program bb)) (src_cfg_blocks p).

Definition eval_r_fn_cfg_program (p : r_fn_cfg_program) : list (nat * option rvalue) :=
  map (fun bb => (r_block_id bb, eval_r_block_env_program bb)) (r_cfg_blocks p).

Definition two_block_fn_cfg_program : src_fn_cfg_program :=
  {| src_cfg_name := "toy_cfg_fn";
     src_cfg_entry := 7%nat;
     src_cfg_body_head := 11%nat;
     src_cfg_preds := fun bid =>
       match bid with
       | 7%nat => []
       | 11%nat => [7%nat]
       | _ => []
       end;
     src_cfg_blocks := [incoming_field_block_program; incoming_branch_block_program] |}.

Lemma two_block_fn_cfg_program_meta_preserved :
  mir_cfg_name (lower_fn_cfg_program two_block_fn_cfg_program) = "toy_cfg_fn" /\
  mir_cfg_entry (lower_fn_cfg_program two_block_fn_cfg_program) = 7%nat /\
  mir_cfg_body_head (lower_fn_cfg_program two_block_fn_cfg_program) = 11%nat /\
  mir_cfg_preds (lower_fn_cfg_program two_block_fn_cfg_program) 11%nat = [7%nat] /\
  r_cfg_preds (emit_r_fn_cfg_program (lower_fn_cfg_program two_block_fn_cfg_program)) 11%nat = [7%nat].
Proof.
  repeat split; reflexivity.
Qed.

Lemma two_block_fn_cfg_program_preserved :
  eval_r_fn_cfg_program
    (emit_r_fn_cfg_program (lower_fn_cfg_program two_block_fn_cfg_program)) =
    [(7%nat, Some (RVInt 7)); (11%nat, Some (RVInt 12))].
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgSubset.
