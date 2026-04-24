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

Module RRPipelineFnCfgOpenSearchFrontierPrioritySubset.

Definition open_search_frontier_priority_round : Type := (nat * list adaptive_summary)%type.

Record src_fn_cfg_open_search_frontier_priority_program : Type := {
  src_frontier_prio_sched_prog : src_fn_cfg_open_search_frontier_scheduler_program;
  src_frontier_prio_tail : list open_search_frontier_priority_round;
  src_frontier_prio_rounds : list open_search_frontier_priority_round;
}.

Record mir_fn_cfg_open_search_frontier_priority_program : Type := {
  mir_frontier_prio_sched_prog : mir_fn_cfg_open_search_frontier_scheduler_program;
  mir_frontier_prio_tail : list open_search_frontier_priority_round;
  mir_frontier_prio_rounds : list open_search_frontier_priority_round;
}.

Record r_fn_cfg_open_search_frontier_priority_program : Type := {
  r_frontier_prio_sched_prog : r_fn_cfg_open_search_frontier_scheduler_program;
  r_frontier_prio_tail : list open_search_frontier_priority_round;
  r_frontier_prio_rounds : list open_search_frontier_priority_round;
}.

Definition r_open_search_frontier_priority_witness
    (p : r_fn_cfg_open_search_frontier_priority_program) : Prop :=
  r_open_search_frontier_scheduler_witness (r_frontier_prio_sched_prog p) /\
  r_frontier_prio_rounds p =
    (5%nat, r_dyn_frontier_next (r_open_sched_dyn_prog (r_frontier_prio_sched_prog p))) ::
      r_frontier_prio_tail p /\
  In (r_open_halt_selected
        (r_open_frontier_halt_prog (r_dyn_frontier_prog (r_open_sched_dyn_prog (r_frontier_prio_sched_prog p)))))
    (r_dyn_frontier_next (r_open_sched_dyn_prog (r_frontier_prio_sched_prog p))).

Definition lower_fn_cfg_open_search_frontier_priority_program
    (p : src_fn_cfg_open_search_frontier_priority_program)
    : mir_fn_cfg_open_search_frontier_priority_program :=
  {| mir_frontier_prio_sched_prog :=
       lower_fn_cfg_open_search_frontier_scheduler_program (src_frontier_prio_sched_prog p);
     mir_frontier_prio_tail := src_frontier_prio_tail p;
     mir_frontier_prio_rounds := src_frontier_prio_rounds p |}.

Definition emit_r_fn_cfg_open_search_frontier_priority_program
    (p : mir_fn_cfg_open_search_frontier_priority_program)
    : r_fn_cfg_open_search_frontier_priority_program :=
  {| r_frontier_prio_sched_prog :=
       emit_r_fn_cfg_open_search_frontier_scheduler_program (mir_frontier_prio_sched_prog p);
     r_frontier_prio_tail := mir_frontier_prio_tail p;
     r_frontier_prio_rounds := mir_frontier_prio_rounds p |}.

Definition eval_r_fn_cfg_open_search_frontier_priority_program
    (p : r_fn_cfg_open_search_frontier_priority_program) : adaptive_summary :=
  eval_r_fn_cfg_open_search_frontier_scheduler_program (r_frontier_prio_sched_prog p).

Definition stable_fn_cfg_open_search_frontier_priority_program :
    src_fn_cfg_open_search_frontier_priority_program :=
  {| src_frontier_prio_sched_prog := stable_fn_cfg_open_search_frontier_scheduler_program;
     src_frontier_prio_tail := [(3%nat, [stable_closed_loop_summary])];
     src_frontier_prio_rounds :=
       [(5%nat, [stable_closed_loop_summary; []]); (3%nat, [stable_closed_loop_summary])] |}.

Lemma stable_fn_cfg_open_search_frontier_priority_program_meta_preserved :
  List.length
    (mir_open_summary_rounds
      (mir_open_conv_summary_prog
        (mir_open_halt_protocol_prog
          (mir_open_frontier_halt_prog
            (mir_dyn_frontier_prog
              (mir_open_sched_dyn_prog
                (mir_frontier_prio_sched_prog
                  (lower_fn_cfg_open_search_frontier_priority_program
                    stable_fn_cfg_open_search_frontier_priority_program)))))))) = 2%nat /\
  mir_frontier_prio_tail
    (lower_fn_cfg_open_search_frontier_priority_program
      stable_fn_cfg_open_search_frontier_priority_program) =
    [(3%nat, [stable_closed_loop_summary])] /\
  mir_frontier_prio_rounds
    (lower_fn_cfg_open_search_frontier_priority_program
      stable_fn_cfg_open_search_frontier_priority_program) =
    [(5%nat, [stable_closed_loop_summary; []]); (3%nat, [stable_closed_loop_summary])].
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_frontier_priority_program_eval_preserved :
  eval_r_fn_cfg_open_search_frontier_priority_program
    (emit_r_fn_cfg_open_search_frontier_priority_program
      (lower_fn_cfg_open_search_frontier_priority_program
        stable_fn_cfg_open_search_frontier_priority_program)) =
    stable_closed_loop_summary.
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_open_search_frontier_priority_program_preserved :
  r_open_search_frontier_priority_witness
    (emit_r_fn_cfg_open_search_frontier_priority_program
      (lower_fn_cfg_open_search_frontier_priority_program
        stable_fn_cfg_open_search_frontier_priority_program)).
Proof.
  split.
  - exact stable_fn_cfg_open_search_frontier_scheduler_program_preserved.
  - split.
    + reflexivity.
    + simpl. tauto.
Qed.

End RRPipelineFnCfgOpenSearchFrontierPrioritySubset.
