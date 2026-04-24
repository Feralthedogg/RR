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

Module RRPipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.

Definition recompute_open_search_frontier_rules
    (rules : list open_search_frontier_priority_rule)
    (feedback : list nat) : list open_search_frontier_priority_rule :=
  let fix go rs fb :=
    match rs, fb with
    | (src, _) :: rs', dst :: fb' => (src, dst) :: go rs' fb'
    | _, _ => []
    end
  in go rules feedback.

Record src_fn_cfg_open_search_frontier_adaptive_policy_program : Type := {
  src_frontier_adapt_policy_prog : src_fn_cfg_open_search_frontier_policy_program;
  src_frontier_adapt_base_rules : list open_search_frontier_priority_rule;
  src_frontier_adapt_feedback : list nat;
  src_frontier_adapt_recomputed_rules : list open_search_frontier_priority_rule;
}.

Record mir_fn_cfg_open_search_frontier_adaptive_policy_program : Type := {
  mir_frontier_adapt_policy_prog : mir_fn_cfg_open_search_frontier_policy_program;
  mir_frontier_adapt_base_rules : list open_search_frontier_priority_rule;
  mir_frontier_adapt_feedback : list nat;
  mir_frontier_adapt_recomputed_rules : list open_search_frontier_priority_rule;
}.

Record r_fn_cfg_open_search_frontier_adaptive_policy_program : Type := {
  r_frontier_adapt_policy_prog : r_fn_cfg_open_search_frontier_policy_program;
  r_frontier_adapt_base_rules : list open_search_frontier_priority_rule;
  r_frontier_adapt_feedback : list nat;
  r_frontier_adapt_recomputed_rules : list open_search_frontier_priority_rule;
}.

Definition r_open_search_frontier_adaptive_policy_witness
    (p : r_fn_cfg_open_search_frontier_adaptive_policy_program) : Prop :=
  r_open_search_frontier_policy_witness (r_frontier_adapt_policy_prog p) /\
  r_frontier_adapt_recomputed_rules p =
    recompute_open_search_frontier_rules
      (r_frontier_adapt_base_rules p) (r_frontier_adapt_feedback p).

Definition lower_fn_cfg_open_search_frontier_adaptive_policy_program
    (p : src_fn_cfg_open_search_frontier_adaptive_policy_program)
    : mir_fn_cfg_open_search_frontier_adaptive_policy_program :=
  {| mir_frontier_adapt_policy_prog :=
       lower_fn_cfg_open_search_frontier_policy_program (src_frontier_adapt_policy_prog p);
     mir_frontier_adapt_base_rules := src_frontier_adapt_base_rules p;
     mir_frontier_adapt_feedback := src_frontier_adapt_feedback p;
     mir_frontier_adapt_recomputed_rules := src_frontier_adapt_recomputed_rules p |}.

Definition emit_r_fn_cfg_open_search_frontier_adaptive_policy_program
    (p : mir_fn_cfg_open_search_frontier_adaptive_policy_program)
    : r_fn_cfg_open_search_frontier_adaptive_policy_program :=
  {| r_frontier_adapt_policy_prog :=
       emit_r_fn_cfg_open_search_frontier_policy_program (mir_frontier_adapt_policy_prog p);
     r_frontier_adapt_base_rules := mir_frontier_adapt_base_rules p;
     r_frontier_adapt_feedback := mir_frontier_adapt_feedback p;
     r_frontier_adapt_recomputed_rules := mir_frontier_adapt_recomputed_rules p |}.

Definition eval_r_fn_cfg_open_search_frontier_adaptive_policy_program
    (p : r_fn_cfg_open_search_frontier_adaptive_policy_program) : adaptive_summary :=
  eval_r_fn_cfg_open_search_frontier_policy_program (r_frontier_adapt_policy_prog p).

Definition stable_fn_cfg_open_search_frontier_adaptive_policy_program :
    src_fn_cfg_open_search_frontier_adaptive_policy_program :=
  {| src_frontier_adapt_policy_prog := stable_fn_cfg_open_search_frontier_policy_program;
     src_frontier_adapt_base_rules := [(5%nat, 9%nat); (3%nat, 9%nat)];
     src_frontier_adapt_feedback := [3%nat; 1%nat];
     src_frontier_adapt_recomputed_rules := [(5%nat, 3%nat); (3%nat, 1%nat)] |}.

Lemma stable_fn_cfg_open_search_frontier_adaptive_policy_program_meta_preserved :
  List.length
    (mir_open_summary_rounds
      (mir_open_conv_summary_prog
        (mir_open_halt_protocol_prog
          (mir_open_frontier_halt_prog
            (mir_dyn_frontier_prog
              (mir_open_sched_dyn_prog
                (mir_frontier_prio_sched_prog
                  (mir_frontier_policy_prio_prog
                    (mir_frontier_adapt_policy_prog
                      (lower_fn_cfg_open_search_frontier_adaptive_policy_program
                        stable_fn_cfg_open_search_frontier_adaptive_policy_program)))))))))) = 2%nat /\
  mir_frontier_adapt_base_rules
    (lower_fn_cfg_open_search_frontier_adaptive_policy_program
      stable_fn_cfg_open_search_frontier_adaptive_policy_program) =
    [(5%nat, 9%nat); (3%nat, 9%nat)] /\
  mir_frontier_adapt_feedback
    (lower_fn_cfg_open_search_frontier_adaptive_policy_program
      stable_fn_cfg_open_search_frontier_adaptive_policy_program) =
    [3%nat; 1%nat] /\
  mir_frontier_adapt_recomputed_rules
    (lower_fn_cfg_open_search_frontier_adaptive_policy_program
      stable_fn_cfg_open_search_frontier_adaptive_policy_program) =
    [(5%nat, 3%nat); (3%nat, 1%nat)].
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_frontier_adaptive_policy_program_eval_preserved :
  eval_r_fn_cfg_open_search_frontier_adaptive_policy_program
    (emit_r_fn_cfg_open_search_frontier_adaptive_policy_program
      (lower_fn_cfg_open_search_frontier_adaptive_policy_program
        stable_fn_cfg_open_search_frontier_adaptive_policy_program)) =
    stable_closed_loop_summary.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_frontier_adaptive_policy_program_preserved :
  r_open_search_frontier_adaptive_policy_witness
    (emit_r_fn_cfg_open_search_frontier_adaptive_policy_program
      (lower_fn_cfg_open_search_frontier_adaptive_policy_program
        stable_fn_cfg_open_search_frontier_adaptive_policy_program)).
Proof.
  split.
  - exact stable_fn_cfg_open_search_frontier_policy_program_preserved.
  - reflexivity.
Qed.

End RRPipelineFnCfgOpenSearchFrontierAdaptivePolicySubset.
