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

Module RRPipelineFnCfgHaltDiscoverSubset.

Record src_fn_cfg_halt_discover_program : Type := {
  src_halt_protocol_prog : src_fn_cfg_convergence_protocol_program;
  src_halt_search_space : list adaptive_summary;
  src_halt_selected : adaptive_summary;
}.

Record mir_fn_cfg_halt_discover_program : Type := {
  mir_halt_protocol_prog : mir_fn_cfg_convergence_protocol_program;
  mir_halt_search_space : list adaptive_summary;
  mir_halt_selected : adaptive_summary;
}.

Record r_fn_cfg_halt_discover_program : Type := {
  r_halt_protocol_prog : r_fn_cfg_convergence_protocol_program;
  r_halt_search_space : list adaptive_summary;
  r_halt_selected : adaptive_summary;
}.

Definition r_halt_discover_witness (p : r_fn_cfg_halt_discover_program) : Prop :=
  r_convergence_protocol_witness (r_halt_protocol_prog p) /\
  In (r_halt_selected p) (r_halt_search_space p) /\
  r_halt_selected p = r_conv_halt_summary (r_halt_protocol_prog p).

Definition lower_fn_cfg_halt_discover_program
    (p : src_fn_cfg_halt_discover_program) : mir_fn_cfg_halt_discover_program :=
  {| mir_halt_protocol_prog := lower_fn_cfg_convergence_protocol_program (src_halt_protocol_prog p);
     mir_halt_search_space := src_halt_search_space p;
     mir_halt_selected := src_halt_selected p |}.

Definition emit_r_fn_cfg_halt_discover_program
    (p : mir_fn_cfg_halt_discover_program) : r_fn_cfg_halt_discover_program :=
  {| r_halt_protocol_prog := emit_r_fn_cfg_convergence_protocol_program (mir_halt_protocol_prog p);
     r_halt_search_space := mir_halt_search_space p;
     r_halt_selected := mir_halt_selected p |}.

Definition eval_r_fn_cfg_halt_discover_program (p : r_fn_cfg_halt_discover_program) : adaptive_summary :=
  r_conv_halt_summary (r_halt_protocol_prog p).

Definition stable_fn_cfg_halt_discover_program : src_fn_cfg_halt_discover_program :=
  {| src_halt_protocol_prog := stable_fn_cfg_convergence_protocol_program;
     src_halt_search_space := [[]; stable_closed_loop_summary];
     src_halt_selected := stable_closed_loop_summary |}.

Lemma stable_fn_cfg_halt_discover_program_meta_preserved :
  List.length (mir_summary_rounds
    (mir_conv_summary_prog (mir_halt_protocol_prog
      (lower_fn_cfg_halt_discover_program stable_fn_cfg_halt_discover_program)))) = 2%nat /\
  mir_halt_search_space (lower_fn_cfg_halt_discover_program stable_fn_cfg_halt_discover_program) =
    [[]; stable_closed_loop_summary] /\
  mir_halt_selected (lower_fn_cfg_halt_discover_program stable_fn_cfg_halt_discover_program) =
    stable_closed_loop_summary.
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_halt_discover_program_eval_preserved :
  eval_r_fn_cfg_halt_discover_program
    (emit_r_fn_cfg_halt_discover_program (lower_fn_cfg_halt_discover_program stable_fn_cfg_halt_discover_program)) =
    stable_closed_loop_summary.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_halt_discover_program_preserved :
  r_halt_discover_witness
    (emit_r_fn_cfg_halt_discover_program (lower_fn_cfg_halt_discover_program stable_fn_cfg_halt_discover_program)).
Proof.
  split.
  - exact stable_fn_cfg_convergence_protocol_program_preserved.
  - split.
    + simpl. tauto.
    + reflexivity.
Qed.

End RRPipelineFnCfgHaltDiscoverSubset.
