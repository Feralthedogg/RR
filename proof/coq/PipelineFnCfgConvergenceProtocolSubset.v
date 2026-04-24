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

Module RRPipelineFnCfgConvergenceProtocolSubset.

Record src_fn_cfg_convergence_protocol_program : Type := {
  src_conv_summary_prog : src_fn_cfg_summary_protocol_program;
  src_conv_halt_summary : adaptive_summary;
}.

Record mir_fn_cfg_convergence_protocol_program : Type := {
  mir_conv_summary_prog : mir_fn_cfg_summary_protocol_program;
  mir_conv_halt_summary : adaptive_summary;
}.

Record r_fn_cfg_convergence_protocol_program : Type := {
  r_conv_summary_prog : r_fn_cfg_summary_protocol_program;
  r_conv_halt_summary : adaptive_summary;
}.

Definition r_convergence_protocol_witness (p : r_fn_cfg_convergence_protocol_program) : Prop :=
  r_summary_protocol_witness (r_conv_summary_prog p) /\
  last_priority_summary_trace (eval_r_fn_cfg_summary_protocol_program (r_conv_summary_prog p)) =
    r_conv_halt_summary p.

Definition lower_fn_cfg_convergence_protocol_program
    (p : src_fn_cfg_convergence_protocol_program) : mir_fn_cfg_convergence_protocol_program :=
  {| mir_conv_summary_prog := lower_fn_cfg_summary_protocol_program (src_conv_summary_prog p);
     mir_conv_halt_summary := src_conv_halt_summary p |}.

Definition emit_r_fn_cfg_convergence_protocol_program
    (p : mir_fn_cfg_convergence_protocol_program) : r_fn_cfg_convergence_protocol_program :=
  {| r_conv_summary_prog := emit_r_fn_cfg_summary_protocol_program (mir_conv_summary_prog p);
     r_conv_halt_summary := mir_conv_halt_summary p |}.

Definition eval_r_fn_cfg_convergence_protocol_program (p : r_fn_cfg_convergence_protocol_program) : adaptive_summary :=
  last_priority_summary_trace (eval_r_fn_cfg_summary_protocol_program (r_conv_summary_prog p)).

Definition stable_fn_cfg_convergence_protocol_program : src_fn_cfg_convergence_protocol_program :=
  {| src_conv_summary_prog := stable_fn_cfg_summary_protocol_program;
     src_conv_halt_summary := stable_closed_loop_summary |}.

Lemma stable_fn_cfg_convergence_protocol_program_meta_preserved :
  List.length (mir_summary_rounds
    (mir_conv_summary_prog (lower_fn_cfg_convergence_protocol_program stable_fn_cfg_convergence_protocol_program))) = 2%nat /\
  mir_conv_halt_summary (lower_fn_cfg_convergence_protocol_program stable_fn_cfg_convergence_protocol_program) =
    stable_closed_loop_summary.
Proof.
  split; reflexivity.
Qed.

Lemma stable_fn_cfg_convergence_protocol_program_eval_preserved :
  eval_r_fn_cfg_convergence_protocol_program
    (emit_r_fn_cfg_convergence_protocol_program
      (lower_fn_cfg_convergence_protocol_program stable_fn_cfg_convergence_protocol_program)) =
    stable_closed_loop_summary.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_convergence_protocol_program_preserved :
  r_convergence_protocol_witness
    (emit_r_fn_cfg_convergence_protocol_program
      (lower_fn_cfg_convergence_protocol_program stable_fn_cfg_convergence_protocol_program)).
Proof.
  split.
  - exact stable_fn_cfg_summary_protocol_program_preserved.
  - exact stable_fn_cfg_convergence_protocol_program_eval_preserved.
Qed.

End RRPipelineFnCfgConvergenceProtocolSubset.
