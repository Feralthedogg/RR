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

Module RRPipelineFnCfgOpenSearchDynamicFrontierSubset.

Record src_fn_cfg_open_search_dynamic_frontier_program : Type := {
  src_dyn_frontier_prog : src_fn_cfg_open_search_frontier_program;
  src_dyn_frontier_discovered : list adaptive_summary;
  src_dyn_frontier_next : list adaptive_summary;
}.

Record mir_fn_cfg_open_search_dynamic_frontier_program : Type := {
  mir_dyn_frontier_prog : mir_fn_cfg_open_search_frontier_program;
  mir_dyn_frontier_discovered : list adaptive_summary;
  mir_dyn_frontier_next : list adaptive_summary;
}.

Record r_fn_cfg_open_search_dynamic_frontier_program : Type := {
  r_dyn_frontier_prog : r_fn_cfg_open_search_frontier_program;
  r_dyn_frontier_discovered : list adaptive_summary;
  r_dyn_frontier_next : list adaptive_summary;
}.

Definition r_open_search_dynamic_frontier_witness
    (p : r_fn_cfg_open_search_dynamic_frontier_program) : Prop :=
  r_open_search_frontier_witness (r_dyn_frontier_prog p) /\
  r_dyn_frontier_next p =
    List.app (r_open_frontier_frontier (r_dyn_frontier_prog p)) (r_dyn_frontier_discovered p) /\
  In (r_open_halt_selected (r_open_frontier_halt_prog (r_dyn_frontier_prog p)))
    (r_dyn_frontier_next p).

Definition lower_fn_cfg_open_search_dynamic_frontier_program
    (p : src_fn_cfg_open_search_dynamic_frontier_program)
    : mir_fn_cfg_open_search_dynamic_frontier_program :=
  {| mir_dyn_frontier_prog := lower_fn_cfg_open_search_frontier_program (src_dyn_frontier_prog p);
     mir_dyn_frontier_discovered := src_dyn_frontier_discovered p;
     mir_dyn_frontier_next := src_dyn_frontier_next p |}.

Definition emit_r_fn_cfg_open_search_dynamic_frontier_program
    (p : mir_fn_cfg_open_search_dynamic_frontier_program)
    : r_fn_cfg_open_search_dynamic_frontier_program :=
  {| r_dyn_frontier_prog := emit_r_fn_cfg_open_search_frontier_program (mir_dyn_frontier_prog p);
     r_dyn_frontier_discovered := mir_dyn_frontier_discovered p;
     r_dyn_frontier_next := mir_dyn_frontier_next p |}.

Definition eval_r_fn_cfg_open_search_dynamic_frontier_program
    (p : r_fn_cfg_open_search_dynamic_frontier_program) : adaptive_summary :=
  eval_r_fn_cfg_open_search_frontier_program (r_dyn_frontier_prog p).

Definition stable_fn_cfg_open_search_dynamic_frontier_program :
    src_fn_cfg_open_search_dynamic_frontier_program :=
  {| src_dyn_frontier_prog := stable_fn_cfg_open_search_frontier_program;
     src_dyn_frontier_discovered := [[]];
     src_dyn_frontier_next := [stable_closed_loop_summary; []] |}.

Lemma stable_fn_cfg_open_search_dynamic_frontier_program_meta_preserved :
  List.length
    (mir_open_summary_rounds
      (mir_open_conv_summary_prog
        (mir_open_halt_protocol_prog
          (mir_open_frontier_halt_prog
            (mir_dyn_frontier_prog
              (lower_fn_cfg_open_search_dynamic_frontier_program
                stable_fn_cfg_open_search_dynamic_frontier_program)))))) = 2%nat /\
  mir_dyn_frontier_discovered
    (lower_fn_cfg_open_search_dynamic_frontier_program
      stable_fn_cfg_open_search_dynamic_frontier_program) = [[]] /\
  mir_dyn_frontier_next
    (lower_fn_cfg_open_search_dynamic_frontier_program
      stable_fn_cfg_open_search_dynamic_frontier_program) =
    [stable_closed_loop_summary; []].
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_dynamic_frontier_program_eval_preserved :
  eval_r_fn_cfg_open_search_dynamic_frontier_program
    (emit_r_fn_cfg_open_search_dynamic_frontier_program
      (lower_fn_cfg_open_search_dynamic_frontier_program
        stable_fn_cfg_open_search_dynamic_frontier_program)) =
    stable_closed_loop_summary.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_dynamic_frontier_program_preserved :
  r_open_search_dynamic_frontier_witness
    (emit_r_fn_cfg_open_search_dynamic_frontier_program
      (lower_fn_cfg_open_search_dynamic_frontier_program
        stable_fn_cfg_open_search_dynamic_frontier_program)).
Proof.
  split.
  - exact stable_fn_cfg_open_search_frontier_program_preserved.
  - split.
    + reflexivity.
    + simpl. tauto.
Qed.

End RRPipelineFnCfgOpenSearchDynamicFrontierSubset.
