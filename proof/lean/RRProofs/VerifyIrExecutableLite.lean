import RRProofs.VerifyIrFlowLite

set_option linter.unusedSimpArgs false

namespace RRProofs

inductive VerifyErrorExecutableLite where
  | badBlock : VerifyErrorExecutableLite
  | badValue : VerifyErrorExecutableLite
  | invalidIntrinsicArity : VerifyErrorExecutableLite
  | badTerminator : VerifyErrorExecutableLite
  | base : VerifyErrorFlowLite -> VerifyErrorExecutableLite
  | reachablePhi : VerifyErrorExecutableLite
deriving DecidableEq, Repr

structure VerifyIrExecutableLiteCase where
  base : VerifyIrFlowLiteCase
  entryBlockValid : Bool
  bodyHeadBlockValid : Bool
  valueIdsValid : Bool
  intrinsicAritiesValid : Bool
  blockIdsValid : Bool
  blockTargetsValid : Bool
  badTerminatorFree : Bool
  emittableReachablePhi : Bool

def VerifyIrExecutableLiteCase.verifyIrExecutableLite
    (c : VerifyIrExecutableLiteCase) : Option VerifyErrorExecutableLite :=
  if !c.entryBlockValid then
    some .badBlock
  else if !c.bodyHeadBlockValid then
    some .badBlock
  else if !c.valueIdsValid then
    some .badValue
  else if !c.intrinsicAritiesValid then
    some .invalidIntrinsicArity
  else if !c.blockIdsValid then
    some .badBlock
  else if !c.blockTargetsValid then
    some .badBlock
  else if !c.badTerminatorFree then
    some .badTerminator
  else
    c.base.verifyIrFlowLite.map VerifyErrorExecutableLite.base

def VerifyIrExecutableLiteCase.verifyEmittableExecutableLite
    (c : VerifyIrExecutableLiteCase) : Option VerifyErrorExecutableLite :=
  match c.verifyIrExecutableLite with
  | some err => some err
  | none =>
      if c.emittableReachablePhi then
        some .reachablePhi
      else
        none

theorem verifyIrExecutableLite_none_implies_flow_clean
    (c : VerifyIrExecutableLiteCase)
    (h : c.verifyIrExecutableLite = none) :
    c.base.verifyIrFlowLite = none := by
  unfold VerifyIrExecutableLiteCase.verifyIrExecutableLite at h
  cases hEntry : c.entryBlockValid <;> simp [hEntry] at h
  cases hBody : c.bodyHeadBlockValid <;> simp [hEntry, hBody] at h
  cases hVals : c.valueIdsValid <;> simp [hEntry, hBody, hVals] at h
  cases hIntr : c.intrinsicAritiesValid <;> simp [hEntry, hBody, hVals, hIntr] at h
  cases hBlocks : c.blockIdsValid <;> simp [hEntry, hBody, hVals, hIntr, hBlocks] at h
  cases hTargets : c.blockTargetsValid <;> simp [hEntry, hBody, hVals, hIntr, hBlocks, hTargets] at h
  cases hTerm : c.badTerminatorFree <;> simp [hEntry, hBody, hVals, hIntr, hBlocks, hTargets, hTerm] at h
  exact h

theorem verifyIrExecutableLite_ok_zero_trip_sound
    (c : VerifyIrExecutableLiteCase)
    (hVerify : c.verifyIrExecutableLite = none)
    (hWf : c.base.base.base.base.wf)
    (hSafe : c.base.base.base.base.licm.safeCandidate)
    (entry locals : State) :
    (runOriginalMachine c.base.base.base.base.licm.fnir false entry locals).result? =
      (runHoistedMachine c.base.base.base.base.licm.fnir false entry locals).result? := by
  exact verifyIrFlowLite_ok_zero_trip_sound
    c.base
    (verifyIrExecutableLite_none_implies_flow_clean c hVerify)
    hWf
    hSafe
    entry
    locals

theorem verifyIrExecutableLite_ok_one_trip_sound
    (c : VerifyIrExecutableLiteCase)
    (hVerify : c.verifyIrExecutableLite = none)
    (hWf : c.base.base.base.base.wf)
    (hSafe : c.base.base.base.base.licm.safeCandidate)
    (entry locals : State) :
    (runOriginalMachine c.base.base.base.base.licm.fnir true entry locals).result? =
      (runHoistedMachine c.base.base.base.base.licm.fnir true entry locals).result? := by
  exact verifyIrFlowLite_ok_one_trip_sound
    c.base
    (verifyIrExecutableLite_none_implies_flow_clean c hVerify)
    hWf
    hSafe
    entry
    locals

def exampleExecutableBase : VerifyIrFlowLiteCase :=
  { base := exampleFlowBase
  , blocks := []
  }

def exampleBadEntryCase : VerifyIrExecutableLiteCase :=
  { base := exampleExecutableBase
  , entryBlockValid := false
  , bodyHeadBlockValid := true
  , valueIdsValid := true
  , intrinsicAritiesValid := true
  , blockIdsValid := true
  , blockTargetsValid := true
  , badTerminatorFree := true
  , emittableReachablePhi := false
  }

def exampleBadValueCase : VerifyIrExecutableLiteCase :=
  { base := exampleExecutableBase
  , entryBlockValid := true
  , bodyHeadBlockValid := true
  , valueIdsValid := false
  , intrinsicAritiesValid := true
  , blockIdsValid := true
  , blockTargetsValid := true
  , badTerminatorFree := true
  , emittableReachablePhi := false
  }

def exampleIntrinsicArityCase : VerifyIrExecutableLiteCase :=
  { base := exampleExecutableBase
  , entryBlockValid := true
  , bodyHeadBlockValid := true
  , valueIdsValid := true
  , intrinsicAritiesValid := false
  , blockIdsValid := true
  , blockTargetsValid := true
  , badTerminatorFree := true
  , emittableReachablePhi := false
  }

def exampleReachablePhiExecutableCase : VerifyIrExecutableLiteCase :=
  { base := exampleExecutableBase
  , entryBlockValid := true
  , bodyHeadBlockValid := true
  , valueIdsValid := true
  , intrinsicAritiesValid := true
  , blockIdsValid := true
  , blockTargetsValid := true
  , badTerminatorFree := true
  , emittableReachablePhi := true
  }

def exampleExecutableCleanCase : VerifyIrExecutableLiteCase :=
  { base := exampleExecutableBase
  , entryBlockValid := true
  , bodyHeadBlockValid := true
  , valueIdsValid := true
  , intrinsicAritiesValid := true
  , blockIdsValid := true
  , blockTargetsValid := true
  , badTerminatorFree := true
  , emittableReachablePhi := false
  }

theorem exampleBadEntryCase_rejects :
    exampleBadEntryCase.verifyIrExecutableLite = some .badBlock := by
  native_decide

theorem exampleBadValueCase_rejects :
    exampleBadValueCase.verifyIrExecutableLite = some .badValue := by
  native_decide

theorem exampleIntrinsicArityCase_rejects :
    exampleIntrinsicArityCase.verifyIrExecutableLite = some .invalidIntrinsicArity := by
  native_decide

theorem exampleReachablePhiExecutableCase_rejects :
    exampleReachablePhiExecutableCase.verifyEmittableExecutableLite = some .reachablePhi := by
  native_decide

theorem exampleExecutableCleanCase_accepts :
    exampleExecutableCleanCase.verifyEmittableExecutableLite = none := by
  native_decide

end RRProofs
