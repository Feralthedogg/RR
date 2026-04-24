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

Module RRPipelineFnCfgLoopWorklistSubset.

Record src_fn_cfg_loop_worklist_program : Type := {
  src_work_discover_prog : src_fn_cfg_loop_discover_program;
  src_work_remaining : list rvalue;
  src_work_done : list rvalue;
}.

Record mir_fn_cfg_loop_worklist_program : Type := {
  mir_work_discover_prog : mir_fn_cfg_loop_discover_program;
  mir_work_remaining : list rvalue;
  mir_work_done : list rvalue;
}.

Record r_fn_cfg_loop_worklist_program : Type := {
  r_work_discover_prog : r_fn_cfg_loop_discover_program;
  r_work_remaining : list rvalue;
  r_work_done : list rvalue;
}.

Definition r_loop_worklist_update (p : r_fn_cfg_loop_worklist_program) : list rvalue * list rvalue :=
  (r_discover_selected (r_work_discover_prog p) :: r_work_done p, r_work_remaining p).

Definition r_loop_worklist_witness (p : r_fn_cfg_loop_worklist_program) : Prop :=
  r_discover_worklist (r_work_discover_prog p) =
    r_discover_selected (r_work_discover_prog p) :: r_work_remaining p /\
  r_loop_discover_witness (r_work_discover_prog p).

Definition lower_fn_cfg_loop_worklist_program
    (p : src_fn_cfg_loop_worklist_program) : mir_fn_cfg_loop_worklist_program :=
  {| mir_work_discover_prog := lower_fn_cfg_loop_discover_program (src_work_discover_prog p);
     mir_work_remaining := src_work_remaining p;
     mir_work_done := src_work_done p |}.

Definition emit_r_fn_cfg_loop_worklist_program
    (p : mir_fn_cfg_loop_worklist_program) : r_fn_cfg_loop_worklist_program :=
  {| r_work_discover_prog := emit_r_fn_cfg_loop_discover_program (mir_work_discover_prog p);
     r_work_remaining := mir_work_remaining p;
     r_work_done := mir_work_done p |}.

Definition stable_head_fn_cfg_loop_discover_program : src_fn_cfg_loop_discover_program :=
  {| src_discover_cycle_prog := stable_fn_cfg_loop_cycle_program;
     src_discover_choice := ThenBranch;
     src_discover_worklist := [RVInt 10; RVInt 12];
     src_discover_selected := RVInt 10 |}.

Definition stable_fn_cfg_loop_worklist_program : src_fn_cfg_loop_worklist_program :=
  {| src_work_discover_prog := stable_head_fn_cfg_loop_discover_program;
     src_work_remaining := [RVInt 12];
     src_work_done := [RVInt 5] |}.

Lemma stable_fn_cfg_loop_worklist_program_meta_preserved :
  mir_phi_join_bid
      (mir_join_exec_phi_prog
        (mir_reentry_join_exec_prog
          (mir_loop_reentry_prog
            (mir_discover_cycle_prog
              (mir_work_discover_prog (lower_fn_cfg_loop_worklist_program stable_fn_cfg_loop_worklist_program)))))) = 17%nat /\
  mir_work_remaining (lower_fn_cfg_loop_worklist_program stable_fn_cfg_loop_worklist_program) = [RVInt 12] /\
  mir_work_done (lower_fn_cfg_loop_worklist_program stable_fn_cfg_loop_worklist_program) = [RVInt 5].
Proof.
  repeat split; reflexivity.
Qed.

Lemma stable_fn_cfg_loop_worklist_program_update_preserved :
  r_loop_worklist_update
    (emit_r_fn_cfg_loop_worklist_program (lower_fn_cfg_loop_worklist_program stable_fn_cfg_loop_worklist_program)) =
    ([RVInt 10; RVInt 5], [RVInt 12]).
Proof.
  reflexivity.
Qed.

Lemma stable_fn_cfg_loop_worklist_program_preserved :
  r_loop_worklist_witness
    (emit_r_fn_cfg_loop_worklist_program (lower_fn_cfg_loop_worklist_program stable_fn_cfg_loop_worklist_program)).
Proof.
  split.
  - reflexivity.
  - split.
    + simpl. tauto.
    + exact stable_fn_cfg_loop_fixpoint_program_preserved.
Qed.

End RRPipelineFnCfgLoopWorklistSubset.
