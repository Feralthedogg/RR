Require Import PipelineFnCfgOpenSearchAdaptivePolicySubset.
Require Import PipelineFnCfgOpenSearchPolicySubset.
Require Import PipelineFnCfgOpenSearchPrioritySubset.
Require Import PipelineFnCfgOpenSearchSchedulerSubset.
Require Import PipelineFnCfgDynamicOpenSearchSubset.
Require Import PipelineFnCfgOpenSearchSubset.
Require Import PipelineFnCfgHaltDiscoverSubset.
Require Import PipelineFnCfgConvergenceProtocolSubset.
Require Import PipelineFnCfgSummaryProtocolSubset.
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
Import RRPipelineFnCfgOpenSearchAdaptivePolicySubset.
Import RRPipelineFnCfgOpenSearchPolicySubset.
Import RRPipelineFnCfgOpenSearchPrioritySubset.
Import RRPipelineFnCfgOpenSearchSchedulerSubset.
Import RRPipelineFnCfgDynamicOpenSearchSubset.
Import RRPipelineFnCfgOpenSearchSubset.
Import RRPipelineFnCfgHaltDiscoverSubset.
Import RRPipelineFnCfgConvergenceProtocolSubset.
Import RRPipelineFnCfgSummaryProtocolSubset.
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

Module RRPipelineFnCfgOpenSearchClosedLoopSubset.

Definition adaptive_open_search_trace : Type := list adaptive_summary.

Record src_fn_cfg_open_search_closed_loop_program : Type := {
  src_closed_rounds : list src_fn_cfg_open_search_adaptive_policy_program;
}.

Record mir_fn_cfg_open_search_closed_loop_program : Type := {
  mir_closed_rounds : list mir_fn_cfg_open_search_adaptive_policy_program;
}.

Record r_fn_cfg_open_search_closed_loop_program : Type := {
  r_closed_rounds : list r_fn_cfg_open_search_adaptive_policy_program;
}.

Definition lower_fn_cfg_open_search_closed_loop_program
    (p : src_fn_cfg_open_search_closed_loop_program)
    : mir_fn_cfg_open_search_closed_loop_program :=
  {| mir_closed_rounds := List.map lower_fn_cfg_open_search_adaptive_policy_program (src_closed_rounds p) |}.

Definition emit_r_fn_cfg_open_search_closed_loop_program
    (p : mir_fn_cfg_open_search_closed_loop_program)
    : r_fn_cfg_open_search_closed_loop_program :=
  {| r_closed_rounds := List.map emit_r_fn_cfg_open_search_adaptive_policy_program (mir_closed_rounds p) |}.

Definition eval_r_fn_cfg_open_search_closed_loop_program
    (p : r_fn_cfg_open_search_closed_loop_program) : adaptive_open_search_trace :=
  List.map eval_r_fn_cfg_open_search_adaptive_policy_program (r_closed_rounds p).

Definition r_open_search_closed_loop_witness
    (p : r_fn_cfg_open_search_closed_loop_program) : Prop :=
  forall round, In round (r_closed_rounds p) -> r_open_search_adaptive_policy_witness round.

Definition stable_fn_cfg_open_search_closed_loop_program :
    src_fn_cfg_open_search_closed_loop_program :=
  {| src_closed_rounds :=
       [stable_fn_cfg_open_search_adaptive_policy_program;
        stable_fn_cfg_open_search_adaptive_policy_program] |}.

Lemma stable_fn_cfg_open_search_closed_loop_program_meta_preserved :
  List.length
    (mir_closed_rounds
      (lower_fn_cfg_open_search_closed_loop_program stable_fn_cfg_open_search_closed_loop_program)) = 2%nat.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_closed_loop_program_eval_preserved :
  eval_r_fn_cfg_open_search_closed_loop_program
    (emit_r_fn_cfg_open_search_closed_loop_program
      (lower_fn_cfg_open_search_closed_loop_program stable_fn_cfg_open_search_closed_loop_program)) =
    [stable_closed_loop_summary; stable_closed_loop_summary].
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_closed_loop_program_preserved :
  r_open_search_closed_loop_witness
    (emit_r_fn_cfg_open_search_closed_loop_program
      (lower_fn_cfg_open_search_closed_loop_program stable_fn_cfg_open_search_closed_loop_program)).
Proof.
  intros round hround.
  cbn in hround.
  destruct hround as [hround | [hround | []]]; subst round.
  - exact stable_fn_cfg_open_search_adaptive_policy_program_preserved.
  - exact stable_fn_cfg_open_search_adaptive_policy_program_preserved.
Qed.

End RRPipelineFnCfgOpenSearchClosedLoopSubset.
