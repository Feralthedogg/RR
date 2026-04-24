From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRInlineSubset.

Inductive inline_value : Type :=
| IVInt : Z -> inline_value
| IVRecord : list (string * inline_value) -> inline_value.

Inductive inline_expr : Type :=
| EConstInt : Z -> inline_expr
| EAdd : inline_expr -> inline_expr -> inline_expr
| ERecord : list (string * inline_expr) -> inline_expr
| EField : inline_expr -> string -> inline_expr.

Inductive helper_shape : Type :=
| HArg
| HAddConst : Z -> helper_shape
| HField : string -> helper_shape
| HFieldAddConst : string -> Z -> helper_shape.

Fixpoint lookup_inline_field (fields : list (string * inline_value)) (name : string)
    : option inline_value :=
  match fields with
  | [] => None
  | (field, value) :: rest =>
      if String.eqb field name then Some value else lookup_inline_field rest name
  end.

Fixpoint eval_inline_expr (e : inline_expr) : option inline_value :=
  match e with
  | EConstInt z => Some (IVInt z)
  | EAdd e1 e2 =>
      match eval_inline_expr e1, eval_inline_expr e2 with
      | Some (IVInt z1), Some (IVInt z2) => Some (IVInt (z1 + z2))
      | _, _ => None
      end
  | ERecord fields =>
      let fix eval_inline_fields (fields : list (string * inline_expr))
          : option (list (string * inline_value)) :=
          match fields with
          | [] => Some []
          | (name, expr) :: rest =>
              match eval_inline_expr expr, eval_inline_fields rest with
              | Some v, Some tail => Some ((name, v) :: tail)
              | _, _ => None
              end
          end
      in
      match eval_inline_fields fields with
      | Some vals => Some (IVRecord vals)
      | None => None
      end
  | EField e s =>
      match eval_inline_expr e with
      | Some (IVRecord fields) => lookup_inline_field fields s
      | _ => None
      end
  end.

Definition eval_helper_shape (helper : helper_shape) (arg : inline_value)
    : option inline_value :=
  match helper with
  | HArg => Some arg
  | HAddConst k =>
      match arg with
      | IVInt z => Some (IVInt (z + k))
      | _ => None
      end
  | HField name =>
      match arg with
      | IVRecord fields => lookup_inline_field fields name
      | _ => None
      end
  | HFieldAddConst name k =>
      match arg with
      | IVRecord fields =>
          match lookup_inline_field fields name with
          | Some (IVInt z) => Some (IVInt (z + k))
          | _ => None
          end
      | _ => None
      end
  end.

Definition inline_call (helper : helper_shape) (arg : inline_expr) : inline_expr :=
  match helper with
  | HArg => arg
  | HAddConst k => EAdd arg (EConstInt k)
  | HField name => EField arg name
  | HFieldAddConst name k => EAdd (EField arg name) (EConstInt k)
  end.

Definition eval_inline_call (helper : helper_shape) (arg : inline_expr)
    : option inline_value :=
  match eval_inline_expr arg with
  | Some v => eval_helper_shape helper v
  | None => None
  end.

Lemma inline_call_preserves_eval :
  forall helper arg,
    eval_inline_expr (inline_call helper arg) = eval_inline_call helper arg.
Proof.
  intros helper arg.
  destruct helper; simpl.
  - unfold eval_inline_call, eval_helper_shape.
    destruct (eval_inline_expr arg) as [v|] eqn:Harg; simpl; reflexivity.
  - unfold eval_inline_call, eval_helper_shape.
    destruct (eval_inline_expr arg) as [[n|fields]|] eqn:Harg; simpl; reflexivity.
  - unfold eval_inline_call, eval_helper_shape.
    destruct (eval_inline_expr arg) as [[n|fields]|] eqn:Harg; simpl; reflexivity.
  - unfold eval_inline_call, eval_helper_shape.
    destruct (eval_inline_expr arg) as [[n|fields]|] eqn:Harg; simpl.
    + reflexivity.
    + destruct (lookup_inline_field fields s) as [[m|fields']|] eqn:Hfield; simpl; reflexivity.
    + reflexivity.
Qed.

Definition inline_add_arg : inline_expr := EConstInt 6.
Definition inline_field_arg : inline_expr := ERecord [("x", EConstInt 9)].
Definition inline_field_add_arg : inline_expr := ERecord [("x", EConstInt 9)].

Lemma add_const_helper_preserved :
  eval_inline_expr (inline_call (HAddConst 3) inline_add_arg) = Some (IVInt 9).
Proof.
  reflexivity.
Qed.

Lemma field_helper_preserved :
  eval_inline_expr (inline_call (HField "x") inline_field_arg) = Some (IVInt 9).
Proof.
  reflexivity.
Qed.

Lemma field_add_helper_preserved :
  eval_inline_expr (inline_call (HFieldAddConst "x" 3) inline_field_add_arg) = Some (IVInt 12).
Proof.
  reflexivity.
Qed.

End RRInlineSubset.
