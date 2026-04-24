import RRProofs.VerifyIrRustErrorLite

namespace RRProofs

structure VerifyIrCheckOrderLiteCase where
  entryBlockError : Option VerifyErrorRustLite
  bodyHeadBlockError : Option VerifyErrorRustLite
  bodyHeadReachabilityError : Option VerifyErrorRustLite
  entryTerminatorError : Option VerifyErrorRustLite
  bodyHeadTerminatorError : Option VerifyErrorRustLite
  valueIdError : Option VerifyErrorRustLite
  nonPhiOwnerError : Option VerifyErrorRustLite
  selfReferenceError : Option VerifyErrorRustLite
  paramIndexError : Option VerifyErrorRustLite
  operandError : Option VerifyErrorRustLite
  callArgNamesError : Option VerifyErrorRustLite
  intrinsicArityError : Option VerifyErrorRustLite
  nonPhiCycleError : Option VerifyErrorRustLite
  blockIdError : Option VerifyErrorRustLite
  blockTargetError : Option VerifyErrorRustLite
  entryPredError : Option VerifyErrorRustLite
  phiShapeError : Option VerifyErrorRustLite
  phiEdgeError : Option VerifyErrorRustLite
  flowError : Option VerifyErrorRustLite
  badTerminatorError : Option VerifyErrorRustLite
  undefinedVarError : Option VerifyErrorRustLite
  reachablePhiError : Option VerifyErrorRustLite
deriving Repr

def VerifyIrCheckOrderLiteCase.verifyIrCheckOrderLite
    (c : VerifyIrCheckOrderLiteCase) : Option VerifyErrorRustLite :=
  match c.entryBlockError with
  | some err => some err
  | none =>
      match c.bodyHeadBlockError with
      | some err => some err
      | none =>
          match c.bodyHeadReachabilityError with
          | some err => some err
          | none =>
              match c.entryTerminatorError with
              | some err => some err
              | none =>
                  match c.bodyHeadTerminatorError with
                  | some err => some err
                  | none =>
                      match c.valueIdError with
                      | some err => some err
                      | none =>
                          match c.nonPhiOwnerError with
                          | some err => some err
                          | none =>
                              match c.selfReferenceError with
                              | some err => some err
                              | none =>
                                  match c.paramIndexError with
                                  | some err => some err
                                  | none =>
                                      match c.operandError with
                                      | some err => some err
                                      | none =>
                                          match c.callArgNamesError with
                                          | some err => some err
                                          | none =>
                                              match c.intrinsicArityError with
                                              | some err => some err
                                              | none =>
                                                  match c.nonPhiCycleError with
                                                  | some err => some err
                                                  | none =>
                                                      match c.blockIdError with
                                                      | some err => some err
                                                      | none =>
                                                          match c.blockTargetError with
                                                          | some err => some err
                                                          | none =>
                                                              match c.entryPredError with
                                                              | some err => some err
                                                              | none =>
                                                                  match c.phiShapeError with
                                                                  | some err => some err
                                                                  | none =>
                                                                      match c.phiEdgeError with
                                                                      | some err => some err
                                                                      | none =>
                                                                          match c.flowError with
                                                                          | some err => some err
                                                                          | none =>
                                                                              match c.badTerminatorError with
                                                                              | some err => some err
                                                                              | none => c.undefinedVarError

def VerifyIrCheckOrderLiteCase.verifyEmittableCheckOrderLite
    (c : VerifyIrCheckOrderLiteCase) : Option VerifyErrorRustLite :=
  match c.verifyIrCheckOrderLite with
  | some err => some err
  | none => c.reachablePhiError

def exampleEntryCheckDominates : VerifyIrCheckOrderLiteCase :=
  { entryBlockError := some .badBlock
  , bodyHeadBlockError := some .badBlock
  , bodyHeadReachabilityError := some .invalidBodyHead
  , entryTerminatorError := some .invalidEntryTerminator
  , bodyHeadTerminatorError := some .invalidBodyHeadTerminator
  , valueIdError := some .badValue
  , nonPhiOwnerError := some .invalidPhiOwner
  , selfReferenceError := some .selfReferentialValue
  , paramIndexError := some .invalidParamIndex
  , operandError := some .badValue
  , callArgNamesError := some .invalidCallArgNames
  , intrinsicArityError := some .invalidIntrinsicArity
  , nonPhiCycleError := some .nonPhiValueCycle
  , blockIdError := some .badBlock
  , blockTargetError := some .badBlock
  , entryPredError := some .invalidEntryPredecessor
  , phiShapeError := some .invalidPhiArgs
  , phiEdgeError := some .invalidPhiEdgeValue
  , flowError := some .useBeforeDef
  , badTerminatorError := some .badTerminator
  , undefinedVarError := some .undefinedVar
  , reachablePhiError := some .reachablePhi
  }

