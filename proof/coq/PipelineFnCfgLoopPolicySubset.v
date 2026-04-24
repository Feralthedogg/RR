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

Module RRPipelineFnCfgLoopPolicySubset.

Definition priority_rule := (nat * nat)%type.

Fixpoint rewrite_priority (rules : list priority_rule) (prio : nat) : nat :=
  match rules with
  | [] => prio
  | (src, dst) :: rest =>
      if Nat.eqb src prio then dst else rewrite_priority rest prio
  end.

Definition priority_policy_trace := priority_trace.

Record src_fn_cfg_loop_policy_program : Type := {
  src_policy_priority_prog : src_fn_cfg_loop_priority_program;
  src_policy_rules : list priority_rule;
}.

Record mir_fn_cfg_loop_policy_program : Type := {
  mir_policy_priority_prog : mir_fn_cfg_loop_priority_program;
  mir_policy_rules : list priority_rule;
}.

Record r_fn_cfg_loop_policy_program : Type := {
  r_policy_priority_prog : r_fn_cfg_loop_priority_program;
  r_policy_rules : list priority_rule;
}.

Definition r_loop_policy_witness (p : r_fn_cfg_loop_policy_program) : Prop :=
  r_loop_priority_witness (r_policy_priority_prog p).

Definition lower_fn_cfg_loop_policy_program
    (p : src_fn_cfg_loop_policy_program) : mir_fn_cfg_loop_policy_program :=
  {| mir_policy_priority_prog := lower_fn_cfg_loop_priority_program (src_policy_priority_prog p);
     mir_policy_rules := src_policy_rules p |}.

Definition emit_r_fn_cfg_loop_policy_program
    (p : mir_fn_cfg_loop_policy_program) : r_fn_cfg_loop_policy_program :=
  {| r_policy_priority_prog := emit_r_fn_cfg_loop_priority_program (mir_policy_priority_prog p);
     r_policy_rules := mir_policy_rules p |}.

Definition eval_r_priority_policy_trace (rules : list priority_rule) (trace : priority_trace) : priority_policy_trace :=
  map (fun '(prio, updates) => (rewrite_priority rules prio, updates)) trace.

Definition eval_r_fn_cfg_loop_policy_program (p : r_fn_cfg_loop_policy_program) : priority_policy_trace :=
  eval_r_priority_policy_trace (r_policy_rules p)
    (eval_r_fn_cfg_loop_priority_program (r_policy_priority_prog p)).

Definition stable_fn_cfg_loop_policy_program : src_fn_cfg_loop_policy_program :=
  {| src_policy_priority_prog := stable_fn_cfg_loop_priority_program;
     src_policy_rules := [(5%nat, 3%nat); (4%nat, 2%nat); (3%nat, 1%nat)] |}.

Lemma stable_fn_cfg_loop_policy_program_meta_preserved :
  List.length (mir_priority_pending
    (mir_policy_priority_prog (lower_fn_cfg_loop_policy_program stable_fn_cfg_loop_policy_program))) = 2%nat /\
  List.length (mir_priority_reinserts
    (mir_policy_priority_prog (lower_fn_cfg_loop_policy_program stable_fn_cfg_loop_policy_program))) = 1%nat /\
  mir_policy_rules (lower_fn_cfg_loop_policy_program stable_fn_cfg_loop_policy_program) =
    [(5%nat, 3%nat); (4%nat, 2%nat); (3%nat, 1%nat)].
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_loop_policy_program_eval_preserved :
  eval_r_fn_cfg_loop_policy_program
    (emit_r_fn_cfg_loop_policy_program (lower_fn_cfg_loop_policy_program stable_fn_cfg_loop_policy_program)) =
    [(3%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])]);
     (2%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])]);
     (1%nat, [([RVInt 10; RVInt 5], [RVInt 12]); ([RVInt 10; RVInt 5], [RVInt 12])])].
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_loop_policy_program_preserved :
  r_loop_policy_witness
    (emit_r_fn_cfg_loop_policy_program (lower_fn_cfg_loop_policy_program stable_fn_cfg_loop_policy_program)).
Proof.
  unfold r_loop_policy_witness.
  exact stable_fn_cfg_loop_priority_program_preserved.
Qed.

End RRPipelineFnCfgLoopPolicySubset.
