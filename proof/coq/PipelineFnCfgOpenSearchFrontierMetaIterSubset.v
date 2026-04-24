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

Module RRPipelineFnCfgOpenSearchFrontierMetaIterSubset.

Definition last_open_search_frontier_summary
    : adaptive_open_search_frontier_trace -> adaptive_summary :=
  fix go traces :=
    match traces with
    | [] => []
    | [trace] => trace
    | _ :: rest => go rest
    end.

Record src_fn_cfg_open_search_frontier_meta_iter_program : Type := {
  src_frontier_meta_closed_prog : src_fn_cfg_open_search_frontier_closed_loop_program;
  src_frontier_meta_summary : adaptive_summary;
}.

Record mir_fn_cfg_open_search_frontier_meta_iter_program : Type := {
  mir_frontier_meta_closed_prog : mir_fn_cfg_open_search_frontier_closed_loop_program;
  mir_frontier_meta_summary : adaptive_summary;
}.

Record r_fn_cfg_open_search_frontier_meta_iter_program : Type := {
  r_frontier_meta_closed_prog : r_fn_cfg_open_search_frontier_closed_loop_program;
  r_frontier_meta_summary : adaptive_summary;
}.

Definition mir_open_search_frontier_closed_loop_round_count
    (p : mir_fn_cfg_open_search_frontier_closed_loop_program) : nat :=
  match p with
  | Build_mir_fn_cfg_open_search_frontier_closed_loop_program rounds => List.length rounds
  end.

Definition lower_fn_cfg_open_search_frontier_meta_iter_program
    (p : src_fn_cfg_open_search_frontier_meta_iter_program)
    : mir_fn_cfg_open_search_frontier_meta_iter_program :=
  {| mir_frontier_meta_closed_prog :=
       lower_fn_cfg_open_search_frontier_closed_loop_program (src_frontier_meta_closed_prog p);
     mir_frontier_meta_summary := src_frontier_meta_summary p |}.

Definition emit_r_fn_cfg_open_search_frontier_meta_iter_program
    (p : mir_fn_cfg_open_search_frontier_meta_iter_program)
    : r_fn_cfg_open_search_frontier_meta_iter_program :=
  {| r_frontier_meta_closed_prog :=
       emit_r_fn_cfg_open_search_frontier_closed_loop_program (mir_frontier_meta_closed_prog p);
     r_frontier_meta_summary := mir_frontier_meta_summary p |}.

Definition eval_r_fn_cfg_open_search_frontier_meta_iter_program
    (p : r_fn_cfg_open_search_frontier_meta_iter_program) : adaptive_summary :=
  last_open_search_frontier_summary
    (eval_r_fn_cfg_open_search_frontier_closed_loop_program (r_frontier_meta_closed_prog p)).

Definition stable_fn_cfg_open_search_frontier_meta_iter_program :
    src_fn_cfg_open_search_frontier_meta_iter_program :=
  {| src_frontier_meta_closed_prog := stable_fn_cfg_open_search_frontier_closed_loop_program;
     src_frontier_meta_summary := stable_closed_loop_summary |}.

Lemma stable_fn_cfg_open_search_frontier_meta_iter_program_meta_preserved :
  mir_open_search_frontier_closed_loop_round_count
    (mir_frontier_meta_closed_prog
      (lower_fn_cfg_open_search_frontier_meta_iter_program
        stable_fn_cfg_open_search_frontier_meta_iter_program)) = 2%nat /\
  mir_frontier_meta_summary
    (lower_fn_cfg_open_search_frontier_meta_iter_program
      stable_fn_cfg_open_search_frontier_meta_iter_program) =
    stable_closed_loop_summary.
Proof.
  split; reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_frontier_meta_iter_program_eval_preserved :
  eval_r_fn_cfg_open_search_frontier_meta_iter_program
    (emit_r_fn_cfg_open_search_frontier_meta_iter_program
      (lower_fn_cfg_open_search_frontier_meta_iter_program
        stable_fn_cfg_open_search_frontier_meta_iter_program)) =
    stable_closed_loop_summary.
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgOpenSearchFrontierMetaIterSubset.
