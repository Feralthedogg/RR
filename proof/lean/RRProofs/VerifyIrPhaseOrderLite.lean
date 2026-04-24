import RRProofs.VerifyIrRustErrorLite

namespace RRProofs

structure VerifyIrPhaseOrderLiteCase where
  entryBlockValid : Bool
  bodyHeadBlockValid : Bool
  bodyHeadReachable : Bool
  entryNotUnreachable : Bool
  bodyHeadNotUnreachable : Bool
  valuePhaseError : Option VerifyErrorRustLite
  nonPhiCycleFree : Bool
  blockPhaseError : Option VerifyErrorRustLite
  entryHasNoPreds : Bool
  phiPhaseError : Option VerifyErrorRustLite
  flowPhaseError : Option VerifyErrorRustLite
  reachablePhi : Bool
deriving Repr

def VerifyIrPhaseOrderLiteCase.verifyIrPhaseOrderLite
    (c : VerifyIrPhaseOrderLiteCase) : Option VerifyErrorRustLite :=
  if !c.entryBlockValid then
    some .badBlock
  else if !c.bodyHeadBlockValid then
    some .badBlock
  else if !c.bodyHeadReachable then
    some .invalidBodyHead
  else if !c.entryNotUnreachable then
    some .invalidEntryTerminator
  else if !c.bodyHeadNotUnreachable then
    some .invalidBodyHeadTerminator
  else
    match c.valuePhaseError with
    | some err => some err
    | none =>
        if !c.nonPhiCycleFree then
          some .nonPhiValueCycle
        else
          match c.blockPhaseError with
          | some err => some err
          | none =>
              if !c.entryHasNoPreds then
                some .invalidEntryPredecessor
              else
                match c.phiPhaseError with
                | some err => some err
                | none => c.flowPhaseError

def VerifyIrPhaseOrderLiteCase.verifyEmittablePhaseOrderLite
    (c : VerifyIrPhaseOrderLiteCase) : Option VerifyErrorRustLite :=
  match c.verifyIrPhaseOrderLite with
  | some err => some err
  | none =>
      if c.reachablePhi then
        some .reachablePhi
      else
        none

def exampleEntryDominatesValuePhase : VerifyIrPhaseOrderLiteCase :=
  { entryBlockValid := false
  , bodyHeadBlockValid := true
  , bodyHeadReachable := true
  , entryNotUnreachable := true
  , bodyHeadNotUnreachable := true
  , valuePhaseError := some .invalidParamIndex
  , nonPhiCycleFree := true
  , blockPhaseError := none
  , entryHasNoPreds := true
  , phiPhaseError := none
  , flowPhaseError := none
  , reachablePhi := false
  }

def exampleValuePhaseDominatesBlockPhase : VerifyIrPhaseOrderLiteCase :=
  { entryBlockValid := true
  , bodyHeadBlockValid := true
  , bodyHeadReachable := true
  , entryNotUnreachable := true
  , bodyHeadNotUnreachable := true
  , valuePhaseError := some .invalidIntrinsicArity
  , nonPhiCycleFree := true
  , blockPhaseError := some .badBlock
  , entryHasNoPreds := true
  , phiPhaseError := none
  , flowPhaseError := none
  , reachablePhi := false
  }

def exampleBlockPhaseDominatesPhiPhase : VerifyIrPhaseOrderLiteCase :=
  { entryBlockValid := true
  , bodyHeadBlockValid := true
  , bodyHeadReachable := true
  , entryNotUnreachable := true
  , bodyHeadNotUnreachable := true
  , valuePhaseError := none
  , nonPhiCycleFree := true
  , blockPhaseError := some .badBlock
  , entryHasNoPreds := true
  , phiPhaseError := some .invalidPhiArgs
  , flowPhaseError := none
  , reachablePhi := false
  }

def examplePhiPhaseDominatesFlowPhase : VerifyIrPhaseOrderLiteCase :=
  { entryBlockValid := true
  , bodyHeadBlockValid := true
  , bodyHeadReachable := true
  , entryNotUnreachable := true
  , bodyHeadNotUnreachable := true
  , valuePhaseError := none
  , nonPhiCycleFree := true
  , blockPhaseError := none
  , entryHasNoPreds := true
  , phiPhaseError := some .invalidPhiEdgeValue
  , flowPhaseError := some .useBeforeDef
  , reachablePhi := false
  }

def exampleFlowPhaseDominatesReachablePhi : VerifyIrPhaseOrderLiteCase :=
  { entryBlockValid := true
  , bodyHeadBlockValid := true
  , bodyHeadReachable := true
  , entryNotUnreachable := true
  , bodyHeadNotUnreachable := true
  , valuePhaseError := none
  , nonPhiCycleFree := true
  , blockPhaseError := none
  , entryHasNoPreds := true
  , phiPhaseError := none
  , flowPhaseError := some .undefinedVar
  , reachablePhi := true
  }

def examplePhaseOrderClean : VerifyIrPhaseOrderLiteCase :=
  { entryBlockValid := true
  , bodyHeadBlockValid := true
  , bodyHeadReachable := true
  , entryNotUnreachable := true
  , bodyHeadNotUnreachable := true
  , valuePhaseError := none
  , nonPhiCycleFree := true
  , blockPhaseError := none
  , entryHasNoPreds := true
  , phiPhaseError := none
  , flowPhaseError := none
  , reachablePhi := false
  }

theorem exampleEntryDominatesValuePhase_rejects :
    exampleEntryDominatesValuePhase.verifyIrPhaseOrderLite = some .badBlock := by
  simp [exampleEntryDominatesValuePhase, VerifyIrPhaseOrderLiteCase.verifyIrPhaseOrderLite]

theorem exampleValuePhaseDominatesBlockPhase_rejects :
    exampleValuePhaseDominatesBlockPhase.verifyIrPhaseOrderLite = some .invalidIntrinsicArity := by
  simp [exampleValuePhaseDominatesBlockPhase, VerifyIrPhaseOrderLiteCase.verifyIrPhaseOrderLite]

theorem exampleBlockPhaseDominatesPhiPhase_rejects :
    exampleBlockPhaseDominatesPhiPhase.verifyIrPhaseOrderLite = some .badBlock := by
  simp [exampleBlockPhaseDominatesPhiPhase, VerifyIrPhaseOrderLiteCase.verifyIrPhaseOrderLite]

theorem examplePhiPhaseDominatesFlowPhase_rejects :
    examplePhiPhaseDominatesFlowPhase.verifyIrPhaseOrderLite = some .invalidPhiEdgeValue := by
  simp [examplePhiPhaseDominatesFlowPhase, VerifyIrPhaseOrderLiteCase.verifyIrPhaseOrderLite]

theorem exampleFlowPhaseDominatesReachablePhi_rejects :
    exampleFlowPhaseDominatesReachablePhi.verifyEmittablePhaseOrderLite = some .undefinedVar := by
  simp [exampleFlowPhaseDominatesReachablePhi,
    VerifyIrPhaseOrderLiteCase.verifyEmittablePhaseOrderLite,
    VerifyIrPhaseOrderLiteCase.verifyIrPhaseOrderLite]

theorem examplePhaseOrderClean_accepts :
    examplePhaseOrderClean.verifyEmittablePhaseOrderLite = none := by
  simp [examplePhaseOrderClean, VerifyIrPhaseOrderLiteCase.verifyEmittablePhaseOrderLite,
    VerifyIrPhaseOrderLiteCase.verifyIrPhaseOrderLite]

end RRProofs
