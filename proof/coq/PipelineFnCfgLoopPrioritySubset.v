Require Import PipelineFnCfgLoopDynamicSchedulerSubset.
Require Import PipelineFnCfgLoopSchedulerSubset.
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
Import RRPipelineFnCfgLoopDynamicSchedulerSubset.
Import RRPipelineFnCfgLoopSchedulerSubset.
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

Module RRPipelineFnCfgLoopPrioritySubset.

Definition priority_trace := list (nat * loop_queue_eval).

Record src_fn_cfg_loop_priority_program : Type := {
  src_priority_pending : list (nat * src_fn_cfg_loop_queue_program);
  src_priority_reinserts : list (nat * src_fn_cfg_loop_queue_program);
}.

Record mir_fn_cfg_loop_priority_program : Type := {
  mir_priority_pending : list (nat * mir_fn_cfg_loop_queue_program);
  mir_priority_reinserts : list (nat * mir_fn_cfg_loop_queue_program);
}.

Record r_fn_cfg_loop_priority_program : Type := {
  r_priority_pending : list (nat * r_fn_cfg_loop_queue_program);
  r_priority_reinserts : list (nat * r_fn_cfg_loop_queue_program);
}.

Definition r_loop_priority_witness (p : r_fn_cfg_loop_priority_program) : Prop :=
  forall pair, In pair (r_priority_pending p ++ r_priority_reinserts p) ->
    r_loop_queue_witness (snd pair).

Definition lower_fn_cfg_loop_priority_program
    (p : src_fn_cfg_loop_priority_program) : mir_fn_cfg_loop_priority_program :=
  {| mir_priority_pending :=
       map (fun '(prio, batch) => (prio, lower_fn_cfg_loop_queue_program batch)) (src_priority_pending p);
     mir_priority_reinserts :=
       map (fun '(prio, batch) => (prio, lower_fn_cfg_loop_queue_program batch)) (src_priority_reinserts p) |}.

Definition emit_r_fn_cfg_loop_priority_program
    (p : mir_fn_cfg_loop_priority_program) : r_fn_cfg_loop_priority_program :=
  {| r_priority_pending :=
       map (fun '(prio, batch) => (prio, emit_r_fn_cfg_loop_queue_program batch)) (mir_priority_pending p);
     r_priority_reinserts :=
       map (fun '(prio, batch) => (prio, emit_r_fn_cfg_loop_queue_program batch)) (mir_priority_reinserts p) |}.

Definition eval_r_priority_batches (batches : list (nat * r_fn_cfg_loop_queue_program)) : priority_trace :=
  map (fun '(prio, batch) => (prio, eval_r_fn_cfg_loop_queue_program batch)) batches.

Definition eval_r_fn_cfg_loop_priority_program (p : r_fn_cfg_loop_priority_program) : priority_trace :=
  eval_r_priority_batches (r_priority_pending p ++ r_priority_reinserts p).

Definition stable_fn_cfg_loop_priority_program : src_fn_cfg_loop_priority_program :=
  {| src_priority_pending := [(5%nat, stable_fn_cfg_loop_queue_program); (4%nat, stable_fn_cfg_loop_queue_program)];
     src_priority_reinserts := [(3%nat, stable_fn_cfg_loop_queue_program)] |}.

Lemma stable_fn_cfg_loop_priority_program_meta_preserved :
  List.length (mir_priority_pending (lower_fn_cfg_loop_priority_program stable_fn_cfg_loop_priority_program)) = 2%nat /\
  List.length (mir_priority_reinserts (lower_fn_cfg_loop_priority_program stable_fn_cfg_loop_priority_program)) = 1%nat.
Proof.
  split; reflexivity.
Qed.

Lemma stable_fn_cfg_loop_priority_program_eval_preserved :
  eval_r_fn_cfg_loop_priority_program
    (emit_r_fn_cfg_loop_priority_program (lower_fn_cfg_loop_priority_program stable_fn_cfg_loop_priority_program)) =
    [(5%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])]);
     (4%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])]);
     (3%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])])].
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_loop_priority_program_preserved :
  r_loop_priority_witness
    (emit_r_fn_cfg_loop_priority_program (lower_fn_cfg_loop_priority_program stable_fn_cfg_loop_priority_program)).
Proof.
  intros pair hIn.
  simpl in hIn.
  destruct hIn as [hIn | [hIn | [hIn | []]]].
  - subst pair. exact stable_fn_cfg_loop_queue_program_preserved.
  - subst pair. exact stable_fn_cfg_loop_queue_program_preserved.
  - subst pair. exact stable_fn_cfg_loop_queue_program_preserved.
Qed.

End RRPipelineFnCfgLoopPrioritySubset.
