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

Module RRPipelineFnCfgDynamicOpenSearchSubset.

Record src_fn_cfg_dynamic_open_search_program : Type := {
  src_dyn_open_prog : src_fn_cfg_open_search_program;
  src_dyn_discovered : list adaptive_summary;
  src_dyn_next_frontier : list adaptive_summary;
}.

Record mir_fn_cfg_dynamic_open_search_program : Type := {
  mir_dyn_open_prog : mir_fn_cfg_open_search_program;
  mir_dyn_discovered : list adaptive_summary;
  mir_dyn_next_frontier : list adaptive_summary;
}.

Record r_fn_cfg_dynamic_open_search_program : Type := {
  r_dyn_open_prog : r_fn_cfg_open_search_program;
  r_dyn_discovered : list adaptive_summary;
  r_dyn_next_frontier : list adaptive_summary;
}.

Definition r_dynamic_open_search_witness (p : r_fn_cfg_dynamic_open_search_program) : Prop :=
  r_open_search_witness (r_dyn_open_prog p) /\
  r_dyn_next_frontier p =
    List.app (r_open_frontier (r_dyn_open_prog p)) (r_dyn_discovered p) /\
  In (r_halt_selected (r_open_halt_prog (r_dyn_open_prog p))) (r_dyn_next_frontier p).

Definition lower_fn_cfg_dynamic_open_search_program
    (p : src_fn_cfg_dynamic_open_search_program) : mir_fn_cfg_dynamic_open_search_program :=
  {| mir_dyn_open_prog := lower_fn_cfg_open_search_program (src_dyn_open_prog p);
     mir_dyn_discovered := src_dyn_discovered p;
     mir_dyn_next_frontier := src_dyn_next_frontier p |}.

Definition emit_r_fn_cfg_dynamic_open_search_program
    (p : mir_fn_cfg_dynamic_open_search_program) : r_fn_cfg_dynamic_open_search_program :=
  {| r_dyn_open_prog := emit_r_fn_cfg_open_search_program (mir_dyn_open_prog p);
     r_dyn_discovered := mir_dyn_discovered p;
     r_dyn_next_frontier := mir_dyn_next_frontier p |}.

Definition eval_r_fn_cfg_dynamic_open_search_program
    (p : r_fn_cfg_dynamic_open_search_program) : adaptive_summary :=
  eval_r_fn_cfg_open_search_program (r_dyn_open_prog p).

Definition stable_fn_cfg_dynamic_open_search_program :
    src_fn_cfg_dynamic_open_search_program :=
  {| src_dyn_open_prog := stable_fn_cfg_open_search_program;
     src_dyn_discovered := [[]];
     src_dyn_next_frontier := [stable_closed_loop_summary; []] |}.

Lemma stable_fn_cfg_dynamic_open_search_program_meta_preserved :
  List.length (mir_summary_rounds
    (mir_conv_summary_prog (mir_halt_protocol_prog (mir_open_halt_prog (mir_dyn_open_prog
      (lower_fn_cfg_dynamic_open_search_program stable_fn_cfg_dynamic_open_search_program)))))) = 2%nat /\
  mir_dyn_discovered (lower_fn_cfg_dynamic_open_search_program stable_fn_cfg_dynamic_open_search_program) = [[]] /\
  mir_dyn_next_frontier
    (lower_fn_cfg_dynamic_open_search_program stable_fn_cfg_dynamic_open_search_program) =
    [stable_closed_loop_summary; []].
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_dynamic_open_search_program_eval_preserved :
  eval_r_fn_cfg_dynamic_open_search_program
    (emit_r_fn_cfg_dynamic_open_search_program
      (lower_fn_cfg_dynamic_open_search_program stable_fn_cfg_dynamic_open_search_program)) =
    stable_closed_loop_summary.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_dynamic_open_search_program_preserved :
  r_dynamic_open_search_witness
    (emit_r_fn_cfg_dynamic_open_search_program
      (lower_fn_cfg_dynamic_open_search_program stable_fn_cfg_dynamic_open_search_program)).
Proof.
  split.
  - exact stable_fn_cfg_open_search_program_preserved.
  - split.
    + reflexivity.
    + simpl. tauto.
Qed.

End RRPipelineFnCfgDynamicOpenSearchSubset.
