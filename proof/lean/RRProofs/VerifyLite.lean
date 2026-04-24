import RRProofs.RRWellFormed

namespace RRProofs

structure VerifyLiteCase where
  rr : RRWfCase

def VerifyLiteCase.verifyLite (c : VerifyLiteCase) : Prop :=
  c.rr.wf ∧
  (∀ entry locals, c.rr.licm.safeCandidate ->
      (runOriginalMachine c.rr.licm.fnir false entry locals).result? =
        (runHoistedMachine c.rr.licm.fnir false entry locals).result?) ∧
  (∀ entry locals, c.rr.licm.safeCandidate ->
      (runOriginalMachine c.rr.licm.fnir true entry locals).result? =
        (runHoistedMachine c.rr.licm.fnir true entry locals).result?) ∧
  (∀ ρ, c.rr.licm.selfBackedgePhi ->
      ρ c.rr.licm.phi.entryVal ≠ ρ c.rr.licm.phi.self ->
      ¬ c.rr.licm.phi.predInvariant c.rr.licm.graph c.rr.licm.shape ρ)

theorem verifyLite_zero_trip_sound
    (c : VerifyLiteCase)
    (h : c.verifyLite)
    (entry locals : State)
    (hSafe : c.rr.licm.safeCandidate) :
    (runOriginalMachine c.rr.licm.fnir false entry locals).result? =
      (runHoistedMachine c.rr.licm.fnir false entry locals).result? := by
  exact h.2.1 entry locals hSafe

theorem verifyLite_one_trip_sound
    (c : VerifyLiteCase)
    (h : c.verifyLite)
    (entry locals : State)
    (hSafe : c.rr.licm.safeCandidate) :
    (runOriginalMachine c.rr.licm.fnir true entry locals).result? =
      (runHoistedMachine c.rr.licm.fnir true entry locals).result? := by
  exact h.2.2.1 entry locals hSafe

theorem verifyLite_rejects_self_backedge_phi
    (c : VerifyLiteCase)
    (h : c.verifyLite)
    (ρ : GValueEnv)
    (hBack : c.rr.licm.selfBackedgePhi)
    (hVals : ρ c.rr.licm.phi.entryVal ≠ ρ c.rr.licm.phi.self) :
    ¬ c.rr.licm.phi.predInvariant c.rr.licm.graph c.rr.licm.shape ρ := by
  exact h.2.2.2 ρ hBack hVals

def exampleVerifyLiteCase : VerifyLiteCase :=
  { rr := exampleRRWfCase }

theorem exampleVerifyLiteCase_holds : exampleVerifyLiteCase.verifyLite := by
  constructor
  · exact exampleRRWfCase_wf
  constructor
  · intro entry locals hSafe
    exact rrwf_safe_candidate_zero_trip exampleRRWfCase entry locals exampleRRWfCase_wf hSafe
  constructor
  · intro entry locals hSafe
    exact rrwf_safe_candidate_one_trip exampleRRWfCase entry locals exampleRRWfCase_wf hSafe
  · intro ρ hBack hVals
    exact rrwf_self_backedge_phi_not_invariant exampleRRWfCase ρ exampleRRWfCase_wf hBack hVals

theorem exampleVerifyLiteCase_unsound_if_phi_forced
    (entry locals : State)
    (h : locals "time" + 1 ≠ entry "time0") :
    (runOriginalMachine exampleVerifyLiteCase.rr.licm.fnir true entry locals).result? ≠
      (runHoistedMachine exampleVerifyLiteCase.rr.licm.fnir true entry locals).result? := by
  simpa [exampleVerifyLiteCase] using exampleRRWfCase_unsound entry locals h

end RRProofs
