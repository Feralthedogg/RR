import RRProofs.GraphLicmSound

namespace RRProofs

structure RRWfCase where
  licm : LicmGraphCase
  uniquePreheaderLatch : licm.shape.preheader ≠ licm.shape.latch
  headerPredsExact : licm.graph.preds licm.shape.header = [licm.shape.preheader, licm.shape.latch]
  tmpFreshInBody : ∀ instr, instr ∈ licm.fnir.bodyInstrs -> instr.write ≠ licm.fnir.tmp

def RRWfCase.wf (c : RRWfCase) : Prop :=
  c.licm.graphWf ∧
  c.licm.selfBackedgePhi = (c.licm.phi.latchVal = c.licm.phi.self) ∧
  c.licm.fnir.bodyInstrs = c.licm.fnir.toCfg.body

theorem rrwf_implies_graph_wf
    (c : RRWfCase)
    (h : c.wf) :
    c.licm.graphWf := by
  exact h.1

theorem rrwf_safe_candidate_zero_trip
    (c : RRWfCase)
    (entry locals : State)
    (_hWf : c.wf)
    (hSafe : c.licm.safeCandidate) :
    (runOriginalMachine c.licm.fnir false entry locals).result? =
      (runHoistedMachine c.licm.fnir false entry locals).result? := by
  exact graph_level_zero_trip_sound c.licm entry locals hSafe

theorem rrwf_safe_candidate_one_trip
    (c : RRWfCase)
    (entry locals : State)
    (hWf : c.wf)
    (hSafe : c.licm.safeCandidate) :
    (runOriginalMachine c.licm.fnir true entry locals).result? =
      (runHoistedMachine c.licm.fnir true entry locals).result? := by
  exact graph_level_one_trip_sound c.licm entry locals (rrwf_implies_graph_wf c hWf) hSafe

theorem rrwf_self_backedge_phi_not_invariant
    (c : RRWfCase)
    (ρ : GValueEnv)
    (hWf : c.wf)
    (hBack : c.licm.selfBackedgePhi)
    (hVals : ρ c.licm.phi.entryVal ≠ ρ c.licm.phi.self) :
    ¬ c.licm.phi.predInvariant c.licm.graph c.licm.shape ρ := by
  exact graph_level_self_backedge_phi_not_invariant
    c.licm ρ (rrwf_implies_graph_wf c hWf) hBack hVals

def exampleRRWfCase : RRWfCase :=
  { licm := exampleLicmGraphCase
  , uniquePreheaderLatch := by decide
  , headerPredsExact := by rfl
  , tmpFreshInBody := by
      intro instr hMem
      simp [reducedPhiTimeFn, exampleLicmGraphCase, phiTimeCfg] at hMem
      rcases hMem with rfl | hRest
      · decide
  }

theorem exampleRRWfCase_wf : exampleRRWfCase.wf := by
  constructor
  · exact exampleLicmGraphCase_graphWf
  constructor
  · rfl
  · rfl

theorem exampleRRWfCase_unsound
    (entry locals : State)
    (h : locals "time" + 1 ≠ entry "time0") :
    (runOriginalMachine exampleRRWfCase.licm.fnir true entry locals).result? ≠
      (runHoistedMachine exampleRRWfCase.licm.fnir true entry locals).result? := by
  simpa [exampleRRWfCase] using exampleLicmGraphCase_unsound_machine entry locals h

end RRProofs
