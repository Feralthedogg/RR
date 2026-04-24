import RRProofs.VerifyLite

set_option linter.unusedSimpArgs false
set_option linter.unnecessarySimpa false

namespace RRProofs

inductive VerifyErrorLite where
  | undefinedVar : Var -> VerifyErrorLite
  | invalidPhiSource : VerifyErrorLite
  | reachablePhi : VerifyErrorLite
deriving DecidableEq, Repr

structure VerifyIrLiteCase where
  base : RRWfCase
  undefinedVar? : Option Var
  phiSourcesValid : Bool
  reachablePhi : Bool

def VerifyIrLiteCase.verifyIrLite (c : VerifyIrLiteCase) : Option VerifyErrorLite :=
  match c.undefinedVar? with
  | some x => some (.undefinedVar x)
  | none =>
      if !c.phiSourcesValid then
        some .invalidPhiSource
      else if c.reachablePhi then
        some .reachablePhi
      else
        none

theorem verifyIrLite_none_implies_clean
    (c : VerifyIrLiteCase)
    (h : c.verifyIrLite = none) :
    c.undefinedVar? = none ∧ c.phiSourcesValid = true ∧ c.reachablePhi = false := by
  cases hU : c.undefinedVar? with
  | some x =>
      simp [VerifyIrLiteCase.verifyIrLite, hU] at h
  | none =>
      constructor
      · simp [hU]
      · by_cases hPhi : c.phiSourcesValid
        · constructor
          · exact hPhi
          · cases hReach : c.reachablePhi with
            | true =>
                simp [VerifyIrLiteCase.verifyIrLite, hU, hPhi, hReach] at h
            | false =>
                simp [hReach]
        · simp [VerifyIrLiteCase.verifyIrLite, hU, hPhi] at h

theorem verifyIrLite_ok_zero_trip_sound
    (c : VerifyIrLiteCase)
    (_hVerify : c.verifyIrLite = none)
    (hWf : c.base.wf)
    (hSafe : c.base.licm.safeCandidate)
    (entry locals : State) :
    (runOriginalMachine c.base.licm.fnir false entry locals).result? =
      (runHoistedMachine c.base.licm.fnir false entry locals).result? := by
  exact rrwf_safe_candidate_zero_trip c.base entry locals hWf hSafe

theorem verifyIrLite_ok_one_trip_sound
    (c : VerifyIrLiteCase)
    (_hVerify : c.verifyIrLite = none)
    (hWf : c.base.wf)
    (hSafe : c.base.licm.safeCandidate)
    (entry locals : State) :
    (runOriginalMachine c.base.licm.fnir true entry locals).result? =
      (runHoistedMachine c.base.licm.fnir true entry locals).result? := by
  exact rrwf_safe_candidate_one_trip c.base entry locals hWf hSafe

def exampleReachablePhiCase : VerifyIrLiteCase :=
  { base := exampleRRWfCase
  , undefinedVar? := none
  , phiSourcesValid := true
  , reachablePhi := true
  }

def exampleInvalidPhiSourceCase : VerifyIrLiteCase :=
  { base := exampleRRWfCase
  , undefinedVar? := none
  , phiSourcesValid := false
  , reachablePhi := false
  }

def exampleUndefinedVarCase : VerifyIrLiteCase :=
  { base := exampleRRWfCase
  , undefinedVar? := some "time"
  , phiSourcesValid := true
  , reachablePhi := false
  }

theorem exampleReachablePhiCase_rejects :
    exampleReachablePhiCase.verifyIrLite = some .reachablePhi := by
  simp [exampleReachablePhiCase, VerifyIrLiteCase.verifyIrLite]

theorem exampleInvalidPhiSourceCase_rejects :
    exampleInvalidPhiSourceCase.verifyIrLite = some .invalidPhiSource := by
  simp [exampleInvalidPhiSourceCase, VerifyIrLiteCase.verifyIrLite]

theorem exampleUndefinedVarCase_rejects :
    exampleUndefinedVarCase.verifyIrLite = some (.undefinedVar "time") := by
  simp [exampleUndefinedVarCase, VerifyIrLiteCase.verifyIrLite]

end RRProofs
