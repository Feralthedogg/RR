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

Module RRPipelineFnCfgLoopAdaptivePolicySubset.

Definition adaptive_priority_trace := priority_policy_trace.

Record src_fn_cfg_loop_adaptive_policy_program : Type := {
  src_adapt_priority_prog : src_fn_cfg_loop_priority_program;
  src_adapt_base_rules : list priority_rule;
  src_adapt_feedback : list nat;
}.

Record mir_fn_cfg_loop_adaptive_policy_program : Type := {
  mir_adapt_priority_prog : mir_fn_cfg_loop_priority_program;
  mir_adapt_base_rules : list priority_rule;
  mir_adapt_feedback : list nat;
}.

Record r_fn_cfg_loop_adaptive_policy_program : Type := {
  r_adapt_priority_prog : r_fn_cfg_loop_priority_program;
  r_adapt_base_rules : list priority_rule;
  r_adapt_feedback : list nat;
}.

Fixpoint recompute_priority_rules (rules : list priority_rule) (feedback : list nat) : list priority_rule :=
  match rules, feedback with
  | (src, _) :: rules', dst :: feedback' =>
      (src, dst) :: recompute_priority_rules rules' feedback'
  | _, _ => []
  end.

Definition r_loop_adaptive_policy_witness (p : r_fn_cfg_loop_adaptive_policy_program) : Prop :=
  r_loop_priority_witness (r_adapt_priority_prog p).

Definition lower_fn_cfg_loop_adaptive_policy_program
    (p : src_fn_cfg_loop_adaptive_policy_program) : mir_fn_cfg_loop_adaptive_policy_program :=
  {| mir_adapt_priority_prog := lower_fn_cfg_loop_priority_program (src_adapt_priority_prog p);
     mir_adapt_base_rules := src_adapt_base_rules p;
     mir_adapt_feedback := src_adapt_feedback p |}.

Definition emit_r_fn_cfg_loop_adaptive_policy_program
    (p : mir_fn_cfg_loop_adaptive_policy_program) : r_fn_cfg_loop_adaptive_policy_program :=
  {| r_adapt_priority_prog := emit_r_fn_cfg_loop_priority_program (mir_adapt_priority_prog p);
     r_adapt_base_rules := mir_adapt_base_rules p;
     r_adapt_feedback := mir_adapt_feedback p |}.

Definition eval_r_fn_cfg_loop_adaptive_policy_program (p : r_fn_cfg_loop_adaptive_policy_program)
  : adaptive_priority_trace :=
  eval_r_priority_policy_trace
    (recompute_priority_rules (r_adapt_base_rules p) (r_adapt_feedback p))
    (eval_r_fn_cfg_loop_priority_program (r_adapt_priority_prog p)).

Definition stable_fn_cfg_loop_adaptive_policy_program : src_fn_cfg_loop_adaptive_policy_program :=
  {| src_adapt_priority_prog := stable_fn_cfg_loop_priority_program;
     src_adapt_base_rules := [(5%nat, 9%nat); (4%nat, 9%nat); (3%nat, 9%nat)];
     src_adapt_feedback := [3%nat; 2%nat; 1%nat] |}.

Lemma stable_fn_cfg_loop_adaptive_policy_program_meta_preserved :
  List.length (mir_priority_pending
    (mir_adapt_priority_prog
      (lower_fn_cfg_loop_adaptive_policy_program stable_fn_cfg_loop_adaptive_policy_program))) = 2%nat /\
  List.length (mir_priority_reinserts
    (mir_adapt_priority_prog
      (lower_fn_cfg_loop_adaptive_policy_program stable_fn_cfg_loop_adaptive_policy_program))) = 1%nat /\
  mir_adapt_base_rules (lower_fn_cfg_loop_adaptive_policy_program stable_fn_cfg_loop_adaptive_policy_program) =
    [(5%nat, 9%nat); (4%nat, 9%nat); (3%nat, 9%nat)] /\
  mir_adapt_feedback (lower_fn_cfg_loop_adaptive_policy_program stable_fn_cfg_loop_adaptive_policy_program) =
    [3%nat; 2%nat; 1%nat].
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_loop_adaptive_policy_program_eval_preserved :
  eval_r_fn_cfg_loop_adaptive_policy_program
    (emit_r_fn_cfg_loop_adaptive_policy_program
      (lower_fn_cfg_loop_adaptive_policy_program stable_fn_cfg_loop_adaptive_policy_program)) =
    [(3%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])]);
     (2%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])]);
     (1%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])])].
Proof.
  vm_compute. reflexivity.
Qed.

Lemma stable_fn_cfg_loop_adaptive_policy_program_preserved :
  r_loop_adaptive_policy_witness
    (emit_r_fn_cfg_loop_adaptive_policy_program
      (lower_fn_cfg_loop_adaptive_policy_program stable_fn_cfg_loop_adaptive_policy_program)).
Proof.
  unfold r_loop_adaptive_policy_witness.
  exact stable_fn_cfg_loop_priority_program_preserved.
Qed.

End RRPipelineFnCfgLoopAdaptivePolicySubset.
