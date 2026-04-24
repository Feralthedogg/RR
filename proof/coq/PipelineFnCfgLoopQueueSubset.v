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

Module RRPipelineFnCfgLoopQueueSubset.

Definition loop_update := (list rvalue * list rvalue)%type.

Record src_fn_cfg_loop_queue_program : Type := {
  src_queue_rounds : list src_fn_cfg_loop_worklist_program;
}.

Record mir_fn_cfg_loop_queue_program : Type := {
  mir_queue_rounds : list mir_fn_cfg_loop_worklist_program;
}.

Record r_fn_cfg_loop_queue_program : Type := {
  r_queue_rounds : list r_fn_cfg_loop_worklist_program;
}.

Definition src_loop_worklist_update (p : src_fn_cfg_loop_worklist_program) : list rvalue * list rvalue :=
  (src_discover_selected (src_work_discover_prog p) :: src_work_done p, src_work_remaining p).

Definition eval_src_fn_cfg_loop_queue_program (p : src_fn_cfg_loop_queue_program) : list loop_update :=
  map src_loop_worklist_update (src_queue_rounds p).

Definition r_loop_queue_witness (p : r_fn_cfg_loop_queue_program) : Prop :=
  forall round, In round (r_queue_rounds p) -> r_loop_worklist_witness round.

Definition lower_fn_cfg_loop_queue_program
    (p : src_fn_cfg_loop_queue_program) : mir_fn_cfg_loop_queue_program :=
  {| mir_queue_rounds := map lower_fn_cfg_loop_worklist_program (src_queue_rounds p) |}.

Definition emit_r_fn_cfg_loop_queue_program
    (p : mir_fn_cfg_loop_queue_program) : r_fn_cfg_loop_queue_program :=
  {| r_queue_rounds := map emit_r_fn_cfg_loop_worklist_program (mir_queue_rounds p) |}.

Definition eval_r_fn_cfg_loop_queue_program (p : r_fn_cfg_loop_queue_program) : list loop_update :=
  map r_loop_worklist_update (r_queue_rounds p).

Definition stable_fn_cfg_loop_queue_program : src_fn_cfg_loop_queue_program :=
  {| src_queue_rounds := [stable_fn_cfg_loop_worklist_program; stable_fn_cfg_loop_worklist_program] |}.

Lemma stable_fn_cfg_loop_queue_program_meta_preserved :
  List.length (mir_queue_rounds (lower_fn_cfg_loop_queue_program stable_fn_cfg_loop_queue_program)) = 2%nat.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_loop_queue_program_eval_preserved :
  eval_r_fn_cfg_loop_queue_program
    (emit_r_fn_cfg_loop_queue_program (lower_fn_cfg_loop_queue_program stable_fn_cfg_loop_queue_program)) =
    [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])].
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_loop_queue_program_preserved :
  r_loop_queue_witness
    (emit_r_fn_cfg_loop_queue_program (lower_fn_cfg_loop_queue_program stable_fn_cfg_loop_queue_program)).
Proof.
  intros round hIn.
  simpl in hIn.
  destruct hIn as [hIn | [hIn | []]].
  - subst round. exact stable_fn_cfg_loop_worklist_program_preserved.
  - subst round. exact stable_fn_cfg_loop_worklist_program_preserved.
Qed.

End RRPipelineFnCfgLoopQueueSubset.
