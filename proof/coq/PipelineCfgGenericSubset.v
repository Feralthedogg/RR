Require Import LoweringSubset.
Require Import CodegenSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRCodegenSubset.

Module RRPipelineCfgGenericSubset.

Definition cfg_result_from_branches
    (cond : r_expr)
    (then_ret else_ret : r_expr) : option rvalue :=
  match eval_r_expr cond with
  | Some (RVBool true) => eval_r_expr then_ret
  | Some (RVBool false) => eval_r_expr else_ret
  | _ => None
  end.

Definition cfg_result_from_mir
    (cond : mir_expr)
    (then_ret else_ret : mir_expr) : option rvalue :=
  match eval_mir_fuel (S (mir_expr_depth cond)) cond with
  | Some (RVBool true) => eval_mir_fuel (S (mir_expr_depth then_ret)) then_ret
  | Some (RVBool false) => eval_mir_fuel (S (mir_expr_depth else_ret)) else_ret
  | _ => None
  end.

Definition cfg_result_from_src_codegen
    (cond then_ret else_ret : src_expr) : option rvalue :=
  match eval_src_fuel (S (mir_expr_depth (lower cond))) cond with
  | Some (RVBool true) => eval_src_fuel (S (mir_expr_depth (lower then_ret))) then_ret
  | Some (RVBool false) => eval_src_fuel (S (mir_expr_depth (lower else_ret))) else_ret
  | _ => None
  end.

Lemma emit_r_cfg_preserves_eval :
  forall cond then_ret else_ret,
    cfg_result_from_branches (emit_r cond) (emit_r then_ret) (emit_r else_ret) =
    cfg_result_from_mir cond then_ret else_ret.
Proof.
  intros cond then_ret else_ret.
  unfold cfg_result_from_branches, cfg_result_from_mir.
  rewrite emit_r_preserves_eval.
  rewrite emit_r_preserves_eval.
  rewrite emit_r_preserves_eval.
  reflexivity.
Qed.

Lemma lower_emit_cfg_preserves_eval :
  forall cond then_ret else_ret,
    cfg_result_from_branches (emit_r (lower cond)) (emit_r (lower then_ret)) (emit_r (lower else_ret)) =
    cfg_result_from_src_codegen cond then_ret else_ret.
Proof.
  intros cond then_ret else_ret.
  rewrite emit_r_cfg_preserves_eval.
  unfold cfg_result_from_mir, cfg_result_from_src_codegen.
  rewrite !lower_preserves_eval_fuel.
  reflexivity.
Qed.

End RRPipelineCfgGenericSubset.
