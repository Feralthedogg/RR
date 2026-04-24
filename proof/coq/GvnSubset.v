From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Bool.
From Stdlib Require Import Arith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRGvnSubset.

Inductive gvn_value : Type :=
| GVInt : Z -> gvn_value
| GVRecord : list (string * gvn_value) -> gvn_value.

Inductive gvn_expr : Type :=
| GConstInt : Z -> gvn_expr
| GAdd : gvn_expr -> gvn_expr -> gvn_expr
| GIntrinsicAbs : gvn_expr -> gvn_expr
| GRecord : list (string * gvn_expr) -> gvn_expr
| GField : gvn_expr -> string -> gvn_expr
| GFieldSet : gvn_expr -> string -> gvn_expr -> gvn_expr.

Fixpoint lookup_field (fields : list (string * gvn_value)) (name : string)
    : option gvn_value :=
  match fields with
  | [] => None
  | (field, value) :: rest =>
      if String.eqb field name then Some value else lookup_field rest name
  end.

Fixpoint set_field (fields : list (string * gvn_value)) (name : string) (value : gvn_value)
    : list (string * gvn_value) :=
  match fields with
  | [] => [(name, value)]
  | (field, current) :: rest =>
      if String.eqb field name
      then (field, value) :: rest
      else (field, current) :: set_field rest name value
  end.

Fixpoint eval (e : gvn_expr) : option gvn_value :=
  match e with
  | GConstInt z => Some (GVInt z)
  | GAdd e1 e2 =>
      match eval e1, eval e2 with
      | Some (GVInt z1), Some (GVInt z2) => Some (GVInt (z1 + z2))
      | _, _ => None
      end
  | GIntrinsicAbs e1 =>
      match eval e1 with
      | Some (GVInt z) => Some (GVInt (Z.abs z))
      | _ => None
      end
  | GRecord fields =>
      let fix eval_fields (fields : list (string * gvn_expr))
          : option (list (string * gvn_value)) :=
          match fields with
          | [] => Some []
          | (name, expr) :: rest =>
              match eval expr, eval_fields rest with
              | Some v, Some tail => Some ((name, v) :: tail)
              | _, _ => None
              end
          end
      in
      match eval_fields fields with
      | Some vals => Some (GVRecord vals)
      | None => None
      end
  | GField e s =>
      match eval e with
      | Some (GVRecord fields) => lookup_field fields s
      | _ => None
      end
  | GFieldSet base name value =>
      match eval base, eval value with
      | Some (GVRecord fields), Some v => Some (GVRecord (set_field fields name v))
      | _, _ => None
      end
  end.

Fixpoint expr_size (e : gvn_expr) : nat :=
  match e with
  | GConstInt _ => 1
  | GAdd e1 e2 => S (expr_size e1 + expr_size e2)
  | GIntrinsicAbs e1 => S (expr_size e1)
  | GRecord fields =>
      let fix fields_size (fields : list (string * gvn_expr)) : nat :=
          match fields with
          | [] => 0%nat
          | (_, expr) :: rest => S (expr_size expr + fields_size rest)
          end
      in
      S (fields_size fields)
  | GField e _ => S (expr_size e)
  | GFieldSet base _ value => S (expr_size base + expr_size value)
  end.

Fixpoint expr_fingerprint (e : gvn_expr) : Z :=
  match e with
  | GConstInt z => z
  | GAdd e1 e2 => 3000 + 37 * expr_fingerprint e1 + 41 * expr_fingerprint e2
  | GIntrinsicAbs e1 => 3500 + 43 * expr_fingerprint e1
  | GRecord fields =>
      let fix fields_fingerprint (fields : list (string * gvn_expr)) : Z :=
          match fields with
          | [] => 0
          | (name, expr) :: rest =>
              61 * expr_fingerprint expr
                + Z.of_nat (String.length name)
                + 67 * fields_fingerprint rest
          end
      in
      4000 + fields_fingerprint fields
  | GField e name => 5000 + 47 * expr_fingerprint e + Z.of_nat (String.length name)
  | GFieldSet base name value =>
      6000
        + 53 * expr_fingerprint base
        + Z.of_nat (String.length name)
        + 59 * expr_fingerprint value
  end.

Definition expr_lt (lhs rhs : gvn_expr) : bool :=
  Z.ltb (expr_fingerprint lhs) (expr_fingerprint rhs)
    || (Z.eqb (expr_fingerprint lhs) (expr_fingerprint rhs)
          && Nat.ltb (expr_size lhs) (expr_size rhs)).

Fixpoint canonicalize (e : gvn_expr) : gvn_expr :=
  match e with
  | GConstInt z => GConstInt z
  | GAdd e1 e2 =>
      let e1' := canonicalize e1 in
      let e2' := canonicalize e2 in
      if expr_lt e2' e1' then GAdd e2' e1' else GAdd e1' e2'
  | GIntrinsicAbs e1 => GIntrinsicAbs (canonicalize e1)
  | GRecord fields =>
      let fix canonicalize_fields (fields : list (string * gvn_expr))
          : list (string * gvn_expr) :=
          match fields with
          | [] => []
          | (name, expr) :: rest =>
              (name, canonicalize expr) :: canonicalize_fields rest
          end
      in
      GRecord (canonicalize_fields fields)
  | GField e name => GField (canonicalize e) name
  | GFieldSet base name value =>
      GFieldSet (canonicalize base) name (canonicalize value)
  end.

