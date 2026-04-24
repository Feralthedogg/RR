Require Import LoweringSubset.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Lia.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.
Import RRLoweringSubset.

Module RRCodegenSubset.

Inductive r_expr : Type :=
| RConstInt : Z -> r_expr
| RConstBool : bool -> r_expr
| RUnaryNeg : r_expr -> r_expr
| RBinaryAdd : r_expr -> r_expr -> r_expr
| RListLit : list (string * r_expr) -> r_expr
| RFieldGet : r_expr -> string -> r_expr.

Fixpoint eval_r_expr (e : r_expr) : option rvalue :=
  match e with
  | RConstInt z => Some (RVInt z)
  | RConstBool b => Some (RVBool b)
  | RUnaryNeg e =>
      match eval_r_expr e with
      | Some (RVInt z) => Some (RVInt (- z))
      | _ => None
      end
  | RBinaryAdd e1 e2 =>
      match eval_r_expr e1, eval_r_expr e2 with
      | Some (RVInt z1), Some (RVInt z2) => Some (RVInt (z1 + z2))
      | _, _ => None
      end
  | RListLit fields =>
      let fix eval_r_fields (fields : list (string * r_expr))
          : option (list (string * rvalue)) :=
          match fields with
          | [] => Some []
          | (name, expr) :: rest =>
              match eval_r_expr expr, eval_r_fields rest with
              | Some v, Some tail => Some ((name, v) :: tail)
              | _, _ => None
              end
          end
      in
      match eval_r_fields fields with
      | Some vals => Some (RVRecord vals)
      | None => None
      end
  | RFieldGet e s =>
      match eval_r_expr e with
      | Some (RVRecord fields) => lookup_field fields s
      | _ => None
      end
  end.

Fixpoint emit_r (e : mir_expr) : r_expr :=
  match e with
  | MConstInt z => RConstInt z
  | MConstBool b => RConstBool b
  | MUnaryNeg e => RUnaryNeg (emit_r e)
  | MBinaryAdd e1 e2 => RBinaryAdd (emit_r e1) (emit_r e2)
  | MRecordLit fields =>
      let fix emit_r_fields (fields : list (string * mir_expr)) : list (string * r_expr) :=
          match fields with
          | [] => []
          | (name, expr) :: rest => (name, emit_r expr) :: emit_r_fields rest
          end
      in
      RListLit (emit_r_fields fields)
  | MFieldGet e s => RFieldGet (emit_r e) s
  end.

Fixpoint mir_expr_depth (e : mir_expr) : nat :=
  match e with
  | MConstInt _ => 0%nat
  | MConstBool _ => 0%nat
  | MUnaryNeg e => S (mir_expr_depth e)
  | MBinaryAdd e1 e2 => S (Nat.max (mir_expr_depth e1) (mir_expr_depth e2))
  | MRecordLit fields =>
      let fix mir_fields_depth_local (fields : list (string * mir_expr)) : nat :=
          match fields with
          | [] => 0%nat
          | (_, expr) :: rest => Nat.max (mir_expr_depth expr) (mir_fields_depth_local rest)
          end
      in
      S (mir_fields_depth_local fields)
  | MFieldGet e _ => S (mir_expr_depth e)
  end.

Fixpoint mir_fields_depth (fields : list (string * mir_expr)) : nat :=
  match fields with
  | [] => 0%nat
  | (_, expr) :: rest => Nat.max (mir_expr_depth expr) (mir_fields_depth rest)
  end.

Lemma emit_r_preserves_eval_fuel :
  forall fuel expr,
    (mir_expr_depth expr < fuel)%nat ->
    eval_r_expr (emit_r expr) = eval_mir_fuel fuel expr.
