import RRProofs.CfgOptSoundness

namespace RRProofs.InlineCleanupRefinementSoundness

open RRProofs.MirSemanticsLite
open RRProofs.MirInvariantBundle
open RRProofs.CfgOptSoundness

structure InlineCleanupRetargetCase where
  fn : MirFnLite
  entryBlk : MirBlock
  targetBlk : MirBlock
  target : Nat
  env : Env
  fuel : Nat
  entryFound : findBlock? fn.blocks fn.entry = some entryBlk
  entryNoPhis : entryBlk.phis = []
  entryNoInstrs : entryBlk.instrs = []
  entryGoto : entryBlk.term = .goto target
  targetFound : findBlock? fn.blocks target = some targetBlk
  targetNoPhis : targetBlk.phis = []
  targetInv : MirInvariantBundle fn

def inlineCleanupRetarget (c : InlineCleanupRetargetCase) : MirFnLite :=
  retargetEntry c.fn c.target

theorem inline_cleanup_retarget_preserves_verify_ir
    (c : InlineCleanupRetargetCase) :
    MirInvariantBundle (inlineCleanupRetarget c) := by
  exact retarget_entry_preserves_verify_ir_bundle c.targetInv c.targetFound c.targetNoPhis

theorem inline_cleanup_retarget_preserves_eval
    (c : InlineCleanupRetargetCase) :
    runFuel c.fn c.env (c.fuel + 1) = runFuel (inlineCleanupRetarget c) c.env c.fuel := by
  exact runFuel_empty_entry_goto_preserved
    c.fn c.entryBlk c.targetBlk c.target c.env c.fuel
    c.entryFound c.entryNoPhis c.entryNoInstrs c.entryGoto c.targetFound c.targetNoPhis

end RRProofs.InlineCleanupRefinementSoundness
