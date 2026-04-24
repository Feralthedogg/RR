import RRProofs.VerifyIrExecutableLite

namespace RRProofs

inductive VerifyErrorRustLite where
  | badValue
  | badBlock
  | badOperand
  | badTerminator
  | useBeforeDef
  | invalidPhiArgs
  | invalidPhiSource
  | invalidPhiOwner
  | invalidPhiOwnerBlock
  | invalidParamIndex
  | invalidCallArgNames
  | selfReferentialValue
  | nonPhiValueCycle
  | invalidBodyHead
  | invalidBodyHeadEntryEdge
  | invalidEntryPrologue
  | invalidEntryPredecessor
  | invalidEntryTerminator
  | invalidBranchTargets
  | invalidLoopHeaderSplit
  | invalidLoopHeaderPredecessors
  | invalidBodyHeadTerminator
  | invalidPhiPlacement
  | invalidPhiPredecessorAliases
  | invalidPhiEdgeValue
  | undefinedVar
  | reachablePhi
  | invalidIntrinsicArity
deriving DecidableEq, Repr

def VerifyErrorLite.toRust (err : VerifyErrorLite) : VerifyErrorRustLite :=
  match err with
  | .undefinedVar _ => .undefinedVar
  | .invalidPhiSource => .invalidPhiSource
  | .reachablePhi => .reachablePhi

def VerifyErrorStructLite.toRust (err : VerifyErrorStructLite) : VerifyErrorRustLite :=
  match err with
  | .base err => err.toRust
  | .invalidBodyHead => .invalidBodyHead
  | .invalidBodyHeadEntryEdge => .invalidBodyHeadEntryEdge
  | .invalidEntryPrologue => .invalidEntryPrologue
  | .invalidBodyHeadTerminator => .invalidBodyHeadTerminator
  | .invalidEntryPredecessor => .invalidEntryPredecessor
  | .invalidEntryTerminator => .invalidEntryTerminator
  | .invalidBranchTargets => .invalidBranchTargets
  | .invalidLoopHeaderSplit => .invalidLoopHeaderSplit
  | .invalidLoopHeaderPredecessors => .invalidLoopHeaderPredecessors
  | .invalidPhiPlacement => .invalidPhiPlacement
  | .invalidPhiPredecessorAliases => .invalidPhiPredecessorAliases
  | .invalidPhiEdgeValue => .invalidPhiEdgeValue
  | .missingPhiBlock => .invalidPhiArgs
  | .nonPhiCarriesPhiBlock => .invalidPhiOwner
  | .invalidPhiOwnerBlock => .invalidPhiOwnerBlock
  | .invalidParamIndex => .invalidParamIndex
  | .invalidCallArgNames => .invalidCallArgNames
  | .selfReferentialValue => .selfReferentialValue
  | .nonPhiValueCycle => .nonPhiValueCycle

def VerifyErrorFlowLite.toRust (err : VerifyErrorFlowLite) : VerifyErrorRustLite :=
  match err with
  | .base err => err.toRust
  | .useBeforeDef _ => .useBeforeDef

def VerifyErrorExecutableLite.toRust (err : VerifyErrorExecutableLite) : VerifyErrorRustLite :=
  match err with
  | .badBlock => .badBlock
  | .badValue => .badValue
  | .invalidIntrinsicArity => .invalidIntrinsicArity
  | .badTerminator => .badTerminator
  | .base err => err.toRust
  | .reachablePhi => .reachablePhi

def VerifyIrExecutableLiteCase.verifyIrRustLite
    (c : VerifyIrExecutableLiteCase) : Option VerifyErrorRustLite :=
  c.verifyIrExecutableLite.map VerifyErrorExecutableLite.toRust

def VerifyIrExecutableLiteCase.verifyEmittableRustLite
    (c : VerifyIrExecutableLiteCase) : Option VerifyErrorRustLite :=
  c.verifyEmittableExecutableLite.map VerifyErrorExecutableLite.toRust

theorem exampleBadEntryCase_maps_to_badBlock :
    exampleBadEntryCase.verifyIrRustLite = some .badBlock := by
  native_decide

theorem exampleIntrinsicArityCase_maps_to_invalidIntrinsicArity :
    exampleIntrinsicArityCase.verifyIrRustLite = some .invalidIntrinsicArity := by
  native_decide

theorem exampleReachablePhiExecutableCase_maps_to_reachablePhi :
    exampleReachablePhiExecutableCase.verifyEmittableRustLite = some .reachablePhi := by
  native_decide

theorem exampleExecutableCleanCase_rust_accepts :
    exampleExecutableCleanCase.verifyEmittableRustLite = none := by
  native_decide

theorem exampleInvalidPhiEdgeValue_maps_to_rust_name :
    exampleInvalidPhiEdgeValueCase.verifyIrStructLite.map VerifyErrorStructLite.toRust =
      some VerifyErrorRustLite.invalidPhiEdgeValue := by
  native_decide

theorem exampleInvalidEntryPrologue_maps_to_rust_name :
    exampleInvalidEntryPrologueCase.verifyIrStructLite.map VerifyErrorStructLite.toRust =
      some VerifyErrorRustLite.invalidEntryPrologue := by
  native_decide

theorem exampleInvalidBranchTargets_maps_to_rust_name :
    exampleInvalidBranchTargetsCase.verifyIrStructLite.map VerifyErrorStructLite.toRust =
      some VerifyErrorRustLite.invalidBranchTargets := by
  native_decide

theorem exampleInvalidLoopHeaderSplit_maps_to_rust_name :
    exampleInvalidLoopHeaderSplitCase.verifyIrStructLite.map VerifyErrorStructLite.toRust =
      some VerifyErrorRustLite.invalidLoopHeaderSplit := by
  native_decide

theorem exampleInvalidLoopHeaderPreds_maps_to_rust_name :
    exampleInvalidLoopHeaderPredsCase.verifyIrStructLite.map VerifyErrorStructLite.toRust =
      some VerifyErrorRustLite.invalidLoopHeaderPredecessors := by
  native_decide

end RRProofs
