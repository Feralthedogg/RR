From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import Bool.
From Stdlib Require Import Arith.

Require Import RRProofs.MirSemanticsLite.
Require Import RRProofs.MirInvariantBundle.

Import ListNotations.
Open Scope string_scope.

Module RRCfgOptSoundness.

Import RRMirSemanticsLite.
Import RRMirInvariantBundle.

Fixpoint find_block (blocks : list mir_block) (bid : nat) : option mir_block :=
  match blocks with
  | [] => None
  | blk :: rest =>
      if Nat.eqb blk.(block_id) bid then Some blk else find_block rest bid
  end.

Fixpoint run_fuel_state (fn : mir_fn_lite) (pred cur : nat) (ρ : env) (fuel : nat)
    : option (option mir_value) :=
  match fuel with
  | O => None
  | S fuel' =>
      match find_block fn.(fn_blocks) cur with
      | None => None
      | Some blk =>
          match exec_block_entry pred blk ρ with
          | BXJump next ρ' => run_fuel_state fn cur next ρ' fuel'
          | BXDone result _ => Some result
          | BXStuck => None
          end
      end
  end.

Definition run_fuel (fn : mir_fn_lite) (ρ : env) (fuel : nat) : option (option mir_value) :=
  run_fuel_state fn fn.(fn_entry) fn.(fn_entry) ρ fuel.

Definition append_dead_block (fn : mir_fn_lite) (dead_blk : mir_block) : mir_fn_lite :=
  {|
    fn_entry := fn.(fn_entry);
    fn_body_head := fn.(fn_body_head);
    fn_blocks := fn.(fn_blocks) ++ [dead_blk];
    fn_unsupported_dynamic := fn.(fn_unsupported_dynamic);
    fn_opaque_interop := fn.(fn_opaque_interop);
  |}.

Definition retarget_entry (fn : mir_fn_lite) (target : nat) : mir_fn_lite :=
  {|
    fn_entry := target;
    fn_body_head := fn.(fn_body_head);
    fn_blocks := fn.(fn_blocks);
    fn_unsupported_dynamic := fn.(fn_unsupported_dynamic);
    fn_opaque_interop := fn.(fn_opaque_interop);
  |}.


Lemma find_block_some_in_blocks :
  forall blocks bid blk,
    find_block blocks bid = Some blk ->
    exists b, In b blocks /\ b.(block_id) = bid.
Proof.
  induction blocks as [|head rest IH]; intros bid blk Hfind; simpl in *.
  - discriminate Hfind.
  - destruct (Nat.eqb (block_id head) bid) eqn:Heq.
    + exists head. split.
      * now left.
      * apply Nat.eqb_eq. exact Heq.
    + destruct (IH _ _ Hfind) as [b [Hb Hbid]].
      exists b. split.
      * now right.
      * exact Hbid.
Qed.

Lemma has_block_append_old :
  forall fn dead_blk bid,
    has_block fn bid ->
    has_block (append_dead_block fn dead_blk) bid.
Proof.
  intros fn dead_blk bid H.
  unfold has_block, block_ids, append_dead_block in *; simpl in *.
  rewrite map_app. simpl.
  apply in_or_app. now left.
Qed.

Lemma phi_preds_within_block_ids_append_dead :
  forall fn dead_blk,
    phi_preds_within_block_ids fn ->
    dead_blk.(block_phis) = [] ->
    phi_preds_within_block_ids (append_dead_block fn dead_blk).
Proof.
  intros fn dead_blk Hold Hdead blk phi arm Hblk Hphi Harm.
  unfold append_dead_block in Hblk; simpl in Hblk.
  apply in_app_or in Hblk.
  destruct Hblk as [HoldBlk | HdeadBlk].
  - apply has_block_append_old.
    eapply Hold; eauto.
  - destruct HdeadBlk as [HdeadBlk | []]. subst blk.
    rewrite Hdead in Hphi. inversion Hphi.
Qed.

Lemma term_targets_within_block_ids_append_dead :
  forall fn dead_blk,
    term_targets_within_block_ids fn ->
    dead_blk.(block_term) = MTUnreachable ->
    term_targets_within_block_ids (append_dead_block fn dead_blk).
Proof.
  intros fn dead_blk Hold Hdead blk Hblk.
  unfold append_dead_block in Hblk; simpl in Hblk.
  apply in_app_or in Hblk.
  destruct Hblk as [HoldBlk | HdeadBlk].
  - specialize (Hold blk HoldBlk).
    destruct (block_term blk); simpl in *; try exact I.
    + apply has_block_append_old. exact Hold.
    + destruct Hold as [Hthen Helse]. split; apply has_block_append_old; assumption.
  - destruct HdeadBlk as [HdeadBlk | []]. subst blk. rewrite Hdead. simpl. exact I.
Qed.

Lemma append_dead_block_preserves_verify_ir_bundle :
  forall fn dead_blk,
    mir_invariant_bundle fn ->
    dead_blk.(block_phis) = [] ->
    dead_blk.(block_term) = MTUnreachable ->
    mir_invariant_bundle (append_dead_block fn dead_blk).
