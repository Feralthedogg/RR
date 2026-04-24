From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRLoweringSubset.

Inductive rvalue : Type :=
| RVInt : Z -> rvalue
| RVBool : bool -> rvalue
| RVRecord : list (string * rvalue) -> rvalue.

Inductive src_expr : Type :=
| SConstInt : Z -> src_expr
| SConstBool : bool -> src_expr
| SNeg : src_expr -> src_expr
| SAdd : src_expr -> src_expr -> src_expr
| SRecord : list (string * src_expr) -> src_expr
| SField : src_expr -> string -> src_expr.

Inductive mir_expr : Type :=
| MConstInt : Z -> mir_expr
| MConstBool : bool -> mir_expr
| MUnaryNeg : mir_expr -> mir_expr
| MBinaryAdd : mir_expr -> mir_expr -> mir_expr
| MRecordLit : list (string * mir_expr) -> mir_expr
| MFieldGet : mir_expr -> string -> mir_expr.

Fixpoint lookup_field (fields : list (string * rvalue)) (name : string)
    : option rvalue :=
  match fields with
  | [] => None
  | (field, value) :: rest =>
      if String.eqb field name then Some value else lookup_field rest name
  end.

Fixpoint eval_src_fuel (fuel : nat) (e : src_expr) : option rvalue :=
  match fuel with
  | O => None
  | S fuel' =>
      match e with
      | SConstInt z => Some (RVInt z)
      | SConstBool b => Some (RVBool b)
      | SNeg e =>
          match eval_src_fuel fuel' e with
          | Some (RVInt z) => Some (RVInt (- z))
          | _ => None
          end
      | SAdd e1 e2 =>
          match eval_src_fuel fuel' e1, eval_src_fuel fuel' e2 with
          | Some (RVInt z1), Some (RVInt z2) => Some (RVInt (z1 + z2))
          | _, _ => None
          end
      | SRecord fields =>
          let fix eval_src_fields_fuel (fields : list (string * src_expr))
              : option (list (string * rvalue)) :=
              match fields with
              | [] => Some []
              | (name, expr) :: rest =>
                  match eval_src_fuel fuel' expr, eval_src_fields_fuel rest with
                  | Some v, Some tail => Some ((name, v) :: tail)
                  | _, _ => None
                  end
              end
          in
          match eval_src_fields_fuel fields with
          | Some vals => Some (RVRecord vals)
          | None => None
          end
      | SField e s =>
          match eval_src_fuel fuel' e with
          | Some (RVRecord fields) => lookup_field fields s
          | _ => None
          end
      end
  end.

Fixpoint eval_mir_fuel (fuel : nat) (e : mir_expr) : option rvalue :=
  match fuel with
  | O => None
  | S fuel' =>
      match e with
      | MConstInt z => Some (RVInt z)
      | MConstBool b => Some (RVBool b)
      | MUnaryNeg e =>
          match eval_mir_fuel fuel' e with
          | Some (RVInt z) => Some (RVInt (- z))
          | _ => None
          end
      | MBinaryAdd e1 e2 =>
          match eval_mir_fuel fuel' e1, eval_mir_fuel fuel' e2 with
          | Some (RVInt z1), Some (RVInt z2) => Some (RVInt (z1 + z2))
          | _, _ => None
          end
      | MRecordLit fields =>
          let fix eval_mir_fields_fuel (fields : list (string * mir_expr))
              : option (list (string * rvalue)) :=
              match fields with
              | [] => Some []
              | (name, expr) :: rest =>
                  match eval_mir_fuel fuel' expr, eval_mir_fields_fuel rest with
                  | Some v, Some tail => Some ((name, v) :: tail)
                  | _, _ => None
                  end
              end
          in
          match eval_mir_fields_fuel fields with
          | Some vals => Some (RVRecord vals)
          | None => None
          end
      | MFieldGet e s =>
          match eval_mir_fuel fuel' e with
          | Some (RVRecord fields) => lookup_field fields s
          | _ => None
          end
      end
  end.

Fixpoint lower (e : src_expr) : mir_expr :=
  match e with
  | SConstInt z => MConstInt z
  | SConstBool b => MConstBool b
  | SNeg e => MUnaryNeg (lower e)
  | SAdd e1 e2 => MBinaryAdd (lower e1) (lower e2)
  | SRecord fields =>
      let fix lower_fields (fields : list (string * src_expr)) : list (string * mir_expr) :=
          match fields with
          | [] => []
          | (name, expr) :: rest => (name, lower expr) :: lower_fields rest
          end
      in
      MRecordLit (lower_fields fields)
  | SField e s => MFieldGet (lower e) s
  end.

Lemma lower_preserves_eval_fuel :
  forall fuel expr,
    eval_mir_fuel fuel (lower expr) = eval_src_fuel fuel expr.
Proof.
  induction fuel as [|fuel IH]; intros expr; [reflexivity|].
  destruct expr; simpl.
  - reflexivity.
  - reflexivity.
  - rewrite IH. destruct (eval_src_fuel fuel expr); [destruct r|]; reflexivity.
  - rewrite IH, IH.
    destruct (eval_src_fuel fuel expr1); [|reflexivity].
    destruct (eval_src_fuel fuel expr2); [|reflexivity].
    destruct r, r0; reflexivity.
  - remember (
        fix lower_fields (fields : list (string * src_expr)) : list (string * mir_expr) :=
          match fields with
          | [] => []
          | (name, expr) :: rest => (name, lower expr) :: lower_fields rest
          end) as lower_fields.
    remember (
        fix eval_src_fields_fuel (fields : list (string * src_expr))
            : option (list (string * rvalue)) :=
          match fields with
          | [] => Some []
          | (name, expr) :: rest =>
              match eval_src_fuel fuel expr, eval_src_fields_fuel rest with
              | Some v, Some tail => Some ((name, v) :: tail)
              | _, _ => None
              end
          end) as eval_src_fields_fuel.
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
      forall fields,
        eval_mir_fields_fuel (lower_fields fields) =
        eval_src_fields_fuel fields).
    {
      subst lower_fields eval_src_fields_fuel eval_mir_fields_fuel.
      intros fields. induction fields as [|[name expr] rest IHfields]; simpl.
      - reflexivity.
      - rewrite IH, IHfields. reflexivity.
    }
    rewrite Hfields. reflexivity.
  - rewrite IH. destruct (eval_src_fuel fuel expr); [destruct r|]; reflexivity.
Qed.

Definition nested_field_src : src_expr :=
  SField (SField (SRecord [("inner", SRecord [("x", SConstInt 7)])]) "inner") "x".

Lemma nested_field_src_preserved :
  eval_mir_fuel 6 (lower nested_field_src) = Some (RVInt 7).
Proof.
  reflexivity.
Qed.

End RRLoweringSubset.
