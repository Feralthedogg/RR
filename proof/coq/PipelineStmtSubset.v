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

Module RRPipelineStmtSubset.

Definition env := list (string * rvalue).

Inductive src_local_expr : Type :=
| SLPure : src_expr -> src_local_expr
| SLVar : string -> src_local_expr
| SLAdd : src_local_expr -> src_local_expr -> src_local_expr
| SLField : src_local_expr -> string -> src_local_expr.

Inductive mir_local_expr : Type :=
| MLPure : mir_expr -> mir_local_expr
| MLVar : string -> mir_local_expr
| MLAdd : mir_local_expr -> mir_local_expr -> mir_local_expr
| MLField : mir_local_expr -> string -> mir_local_expr.

Inductive r_local_expr : Type :=
| RLPure : r_expr -> r_local_expr
| RLVar : string -> r_local_expr
| RLAdd : r_local_expr -> r_local_expr -> r_local_expr
| RLField : r_local_expr -> string -> r_local_expr.

Fixpoint eval_src_local (rho : env) (e : src_local_expr) : option rvalue :=
  match e with
  | SLPure expr => eval_src_fuel 32 expr
  | SLVar name => lookup_field rho name
  | SLAdd lhs rhs =>
      match eval_src_local rho lhs, eval_src_local rho rhs with
      | Some (RVInt z1), Some (RVInt z2) => Some (RVInt (z1 + z2))
      | _, _ => None
      end
  | SLField base name =>
      match eval_src_local rho base with
      | Some (RVRecord fields) => lookup_field fields name
      | _ => None
      end
  end.

Fixpoint eval_mir_local (rho : env) (e : mir_local_expr) : option rvalue :=
  match e with
  | MLPure expr => eval_mir_fuel 32 expr
  | MLVar name => lookup_field rho name
  | MLAdd lhs rhs =>
      match eval_mir_local rho lhs, eval_mir_local rho rhs with
      | Some (RVInt z1), Some (RVInt z2) => Some (RVInt (z1 + z2))
      | _, _ => None
      end
  | MLField base name =>
      match eval_mir_local rho base with
      | Some (RVRecord fields) => lookup_field fields name
      | _ => None
      end
  end.

Fixpoint eval_r_local (rho : env) (e : r_local_expr) : option rvalue :=
  match e with
  | RLPure expr => eval_r_expr expr
  | RLVar name => lookup_field rho name
  | RLAdd lhs rhs =>
      match eval_r_local rho lhs, eval_r_local rho rhs with
      | Some (RVInt z1), Some (RVInt z2) => Some (RVInt (z1 + z2))
      | _, _ => None
      end
  | RLField base name =>
      match eval_r_local rho base with
      | Some (RVRecord fields) => lookup_field fields name
      | _ => None
      end
  end.

Inductive src_stmt : Type :=
| SAssign : string -> src_expr -> src_stmt
| SIfAssign : string -> src_expr -> src_expr -> src_expr -> src_stmt.

Inductive mir_stmt : Type :=
| MAssign : string -> mir_expr -> mir_stmt
| MIfAssign : string -> mir_expr -> mir_expr -> mir_expr -> mir_stmt.

Inductive r_stmt : Type :=
| RAssign : string -> r_expr -> r_stmt
| RIfAssign : string -> r_expr -> r_expr -> r_expr -> r_stmt.

Definition exec_src_stmt (rho : env) (s : src_stmt) : option env :=
  match s with
  | SAssign name rhs =>
      match eval_src_fuel 32 rhs with
      | Some v => Some ((name, v) :: rho)
      | None => None
      end
  | SIfAssign name cond then_rhs else_rhs =>
      match eval_src_fuel 32 cond with
      | Some (RVBool true) =>
          match eval_src_fuel 32 then_rhs with
          | Some v => Some ((name, v) :: rho)
          | None => None
          end
      | Some (RVBool false) =>
          match eval_src_fuel 32 else_rhs with
          | Some v => Some ((name, v) :: rho)
          | None => None
          end
      | _ => None
      end
  end.

Definition exec_mir_stmt (rho : env) (s : mir_stmt) : option env :=
  match s with
  | MAssign name rhs =>
      match eval_mir_fuel 32 rhs with
      | Some v => Some ((name, v) :: rho)
      | None => None
      end
  | MIfAssign name cond then_rhs else_rhs =>
      match eval_mir_fuel 32 cond with
      | Some (RVBool true) =>
          match eval_mir_fuel 32 then_rhs with
          | Some v => Some ((name, v) :: rho)
          | None => None
          end
      | Some (RVBool false) =>
          match eval_mir_fuel 32 else_rhs with
          | Some v => Some ((name, v) :: rho)
          | None => None
          end
      | _ => None
      end
  end.

Definition exec_r_stmt (rho : env) (s : r_stmt) : option env :=
  match s with
  | RAssign name rhs =>
      match eval_r_expr rhs with
      | Some v => Some ((name, v) :: rho)
      | None => None
      end
  | RIfAssign name cond then_rhs else_rhs =>
      match eval_r_expr cond with
      | Some (RVBool true) =>
          match eval_r_expr then_rhs with
          | Some v => Some ((name, v) :: rho)
          | None => None
          end
      | Some (RVBool false) =>
          match eval_r_expr else_rhs with
          | Some v => Some ((name, v) :: rho)
          | None => None
          end
      | _ => None
      end
  end.

