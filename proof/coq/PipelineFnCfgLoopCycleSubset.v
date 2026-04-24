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

Module RRPipelineFnCfgLoopCycleSubset.

Record src_fn_cfg_loop_cycle_program : Type := {
  src_loop_reentry_prog : src_fn_cfg_reentry_program;
  src_loop_acc_name : string;
  src_loop_cycle_name : string;
  src_loop_init : rvalue;
  src_loop_choices : list branch_choice;
  src_loop_block : src_block_env_program;
}.

Record mir_fn_cfg_loop_cycle_program : Type := {
  mir_loop_reentry_prog : mir_fn_cfg_reentry_program;
  mir_loop_acc_name : string;
  mir_loop_cycle_name : string;
  mir_loop_init : rvalue;
  mir_loop_choices : list branch_choice;
  mir_loop_block : mir_block_env_program;
}.

Record r_fn_cfg_loop_cycle_program : Type := {
  r_loop_reentry_prog : r_fn_cfg_reentry_program;
  r_loop_acc_name : string;
  r_loop_cycle_name : string;
  r_loop_init : rvalue;
  r_loop_choices : list branch_choice;
  r_loop_block : r_block_env_program;
}.

Fixpoint eval_src_loop_choices
    (p : src_fn_cfg_loop_cycle_program) (choices : list branch_choice) (current : rvalue) : option rvalue :=
  match choices with
  | [] => Some current
  | choice :: rest =>
      match eval_src_fn_cfg_reentry_program (src_loop_reentry_prog p) choice with
      | Some cycle_val =>
          match eval_src_block_env_program
                  {| src_block_id := src_block_id (src_loop_block p);
                     src_block_in_env :=
                       ((src_loop_acc_name p, current) ::
                        (src_loop_cycle_name p, cycle_val) ::
                        src_block_in_env (src_loop_block p));
                     src_block_stmts := src_block_stmts (src_loop_block p);
                     src_block_ret := src_block_ret (src_loop_block p) |} with
          | Some next => eval_src_loop_choices p rest next
          | None => None
          end
      | None => None
      end
  end.

Fixpoint eval_r_loop_choices
    (p : r_fn_cfg_loop_cycle_program) (choices : list branch_choice) (current : rvalue) : option rvalue :=
  match choices with
  | [] => Some current
  | choice :: rest =>
      match eval_r_fn_cfg_reentry_program (r_loop_reentry_prog p) choice with
      | Some cycle_val =>
          match eval_r_block_env_program
                  {| r_block_id := r_block_id (r_loop_block p);
                     r_block_in_env :=
                       ((r_loop_acc_name p, current) ::
                        (r_loop_cycle_name p, cycle_val) ::
                        r_block_in_env (r_loop_block p));
                     r_block_stmts := r_block_stmts (r_loop_block p);
                     r_block_ret := r_block_ret (r_loop_block p) |} with
          | Some next => eval_r_loop_choices p rest next
          | None => None
          end
      | None => None
      end
  end.

Definition lower_fn_cfg_loop_cycle_program
    (p : src_fn_cfg_loop_cycle_program) : mir_fn_cfg_loop_cycle_program :=
  {| mir_loop_reentry_prog := lower_fn_cfg_reentry_program (src_loop_reentry_prog p);
     mir_loop_acc_name := src_loop_acc_name p;
     mir_loop_cycle_name := src_loop_cycle_name p;
     mir_loop_init := src_loop_init p;
     mir_loop_choices := src_loop_choices p;
     mir_loop_block := lower_block_env_program (src_loop_block p) |}.

Definition emit_r_fn_cfg_loop_cycle_program
    (p : mir_fn_cfg_loop_cycle_program) : r_fn_cfg_loop_cycle_program :=
  {| r_loop_reentry_prog := emit_r_fn_cfg_reentry_program (mir_loop_reentry_prog p);
     r_loop_acc_name := mir_loop_acc_name p;
     r_loop_cycle_name := mir_loop_cycle_name p;
     r_loop_init := mir_loop_init p;
     r_loop_choices := mir_loop_choices p;
     r_loop_block := emit_r_block_env_program (mir_loop_block p) |}.

Definition eval_src_fn_cfg_loop_cycle_program (p : src_fn_cfg_loop_cycle_program) : option rvalue :=
  eval_src_loop_choices p (src_loop_choices p) (src_loop_init p).

Definition eval_r_fn_cfg_loop_cycle_program (p : r_fn_cfg_loop_cycle_program) : option rvalue :=
  eval_r_loop_choices p (r_loop_choices p) (r_loop_init p).

Definition branching_fn_cfg_loop_cycle_program : src_fn_cfg_loop_cycle_program :=
  {| src_loop_reentry_prog := branching_fn_cfg_reentry_program;
     src_loop_acc_name := "acc";
     src_loop_cycle_name := "cycle";
     src_loop_init := RVInt 1;
     src_loop_choices := [ThenBranch; ElseBranch; ThenBranch];
     src_loop_block :=
       {| src_block_id := 31%nat;
          src_block_in_env := [];
          src_block_stmts := [SAssign "bonus" (SConstInt 1)];
          src_block_ret := SLAdd (SLVar "acc") (SLAdd (SLVar "cycle") (SLVar "bonus")) |} |}.

Lemma branching_fn_cfg_loop_cycle_program_meta_preserved :
  mir_phi_join_bid
      (mir_join_exec_phi_prog
        (mir_reentry_join_exec_prog (mir_loop_reentry_prog (lower_fn_cfg_loop_cycle_program branching_fn_cfg_loop_cycle_program)))) = 17%nat /\
  mir_loop_choices (lower_fn_cfg_loop_cycle_program branching_fn_cfg_loop_cycle_program) =
    [ThenBranch; ElseBranch; ThenBranch] /\
  mir_block_id (mir_loop_block (lower_fn_cfg_loop_cycle_program branching_fn_cfg_loop_cycle_program)) = 31%nat.
Proof.
  repeat split; reflexivity.
Qed.

Lemma branching_fn_cfg_loop_cycle_program_preserved :
  eval_r_fn_cfg_loop_cycle_program
    (emit_r_fn_cfg_loop_cycle_program (lower_fn_cfg_loop_cycle_program branching_fn_cfg_loop_cycle_program)) =
    Some (RVInt 122).
Proof.
  reflexivity.
Qed.

End RRPipelineFnCfgLoopCycleSubset.
