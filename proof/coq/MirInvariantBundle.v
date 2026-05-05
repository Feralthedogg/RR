From Stdlib Require Import List.
From Stdlib Require Import String.
From Stdlib Require Import Bool.
From Stdlib Require Import Arith.

Require Import MirSemanticsLite.

Import ListNotations.
Open Scope string_scope.

Module RRMirInvariantBundle.

Import RRMirSemanticsLite.

Record mir_fn_lite : Type := {
  fn_entry : nat;
  fn_body_head : nat;
  fn_blocks : list mir_block;
  fn_unsupported_dynamic : bool;
  fn_opaque_interop : bool;
}.

Definition block_ids (fn : mir_fn_lite) : list nat :=
  map block_id fn.(fn_blocks).

Definition has_block (fn : mir_fn_lite) (bid : nat) : Prop :=
  In bid (block_ids fn).

Fixpoint find_block (blocks : list mir_block) (bid : nat) : option mir_block :=
  match blocks with
  | [] => None
  | blk :: rest =>
      if Nat.eqb blk.(block_id) bid then Some blk else find_block rest bid
  end.

Definition phi_preds_within_block_ids (fn : mir_fn_lite) : Prop :=
  forall blk phi arm,
    In blk fn.(fn_blocks) ->
    In phi blk.(block_phis) ->
    In arm phi.(phi_arms) ->
    has_block fn arm.(phi_pred).

Definition term_targets_within_block_ids (fn : mir_fn_lite) : Prop :=
  forall blk,
    In blk fn.(fn_blocks) ->
    match blk.(block_term) with
    | MTGoto target => has_block fn target
    | MTIf _ then_blk else_blk => has_block fn then_blk /\ has_block fn else_blk
    | MTRet _ | MTUnreachable => True
    end.

Record mir_invariant_bundle (fn : mir_fn_lite) : Prop := {
  entry_valid : has_block fn fn.(fn_entry);
  body_head_valid : has_block fn fn.(fn_body_head);
  phi_preds_valid : phi_preds_within_block_ids fn;
  term_targets_valid : term_targets_within_block_ids fn;
  optimizer_scope :
    fn.(fn_unsupported_dynamic) = false /\ fn.(fn_opaque_interop) = false;
}.

Definition optimizer_eligible (fn : mir_fn_lite) : Prop :=
  mir_invariant_bundle fn.

Definition entry_block (fn : mir_fn_lite) : option mir_block :=
  find_block fn.(fn_blocks) fn.(fn_entry).

Definition exec_entry (fn : mir_fn_lite) (ρ : env) : block_exit :=
  match entry_block fn with
  | Some blk => exec_block_entry fn.(fn_entry) blk ρ
  | None => BXStuck
  end.

Definition identity_pass (fn : mir_fn_lite) : mir_fn_lite := fn.

Lemma identity_pass_preserves_verify_ir_bundle :
  forall fn,
    mir_invariant_bundle fn ->
    mir_invariant_bundle (identity_pass fn).
Proof.
  intros fn H.
  exact H.
Qed.

Lemma identity_pass_preserves_semantics :
  forall fn ρ,
    exec_entry (identity_pass fn) ρ = exec_entry fn ρ.
Proof.
  reflexivity.
Qed.

Lemma optimizer_eligible_excludes_dynamic :
  forall fn,
    optimizer_eligible fn ->
    fn.(fn_unsupported_dynamic) = false /\ fn.(fn_opaque_interop) = false.
Proof.
  intros fn H.
  exact (optimizer_scope fn H).
Qed.

End RRMirInvariantBundle.
