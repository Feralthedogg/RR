Require Import PipelineFnCfgLoopMetaIterSubset.
Require Import PipelineFnCfgLoopClosedLoopSubset.
Require Import PipelineFnCfgLoopAdaptivePolicySubset.
Require Import PipelineFnCfgLoopPolicySubset.
Require Import PipelineFnCfgLoopPrioritySubset.
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
Import RRPipelineFnCfgLoopMetaIterSubset.
Import RRPipelineFnCfgLoopClosedLoopSubset.
Import RRPipelineFnCfgLoopAdaptivePolicySubset.
Import RRPipelineFnCfgLoopPolicySubset.
Import RRPipelineFnCfgLoopPrioritySubset.
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

Module RRPipelineFnCfgSummaryProtocolSubset.

Definition summary_trace := list adaptive_summary.

Record src_fn_cfg_summary_protocol_program : Type := {
  src_summary_rounds : list src_fn_cfg_loop_meta_iter_program;
  src_summary_stable : adaptive_summary;
}.

Record mir_fn_cfg_summary_protocol_program : Type := {
  mir_summary_rounds : list mir_fn_cfg_loop_meta_iter_program;
  mir_summary_stable : adaptive_summary;
}.

Record r_fn_cfg_summary_protocol_program : Type := {
  r_summary_rounds : list r_fn_cfg_loop_meta_iter_program;
  r_summary_stable : adaptive_summary;
}.

Fixpoint last_priority_summary_trace (traces : summary_trace) : adaptive_summary :=
  match traces with
  | [] => []
  | [summary] => summary
  | _ :: rest => last_priority_summary_trace rest
  end.

Definition r_summary_protocol_witness (p : r_fn_cfg_summary_protocol_program) : Prop :=
  forall round, In round (r_summary_rounds p) -> r_loop_meta_iter_witness round.

Definition lower_fn_cfg_summary_protocol_program
    (p : src_fn_cfg_summary_protocol_program) : mir_fn_cfg_summary_protocol_program :=
  {| mir_summary_rounds := map lower_fn_cfg_loop_meta_iter_program (src_summary_rounds p);
     mir_summary_stable := src_summary_stable p |}.

Definition emit_r_fn_cfg_summary_protocol_program
    (p : mir_fn_cfg_summary_protocol_program) : r_fn_cfg_summary_protocol_program :=
  {| r_summary_rounds := map emit_r_fn_cfg_loop_meta_iter_program (mir_summary_rounds p);
     r_summary_stable := mir_summary_stable p |}.

Definition eval_r_fn_cfg_summary_protocol_program (p : r_fn_cfg_summary_protocol_program) : summary_trace :=
  map eval_r_fn_cfg_loop_meta_iter_program (r_summary_rounds p).

Definition stable_fn_cfg_summary_protocol_program : src_fn_cfg_summary_protocol_program :=
  {| src_summary_rounds := [stable_fn_cfg_loop_meta_iter_program; stable_fn_cfg_loop_meta_iter_program];
     src_summary_stable := stable_closed_loop_summary |}.

Lemma stable_fn_cfg_summary_protocol_program_meta_preserved :
  List.length (mir_summary_rounds (lower_fn_cfg_summary_protocol_program stable_fn_cfg_summary_protocol_program)) = 2%nat /\
  mir_summary_stable (lower_fn_cfg_summary_protocol_program stable_fn_cfg_summary_protocol_program) =
    stable_closed_loop_summary.
Proof.
  split; reflexivity.
Qed.

Lemma stable_fn_cfg_summary_protocol_program_eval_preserved :
  eval_r_fn_cfg_summary_protocol_program
    (emit_r_fn_cfg_summary_protocol_program (lower_fn_cfg_summary_protocol_program stable_fn_cfg_summary_protocol_program)) =
    [stable_closed_loop_summary; stable_closed_loop_summary].
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_summary_protocol_program_preserved :
  r_summary_protocol_witness
    (emit_r_fn_cfg_summary_protocol_program (lower_fn_cfg_summary_protocol_program stable_fn_cfg_summary_protocol_program)).
Proof.
  intros round hIn.
  simpl in hIn.
  destruct hIn as [hIn | [hIn | []]].
  - subst round. exact stable_fn_cfg_loop_meta_iter_program_preserved.
  - subst round. exact stable_fn_cfg_loop_meta_iter_program_preserved.
Qed.

End RRPipelineFnCfgSummaryProtocolSubset.
