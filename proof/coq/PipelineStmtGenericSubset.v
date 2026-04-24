Require Import PipelineStmtSubset.
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
Import RRPipelineStmtSubset.

Module RRPipelineStmtGenericSubset.

Fixpoint eval_mir_local_codegen (rho : env) (e : mir_local_expr) : option rvalue :=
  match e with
  | MLPure expr => eval_mir_fuel (S (mir_expr_depth expr)) expr
  | MLVar name => lookup_field rho name
  | MLAdd lhs rhs =>
      match eval_mir_local_codegen rho lhs, eval_mir_local_codegen rho rhs with
      | Some (RVInt z1), Some (RVInt z2) => Some (RVInt (z1 + z2))
      | _, _ => None
      end
  | MLField base name =>
      match eval_mir_local_codegen rho base with
      | Some (RVRecord fields) => lookup_field fields name
      | _ => None
      end
  end.

Definition exec_mir_stmt_codegen (rho : env) (s : mir_stmt) : option env :=
  match s with
  | MAssign name rhs =>
      match eval_mir_fuel (S (mir_expr_depth rhs)) rhs with
      | Some v => Some ((name, v) :: rho)
      | None => None
      end
  | MIfAssign name cond then_rhs else_rhs =>
      match eval_mir_fuel (S (mir_expr_depth cond)) cond with
      | Some (RVBool true) =>
          match eval_mir_fuel (S (mir_expr_depth then_rhs)) then_rhs with
          | Some v => Some ((name, v) :: rho)
          | None => None
          end
      | Some (RVBool false) =>
          match eval_mir_fuel (S (mir_expr_depth else_rhs)) else_rhs with
          | Some v => Some ((name, v) :: rho)
          | None => None
          end
      | _ => None
      end
  end.

Fixpoint exec_mir_stmts_codegen (rho : env) (stmts : list mir_stmt) : option env :=
  match stmts with
  | [] => Some rho
  | stmt :: rest =>
      match exec_mir_stmt_codegen rho stmt with
      | Some rho' => exec_mir_stmts_codegen rho' rest
      | None => None
      end
  end.

Lemma emit_r_local_preserves_eval :
  forall rho expr,
    eval_r_local rho (emit_r_local expr) = eval_mir_local_codegen rho expr.
Proof.
  intros rho expr.
  induction expr as [expr|name|lhs IHlhs rhs IHrhs|base IHbase name]; simpl.
  - rewrite emit_r_preserves_eval. reflexivity.
  - reflexivity.
  - rewrite IHlhs, IHrhs.
    destruct (eval_mir_local_codegen rho lhs); [|reflexivity].
    destruct (eval_mir_local_codegen rho rhs); [|reflexivity].
    destruct r, r0; reflexivity.
  - rewrite IHbase.
    destruct (eval_mir_local_codegen rho base); [destruct r|]; reflexivity.
Qed.

Lemma emit_r_stmt_preserves_exec :
  forall rho stmt,
    exec_r_stmt rho (emit_r_stmt_from_mir stmt) = exec_mir_stmt_codegen rho stmt.
Proof.
  intros rho stmt.
  destruct stmt as [name rhs|name cond then_rhs else_rhs].
  - unfold emit_r_stmt_from_mir, exec_r_stmt, exec_mir_stmt_codegen.
    simpl.
    rewrite emit_r_preserves_eval.
    reflexivity.
  - unfold emit_r_stmt_from_mir, exec_r_stmt, exec_mir_stmt_codegen.
    simpl.
    rewrite emit_r_preserves_eval.
    rewrite emit_r_preserves_eval.
    rewrite emit_r_preserves_eval.
    reflexivity.
Qed.

Lemma emit_r_stmts_preserves_exec :
  forall rho stmts,
    exec_r_stmts rho (emit_r_stmts stmts) = exec_mir_stmts_codegen rho stmts.
Proof.
  intros rho stmts.
  revert rho.
  induction stmts as [|stmt rest IH]; intros rho; simpl.
  - reflexivity.
  - rewrite emit_r_stmt_preserves_exec.
    destruct (exec_mir_stmt_codegen rho stmt); [|reflexivity].
    apply IH.
Qed.

Definition eval_mir_program_codegen (p : mir_program) : option rvalue :=
  match exec_mir_stmts_codegen [] (mir_stmts_p p) with
  | Some rho => eval_mir_local_codegen rho (mir_ret_p p)
  | None => None
  end.

Lemma emit_r_program_preserves_eval_codegen :
  forall p,
    eval_r_program (emit_r_program p) = eval_mir_program_codegen p.
Proof.
  intros [stmts ret].
  unfold eval_r_program, emit_r_program, eval_mir_program_codegen.
  simpl.
  rewrite emit_r_stmts_preserves_exec.
  destruct (exec_mir_stmts_codegen [] stmts); [|reflexivity].
  apply emit_r_local_preserves_eval.
Qed.

End RRPipelineStmtGenericSubset.
