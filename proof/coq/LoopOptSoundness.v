From Stdlib Require Import Lia.
From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Require Import MirInvariantBundle.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRLoopOptSoundness.

Import RRMirInvariantBundle.

Definition loop_state : Type := string -> Z.

Inductive reduced_licm_case : Type :=
| RLSafe
| RLCarried.

Definition original_zero_trip (_ : reduced_licm_case) (entry locals : loop_state) : Z :=
  locals "x".

Definition hoisted_zero_trip (_ : reduced_licm_case) (entry locals : loop_state) : Z :=
  locals "x".

Definition original_one_trip (c : reduced_licm_case) (entry locals : loop_state) : Z :=
  match c with
  | RLSafe => locals "x" + locals "dt"
  | RLCarried => locals "time" + 1
  end.

Definition hoisted_one_trip (c : reduced_licm_case) (entry locals : loop_state) : Z :=
  match c with
  | RLSafe => locals "x" + locals "dt"
  | RLCarried => entry "time0"
  end.

Definition safe_case (c : reduced_licm_case) : Prop :=
  c = RLSafe.

Lemma licm_zero_trip_preserves_semantics :
  forall c entry locals,
    original_zero_trip c entry locals =
    hoisted_zero_trip c entry locals.
Proof.
  reflexivity.
Qed.

Lemma licm_one_trip_preserves_semantics :
  forall c entry locals,
    safe_case c ->
    original_one_trip c entry locals =
    hoisted_one_trip c entry locals.
Proof.
  intros c entry locals Hsafe.
  unfold safe_case in Hsafe.
  subst c.
  reflexivity.
Qed.

Lemma licm_loop_carried_state_not_sound :
  forall entry locals,
    locals "time" + 1 <> entry "time0" ->
    original_one_trip RLCarried entry locals <>
    hoisted_one_trip RLCarried entry locals.
Proof.
  intros entry locals Hneq.
  exact Hneq.
Qed.

Fixpoint get_nat_at (xs : list nat) (idx : nat) : option nat :=
  match xs, idx with
  | [], _ => None
  | head :: _, O => Some head
  | _ :: rest, S idx' => get_nat_at rest idx'
  end.

Definition bce_original_read (xs : list nat) (idx : nat) : option nat :=
  if Nat.ltb idx (List.length xs) then get_nat_at xs idx else None.

Definition bce_optimized_read (xs : list nat) (idx : nat) : option nat :=
  get_nat_at xs idx.

Lemma get_nat_at_none_of_ge :
  forall xs idx,
    (List.length xs <= idx)%nat ->
    get_nat_at xs idx = None.
Proof.
  induction xs as [|head rest IH]; intros idx Hle; simpl in *.
  - reflexivity.
  - destruct idx as [|idx].
    + inversion Hle.
    + apply IH. apply le_S_n. exact Hle.
Qed.

Lemma bce_reduced_preserves_semantics :
  forall xs idx,
    bce_original_read xs idx = bce_optimized_read xs idx.
Proof.
  intros xs idx.
  unfold bce_original_read, bce_optimized_read.
  destruct (Nat.ltb idx (List.length xs)) eqn:Hlt.
  - reflexivity.
  - apply Nat.ltb_ge in Hlt.
    symmetry.
    exact (get_nat_at_none_of_ge xs idx Hlt).
Qed.

Fixpoint tco_original (n acc : nat) : nat :=
  match n with
  | O => acc
  | S n' => tco_original n' (S acc)
  end.

Definition tco_optimized (n acc : nat) : nat :=
  acc + n.

Lemma tco_reduced_preserves_semantics :
  forall n acc,
    tco_original n acc = tco_optimized n acc.
Proof.
  induction n as [|n IH]; intros acc; simpl.
  - unfold tco_optimized. simpl. rewrite Nat.add_0_r. reflexivity.
  - rewrite IH.
    unfold tco_optimized. simpl.
    lia.
Qed.

Lemma loop_opt_identity_preserves_verify_ir_bundle :
  forall fn,
    optimizer_eligible fn ->
    optimizer_eligible (identity_pass fn).
Proof.
  intros fn H.
  exact (identity_pass_preserves_verify_ir_bundle fn H).
Qed.

End RRLoopOptSoundness.
