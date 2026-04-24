import RRProofs.MirSemanticsLite
import RRProofs.MirInvariantBundle

namespace RRProofs.CfgOptSoundness

open RRProofs.MirSemanticsLite
open RRProofs.MirInvariantBundle

def findBlock? (blocks : List MirBlock) (bid : Nat) : Option MirBlock :=
  match blocks with
  | [] => none
  | blk :: rest =>
      if blk.id = bid then some blk else findBlock? rest bid

def runFuelState (fn : MirFnLite) (pred cur : Nat) (env : Env) : Nat -> Option (Option MirValue)
  | 0 => none
  | fuel + 1 =>
      match findBlock? fn.blocks cur with
      | none => none
      | some blk =>
          match execBlockEntry pred blk env with
          | .jump next env' => runFuelState fn cur next env' fuel
          | .done result _ => some result
          | .stuck => none

def runFuel (fn : MirFnLite) (env : Env) (fuel : Nat) : Option (Option MirValue) :=
  runFuelState fn fn.entry fn.entry env fuel

def appendDeadBlock (fn : MirFnLite) (deadBlk : MirBlock) : MirFnLite :=
  { fn with blocks := fn.blocks ++ [deadBlk] }

def retargetEntry (fn : MirFnLite) (target : Nat) : MirFnLite :=
  { fn with entry := target }

theorem findBlock_some_in_blocks
    {blocks : List MirBlock} {bid : Nat} {blk : MirBlock}
    (h : findBlock? blocks bid = some blk) :
    ∃ b, b ∈ blocks ∧ b.id = bid := by
  induction blocks with
  | nil =>
      simp [findBlock?] at h
  | cons head rest ih =>
      by_cases hEq : head.id = bid
      · exact ⟨head, by simp, hEq⟩
      · simp [findBlock?, hEq] at h
        rcases ih h with ⟨b, hb, hbid⟩
        exact ⟨b, by simp [hb], hbid⟩

theorem hasBlock_append_old {fn : MirFnLite} {deadBlk : MirBlock} {bid : Nat} :
    hasBlock fn bid -> hasBlock (appendDeadBlock fn deadBlk) bid := by
  intro h
  simp [hasBlock, blockIds] at h ⊢
  rcases h with ⟨blk, hBlk, hId⟩
  exact ⟨blk, by simp [appendDeadBlock, hBlk], hId⟩

theorem phiPredsWithinBlockIds_append_dead
    {fn : MirFnLite} {deadBlk : MirBlock}
    (hOld : phiPredsWithinBlockIds fn)
    (hDead : deadBlk.phis = []) :
    phiPredsWithinBlockIds (appendDeadBlock fn deadBlk) := by
  intro blk hBlk phi hPhi arm hArm
  unfold appendDeadBlock at hBlk
  simp at hBlk
  cases hBlk with
  | inl hOldBlk =>
      exact hasBlock_append_old (hOld blk hOldBlk phi hPhi arm hArm)
  | inr hDeadBlk =>
      subst hDeadBlk
      simp [hDead] at hPhi

theorem termTargetsWithinBlockIds_append_dead
    {fn : MirFnLite} {deadBlk : MirBlock}
    (hOld : termTargetsWithinBlockIds fn)
    (hDead : deadBlk.term = .unreachable) :
    termTargetsWithinBlockIds (appendDeadBlock fn deadBlk) := by
  intro blk hBlk
  unfold appendDeadBlock at hBlk
  simp at hBlk
  cases hBlk with
  | inl hOldBlk =>
      have h := hOld blk hOldBlk
      cases hTerm : blk.term <;> simp [hTerm] at h ⊢
      · exact hasBlock_append_old h
      · exact ⟨hasBlock_append_old h.1, hasBlock_append_old h.2⟩
  | inr hDeadBlk =>
      subst hDeadBlk
      simp [hDead]

