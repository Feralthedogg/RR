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

Module RRPipelineFnCfgOpenSearchPolicySubset.

Definition open_search_priority_rule : Type := (nat * nat)%type.

Definition rewrite_open_search_priority (rules : list open_search_priority_rule) (prio : nat) : nat :=
  match List.find (fun entry => Nat.eqb (fst entry) prio) rules with
  | Some (_, new_prio) => new_prio
  | None => prio
  end.

Definition rewrite_open_search_rounds
    (rules : list open_search_priority_rule)
    (rounds : list open_search_priority_round) : list open_search_priority_round :=
  List.map (fun '(prio, updates) => (rewrite_open_search_priority rules prio, updates)) rounds.

Record src_fn_cfg_open_search_policy_program : Type := {
  src_policy_prio_prog : src_fn_cfg_open_search_priority_program;
  src_policy_rules : list open_search_priority_rule;
  src_policy_normalized_rounds : list open_search_priority_round;
}.

Record mir_fn_cfg_open_search_policy_program : Type := {
  mir_policy_prio_prog : mir_fn_cfg_open_search_priority_program;
  mir_policy_rules : list open_search_priority_rule;
  mir_policy_normalized_rounds : list open_search_priority_round;
}.

Record r_fn_cfg_open_search_policy_program : Type := {
  r_policy_prio_prog : r_fn_cfg_open_search_priority_program;
  r_policy_rules : list open_search_priority_rule;
  r_policy_normalized_rounds : list open_search_priority_round;
}.

Definition r_open_search_policy_witness (p : r_fn_cfg_open_search_policy_program) : Prop :=
  r_open_search_priority_witness (r_policy_prio_prog p) /\
  r_policy_normalized_rounds p =
    rewrite_open_search_rounds (r_policy_rules p) (r_prio_rounds (r_policy_prio_prog p)).

Definition lower_fn_cfg_open_search_policy_program
    (p : src_fn_cfg_open_search_policy_program) : mir_fn_cfg_open_search_policy_program :=
  {| mir_policy_prio_prog := lower_fn_cfg_open_search_priority_program (src_policy_prio_prog p);
     mir_policy_rules := src_policy_rules p;
     mir_policy_normalized_rounds := src_policy_normalized_rounds p |}.

Definition emit_r_fn_cfg_open_search_policy_program
    (p : mir_fn_cfg_open_search_policy_program) : r_fn_cfg_open_search_policy_program :=
  {| r_policy_prio_prog := emit_r_fn_cfg_open_search_priority_program (mir_policy_prio_prog p);
     r_policy_rules := mir_policy_rules p;
     r_policy_normalized_rounds := mir_policy_normalized_rounds p |}.

Definition eval_r_fn_cfg_open_search_policy_program
    (p : r_fn_cfg_open_search_policy_program) : adaptive_summary :=
  eval_r_fn_cfg_open_search_priority_program (r_policy_prio_prog p).

Definition stable_fn_cfg_open_search_policy_program :
    src_fn_cfg_open_search_policy_program :=
  {| src_policy_prio_prog := stable_fn_cfg_open_search_priority_program;
     src_policy_rules := [(5%nat, 3%nat); (3%nat, 1%nat)];
     src_policy_normalized_rounds := [(3%nat, [stable_closed_loop_summary; []]); (1%nat, [stable_closed_loop_summary])] |}.

Lemma stable_fn_cfg_open_search_policy_program_meta_preserved :
  List.length (mir_summary_rounds
    (mir_conv_summary_prog (mir_halt_protocol_prog (mir_open_halt_prog (mir_dyn_open_prog
      (mir_sched_dyn_prog (mir_prio_sched_prog (mir_policy_prio_prog
        (lower_fn_cfg_open_search_policy_program stable_fn_cfg_open_search_policy_program))))))))) = 2%nat /\
  mir_policy_rules (lower_fn_cfg_open_search_policy_program stable_fn_cfg_open_search_policy_program) =
    [(5%nat, 3%nat); (3%nat, 1%nat)] /\
  mir_policy_normalized_rounds
    (lower_fn_cfg_open_search_policy_program stable_fn_cfg_open_search_policy_program) =
    [(3%nat, [stable_closed_loop_summary; []]); (1%nat, [stable_closed_loop_summary])].
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_policy_program_eval_preserved :
  eval_r_fn_cfg_open_search_policy_program
    (emit_r_fn_cfg_open_search_policy_program
      (lower_fn_cfg_open_search_policy_program stable_fn_cfg_open_search_policy_program)) =
    stable_closed_loop_summary.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_policy_program_preserved :
  r_open_search_policy_witness
    (emit_r_fn_cfg_open_search_policy_program
      (lower_fn_cfg_open_search_policy_program stable_fn_cfg_open_search_policy_program)).
Proof.
  split.
  - exact stable_fn_cfg_open_search_priority_program_preserved.
  - reflexivity.
Qed.

End RRPipelineFnCfgOpenSearchPolicySubset.
