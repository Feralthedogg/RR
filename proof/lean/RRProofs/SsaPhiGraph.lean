set_option linter.unusedSimpArgs false

namespace RRProofs.SsaPhiGraph

abbrev ValueId := Nat
abbrev BlockId := Nat
abbrev ValueEnv := ValueId -> Int

structure PhiArm where
  pred : BlockId
  value : ValueId
deriving DecidableEq, Repr

structure HeaderPhi where
  self : ValueId
  header : BlockId
  entryPred : BlockId
  latchPred : BlockId
  entryVal : ValueId
  latchVal : ValueId
deriving DecidableEq, Repr

def HeaderPhi.eval (φ : HeaderPhi) (iter : Nat) (ρ : ValueEnv) : Int :=
  if iter = 0 then ρ φ.entryVal else ρ φ.latchVal

def HeaderPhi.invariant (φ : HeaderPhi) (ρ : ValueEnv) : Prop :=
  ∀ i j, φ.eval i ρ = φ.eval j ρ

def HeaderPhi.selfBackedge (φ : HeaderPhi) : Prop :=
  φ.latchVal = φ.self

  theorem headerPhi_invariant_of_equal_entry_and_latch
    (φ : HeaderPhi)
    (ρ : ValueEnv)
    (h : ρ φ.entryVal = ρ φ.latchVal) :
    φ.invariant ρ := by
  intro i j
  simp [HeaderPhi.eval]
  split <;> split <;> simp [h]

theorem headerPhi_not_invariant_if_entry_and_latch_differ
    (φ : HeaderPhi)
    (ρ : ValueEnv)
    (h : ρ φ.entryVal ≠ ρ φ.latchVal) :
    ¬ φ.invariant ρ := by
  intro hInv
  have h01 := hInv 0 1
  simp [HeaderPhi.eval] at h01
  exact h h01

theorem self_backedge_phi_not_invariant_if_self_and_entry_differ
    (φ : HeaderPhi)
    (ρ : ValueEnv)
    (hBack : φ.selfBackedge)
    (h : ρ φ.entryVal ≠ ρ φ.self) :
    ¬ φ.invariant ρ := by
  apply headerPhi_not_invariant_if_entry_and_latch_differ
  intro hEq
  apply h
  simpa [HeaderPhi.selfBackedge] using hBack ▸ hEq

def exampleLoopPhi : HeaderPhi :=
  { self := 7
  , header := 10
  , entryPred := 1
  , latchPred := 9
  , entryVal := 3
  , latchVal := 7
  }

theorem exampleLoopPhi_has_self_backedge : exampleLoopPhi.selfBackedge := by
  rfl

theorem exampleLoopPhi_not_invariant
    (ρ : ValueEnv)
    (h : ρ 3 ≠ ρ 7) :
    ¬ exampleLoopPhi.invariant ρ := by
  apply self_backedge_phi_not_invariant_if_self_and_entry_differ
  · exact exampleLoopPhi_has_self_backedge
  · simpa using h

end RRProofs.SsaPhiGraph
