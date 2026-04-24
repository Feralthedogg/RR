import RRProofs.MirSemanticsLite

namespace RRProofs.MirInvariantBundle

open RRProofs.MirSemanticsLite

structure MirFnLite where
  entry : Nat
  bodyHead : Nat
  blocks : List MirBlock
  unsupportedDynamic : Bool := false
  opaqueInterop : Bool := false
deriving Repr

def blockIds (fn : MirFnLite) : List Nat :=
  fn.blocks.map (·.id)

def hasBlock (fn : MirFnLite) (bid : Nat) : Prop :=
  bid ∈ blockIds fn

def entryBlock? (fn : MirFnLite) : Option MirBlock :=
  fn.blocks.find? (fun blk => blk.id = fn.entry)

def phiPredsWithinBlockIds (fn : MirFnLite) : Prop :=
  ∀ blk ∈ fn.blocks, ∀ phi ∈ blk.phis, ∀ arm ∈ phi.arms, hasBlock fn arm.pred

def termTargetsWithinBlockIds (fn : MirFnLite) : Prop :=
  ∀ blk ∈ fn.blocks,
    match blk.term with
    | .goto target => hasBlock fn target
    | .ite _ thenBlk elseBlk => hasBlock fn thenBlk ∧ hasBlock fn elseBlk
    | .ret _ | .unreachable => True

structure MirInvariantBundle (fn : MirFnLite) : Prop where
  entry_valid : hasBlock fn fn.entry
  body_head_valid : hasBlock fn fn.bodyHead
  phi_preds_valid : phiPredsWithinBlockIds fn
  term_targets_valid : termTargetsWithinBlockIds fn
  optimizer_scope : fn.unsupportedDynamic = false ∧ fn.opaqueInterop = false

def OptimizerEligible (fn : MirFnLite) : Prop :=
  MirInvariantBundle fn

def execEntry (fn : MirFnLite) (env : Env) : BlockExit :=
  match entryBlock? fn with
  | some blk => execBlockEntry fn.entry blk env
  | none => .stuck

def identityPass (fn : MirFnLite) : MirFnLite := fn

theorem identity_pass_preserves_verify_ir_bundle
    {fn : MirFnLite} (h : MirInvariantBundle fn) :
    MirInvariantBundle (identityPass fn) := by
  simpa [identityPass]

theorem identity_pass_preserves_semantics (fn : MirFnLite) (env : Env) :
    execEntry (identityPass fn) env = execEntry fn env := by
  rfl

theorem optimizer_eligible_excludes_dynamic
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    fn.unsupportedDynamic = false ∧ fn.opaqueInterop = false := by
  exact h.optimizer_scope

end RRProofs.MirInvariantBundle