Fixpoint exec_src_stmts (rho : env) (stmts : list src_stmt) : option env :=
  match stmts with
  | [] => Some rho
  | stmt :: rest =>
      match exec_src_stmt rho stmt with
      | Some rho' => exec_src_stmts rho' rest
      | None => None
      end
  end.

Fixpoint exec_mir_stmts (rho : env) (stmts : list mir_stmt) : option env :=
  match stmts with
  | [] => Some rho
  | stmt :: rest =>
      match exec_mir_stmt rho stmt with
      | Some rho' => exec_mir_stmts rho' rest
      | None => None
      end
  end.

Fixpoint exec_r_stmts (rho : env) (stmts : list r_stmt) : option env :=
  match stmts with
  | [] => Some rho
  | stmt :: rest =>
      match exec_r_stmt rho stmt with
      | Some rho' => exec_r_stmts rho' rest
      | None => None
      end
  end.

Fixpoint lower_local_expr (e : src_local_expr) : mir_local_expr :=
  match e with
  | SLPure expr => MLPure (lower expr)
  | SLVar name => MLVar name
  | SLAdd lhs rhs => MLAdd (lower_local_expr lhs) (lower_local_expr rhs)
  | SLField base name => MLField (lower_local_expr base) name
  end.

Fixpoint emit_r_local (e : mir_local_expr) : r_local_expr :=
  match e with
  | MLPure expr => RLPure (emit_r expr)
  | MLVar name => RLVar name
  | MLAdd lhs rhs => RLAdd (emit_r_local lhs) (emit_r_local rhs)
  | MLField base name => RLField (emit_r_local base) name
  end.

Definition lower_stmt (s : src_stmt) : mir_stmt :=
  match s with
  | SAssign name rhs => MAssign name (lower rhs)
  | SIfAssign name cond then_rhs else_rhs =>
      MIfAssign name (lower cond) (lower then_rhs) (lower else_rhs)
  end.

Definition emit_r_stmt_from_mir (s : mir_stmt) : r_stmt :=
  match s with
  | MAssign name rhs => RAssign name (emit_r rhs)
  | MIfAssign name cond then_rhs else_rhs =>
      RIfAssign name (emit_r cond) (emit_r then_rhs) (emit_r else_rhs)
  end.

Fixpoint lower_stmts (stmts : list src_stmt) : list mir_stmt :=
  match stmts with
  | [] => []
  | stmt :: rest => lower_stmt stmt :: lower_stmts rest
  end.

Fixpoint emit_r_stmts (stmts : list mir_stmt) : list r_stmt :=
  match stmts with
  | [] => []
  | stmt :: rest => emit_r_stmt_from_mir stmt :: emit_r_stmts rest
  end.

Record src_program : Type := {
  src_stmts : list src_stmt;
  src_ret : src_local_expr;
}.

Record mir_program : Type := {
  mir_stmts_p : list mir_stmt;
  mir_ret_p : mir_local_expr;
}.

Record r_program : Type := {
  r_stmts_p : list r_stmt;
  r_ret_p : r_local_expr;
}.

Definition lower_program (p : src_program) : mir_program :=
  {| mir_stmts_p := lower_stmts (src_stmts p);
     mir_ret_p := lower_local_expr (src_ret p) |}.

Definition emit_r_program (p : mir_program) : r_program :=
  {| r_stmts_p := emit_r_stmts (mir_stmts_p p);
     r_ret_p := emit_r_local (mir_ret_p p) |}.

Definition eval_src_program (p : src_program) : option rvalue :=
  match exec_src_stmts [] (src_stmts p) with
  | Some rho => eval_src_local rho (src_ret p)
  | None => None
  end.

Definition eval_mir_program (p : mir_program) : option rvalue :=
  match exec_mir_stmts [] (mir_stmts_p p) with
  | Some rho => eval_mir_local rho (mir_ret_p p)
  | None => None
  end.

Definition eval_r_program (p : r_program) : option rvalue :=
  match exec_r_stmts [] (r_stmts_p p) with
  | Some rho => eval_r_local rho (r_ret_p p)
  | None => None
  end.

Definition straight_line_program : src_program :=
  {| src_stmts := [SAssign "x" (SConstInt 4)];
     src_ret := SLAdd (SLVar "x") (SLPure (SConstInt 3)) |}.

Lemma straight_line_program_preserved :
  eval_r_program (emit_r_program (lower_program straight_line_program)) = Some (RVInt 7).
Proof.
  reflexivity.
Qed.

Definition branch_nested_record_program : src_program :=
  {| src_stmts :=
       [SIfAssign "rec"
          (SConstBool true)
          (SRecord [("inner", SRecord [("x", SConstInt 1)])])
          (SRecord [("inner", SRecord [("x", SConstInt 2)])])];
     src_ret := SLAdd (SLField (SLField (SLVar "rec") "inner") "x") (SLPure (SConstInt 3)) |}.

Lemma branch_nested_record_program_preserved :
  eval_r_program (emit_r_program (lower_program branch_nested_record_program)) = Some (RVInt 4).
Proof.
  reflexivity.
Qed.

End RRPipelineStmtSubset.