def exampleValueCheckDominatesLater : VerifyIrCheckOrderLiteCase :=
  { entryBlockError := none
  , bodyHeadBlockError := none
  , bodyHeadReachabilityError := none
  , entryTerminatorError := none
  , bodyHeadTerminatorError := none
  , valueIdError := none
  , nonPhiOwnerError := none
  , selfReferenceError := none
  , paramIndexError := some .invalidParamIndex
  , operandError := some .badValue
  , callArgNamesError := some .invalidCallArgNames
  , intrinsicArityError := some .invalidIntrinsicArity
  , nonPhiCycleError := some .nonPhiValueCycle
  , blockIdError := some .badBlock
  , blockTargetError := some .badBlock
  , entryPredError := some .invalidEntryPredecessor
  , phiShapeError := some .invalidPhiArgs
  , phiEdgeError := some .invalidPhiEdgeValue
  , flowError := some .useBeforeDef
  , badTerminatorError := some .badTerminator
  , undefinedVarError := some .undefinedVar
  , reachablePhiError := some .reachablePhi
  }

def examplePhiCheckDominatesFlow : VerifyIrCheckOrderLiteCase :=
  { entryBlockError := none
  , bodyHeadBlockError := none
  , bodyHeadReachabilityError := none
  , entryTerminatorError := none
  , bodyHeadTerminatorError := none
  , valueIdError := none
  , nonPhiOwnerError := none
  , selfReferenceError := none
  , paramIndexError := none
  , operandError := none
  , callArgNamesError := none
  , intrinsicArityError := none
  , nonPhiCycleError := none
  , blockIdError := none
  , blockTargetError := none
  , entryPredError := none
  , phiShapeError := some .invalidPhiArgs
  , phiEdgeError := some .invalidPhiEdgeValue
  , flowError := some .useBeforeDef
  , badTerminatorError := some .badTerminator
  , undefinedVarError := some .undefinedVar
  , reachablePhiError := some .reachablePhi
  }

def exampleUndefinedVarDominatesReachablePhi : VerifyIrCheckOrderLiteCase :=
  { entryBlockError := none
  , bodyHeadBlockError := none
  , bodyHeadReachabilityError := none
  , entryTerminatorError := none
  , bodyHeadTerminatorError := none
  , valueIdError := none
  , nonPhiOwnerError := none
  , selfReferenceError := none
  , paramIndexError := none
  , operandError := none
  , callArgNamesError := none
  , intrinsicArityError := none
  , nonPhiCycleError := none
  , blockIdError := none
  , blockTargetError := none
  , entryPredError := none
  , phiShapeError := none
  , phiEdgeError := none
  , flowError := none
  , badTerminatorError := none
  , undefinedVarError := some .undefinedVar
  , reachablePhiError := some .reachablePhi
  }

def exampleCheckOrderClean : VerifyIrCheckOrderLiteCase :=
  { entryBlockError := none
  , bodyHeadBlockError := none
  , bodyHeadReachabilityError := none
  , entryTerminatorError := none
  , bodyHeadTerminatorError := none
  , valueIdError := none
  , nonPhiOwnerError := none
  , selfReferenceError := none
  , paramIndexError := none
  , operandError := none
  , callArgNamesError := none
  , intrinsicArityError := none
  , nonPhiCycleError := none
  , blockIdError := none
  , blockTargetError := none
  , entryPredError := none
  , phiShapeError := none
  , phiEdgeError := none
  , flowError := none
  , badTerminatorError := none
  , undefinedVarError := none
  , reachablePhiError := none
  }

theorem exampleEntryCheckDominates_rejects :
    exampleEntryCheckDominates.verifyIrCheckOrderLite = some .badBlock := by
  simp [exampleEntryCheckDominates, VerifyIrCheckOrderLiteCase.verifyIrCheckOrderLite]

theorem exampleValueCheckDominatesLater_rejects :
    exampleValueCheckDominatesLater.verifyIrCheckOrderLite = some .invalidParamIndex := by
  simp [exampleValueCheckDominatesLater, VerifyIrCheckOrderLiteCase.verifyIrCheckOrderLite]

theorem examplePhiCheckDominatesFlow_rejects :
    examplePhiCheckDominatesFlow.verifyIrCheckOrderLite = some .invalidPhiArgs := by
  simp [examplePhiCheckDominatesFlow, VerifyIrCheckOrderLiteCase.verifyIrCheckOrderLite]

theorem exampleUndefinedVarDominatesReachablePhi_rejects :
    exampleUndefinedVarDominatesReachablePhi.verifyEmittableCheckOrderLite = some .undefinedVar := by
  simp [exampleUndefinedVarDominatesReachablePhi,
    VerifyIrCheckOrderLiteCase.verifyEmittableCheckOrderLite,
    VerifyIrCheckOrderLiteCase.verifyIrCheckOrderLite]

theorem exampleCheckOrderClean_accepts :
    exampleCheckOrderClean.verifyEmittableCheckOrderLite = none := by
  simp [exampleCheckOrderClean, VerifyIrCheckOrderLiteCase.verifyEmittableCheckOrderLite,
    VerifyIrCheckOrderLiteCase.verifyIrCheckOrderLite]

end RRProofs