Proof.
  intros fn dead_blk Hinv Hphis Hterm.
  refine {| 
    entry_valid := has_block_append_old fn dead_blk fn.(fn_entry) (entry_valid fn Hinv);
    body_head_valid := has_block_append_old fn dead_blk fn.(fn_body_head) (body_head_valid fn Hinv);
    phi_preds_valid := phi_preds_within_block_ids_append_dead fn dead_blk (phi_preds_valid fn Hinv) Hphis;
    term_targets_valid := term_targets_within_block_ids_append_dead fn dead_blk (term_targets_valid fn Hinv) Hterm;
    optimizer_scope := optimizer_scope fn Hinv
  |}.
Qed.

Lemma exec_block_entry_pred_irrelevant_when_no_phis :
  forall blk ρ pred1 pred2,
    blk.(block_phis) = [] ->
    exec_block_entry pred1 blk ρ = exec_block_entry pred2 blk ρ.
Proof.
  intros blk ρ pred1 pred2 Hphis.
  unfold exec_block_entry.
  rewrite Hphis.
  repeat rewrite apply_phi_nodes_nil.
  reflexivity.
Qed.

Lemma run_fuel_state_retarget_entry_same_blocks :
  forall fn target pred cur ρ fuel,
    run_fuel_state (retarget_entry fn target) pred cur ρ fuel =
    run_fuel_state fn pred cur ρ fuel.
Proof.
  intros fn target pred cur ρ fuel.
  revert fn target pred cur ρ.
  induction fuel as [|fuel IH]; intros fn target pred cur ρ; simpl.
  - reflexivity.
  - destruct (find_block (fn_blocks fn) cur) as [blk|] eqn:Hfind; simpl; try reflexivity.
    destruct (exec_block_entry pred blk ρ) as [next ρ'|result ρ'|] eqn:Hexit; simpl; try reflexivity.
    apply IH.
Qed.

Lemma run_fuel_state_pred_irrelevant_when_block_has_no_phis :
  forall fn blk cur pred1 pred2 ρ fuel,
    find_block fn.(fn_blocks) cur = Some blk ->
    blk.(block_phis) = [] ->
    run_fuel_state fn pred1 cur ρ fuel =
    run_fuel_state fn pred2 cur ρ fuel.
Proof.
  intros fn blk cur pred1 pred2 ρ fuel Hfind Hphis.
  revert fn blk cur pred1 pred2 ρ Hfind Hphis.
  induction fuel as [|fuel IH]; intros fn blk cur pred1 pred2 ρ Hfind Hphis; simpl.
  - reflexivity.
  - rewrite Hfind.
    rewrite (exec_block_entry_pred_irrelevant_when_no_phis blk ρ pred1 pred2 Hphis).
    destruct (exec_block_entry pred2 blk ρ) as [next ρ'|result ρ'|] eqn:Hexit; simpl; try reflexivity.
Qed.

Lemma run_fuel_empty_entry_goto_preserved :
  forall fn entry_blk target_blk target ρ fuel,
    find_block fn.(fn_blocks) fn.(fn_entry) = Some entry_blk ->
    entry_blk.(block_phis) = [] ->
    entry_blk.(block_instrs) = [] ->
    entry_blk.(block_term) = MTGoto target ->
    find_block fn.(fn_blocks) target = Some target_blk ->
    target_blk.(block_phis) = [] ->
    run_fuel fn ρ (S fuel) =
    run_fuel (retarget_entry fn target) ρ fuel.
Proof.
  intros fn entry_blk target_blk target ρ fuel Hentry HentryPhis HentryInstrs HentryTerm Htarget HtargetPhis.
  unfold run_fuel. simpl. rewrite Hentry.
  unfold exec_block_entry.
  rewrite HentryPhis. rewrite apply_phi_nodes_nil.
  rewrite HentryInstrs. simpl. rewrite HentryTerm. simpl.
  rewrite (run_fuel_state_pred_irrelevant_when_block_has_no_phis fn target_blk target fn.(fn_entry) target ρ fuel Htarget HtargetPhis).
  symmetry. apply run_fuel_state_retarget_entry_same_blocks.
Qed.

Lemma retarget_entry_preserves_verify_ir_bundle :
  forall fn target target_blk,
    mir_invariant_bundle fn ->
    find_block fn.(fn_blocks) target = Some target_blk ->
    target_blk.(block_phis) = [] ->
    mir_invariant_bundle (retarget_entry fn target).
Proof.
  intros fn target target_blk Hinv Htarget _.
  refine {| 
    entry_valid := _;
    body_head_valid := _;
    phi_preds_valid := _;
    term_targets_valid := _;
    optimizer_scope := _
  |}.
  unfold has_block, block_ids.
  destruct (find_block_some_in_blocks _ _ _ Htarget) as [b [Hb Hbid]].
  simpl. rewrite <- Hbid. apply in_map. exact Hb.
  - exact (body_head_valid fn Hinv).
  - exact (phi_preds_valid fn Hinv).
  - exact (term_targets_valid fn Hinv).
  - exact (optimizer_scope fn Hinv).
Qed.

End RRCfgOptSoundness.
