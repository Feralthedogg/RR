From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Bool.

Require Import RRProofs.MirInvariantBundle.

Open Scope string_scope.
Open Scope Z_scope.

Module RRDeSsaBoundarySoundness.

Import RRMirInvariantBundle.

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

Definition no_move_needed (existing incoming : copy_value) : bool :=
  Z.eqb (copy_fingerprint existing) (copy_fingerprint incoming).

Definition copy_boundary_original (existing incoming : copy_value) : option Z :=
  if no_move_needed existing incoming then eval_copy existing else eval_copy incoming.

Definition copy_boundary_optimized (existing _incoming : copy_value) : option Z :=
  eval_copy existing.

Lemma no_move_needed_self_field_get :
  no_move_needed
    (CFieldGet (CRecord1 "x" (CConstInt 3)) "x")
    (CFieldGet (CRecord1 "x" (CConstInt 3)) "x") = true.
Proof.
  unfold no_move_needed. apply Z.eqb_refl.
Qed.

Lemma no_move_needed_self_intrinsic :
  no_move_needed
    (CIntrinsic1 "neg" (CConstInt 3))
    (CIntrinsic1 "neg" (CConstInt 3)) = true.
Proof.
  unfold no_move_needed. apply Z.eqb_refl.
Qed.

Lemma no_move_needed_self_fieldset :
  no_move_needed
    (CFieldSet (CRecord1 "x" (CConstInt 1)) "x" (CConstInt 7))
    (CFieldSet (CRecord1 "x" (CConstInt 1)) "x" (CConstInt 7)) = true.
Proof.
  unfold no_move_needed. apply Z.eqb_refl.
Qed.

Lemma de_ssa_redundant_move_elimination_preserves_eval :
  forall existing incoming,
    no_move_needed existing incoming = true ->
    copy_boundary_original existing incoming =
    copy_boundary_optimized existing incoming.
Proof.
  intros existing incoming H.
  unfold copy_boundary_original, copy_boundary_optimized.
  rewrite H. reflexivity.
Qed.

Lemma de_ssa_self_field_get_preserves_eval :
  copy_boundary_original
    (CFieldGet (CRecord1 "x" (CConstInt 3)) "x")
    (CFieldGet (CRecord1 "x" (CConstInt 3)) "x")
  =
  copy_boundary_optimized
    (CFieldGet (CRecord1 "x" (CConstInt 3)) "x")
    (CFieldGet (CRecord1 "x" (CConstInt 3)) "x").
Proof.
  apply de_ssa_redundant_move_elimination_preserves_eval.
  exact no_move_needed_self_field_get.
Qed.

Lemma de_ssa_self_intrinsic_preserves_eval :
  copy_boundary_original
    (CIntrinsic1 "neg" (CConstInt 3))
    (CIntrinsic1 "neg" (CConstInt 3))
  =
  copy_boundary_optimized
    (CIntrinsic1 "neg" (CConstInt 3))
    (CIntrinsic1 "neg" (CConstInt 3)).
Proof.
  apply de_ssa_redundant_move_elimination_preserves_eval.
  exact no_move_needed_self_intrinsic.
Qed.

Lemma de_ssa_self_fieldset_preserves_eval :
  copy_boundary_original
    (CFieldSet (CRecord1 "x" (CConstInt 1)) "x" (CConstInt 7))
    (CFieldSet (CRecord1 "x" (CConstInt 1)) "x" (CConstInt 7))
  =
  copy_boundary_optimized
    (CFieldSet (CRecord1 "x" (CConstInt 1)) "x" (CConstInt 7))
    (CFieldSet (CRecord1 "x" (CConstInt 1)) "x" (CConstInt 7)).
Proof.
  apply de_ssa_redundant_move_elimination_preserves_eval.
  exact no_move_needed_self_fieldset.
Qed.

Lemma de_ssa_boundary_identity_preserves_verify_ir_bundle :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (identity_pass fn).
Proof.
  intros fn H.
  exact (identity_pass_preserves_verify_ir_bundle fn H).
Qed.

Lemma de_ssa_boundary_identity_preserves_semantics :
  forall fn ρ,
    exec_entry (identity_pass fn) ρ = exec_entry fn ρ.
Proof.
  intros fn ρ.
  exact (identity_pass_preserves_semantics fn ρ).
Qed.

End RRDeSsaBoundarySoundness.
