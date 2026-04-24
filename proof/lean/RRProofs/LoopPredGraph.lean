namespace RRProofs

abbrev GBlockId := Nat
abbrev GValueId := Nat
abbrev GValueEnv := GValueId -> Int

structure PredGraph where
  preds : GBlockId -> List GBlockId

structure LoopShape where
  header : GBlockId
  preheader : GBlockId
  latch : GBlockId
deriving DecidableEq, Repr

def LoopShape.wf (g : PredGraph) (s : LoopShape) : Prop :=
  g.preds s.header = [s.preheader, s.latch] ∧ s.preheader ≠ s.latch

structure HeaderPhiGraph where
  self : GValueId
  entryVal : GValueId
  latchVal : GValueId
deriving DecidableEq, Repr

def HeaderPhiGraph.evalOnPred (φ : HeaderPhiGraph) (s : LoopShape) (pred : GBlockId) (ρ : GValueEnv) : Int :=
  if pred = s.preheader then ρ φ.entryVal
  else if pred = s.latch then ρ φ.latchVal
  else 0

def HeaderPhiGraph.predInvariant (g : PredGraph) (s : LoopShape) (φ : HeaderPhiGraph) (ρ : GValueEnv) : Prop :=
  ∀ p q, p ∈ g.preds s.header -> q ∈ g.preds s.header ->
    φ.evalOnPred s p ρ = φ.evalOnPred s q ρ

def HeaderPhiGraph.selfBackedge (φ : HeaderPhiGraph) : Prop :=
  φ.latchVal = φ.self

theorem wf_loop_header_has_preheader
    (g : PredGraph) (s : LoopShape)
    (h : s.wf g) :
    s.preheader ∈ g.preds s.header := by
  rcases h with ⟨hPreds, _hNe⟩
  simp [hPreds]

theorem wf_loop_header_has_latch
    (g : PredGraph) (s : LoopShape)
    (h : s.wf g) :
    s.latch ∈ g.preds s.header := by
  rcases h with ⟨hPreds, _hNe⟩
  simp [hPreds]

theorem header_phi_not_pred_invariant_if_entry_and_latch_differ
    (g : PredGraph)
    (s : LoopShape)
    (φ : HeaderPhiGraph)
    (ρ : GValueEnv)
    (hWf : s.wf g)
    (hVals : ρ φ.entryVal ≠ ρ φ.latchVal) :
    ¬ φ.predInvariant g s ρ := by
  intro hInv
  have hPre := wf_loop_header_has_preheader g s hWf
  have hLatch := wf_loop_header_has_latch g s hWf
  have hEq := hInv s.preheader s.latch hPre hLatch
  have hNe : s.latch ≠ s.preheader := by
    intro hEq'
    exact hWf.2 hEq'.symm
  simp [HeaderPhiGraph.evalOnPred, hNe] at hEq
  exact hVals hEq

theorem self_backedge_header_phi_not_pred_invariant
    (g : PredGraph)
    (s : LoopShape)
    (φ : HeaderPhiGraph)
    (ρ : GValueEnv)
    (hWf : s.wf g)
    (hBack : φ.selfBackedge)
    (hVals : ρ φ.entryVal ≠ ρ φ.self) :
    ¬ φ.predInvariant g s ρ := by
  have hLatch : ρ φ.entryVal ≠ ρ φ.latchVal := by
    intro hEq
    rw [hBack] at hEq
    exact hVals hEq
  apply header_phi_not_pred_invariant_if_entry_and_latch_differ g s φ ρ hWf
  exact hLatch

def exampleGraph : PredGraph :=
  { preds := fun
      | 10 => [1, 9]
      | _ => [] }

def exampleLoopShape : LoopShape :=
  { header := 10, preheader := 1, latch := 9 }

def exampleHeaderPhi : HeaderPhiGraph :=
  { self := 7, entryVal := 3, latchVal := 7 }

theorem exampleLoopShape_wf : exampleLoopShape.wf exampleGraph := by
  simp [exampleLoopShape, exampleGraph, LoopShape.wf]

theorem exampleHeaderPhi_selfBackedge : exampleHeaderPhi.selfBackedge := by
  rfl

theorem exampleHeaderPhi_not_pred_invariant
    (ρ : GValueEnv)
    (h : ρ 3 ≠ ρ 7) :
    ¬ exampleHeaderPhi.predInvariant exampleGraph exampleLoopShape ρ := by
  apply self_backedge_header_phi_not_pred_invariant exampleGraph exampleLoopShape exampleHeaderPhi ρ
  · exact exampleLoopShape_wf
  · exact exampleHeaderPhi_selfBackedge
  · simpa using h

end RRProofs
