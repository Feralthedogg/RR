From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Arith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRDceSubset.

Inductive dce_expr : Type :=
| DPureConst : Z -> dce_expr
| DImpureCall : string -> dce_expr
| DAdd : dce_expr -> dce_expr -> dce_expr
| DIntrinsic : string -> dce_expr -> dce_expr
| DRecord1 : string -> dce_expr -> dce_expr
| DFieldGet : dce_expr -> string -> dce_expr
| DFieldSet : dce_expr -> string -> dce_expr -> dce_expr
| DLen : dce_expr -> dce_expr
| DRange : dce_expr -> dce_expr -> dce_expr
| DIndices : dce_expr -> dce_expr
| DIndex1D : dce_expr -> dce_expr -> dce_expr
| DIndex2D : dce_expr -> dce_expr -> dce_expr -> dce_expr
| DIndex3D : dce_expr -> dce_expr -> dce_expr -> dce_expr -> dce_expr
| DPhi : dce_expr -> dce_expr -> dce_expr.

Fixpoint effect_count (e : dce_expr) : nat :=
  match e with
  | DPureConst _ => 0%nat
  | DImpureCall _ => 1%nat
  | DAdd lhs rhs => effect_count lhs + effect_count rhs
  | DIntrinsic _ arg => effect_count arg
  | DRecord1 _ value => effect_count value
  | DFieldGet base _ => effect_count base
  | DFieldSet base _ value => effect_count base + effect_count value
  | DLen base => effect_count base
  | DRange start stop => effect_count start + effect_count stop
  | DIndices base => effect_count base
  | DIndex1D base idx => effect_count base + effect_count idx
  | DIndex2D base r c => effect_count base + effect_count r + effect_count c
  | DIndex3D base i j k => effect_count base + effect_count i + effect_count j + effect_count k
  | DPhi lhs rhs => effect_count lhs + effect_count rhs
  end.

Inductive dce_instr : Type :=
| DEval : dce_expr -> dce_instr.

Definition effect_instr (i : dce_instr) : nat :=
  match i with
  | DEval e => effect_count e
  end.

Fixpoint effect_instrs (instrs : list dce_instr) : nat :=
  match instrs with
  | [] => 0%nat
  | instr :: rest => effect_instr instr + effect_instrs rest
  end.

Definition dce_dead_assign (expr : dce_expr) : list dce_instr :=
  if Nat.eqb (effect_count expr) 0%nat then [] else [DEval expr].

Lemma dce_dead_assign_preserves_effects :
  forall expr,
    effect_instrs (dce_dead_assign expr) = effect_count expr.
Proof.
  intro expr.
  unfold dce_dead_assign.
  destruct (Nat.eqb (effect_count expr) 0%nat) eqn:Heq.
  - apply Nat.eqb_eq in Heq. rewrite Heq. reflexivity.
  - simpl. rewrite Nat.add_0_r. reflexivity.
Qed.

Lemma dce_dead_assign_pure_erases :
  forall expr,
    effect_count expr = 0%nat ->
    dce_dead_assign expr = [].
Proof.
  intros expr H.
  unfold dce_dead_assign.
  rewrite H. reflexivity.
Qed.

Lemma dce_dead_assign_impure_demotes_to_eval :
  forall expr,
    effect_count expr <> 0%nat ->
    dce_dead_assign expr = [DEval expr].
Proof.
  intros expr H.
  unfold dce_dead_assign.
  destruct (Nat.eqb_spec (effect_count expr) 0%nat) as [Hz|Hz].
  - contradiction.
  - reflexivity.
Qed.

Definition nested_fieldset_expr : dce_expr :=
  DFieldSet (DRecord1 "x" (DPureConst 1)) "x" (DImpureCall "f").

Definition nested_index3d_expr : dce_expr :=
  DIndex3D (DPureConst 1) (DImpureCall "f") (DPureConst 1) (DPureConst 1).

Definition nested_phi_expr : dce_expr :=
  DPhi (DImpureCall "f") (DPureConst 1).

Definition nested_range_expr : dce_expr :=
  DRange (DImpureCall "f") (DPureConst 1).

Definition nested_indices_expr : dce_expr :=
  DIndices (DImpureCall "f").

Lemma nested_fieldset_preserved :
  effect_instrs (dce_dead_assign nested_fieldset_expr) = 1%nat.
Proof.
  reflexivity.
Qed.

Lemma nested_index3d_preserved :
  effect_instrs (dce_dead_assign nested_index3d_expr) = 1%nat.
Proof.
  reflexivity.
Qed.

Lemma nested_phi_preserved :
  effect_instrs (dce_dead_assign nested_phi_expr) = 1%nat.
Proof.
  reflexivity.
Qed.

Lemma nested_range_preserved :
  effect_instrs (dce_dead_assign nested_range_expr) = 1%nat.
Proof.
  reflexivity.
Qed.

Lemma nested_indices_preserved :
  effect_instrs (dce_dead_assign nested_indices_expr) = 1%nat.
Proof.
  reflexivity.
Qed.

End RRDceSubset.
