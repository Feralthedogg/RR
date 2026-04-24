import RRProofs.CfgSmallStep
import RRProofs.LoopPredGraph

set_option linter.unusedVariables false

namespace RRProofs

structure LicmGraphCase where
  fnir : ReducedFnIR
  graph : PredGraph
  shape : LoopShape
  phi : HeaderPhiGraph

def LicmGraphCase.graphWf (c : LicmGraphCase) : Prop :=
  c.shape.wf c.graph

def LicmGraphCase.safeCandidate (c : LicmGraphCase) : Prop :=
  safeToHoistCfg c.fnir.toCfg

def LicmGraphCase.selfBackedgePhi (c : LicmGraphCase) : Prop :=
  c.phi.selfBackedge

theorem graph_level_zero_trip_sound
    (c : LicmGraphCase)
    (entry locals : State)
    (h : c.safeCandidate) :
    (runOriginalMachine c.fnir false entry locals).result? =
      (runHoistedMachine c.fnir false entry locals).result? := by
  exact smallStepZeroTripSound c.fnir entry locals

theorem graph_level_one_trip_sound
    (c : LicmGraphCase)
    (entry locals : State)
    (hGraph : c.graphWf)
    (hSafe : c.safeCandidate) :
    (runOriginalMachine c.fnir true entry locals).result? =
      (runHoistedMachine c.fnir true entry locals).result? := by
  let _ := hGraph
  exact smallStepOneTripSound c.fnir entry locals hSafe

theorem graph_level_self_backedge_phi_not_invariant
    (c : LicmGraphCase)
    (ρ : GValueEnv)
    (hGraph : c.graphWf)
    (hBack : c.selfBackedgePhi)
    (hVals : ρ c.phi.entryVal ≠ ρ c.phi.self) :
    ¬ c.phi.predInvariant c.graph c.shape ρ := by
  exact self_backedge_header_phi_not_pred_invariant c.graph c.shape c.phi ρ hGraph hBack hVals

def exampleLicmGraphCase : LicmGraphCase :=
  { fnir := reducedPhiTimeFn
  , graph := exampleGraph
  , shape := exampleLoopShape
  , phi := exampleHeaderPhi
  }

theorem exampleLicmGraphCase_graphWf : exampleLicmGraphCase.graphWf := by
  exact exampleLoopShape_wf

theorem exampleLicmGraphCase_selfBackedgePhi : exampleLicmGraphCase.selfBackedgePhi := by
  exact exampleHeaderPhi_selfBackedge

theorem exampleLicmGraphCase_phi_not_invariant
    (ρ : GValueEnv)
    (h : ρ 3 ≠ ρ 7) :
    ¬ exampleLicmGraphCase.phi.predInvariant exampleLicmGraphCase.graph exampleLicmGraphCase.shape ρ := by
  exact graph_level_self_backedge_phi_not_invariant
    exampleLicmGraphCase ρ
    exampleLicmGraphCase_graphWf
    exampleLicmGraphCase_selfBackedgePhi
    (by simpa using h)

theorem exampleLicmGraphCase_unsound_machine
    (entry locals : State)
    (h : locals "time" + 1 ≠ entry "time0") :
    (runOriginalMachine exampleLicmGraphCase.fnir true entry locals).result? ≠
      (runHoistedMachine exampleLicmGraphCase.fnir true entry locals).result? := by
  simpa [exampleLicmGraphCase] using smallStepPhiTimeUnsound entry locals h

end RRProofs
