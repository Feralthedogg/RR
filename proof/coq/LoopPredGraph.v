From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import ZArith.

Import ListNotations.
Open Scope string_scope.
Open Scope Z_scope.

Module RRLoopPredGraph.

Definition block_id := nat.
Definition value_id := nat.
Definition value_env := value_id -> Z.

Record pred_graph : Type := {
  preds_of : block_id -> list block_id;
}.

Record loop_shape : Type := {
  header_of : block_id;
  preheader_of : block_id;
  latch_of : block_id;
}.

Definition wf_loop (g : pred_graph) (s : loop_shape) : Prop :=
  preds_of g (header_of s) = [preheader_of s; latch_of s] /\
  preheader_of s <> latch_of s.

Record header_phi_graph : Type := {
  phi_self_of : value_id;
  phi_entry_val_of : value_id;
  phi_latch_val_of : value_id;
}.

Definition eval_on_pred
    (phi : header_phi_graph)
    (s : loop_shape)
    (pred : block_id)
    (ρ : value_env) : Z :=
  if Nat.eqb pred (preheader_of s) then ρ (phi_entry_val_of phi)
  else if Nat.eqb pred (latch_of s) then ρ (phi_latch_val_of phi)
  else 0.

Definition pred_invariant
    (g : pred_graph)
    (s : loop_shape)
    (phi : header_phi_graph)
    (ρ : value_env) : Prop :=
  forall p q,
    In p (preds_of g (header_of s)) ->
    In q (preds_of g (header_of s)) ->
    eval_on_pred phi s p ρ = eval_on_pred phi s q ρ.

Definition self_backedge (phi : header_phi_graph) : Prop :=
  phi_latch_val_of phi = phi_self_of phi.

Lemma wf_loop_header_has_preheader :
  forall g s,
    wf_loop g s ->
    In (preheader_of s) (preds_of g (header_of s)).
Proof.
  intros g s [Hpreds _].
  rewrite Hpreds.
  simpl; auto.
Qed.

Lemma wf_loop_header_has_latch :
  forall g s,
    wf_loop g s ->
    In (latch_of s) (preds_of g (header_of s)).
Proof.
  intros g s [Hpreds _].
  rewrite Hpreds.
  simpl; auto.
Qed.

Lemma header_phi_not_pred_invariant_if_entry_and_latch_differ :
  forall g s phi ρ,
    wf_loop g s ->
    ρ (phi_entry_val_of phi) <> ρ (phi_latch_val_of phi) ->
    ~ pred_invariant g s phi ρ.
Proof.
  intros g s phi ρ Hwf Hneq Hinv.
  pose proof (wf_loop_header_has_preheader g s Hwf) as Hpre.
  pose proof (wf_loop_header_has_latch g s Hwf) as Hlatch.
  pose proof (Hinv (preheader_of s) (latch_of s) Hpre Hlatch) as Heq.
  unfold eval_on_pred in Heq.
  destruct Hwf as [_ Hdistinct].
  rewrite Nat.eqb_refl in Heq.
  destruct (Nat.eqb_spec (latch_of s) (preheader_of s)).
  - subst.
    exfalso.
    exact (Hdistinct (eq_sym e)).
  - rewrite Nat.eqb_refl in Heq.
  exact (Hneq Heq).
Qed.

Lemma self_backedge_header_phi_not_pred_invariant :
  forall g s phi ρ,
    wf_loop g s ->
    self_backedge phi ->
    ρ (phi_entry_val_of phi) <> ρ (phi_self_of phi) ->
    ~ pred_invariant g s phi ρ.
Proof.
  intros g s phi ρ Hwf Hback Hneq.
  apply header_phi_not_pred_invariant_if_entry_and_latch_differ with (g := g) (s := s).
  - exact Hwf.
  - rewrite Hback.
    exact Hneq.
Qed.

Definition example_graph : pred_graph :=
  {| preds_of := fun b =>
       if Nat.eqb b 10%nat then [1%nat; 9%nat] else [] |}.

Definition example_loop_shape : loop_shape :=
  {| header_of := 10%nat; preheader_of := 1%nat; latch_of := 9%nat |}.

Definition example_header_phi : header_phi_graph :=
  {| phi_self_of := 7%nat; phi_entry_val_of := 3%nat; phi_latch_val_of := 7%nat |}.

Lemma example_loop_shape_wf :
  wf_loop example_graph example_loop_shape.
Proof.
  unfold wf_loop, example_graph, example_loop_shape.
  simpl.
  split; auto.
Qed.

Lemma example_header_phi_self_backedge :
  self_backedge example_header_phi.
Proof.
  reflexivity.
Qed.

Lemma example_header_phi_not_pred_invariant :
  forall ρ,
    ρ 3%nat <> ρ 7%nat ->
    ~ pred_invariant example_graph example_loop_shape example_header_phi ρ.
Proof.
  intros ρ Hneq.
  apply self_backedge_header_phi_not_pred_invariant.
  - exact example_loop_shape_wf.
  - exact example_header_phi_self_backedge.
  - exact Hneq.
Qed.

End RRLoopPredGraph.