Proof.
  induction fuel as [|fuel IH]; intros expr Hfuel; [lia|].
  destruct expr as [z|b|e|e1 e2|fields|e s]; simpl in *.
  - reflexivity.
  - reflexivity.
  - rewrite IH by lia.
    destruct (eval_mir_fuel fuel e); [destruct r|]; reflexivity.
  - rewrite IH by lia.
    rewrite IH by lia.
    destruct (eval_mir_fuel fuel e1); [|reflexivity].
    destruct (eval_mir_fuel fuel e2); [|reflexivity].
    destruct r, r0; reflexivity.
  - remember (
      fix emit_r_fields_local (fields : list (string * mir_expr)) : list (string * r_expr) :=
          match fields with
          | [] => []
          | (name, expr) :: rest => (name, emit_r expr) :: emit_r_fields_local rest
          end) as emit_r_fields_local.
    remember (
      fix eval_r_fields_local (fields : list (string * r_expr))
          : option (list (string * rvalue)) :=
          match fields with
          | [] => Some []
          | (name, expr) :: rest =>
              match eval_r_expr expr, eval_r_fields_local rest with
              | Some v, Some tail => Some ((name, v) :: tail)
              | _, _ => None
              end
          end) as eval_r_fields_local.
    remember (
      fix eval_mir_fields_fuel (fields : list (string * mir_expr))
          : option (list (string * rvalue)) :=
          match fields with
          | [] => Some []
          | (name, expr) :: rest =>
              match eval_mir_fuel fuel expr, eval_mir_fields_fuel rest with
              | Some v, Some tail => Some ((name, v) :: tail)
              | _, _ => None
              end
          end) as eval_mir_fields_fuel.
    assert (Hfields :
      forall fields0,
        (mir_fields_depth fields0 < fuel)%nat ->
        eval_r_fields_local (emit_r_fields_local fields0) = eval_mir_fields_fuel fields0).
    {
      subst emit_r_fields_local eval_r_fields_local eval_mir_fields_fuel.
      intros fields0 Hdepth.
      induction fields0 as [|[name expr] rest IHfields]; simpl in *.
      - reflexivity.
      - rewrite (IH expr) by lia.
        rewrite IHfields by lia.
        reflexivity.
    }
    change (S (mir_fields_depth fields) < S fuel)%nat in Hfuel.
    assert (HfieldsBound : (mir_fields_depth fields < fuel)%nat) by lia.
    rewrite Hfields by exact HfieldsBound.
    reflexivity.
  - rewrite IH by lia.
    destruct (eval_mir_fuel fuel e); [destruct r|]; reflexivity.
Qed.

Lemma emit_r_preserves_eval :
  forall expr,
    eval_r_expr (emit_r expr) = eval_mir_fuel (S (mir_expr_depth expr)) expr.
Proof.
  intro expr.
  apply emit_r_preserves_eval_fuel.
  lia.
Qed.

Lemma emit_r_const_preserved :
  forall z,
    eval_r_expr (emit_r (MConstInt z)) = eval_mir_fuel 32 (MConstInt z).
Proof.
  reflexivity.
Qed.

Lemma emit_r_add_preserved :
  eval_r_expr (emit_r (MBinaryAdd (MConstInt 2) (MConstInt 5))) =
    eval_mir_fuel 32 (MBinaryAdd (MConstInt 2) (MConstInt 5)).
Proof.
  reflexivity.
Qed.

Lemma emit_r_field_preserved :
  eval_r_expr (emit_r (MFieldGet (MRecordLit [("x", MConstInt 9)]) "x")) =
    eval_mir_fuel 32 (MFieldGet (MRecordLit [("x", MConstInt 9)]) "x").
Proof.
  reflexivity.
Qed.

Definition nested_field_mir_expr : mir_expr :=
  MFieldGet (MFieldGet (MRecordLit [("inner", MRecordLit [("x", MConstInt 7)])]) "inner") "x".

Lemma nested_field_mir_expr_codegen_preserved :
  eval_r_expr (emit_r nested_field_mir_expr) = Some (RVInt 7).
Proof.
  reflexivity.
Qed.

End RRCodegenSubset.
