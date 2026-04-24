From Stdlib Require Import String.
From Stdlib Require Import ZArith.
From Stdlib Require Import Lia.

Open Scope string_scope.
Open Scope Z_scope.

Module RRSsaPhiGraph.

Definition value_id := nat.
Definition block_id := nat.
Definition value_env := value_id -> Z.

Record phi_arm : Type := {
  arm_pred : block_id;
  arm_value : value_id;
}.

Record header_phi : Type := {
  phi_self : value_id;
  phi_header : block_id;
  phi_entry_pred : block_id;
  phi_latch_pred : block_id;
  phi_entry_val : value_id;
  phi_latch_val : value_id;
}.

Definition eval (phi : header_phi) (iter : nat) (ρ : value_env) : Z :=
  match iter with
  | O => ρ (phi_entry_val phi)
  | S _ => ρ (phi_latch_val phi)
  end.

Definition invariant (phi : header_phi) (ρ : value_env) : Prop :=
  forall i j, eval phi i ρ = eval phi j ρ.

Definition self_backedge (phi : header_phi) : Prop :=
  phi_latch_val phi = phi_self phi.

Lemma invariant_of_equal_entry_and_latch :
  forall phi ρ,
    ρ (phi_entry_val phi) = ρ (phi_latch_val phi) ->
    invariant phi ρ.
Proof.
  intros phi ρ Heq i j.
  destruct i, j; simpl; auto.
Qed.

Lemma not_invariant_if_entry_and_latch_differ :
  forall phi ρ,
    ρ (phi_entry_val phi) <> ρ (phi_latch_val phi) ->
    ~ invariant phi ρ.
Proof.
  intros phi ρ Hneq Hinv.
  pose proof (Hinv O (S O)) as H01.
  simpl in H01.
  exact (Hneq H01).
Qed.

Lemma self_backedge_phi_not_invariant_if_self_and_entry_differ :
  forall phi ρ,
    self_backedge phi ->
    ρ (phi_entry_val phi) <> ρ (phi_self phi) ->
    ~ invariant phi ρ.
Proof.
  intros phi ρ Hback Hneq.
  apply not_invariant_if_entry_and_latch_differ.
  rewrite Hback.
  exact Hneq.
Qed.

Definition example_loop_phi : header_phi :=
  {| phi_self := 7%nat;
     phi_header := 10%nat;
     phi_entry_pred := 1%nat;
     phi_latch_pred := 9%nat;
     phi_entry_val := 3%nat;
     phi_latch_val := 7%nat |}.

Lemma example_loop_phi_has_self_backedge :
  self_backedge example_loop_phi.
Proof.
  reflexivity.
Qed.

Lemma example_loop_phi_not_invariant :
  forall ρ,
    ρ 3%nat <> ρ 7%nat ->
    ~ invariant example_loop_phi ρ.
Proof.
  intros ρ Hneq.
  apply self_backedge_phi_not_invariant_if_self_and_entry_differ.
  - exact example_loop_phi_has_self_backedge.
  - exact Hneq.
Qed.

End RRSsaPhiGraph.
