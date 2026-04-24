Require Import LoweringSubset.
Require Import LoweringIfPhiSubset.
Require Import CodegenSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Lia.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.
Import RRLoweringIfPhiSubset.
Import RRCodegenSubset.

Module RRPipelineIfPhiGenericSubset.

Inductive r_if_phi_expr : Type :=
| RIPure : r_expr -> r_if_phi_expr
| RIfPhi : r_expr -> r_expr -> r_expr -> r_if_phi_expr.

Definition eval_r_if_phi (e : r_if_phi_expr) : option rvalue :=
  match e with
  | RIPure expr => eval_r_expr expr
  | RIfPhi cond then_val else_val =>
      match eval_r_expr cond with
      | Some (RVBool true) => eval_r_expr then_val
      | Some (RVBool false) => eval_r_expr else_val
      | _ => None
      end
  end.

Definition emit_r_if_phi (e : mir_if_phi_expr) : r_if_phi_expr :=
  match e with
  | MIPure expr => RIPure (emit_r expr)
  | MIfPhi cond then_val else_val =>
      RIfPhi (emit_r cond) (emit_r then_val) (emit_r else_val)
  end.

Definition eval_mir_if_phi_codegen (e : mir_if_phi_expr) : option rvalue :=
  match e with
  | MIPure expr => eval_mir_fuel (S (mir_expr_depth expr)) expr
  | MIfPhi cond then_val else_val =>
      match eval_mir_fuel (S (mir_expr_depth cond)) cond with
      | Some (RVBool true) => eval_mir_fuel (S (mir_expr_depth then_val)) then_val
      | Some (RVBool false) => eval_mir_fuel (S (mir_expr_depth else_val)) else_val
      | _ => None
      end
  end.

Definition eval_src_if_codegen (e : src_if_expr) : option rvalue :=
  match e with
  | SIPure expr => eval_src_fuel (S (mir_expr_depth (lower expr))) expr
  | SIte cond then_expr else_expr =>
      match eval_src_fuel (S (mir_expr_depth (lower cond))) cond with
      | Some (RVBool true) => eval_src_fuel (S (mir_expr_depth (lower then_expr))) then_expr
      | Some (RVBool false) => eval_src_fuel (S (mir_expr_depth (lower else_expr))) else_expr
      | _ => None
      end
  end.

Lemma emit_r_if_phi_preserves_eval_codegen :
  forall expr,
    eval_r_if_phi (emit_r_if_phi expr) = eval_mir_if_phi_codegen expr.
Proof.
  intros expr.
  destruct expr as [expr | cond then_val else_val]; simpl.
  - rewrite emit_r_preserves_eval. reflexivity.
  - rewrite emit_r_preserves_eval.
    rewrite emit_r_preserves_eval.
    rewrite emit_r_preserves_eval.
    reflexivity.
Qed.

Lemma lower_emit_if_phi_preserves_eval :
  forall expr,
    eval_r_if_phi (emit_r_if_phi (lower_if_phi expr)) = eval_src_if_codegen expr.
Proof.
  intros expr.
  destruct expr as [expr | cond then_expr else_expr]; simpl.
  - rewrite emit_r_preserves_eval.
    rewrite lower_preserves_eval_fuel by lia.
    reflexivity.
  - rewrite emit_r_preserves_eval.
    rewrite emit_r_preserves_eval.
    rewrite emit_r_preserves_eval.
    rewrite !lower_preserves_eval_fuel by lia.
    reflexivity.
Qed.

End RRPipelineIfPhiGenericSubset.