theorem append_dead_block_preserves_verify_ir_bundle
    {fn : MirFnLite} {deadBlk : MirBlock}
    (hInv : MirInvariantBundle fn)
    (hDeadPhis : deadBlk.phis = [])
    (hDeadTerm : deadBlk.term = .unreachable) :
    MirInvariantBundle (appendDeadBlock fn deadBlk) := by
  refine {
    entry_valid := hasBlock_append_old hInv.entry_valid
    body_head_valid := hasBlock_append_old hInv.body_head_valid
    phi_preds_valid := phiPredsWithinBlockIds_append_dead hInv.phi_preds_valid hDeadPhis
    term_targets_valid := termTargetsWithinBlockIds_append_dead hInv.term_targets_valid hDeadTerm
    optimizer_scope := hInv.optimizer_scope
  }

theorem execBlockEntry_pred_irrelevant_when_no_phis
    (blk : MirBlock) (env : Env) (pred₁ pred₂ : Nat)
    (hNoPhis : blk.phis = []) :
    execBlockEntry pred₁ blk env = execBlockEntry pred₂ blk env := by
  simp [execBlockEntry, hNoPhis, applyPhiNodes_nil]

theorem runFuelState_retargetEntry_same_blocks
    (fn : MirFnLite) (target pred cur : Nat) (env : Env) (fuel : Nat) :
    runFuelState (retargetEntry fn target) pred cur env fuel = runFuelState fn pred cur env fuel := by
  induction fuel generalizing pred cur env with
  | zero =>
      simp [runFuelState]
  | succ fuel ih =>
      simp [runFuelState, retargetEntry]
      cases hFind : findBlock? fn.blocks cur <;> simp
      case some blk =>
        cases hExit : execBlockEntry pred blk env <;> simp
        exact ih cur _ _

theorem runFuelState_pred_irrelevant_when_block_has_no_phis
    (fn : MirFnLite) (blk : MirBlock) (cur pred₁ pred₂ : Nat) (env : Env) (fuel : Nat)
    (hFind : findBlock? fn.blocks cur = some blk)
    (hNoPhis : blk.phis = []) :
    runFuelState fn pred₁ cur env fuel = runFuelState fn pred₂ cur env fuel := by
  induction fuel generalizing pred₁ pred₂ env with
  | zero =>
      simp [runFuelState]
  | succ fuel ih =>
      simp [runFuelState, hFind]
      rw [execBlockEntry_pred_irrelevant_when_no_phis blk env pred₁ pred₂ hNoPhis]

theorem runFuel_empty_entry_goto_preserved
    (fn : MirFnLite) (entryBlk targetBlk : MirBlock) (target : Nat) (env : Env) (fuel : Nat)
    (hEntry : findBlock? fn.blocks fn.entry = some entryBlk)
    (hEntryPhis : entryBlk.phis = [])
    (hEntryInstrs : entryBlk.instrs = [])
    (hEntryTerm : entryBlk.term = .goto target)
    (hTarget : findBlock? fn.blocks target = some targetBlk)
    (hTargetPhis : targetBlk.phis = []) :
    runFuel fn env (fuel + 1) = runFuel (retargetEntry fn target) env fuel := by
  unfold runFuel
  simp [runFuelState, hEntry]
  simp [execBlockEntry, hEntryPhis, hEntryInstrs, hEntryTerm, applyPhiNodes_nil, execInstrs, execTerm]
  calc
    runFuelState fn fn.entry target env fuel
      = runFuelState fn target target env fuel :=
          runFuelState_pred_irrelevant_when_block_has_no_phis fn targetBlk target fn.entry target env fuel hTarget hTargetPhis
    _ = runFuelState (retargetEntry fn target) target target env fuel := by
          symm
          exact runFuelState_retargetEntry_same_blocks fn target target target env fuel

theorem retarget_entry_preserves_verify_ir_bundle
    {fn : MirFnLite} {target : Nat} {targetBlk : MirBlock}
    (hInv : MirInvariantBundle fn)
    (hTarget : findBlock? fn.blocks target = some targetBlk)
    (_hTargetPhis : targetBlk.phis = []) :
    MirInvariantBundle (retargetEntry fn target) := by
  refine {
    entry_valid := by
      simpa [hasBlock, blockIds, retargetEntry] using findBlock_some_in_blocks hTarget
    body_head_valid := hInv.body_head_valid
    phi_preds_valid := by simpa [retargetEntry] using hInv.phi_preds_valid
    term_targets_valid := by simpa [retargetEntry] using hInv.term_targets_valid
    optimizer_scope := hInv.optimizer_scope
  }

end RRProofs.CfgOptSoundness
