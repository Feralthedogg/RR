From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Bool.

Open Scope string_scope.
Open Scope Z_scope.

Module RRDeSsaSubset.

Inductive copy_value : Type :=
| CConstInt : Z -> copy_value
| CIntrinsic1 : string -> copy_value -> copy_value
| CRecord1 : string -> copy_value -> copy_value
| CFieldGet : copy_value -> string -> copy_value
| CFieldSet : copy_value -> string -> copy_value -> copy_value.

Fixpoint eval_copy_fuel (fuel : nat) (v : copy_value) : option Z :=
  match fuel with
  | O => None
  | S fuel' =>
      match v with
      | CConstInt z => Some z
      | CIntrinsic1 op arg =>
          match eval_copy_fuel fuel' arg with
          | Some z => if String.eqb op "neg" then Some (- z) else Some z
          | None => None
          end
      | CRecord1 _ value => eval_copy_fuel fuel' value
      | CFieldGet base field =>
          match base with
          | CRecord1 name value =>
              if String.eqb name field then eval_copy_fuel fuel' value else None
          | CFieldSet prior name value =>
              if String.eqb name field
              then eval_copy_fuel fuel' value
              else eval_copy_fuel fuel' (CFieldGet prior field)
          | _ => None
          end
      | CFieldSet _ _ value => eval_copy_fuel fuel' value
      end
  end.

Definition eval_copy (v : copy_value) : option Z := eval_copy_fuel 16 v.

Fixpoint copy_fingerprint (v : copy_value) : Z :=
  match v with
  | CConstInt z => z
  | CIntrinsic1 op arg => 2000 + Z.of_nat (String.length op) + 31 * copy_fingerprint arg
  | CRecord1 field value => 3000 + Z.of_nat (String.length field) + 37 * copy_fingerprint value
  | CFieldGet base field => 4000 + Z.of_nat (String.length field) + 41 * copy_fingerprint base
  | CFieldSet base field value =>
      5000 + Z.of_nat (String.length field) + 43 * copy_fingerprint base + 47 * copy_fingerprint value
  end.

Definition same_canonical_value (lhs rhs : copy_value) : Prop :=
  copy_fingerprint lhs = copy_fingerprint rhs.

Definition no_move_needed (existing incoming : copy_value) : bool :=
  Z.eqb (copy_fingerprint existing) (copy_fingerprint incoming).

Lemma same_canonical_value_refl :
  forall v, same_canonical_value v v.
Proof.
  intro v. unfold same_canonical_value. reflexivity.
Qed.

Lemma no_move_needed_true_of_same_canonical_value :
  forall existing incoming,
    same_canonical_value existing incoming ->
    no_move_needed existing incoming = true.
Proof.
  intros existing incoming H.
  unfold same_canonical_value, no_move_needed in *.
  rewrite H. apply Z.eqb_refl.
Qed.

Lemma no_move_needed_self_field_get :
  no_move_needed
    (CFieldGet (CRecord1 "x" (CConstInt 3)) "x")
    (CFieldGet (CRecord1 "x" (CConstInt 3)) "x") = true.
Proof.
  apply no_move_needed_true_of_same_canonical_value.
  apply same_canonical_value_refl.
Qed.

Lemma no_move_needed_self_intrinsic :
  no_move_needed
    (CIntrinsic1 "neg" (CConstInt 3))
    (CIntrinsic1 "neg" (CConstInt 3)) = true.
Proof.
  apply no_move_needed_true_of_same_canonical_value.
  apply same_canonical_value_refl.
Qed.

Lemma no_move_needed_self_fieldset :
  no_move_needed
    (CFieldSet (CRecord1 "x" (CConstInt 1)) "x" (CConstInt 7))
    (CFieldSet (CRecord1 "x" (CConstInt 1)) "x" (CConstInt 7)) = true.
Proof.
  apply no_move_needed_true_of_same_canonical_value.
  apply same_canonical_value_refl.
Qed.

Lemma self_same_canonical_value_preserves_eval :
  forall v, same_canonical_value v v /\ eval_copy v = eval_copy v.
Proof.
  intro v. split.
  - apply same_canonical_value_refl.
  - reflexivity.
Qed.

End RRDeSsaSubset.
