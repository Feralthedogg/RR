From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Lia.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRRuntimeSafetyFieldRangeSubset.

Inductive rs_value : Type :=
| RSInt : Z -> rs_value
| RSRecord : list (string * rs_value) -> rs_value.

Inductive rs_expr : Type :=
| RSConstInt : Z -> rs_expr
| RSRecordExpr : list (string * rs_expr) -> rs_expr
| RSField : rs_expr -> string -> rs_expr
| RSFieldSet : rs_expr -> string -> rs_expr -> rs_expr.

Fixpoint lookup_rs_field (fields : list (string * rs_value)) (name : string)
    : option rs_value :=
  match fields with
  | [] => None
  | (field, value) :: rest =>
      if String.eqb field name then Some value else lookup_rs_field rest name
  end.

Fixpoint set_rs_field (fields : list (string * rs_value)) (name : string) (value : rs_value)
    : list (string * rs_value) :=
  match fields with
  | [] => [(name, value)]
  | (field, current) :: rest =>
      if String.eqb field name
      then (field, value) :: rest
      else (field, current) :: set_rs_field rest name value
  end.

Fixpoint eval_rs (expr : rs_expr) : option rs_value :=
  match expr with
  | RSConstInt z => Some (RSInt z)
  | RSRecordExpr fields =>
      let fix eval_fields (fields : list (string * rs_expr))
          : option (list (string * rs_value)) :=
          match fields with
          | [] => Some []
          | (name, value) :: rest =>
              match eval_rs value, eval_fields rest with
              | Some v, Some tail => Some ((name, v) :: tail)
              | _, _ => None
              end
          end
      in
      match eval_fields fields with
      | Some vals => Some (RSRecord vals)
      | None => None
      end
  | RSField base name =>
      match eval_rs base with
      | Some (RSRecord fields) => lookup_rs_field fields name
      | _ => None
      end
  | RSFieldSet base name value =>
      match eval_rs base, eval_rs value with
      | Some (RSRecord fields), Some v => Some (RSRecord (set_rs_field fields name v))
      | _, _ => None
      end
  end.

Definition exact_interval_of (expr : rs_expr) : option (Z * Z) :=
  match eval_rs expr with
  | Some (RSInt z) => Some (z, z)
  | _ => None
  end.

Definition interval_below_one (bounds : Z * Z) : bool :=
  Z.ltb (snd bounds) 1.

Definition interval_negative (bounds : Z * Z) : bool :=
  Z.ltb (snd bounds) 0.

Lemma eval_rs_int_implies_exact_interval_of :
  forall expr z,
    eval_rs expr = Some (RSInt z) ->
    exact_interval_of expr = Some (z, z).
Proof.
  intros expr z H.
  unfold exact_interval_of. now rewrite H.
Qed.

Lemma exact_negative_interval_implies_below_one :
  forall z,
    z < 0 ->
    interval_below_one (z, z) = true.
Proof.
  intros z Hz.
  unfold interval_below_one.
  apply Z.ltb_lt.
  eapply Z.lt_trans.
  - exact Hz.
  - lia.
Qed.

Definition example_record_field_negative : rs_expr :=
  RSField (RSRecordExpr [("i", RSConstInt (-1)); ("j", RSConstInt 2)]) "i".

Lemma example_record_field_negative_interval :
  exact_interval_of example_record_field_negative = Some (-1, -1).
Proof.
  apply eval_rs_int_implies_exact_interval_of.
  reflexivity.
Qed.

Lemma example_record_field_negative_below_one :
  interval_below_one (-1, -1) = true.
Proof.
  apply exact_negative_interval_implies_below_one.
  lia.
Qed.

Lemma example_record_field_negative_is_negative :
  interval_negative (-1, -1) = true.
Proof.
  unfold interval_negative. reflexivity.
Qed.

Definition example_fieldset_negative : rs_expr :=
  RSField (RSFieldSet (RSRecordExpr [("i", RSConstInt 5)]) "i" (RSConstInt (-2))) "i".

Lemma example_fieldset_negative_interval :
  exact_interval_of example_fieldset_negative = Some (-2, -2).
Proof.
  apply eval_rs_int_implies_exact_interval_of.
  reflexivity.
Qed.

Lemma example_fieldset_negative_below_one :
  interval_below_one (-2, -2) = true.
Proof.
  apply exact_negative_interval_implies_below_one.
  lia.
Qed.

Definition example_fieldset_override_positive : rs_expr :=
  RSField (RSFieldSet (RSRecordExpr [("i", RSConstInt (-1))]) "i" (RSConstInt 5)) "i".

Lemma example_fieldset_override_positive_interval :
  exact_interval_of example_fieldset_override_positive = Some (5, 5).
Proof.
  apply eval_rs_int_implies_exact_interval_of.
  reflexivity.
Qed.

Lemma example_fieldset_override_positive_not_below_one :
  interval_below_one (5, 5) = false.
Proof.
  unfold interval_below_one. reflexivity.
Qed.

Definition example_nested_record_field_negative : rs_expr :=
  RSField (RSField (RSRecordExpr [("inner", RSRecordExpr [("i", RSConstInt (-1))])]) "inner") "i".

Lemma example_nested_record_field_negative_interval :
  exact_interval_of example_nested_record_field_negative = Some (-1, -1).
Proof.
  apply eval_rs_int_implies_exact_interval_of.
  reflexivity.
Qed.

Lemma example_nested_record_field_negative_below_one :
  interval_below_one (-1, -1) = true.
Proof.
  apply exact_negative_interval_implies_below_one.
  lia.
Qed.

End RRRuntimeSafetyFieldRangeSubset.
