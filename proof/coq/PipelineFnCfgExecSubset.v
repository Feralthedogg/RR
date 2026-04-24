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
Import RRPipelineFnCfgSubset.
Import RRPipelineFnEnvSubset.
Import RRPipelineBlockEnvSubset.

Module RRPipelineFnCfgExecSubset.

Definition lookup_fn_block_result
    (results : list (nat * option rvalue)) (bid : nat) : option rvalue :=
  match find (fun entry => Nat.eqb (fst entry) bid) results with
  | Some (_, value) => value
  | None => None
  end.

Fixpoint path_edges_ok_from (preds : nat -> list nat) (src : nat) (rest : list nat) : Prop :=
  match rest with
  | [] => True
  | dst :: tail => In src (preds dst) /\ path_edges_ok_from preds dst tail
  end.

Definition path_edges_ok (preds : nat -> list nat) (path : list nat) : Prop :=
  match path with
  | [] => True
  | src :: rest => path_edges_ok_from preds src rest
  end.

Record src_fn_cfg_exec_program : Type := {
  src_exec_fn_cfg : src_fn_cfg_program;
  src_exec_block_order : list nat;
  src_exec_path : list nat;
}.

Record mir_fn_cfg_exec_program : Type := {
  mir_exec_fn_cfg : mir_fn_cfg_program;
  mir_exec_block_order : list nat;
  mir_exec_path : list nat;
}.

Record r_fn_cfg_exec_program : Type := {
  r_exec_fn_cfg : r_fn_cfg_program;
  r_exec_block_order : list nat;
  r_exec_path : list nat;
}.

Definition lower_fn_cfg_exec_program (p : src_fn_cfg_exec_program) : mir_fn_cfg_exec_program :=
  {| mir_exec_fn_cfg := lower_fn_cfg_program (src_exec_fn_cfg p);
     mir_exec_block_order := src_exec_block_order p;
     mir_exec_path := src_exec_path p |}.

Definition emit_r_fn_cfg_exec_program (p : mir_fn_cfg_exec_program) : r_fn_cfg_exec_program :=
  {| r_exec_fn_cfg := emit_r_fn_cfg_program (mir_exec_fn_cfg p);
     r_exec_block_order := mir_exec_block_order p;
     r_exec_path := mir_exec_path p |}.

Definition eval_src_fn_cfg_exec_program (p : src_fn_cfg_exec_program) : list (nat * option rvalue) :=
  map (fun bid => (bid, lookup_fn_block_result (eval_src_fn_cfg_program (src_exec_fn_cfg p)) bid))
    (src_exec_path p).

Definition eval_r_fn_cfg_exec_program (p : r_fn_cfg_exec_program) : list (nat * option rvalue) :=
  map (fun bid => (bid, lookup_fn_block_result (eval_r_fn_cfg_program (r_exec_fn_cfg p)) bid))
    (r_exec_path p).

Definition path_starts_at_entry (entry : nat) (path : list nat) : Prop :=
  match path with
  | [] => True
  | bid :: _ => bid = entry
  end.

Definition two_block_fn_cfg_exec_program : src_fn_cfg_exec_program :=
  {| src_exec_fn_cfg := two_block_fn_cfg_program;
     src_exec_block_order := [7%nat; 11%nat];
     src_exec_path := [7%nat; 11%nat] |}.

Lemma two_block_fn_cfg_exec_program_path_starts_at_entry :
  path_starts_at_entry (src_cfg_entry (src_exec_fn_cfg two_block_fn_cfg_exec_program))
    (src_exec_path two_block_fn_cfg_exec_program).
Proof.
  reflexivity.
Qed.

Lemma two_block_fn_cfg_exec_program_path_edges_ok :
  path_edges_ok (src_cfg_preds (src_exec_fn_cfg two_block_fn_cfg_exec_program))
    (src_exec_path two_block_fn_cfg_exec_program).
Proof.
  simpl. split; [left; reflexivity|exact I].
Qed.

Lemma two_block_fn_cfg_exec_program_meta_preserved :
  mir_cfg_name (mir_exec_fn_cfg (lower_fn_cfg_exec_program two_block_fn_cfg_exec_program)) = "toy_cfg_fn" /\
  mir_cfg_entry (mir_exec_fn_cfg (lower_fn_cfg_exec_program two_block_fn_cfg_exec_program)) = 7%nat /\
  mir_cfg_body_head (mir_exec_fn_cfg (lower_fn_cfg_exec_program two_block_fn_cfg_exec_program)) = 11%nat /\
  mir_cfg_preds (mir_exec_fn_cfg (lower_fn_cfg_exec_program two_block_fn_cfg_exec_program)) 11%nat = [7%nat] /\
  r_exec_path (emit_r_fn_cfg_exec_program (lower_fn_cfg_exec_program two_block_fn_cfg_exec_program)) = [7%nat; 11%nat].
Proof.
  repeat split; reflexivity.
Qed.

Lemma two_block_fn_cfg_exec_program_preserved :
  eval_r_fn_cfg_exec_program
    (emit_r_fn_cfg_exec_program (lower_fn_cfg_exec_program two_block_fn_cfg_exec_program)) =
    [(7%nat, Some (RVInt 7)); (11%nat, Some (RVInt 12))].
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgExecSubset.
