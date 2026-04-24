Require Import PipelineFnCfgLoopQueueSubset.
Require Import PipelineFnCfgLoopWorklistSubset.
Require Import PipelineFnCfgLoopDiscoverSubset.
Require Import PipelineFnCfgLoopFixpointSubset.
Require Import PipelineFnCfgLoopCycleSubset.
Require Import PipelineFnCfgReentrySubset.
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
Import RRPipelineFnCfgLoopQueueSubset.
Import RRPipelineFnCfgLoopWorklistSubset.
Import RRPipelineFnCfgLoopDiscoverSubset.
Import RRPipelineFnCfgLoopFixpointSubset.
Import RRPipelineFnCfgLoopCycleSubset.
Import RRPipelineFnCfgReentrySubset.
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

Module RRPipelineFnCfgLoopSchedulerSubset.

Definition loop_queue_eval := list loop_update.

Record src_fn_cfg_loop_scheduler_program : Type := {
  src_scheduler_batches : list src_fn_cfg_loop_queue_program;
}.

Record mir_fn_cfg_loop_scheduler_program : Type := {
  mir_scheduler_batches : list mir_fn_cfg_loop_queue_program;
}.

Record r_fn_cfg_loop_scheduler_program : Type := {
  r_scheduler_batches : list r_fn_cfg_loop_queue_program;
}.

Definition r_loop_scheduler_witness (p : r_fn_cfg_loop_scheduler_program) : Prop :=
  forall batch, In batch (r_scheduler_batches p) -> r_loop_queue_witness batch.

Definition lower_fn_cfg_loop_scheduler_program
    (p : src_fn_cfg_loop_scheduler_program) : mir_fn_cfg_loop_scheduler_program :=
  {| mir_scheduler_batches := map lower_fn_cfg_loop_queue_program (src_scheduler_batches p) |}.

Definition emit_r_fn_cfg_loop_scheduler_program
    (p : mir_fn_cfg_loop_scheduler_program) : r_fn_cfg_loop_scheduler_program :=
  {| r_scheduler_batches := map emit_r_fn_cfg_loop_queue_program (mir_scheduler_batches p) |}.

Definition eval_r_fn_cfg_loop_scheduler_program (p : r_fn_cfg_loop_scheduler_program) : list loop_queue_eval :=
  map eval_r_fn_cfg_loop_queue_program (r_scheduler_batches p).

Definition stable_fn_cfg_loop_scheduler_program : src_fn_cfg_loop_scheduler_program :=
  {| src_scheduler_batches := [stable_fn_cfg_loop_queue_program; stable_fn_cfg_loop_queue_program] |}.

Lemma stable_fn_cfg_loop_scheduler_program_meta_preserved :
  List.length (mir_scheduler_batches (lower_fn_cfg_loop_scheduler_program stable_fn_cfg_loop_scheduler_program)) = 2%nat.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_loop_scheduler_program_eval_preserved :
  eval_r_fn_cfg_loop_scheduler_program
    (emit_r_fn_cfg_loop_scheduler_program (lower_fn_cfg_loop_scheduler_program stable_fn_cfg_loop_scheduler_program)) =
    [[([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])];
     [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])]].
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_loop_scheduler_program_preserved :
  r_loop_scheduler_witness
    (emit_r_fn_cfg_loop_scheduler_program (lower_fn_cfg_loop_scheduler_program stable_fn_cfg_loop_scheduler_program)).
Proof.
  intros batch hIn.
  simpl in hIn.
  destruct hIn as [hIn | [hIn | []]].
  - subst batch. exact stable_fn_cfg_loop_queue_program_preserved.
  - subst batch. exact stable_fn_cfg_loop_queue_program_preserved.
Qed.

End RRPipelineFnCfgLoopSchedulerSubset.
