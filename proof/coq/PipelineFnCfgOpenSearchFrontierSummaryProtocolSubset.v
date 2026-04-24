Require Import PipelineFnCfgOpenSearchFrontierMetaIterSubset.
Require Import PipelineFnCfgOpenSearchFrontierClosedLoopSubset.
Require Import PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.
Require Import PipelineFnCfgOpenSearchFrontierPolicySubset.
Require Import PipelineFnCfgOpenSearchFrontierPrioritySubset.
Require Import PipelineFnCfgOpenSearchFrontierSchedulerSubset.
Require Import PipelineFnCfgOpenSearchDynamicFrontierSubset.
Require Import PipelineFnCfgOpenSearchFrontierSubset.
Require Import PipelineFnCfgOpenSearchHaltDiscoverSubset.
Require Import PipelineFnCfgOpenSearchConvergenceProtocolSubset.
Require Import PipelineFnCfgOpenSearchSummaryProtocolSubset.
Require Import PipelineFnCfgOpenSearchMetaIterSubset.
Require Import PipelineFnCfgOpenSearchClosedLoopSubset.
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
Import RRPipelineFnCfgOpenSearchFrontierMetaIterSubset.
Import RRPipelineFnCfgOpenSearchFrontierClosedLoopSubset.
Import RRPipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.
Import RRPipelineFnCfgOpenSearchFrontierPolicySubset.
Import RRPipelineFnCfgOpenSearchFrontierPrioritySubset.
Import RRPipelineFnCfgOpenSearchFrontierSchedulerSubset.
Import RRPipelineFnCfgOpenSearchDynamicFrontierSubset.
Import RRPipelineFnCfgOpenSearchFrontierSubset.
Import RRPipelineFnCfgOpenSearchHaltDiscoverSubset.
Import RRPipelineFnCfgOpenSearchConvergenceProtocolSubset.
Import RRPipelineFnCfgOpenSearchSummaryProtocolSubset.
Import RRPipelineFnCfgOpenSearchMetaIterSubset.
Import RRPipelineFnCfgOpenSearchClosedLoopSubset.
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

Module RRPipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.

Definition open_search_frontier_summary_trace := list adaptive_summary.

Record src_fn_cfg_open_search_frontier_summary_protocol_program : Type := {
  src_frontier_summary_rounds : list src_fn_cfg_open_search_frontier_meta_iter_program;
  src_frontier_summary_stable : adaptive_summary;
}.

Record mir_fn_cfg_open_search_frontier_summary_protocol_program : Type := {
  mir_frontier_summary_rounds : list mir_fn_cfg_open_search_frontier_meta_iter_program;
  mir_frontier_summary_stable : adaptive_summary;
}.

Record r_fn_cfg_open_search_frontier_summary_protocol_program : Type := {
  r_frontier_summary_rounds : list r_fn_cfg_open_search_frontier_meta_iter_program;
  r_frontier_summary_stable : adaptive_summary;
}.

Fixpoint last_open_search_frontier_summary_trace
    (traces : open_search_frontier_summary_trace) : adaptive_summary :=
  match traces with
  | [] => []
  | [summary] => summary
  | _ :: rest => last_open_search_frontier_summary_trace rest
  end.

Definition r_open_search_frontier_summary_protocol_witness
    (p : r_fn_cfg_open_search_frontier_summary_protocol_program) : Prop :=
  last_open_search_frontier_summary_trace
    (List.map eval_r_fn_cfg_open_search_frontier_meta_iter_program (r_frontier_summary_rounds p)) =
    r_frontier_summary_stable p.

Definition lower_fn_cfg_open_search_frontier_summary_protocol_program
    (p : src_fn_cfg_open_search_frontier_summary_protocol_program)
    : mir_fn_cfg_open_search_frontier_summary_protocol_program :=
  {| mir_frontier_summary_rounds :=
       List.map lower_fn_cfg_open_search_frontier_meta_iter_program (src_frontier_summary_rounds p);
     mir_frontier_summary_stable := src_frontier_summary_stable p |}.

Definition emit_r_fn_cfg_open_search_frontier_summary_protocol_program
    (p : mir_fn_cfg_open_search_frontier_summary_protocol_program)
    : r_fn_cfg_open_search_frontier_summary_protocol_program :=
  {| r_frontier_summary_rounds :=
       List.map emit_r_fn_cfg_open_search_frontier_meta_iter_program (mir_frontier_summary_rounds p);
     r_frontier_summary_stable := mir_frontier_summary_stable p |}.

Definition eval_r_fn_cfg_open_search_frontier_summary_protocol_program
    (p : r_fn_cfg_open_search_frontier_summary_protocol_program)
    : open_search_frontier_summary_trace :=
  List.map eval_r_fn_cfg_open_search_frontier_meta_iter_program (r_frontier_summary_rounds p).

Definition stable_fn_cfg_open_search_frontier_summary_protocol_program :
    src_fn_cfg_open_search_frontier_summary_protocol_program :=
  {| src_frontier_summary_rounds :=
       [stable_fn_cfg_open_search_frontier_meta_iter_program;
        stable_fn_cfg_open_search_frontier_meta_iter_program];
     src_frontier_summary_stable := stable_closed_loop_summary |}.

Lemma stable_fn_cfg_open_search_frontier_summary_protocol_program_meta_preserved :
  List.length
    (mir_frontier_summary_rounds
      (lower_fn_cfg_open_search_frontier_summary_protocol_program
        stable_fn_cfg_open_search_frontier_summary_protocol_program)) = 2%nat /\
  mir_frontier_summary_stable
    (lower_fn_cfg_open_search_frontier_summary_protocol_program
      stable_fn_cfg_open_search_frontier_summary_protocol_program) =
    stable_closed_loop_summary.
Proof.
  split; reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_frontier_summary_protocol_program_eval_preserved :
  eval_r_fn_cfg_open_search_frontier_summary_protocol_program
    (emit_r_fn_cfg_open_search_frontier_summary_protocol_program
      (lower_fn_cfg_open_search_frontier_summary_protocol_program
        stable_fn_cfg_open_search_frontier_summary_protocol_program)) =
    [stable_closed_loop_summary; stable_closed_loop_summary].
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_frontier_summary_protocol_program_preserved :
  r_open_search_frontier_summary_protocol_witness
    (emit_r_fn_cfg_open_search_frontier_summary_protocol_program
      (lower_fn_cfg_open_search_frontier_summary_protocol_program
        stable_fn_cfg_open_search_frontier_summary_protocol_program)).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgOpenSearchFrontierSummaryProtocolSubset.
