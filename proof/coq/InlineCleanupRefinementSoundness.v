Require Import MirSemanticsLite.
Require Import MirInvariantBundle.
Require Import CfgOptSoundness.

Module RRInlineCleanupRefinementSoundness.

Import RRMirSemanticsLite.
Import RRMirInvariantBundle.
Import RRCfgOptSoundness.

Record inline_cleanup_retarget_case : Type := {
  ic_fn : mir_fn_lite;
  ic_entry_blk : mir_block;
  ic_target_blk : mir_block;
  ic_target : nat;
  ic_env : env;
  ic_fuel : nat;
  ic_entry_found : find_block (fn_blocks ic_fn) (fn_entry ic_fn) = Some ic_entry_blk;
  ic_entry_no_phis : block_phis ic_entry_blk = @nil mir_phi;
  ic_entry_no_instrs : block_instrs ic_entry_blk = @nil mir_instr;
  ic_entry_goto : block_term ic_entry_blk = MTGoto ic_target;
  ic_target_found : find_block (fn_blocks ic_fn) ic_target = Some ic_target_blk;
  ic_target_no_phis : block_phis ic_target_blk = @nil mir_phi;
  ic_target_inv : mir_invariant_bundle ic_fn;
}.

Definition inline_cleanup_retarget (c : inline_cleanup_retarget_case) : mir_fn_lite :=
  retarget_entry c.(ic_fn) c.(ic_target).

Lemma inline_cleanup_retarget_preserves_verify_ir :
  forall c,
    mir_invariant_bundle (inline_cleanup_retarget c).
Proof.
  intros c.
  exact (retarget_entry_preserves_verify_ir_bundle
    c.(ic_fn) c.(ic_target) c.(ic_target_blk) c.(ic_target_inv) c.(ic_target_found) c.(ic_target_no_phis)).
Qed.

Lemma inline_cleanup_retarget_preserves_eval :
  forall c,
    run_fuel c.(ic_fn) c.(ic_env) (S c.(ic_fuel)) =
    run_fuel (inline_cleanup_retarget c) c.(ic_env) c.(ic_fuel).
Proof.
  intros c.
  exact (run_fuel_empty_entry_goto_preserved
    c.(ic_fn) c.(ic_entry_blk) c.(ic_target_blk) c.(ic_target) c.(ic_env) c.(ic_fuel)
    c.(ic_entry_found) c.(ic_entry_no_phis) c.(ic_entry_no_instrs) c.(ic_entry_goto)
    c.(ic_target_found) c.(ic_target_no_phis)).
Qed.

End RRInlineCleanupRefinementSoundness.
