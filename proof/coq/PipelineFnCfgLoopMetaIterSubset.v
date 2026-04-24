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

Module RRPipelineFnCfgLoopMetaIterSubset.

Definition adaptive_summary := priority_trace.

Fixpoint last_priority_summary (traces : list adaptive_round_trace) : adaptive_summary :=
  match traces with
  | [] => []
  | [trace] => trace
  | _ :: rest => last_priority_summary rest
  end.

Record src_fn_cfg_loop_meta_iter_program : Type := {
  src_meta_closed_prog : src_fn_cfg_loop_closed_loop_program;
  src_meta_summary : adaptive_summary;
}.

Record mir_fn_cfg_loop_meta_iter_program : Type := {
  mir_meta_closed_prog : mir_fn_cfg_loop_closed_loop_program;
  mir_meta_summary : adaptive_summary;
}.

Record r_fn_cfg_loop_meta_iter_program : Type := {
  r_meta_closed_prog : r_fn_cfg_loop_closed_loop_program;
  r_meta_summary : adaptive_summary;
}.

Definition r_loop_meta_iter_witness (p : r_fn_cfg_loop_meta_iter_program) : Prop :=
  last_priority_summary (eval_r_fn_cfg_loop_closed_loop_program (r_meta_closed_prog p)) = r_meta_summary p.

Definition lower_fn_cfg_loop_meta_iter_program
    (p : src_fn_cfg_loop_meta_iter_program) : mir_fn_cfg_loop_meta_iter_program :=
  {| mir_meta_closed_prog := lower_fn_cfg_loop_closed_loop_program (src_meta_closed_prog p);
     mir_meta_summary := src_meta_summary p |}.

Definition emit_r_fn_cfg_loop_meta_iter_program
    (p : mir_fn_cfg_loop_meta_iter_program) : r_fn_cfg_loop_meta_iter_program :=
  {| r_meta_closed_prog := emit_r_fn_cfg_loop_closed_loop_program (mir_meta_closed_prog p);
     r_meta_summary := mir_meta_summary p |}.

Definition eval_r_fn_cfg_loop_meta_iter_program (p : r_fn_cfg_loop_meta_iter_program) : adaptive_summary :=
  last_priority_summary (eval_r_fn_cfg_loop_closed_loop_program (r_meta_closed_prog p)).

Definition stable_closed_loop_summary : adaptive_summary :=
  [(3%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])]);
   (2%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])]);
   (1%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])])].

Definition stable_fn_cfg_loop_meta_iter_program : src_fn_cfg_loop_meta_iter_program :=
  {| src_meta_closed_prog := stable_fn_cfg_loop_closed_loop_program;
     src_meta_summary := stable_closed_loop_summary |}.

Lemma stable_fn_cfg_loop_meta_iter_program_meta_preserved :
  List.length (mir_closed_rounds
    (mir_meta_closed_prog (lower_fn_cfg_loop_meta_iter_program stable_fn_cfg_loop_meta_iter_program))) = 2%nat /\
  mir_meta_summary (lower_fn_cfg_loop_meta_iter_program stable_fn_cfg_loop_meta_iter_program) =
    stable_closed_loop_summary.
Proof.
  split; reflexivity.
Qed.

Lemma stable_fn_cfg_loop_meta_iter_program_eval_preserved :
  eval_r_fn_cfg_loop_meta_iter_program
    (emit_r_fn_cfg_loop_meta_iter_program (lower_fn_cfg_loop_meta_iter_program stable_fn_cfg_loop_meta_iter_program)) =
    stable_closed_loop_summary.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_loop_meta_iter_program_preserved :
  r_loop_meta_iter_witness
    (emit_r_fn_cfg_loop_meta_iter_program (lower_fn_cfg_loop_meta_iter_program stable_fn_cfg_loop_meta_iter_program)).
Proof.
  exact stable_fn_cfg_loop_meta_iter_program_eval_preserved.
Qed.

End RRPipelineFnCfgLoopMetaIterSubset.