Lemma eval_add_comm :
  forall e1 e2,
    eval (GAdd e1 e2) = eval (GAdd e2 e1).
Proof.
  intros e1 e2.
  simpl.
  destruct (eval e1) as [[z1|fields1]|] eqn:He1;
  destruct (eval e2) as [[z2|fields2]|] eqn:He2; simpl; try reflexivity.
  rewrite Z.add_comm. reflexivity.
Qed.

Fixpoint canonicalize_preserves_eval (expr : gvn_expr) : eval (canonicalize expr) = eval expr.
Proof.
  destruct expr as [z|e1 e2|e1|fields|e name|base name value]; simpl.
  - reflexivity.
  - remember (canonicalize e1) as e1'.
    remember (canonicalize e2) as e2'.
    destruct (expr_lt e2' e1') eqn:Hlt.
    + subst e1' e2'. rewrite eval_add_comm. simpl.
      now rewrite canonicalize_preserves_eval, canonicalize_preserves_eval.
    + subst e1' e2'. simpl.
      now rewrite canonicalize_preserves_eval, canonicalize_preserves_eval.
  - simpl. now rewrite canonicalize_preserves_eval.
  - remember (
      fix canonicalize_fields (fields : list (string * gvn_expr))
          : list (string * gvn_expr) :=
          match fields with
          | [] => []
          | (name, expr) :: rest =>
              (name, canonicalize expr) :: canonicalize_fields rest
          end) as canonicalize_fields.
    remember (
      fix eval_fields (fields : list (string * gvn_expr))
          : option (list (string * gvn_value)) :=
          match fields with
          | [] => Some []
          | (name, expr) :: rest =>
              match eval expr, eval_fields rest with
              | Some v, Some tail => Some ((name, v) :: tail)
              | _, _ => None
              end
          end) as eval_fields.
    assert (Hfields :
      forall fields0,
        eval_fields (canonicalize_fields fields0) = eval_fields fields0).
    {
      subst canonicalize_fields eval_fields.
      intro fields0.
      induction fields0 as [|[field_name field_expr] rest IHfields]; simpl.
      - reflexivity.
      - rewrite canonicalize_preserves_eval, IHfields. reflexivity.
    }
    rewrite Hfields. reflexivity.
  - now rewrite canonicalize_preserves_eval.
  - simpl. now rewrite canonicalize_preserves_eval, canonicalize_preserves_eval.
Defined.

Lemma canonical_forms_equal_same_eval :
  forall e1 e2,
    canonicalize e1 = canonicalize e2 ->
    eval e1 = eval e2.
Proof.
  intros e1 e2 Hcanon.
  rewrite <- (canonicalize_preserves_eval e1).
  rewrite Hcanon.
  apply canonicalize_preserves_eval.
Qed.

Definition swapped_add_a : gvn_expr := GAdd (GConstInt 2) (GConstInt 5).
Definition swapped_add_b : gvn_expr := GAdd (GConstInt 5) (GConstInt 2).

Lemma swapped_add_cse_preserved :
  eval swapped_add_a = eval swapped_add_b.
Proof.
  apply canonical_forms_equal_same_eval.
  reflexivity.
Qed.

Definition duplicate_intrinsic_a : gvn_expr :=
  GIntrinsicAbs (GAdd (GConstInt (-5)) (GConstInt 2)).

Definition duplicate_intrinsic_b : gvn_expr :=
  GIntrinsicAbs (GAdd (GConstInt 2) (GConstInt (-5))).

Lemma duplicate_intrinsic_cse_preserved :
  eval duplicate_intrinsic_a = eval duplicate_intrinsic_b.
Proof.
  apply canonical_forms_equal_same_eval.
  reflexivity.
Qed.

Definition duplicate_field_get_a : gvn_expr :=
  GField (GRecord [("x", GAdd (GConstInt 2) (GConstInt 5))]) "x".

Definition duplicate_field_get_b : gvn_expr :=
  GField (GRecord [("x", GAdd (GConstInt 5) (GConstInt 2))]) "x".

Lemma duplicate_field_get_cse_preserved :
  eval duplicate_field_get_a = eval duplicate_field_get_b.
Proof.
  apply canonical_forms_equal_same_eval.
  reflexivity.
Qed.

Definition duplicate_fieldset_get_a : gvn_expr :=
  GField (GFieldSet (GRecord [("x", GConstInt 1)]) "x" (GAdd (GConstInt 2) (GConstInt 5))) "x".

Definition duplicate_fieldset_get_b : gvn_expr :=
  GField (GFieldSet (GRecord [("x", GConstInt 1)]) "x" (GAdd (GConstInt 5) (GConstInt 2))) "x".

Lemma duplicate_fieldset_get_cse_preserved :
  eval duplicate_fieldset_get_a = eval duplicate_fieldset_get_b.
Proof.
  apply canonical_forms_equal_same_eval.
  reflexivity.
Qed.

End RRGvnSubset.
