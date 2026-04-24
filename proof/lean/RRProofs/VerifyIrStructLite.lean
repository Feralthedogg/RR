import RRProofs.VerifyIrLite

set_option linter.unusedSimpArgs false

namespace RRProofs

inductive VerifyErrorStructLite where
  | base : VerifyErrorLite -> VerifyErrorStructLite
  | invalidBodyHead : VerifyErrorStructLite
  | invalidBodyHeadEntryEdge : VerifyErrorStructLite
  | invalidEntryPrologue : VerifyErrorStructLite
  | invalidBodyHeadTerminator : VerifyErrorStructLite
  | invalidEntryPredecessor : VerifyErrorStructLite
  | invalidEntryTerminator : VerifyErrorStructLite
  | invalidBranchTargets : VerifyErrorStructLite
  | invalidLoopHeaderSplit : VerifyErrorStructLite
  | invalidLoopHeaderPredecessors : VerifyErrorStructLite
  | invalidPhiPlacement : VerifyErrorStructLite
  | invalidPhiPredecessorAliases : VerifyErrorStructLite
  | invalidPhiEdgeValue : VerifyErrorStructLite
  | missingPhiBlock : VerifyErrorStructLite
  | nonPhiCarriesPhiBlock : VerifyErrorStructLite
  | invalidPhiOwnerBlock : VerifyErrorStructLite
  | invalidParamIndex : VerifyErrorStructLite
  | invalidCallArgNames : VerifyErrorStructLite
  | selfReferentialValue : VerifyErrorStructLite
  | nonPhiValueCycle : VerifyErrorStructLite
deriving DecidableEq, Repr

structure ValueStructCase where
  isPhi : Bool
  phiBlock? : Option BlockId
  ownerBlockValid : Bool
  ownerBlockHasPreds : Bool
  ownerBlockHasDistinctPreds : Bool
  phiArgsEdgeAvailable : Bool
  paramIndexValid : Bool
  callNamesValid : Bool
  selfReferenceFree : Bool
  nonPhiAcyclic : Bool
deriving Repr

def ValueStructCase.verifyStruct (v : ValueStructCase) : Option VerifyErrorStructLite :=
  if !v.paramIndexValid then
    some .invalidParamIndex
  else if !v.callNamesValid then
    some .invalidCallArgNames
  else if !v.selfReferenceFree then
    some .selfReferentialValue
  else if !v.nonPhiAcyclic then
    some .nonPhiValueCycle
  else
    match v.isPhi, v.phiBlock? with
  | true, none => some .missingPhiBlock
  | false, some _ => some .nonPhiCarriesPhiBlock
  | true, some _ =>
      if !v.ownerBlockValid then
        some .invalidPhiOwnerBlock
      else if !v.ownerBlockHasPreds then
        some .invalidPhiPlacement
      else if !v.ownerBlockHasDistinctPreds then
        some .invalidPhiPredecessorAliases
      else if !v.phiArgsEdgeAvailable then
        some .invalidPhiEdgeValue
      else
        none
  | false, none => none

def verifyValueStructs : List ValueStructCase -> Option VerifyErrorStructLite
  | [] => none
  | v :: rest =>
      match v.verifyStruct with
      | some err => some err
      | none => verifyValueStructs rest

structure VerifyIrStructLiteCase where
  base : VerifyIrLiteCase
  bodyHeadReachable : Bool
  bodyHeadDirectEntryEdge : Bool
  entryPrologueSafe : Bool
  bodyHeadNotUnreachable : Bool
  entryHasNoPreds : Bool
  entryNotUnreachable : Bool
  branchTargetsDistinct : Bool
  loopHeaderSplitValid : Bool
  loopHeaderPredsValid : Bool
  values : List ValueStructCase

def VerifyIrStructLiteCase.verifyIrStructLite
    (c : VerifyIrStructLiteCase) : Option VerifyErrorStructLite :=
  if !c.bodyHeadReachable then
    some .invalidBodyHead
  else if !c.bodyHeadDirectEntryEdge then
    some .invalidBodyHeadEntryEdge
  else if !c.entryPrologueSafe then
    some .invalidEntryPrologue
  else if !c.bodyHeadNotUnreachable then
    some .invalidBodyHeadTerminator
  else if !c.entryHasNoPreds then
    some .invalidEntryPredecessor
  else if !c.entryNotUnreachable then
    some .invalidEntryTerminator
  else if !c.branchTargetsDistinct then
    some .invalidBranchTargets
  else if !c.loopHeaderSplitValid then
    some .invalidLoopHeaderSplit
  else if !c.loopHeaderPredsValid then
    some .invalidLoopHeaderPredecessors
  else
    match verifyValueStructs c.values with
    | some err => some err
    | none => c.base.verifyIrLite.map VerifyErrorStructLite.base

theorem verifyIrStructLite_none_implies_base_clean
    (c : VerifyIrStructLiteCase)
    (h : c.verifyIrStructLite = none) :
    c.base.verifyIrLite = none := by
  unfold VerifyIrStructLiteCase.verifyIrStructLite at h
  cases hHead : c.bodyHeadReachable <;> simp [hHead] at h
  cases hEdge : c.bodyHeadDirectEntryEdge <;> simp [hHead, hEdge] at h
  cases hPro : c.entryPrologueSafe <;> simp [hHead, hEdge, hPro] at h
  cases hBody : c.bodyHeadNotUnreachable <;> simp [hHead, hEdge, hPro, hBody] at h
  cases hPred : c.entryHasNoPreds <;> simp [hHead, hEdge, hPro, hBody, hPred] at h
  cases hEntry : c.entryNotUnreachable <;> simp [hHead, hEdge, hPro, hBody, hPred, hEntry] at h
  cases hBranch : c.branchTargetsDistinct <;> simp [hHead, hEdge, hPro, hBody, hPred, hEntry, hBranch] at h
  cases hLoop : c.loopHeaderSplitValid <;> simp [hHead, hEdge, hPro, hBody, hPred, hEntry, hBranch, hLoop] at h
  cases hLoopPreds : c.loopHeaderPredsValid <;> simp [hHead, hEdge, hPro, hBody, hPred, hEntry, hBranch, hLoop, hLoopPreds] at h
  cases hVals : verifyValueStructs c.values <;> simp [hHead, hEdge, hPro, hBody, hPred, hEntry, hBranch, hLoop, hLoopPreds, hVals] at h
  exact h

theorem verifyIrStructLite_ok_zero_trip_sound
    (c : VerifyIrStructLiteCase)
    (hVerify : c.verifyIrStructLite = none)
    (hWf : c.base.base.wf)
    (hSafe : c.base.base.licm.safeCandidate)
    (entry locals : State) :
    (runOriginalMachine c.base.base.licm.fnir false entry locals).result? =
      (runHoistedMachine c.base.base.licm.fnir false entry locals).result? := by
  exact verifyIrLite_ok_zero_trip_sound
    c.base
    (verifyIrStructLite_none_implies_base_clean c hVerify)
    hWf
    hSafe
    entry
    locals

theorem verifyIrStructLite_ok_one_trip_sound
    (c : VerifyIrStructLiteCase)
    (hVerify : c.verifyIrStructLite = none)
    (hWf : c.base.base.wf)
    (hSafe : c.base.base.licm.safeCandidate)
    (entry locals : State) :
    (runOriginalMachine c.base.base.licm.fnir true entry locals).result? =
      (runHoistedMachine c.base.base.licm.fnir true entry locals).result? := by
  exact verifyIrLite_ok_one_trip_sound
    c.base
    (verifyIrStructLite_none_implies_base_clean c hVerify)
    hWf
    hSafe
    entry
    locals

def exampleCleanStructBase : VerifyIrLiteCase :=
  { base := exampleRRWfCase
  , undefinedVar? := none
  , phiSourcesValid := true
  , reachablePhi := false
  }

def exampleMissingPhiBlockCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := true, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleTaggedNonPhiCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := some .header, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidPhiOwnerBlockCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := true, phiBlock? := some .header, ownerBlockValid := false, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidParamIndexCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := false, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidCallArgNamesCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := false, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleSelfReferentialValueCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := false, nonPhiAcyclic := true }]
  }

def exampleNonPhiCycleCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := false }]
  }

def exampleInvalidBodyHeadCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := false
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidBodyHeadEntryEdgeCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := false
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidEntryPrologueCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := false
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidEntryPredCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := false
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidPhiPlacementCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := true, phiBlock? := some .header, ownerBlockValid := true, ownerBlockHasPreds := false, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidPhiPredecessorAliasesCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := true, phiBlock? := some .header, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := false, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidPhiEdgeValueCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := true, phiBlock? := some .header, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := false, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidEntryTerminatorCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := false
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidBranchTargetsCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := false
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidLoopHeaderSplitCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := false
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidLoopHeaderPredsCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := false
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleInvalidBodyHeadTerminatorCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := false
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleStructCleanCase : VerifyIrStructLiteCase :=
  { base := exampleCleanStructBase
  , bodyHeadReachable := true
  , bodyHeadDirectEntryEdge := true
  , entryPrologueSafe := true
  , bodyHeadNotUnreachable := true
  , entryHasNoPreds := true
  , entryNotUnreachable := true
  , branchTargetsDistinct := true
  , loopHeaderSplitValid := true
  , loopHeaderPredsValid := true
  , values :=
      [ { isPhi := true, phiBlock? := some .header, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }
      , { isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }
      ]
  }

theorem exampleMissingPhiBlockCase_rejects :
    exampleMissingPhiBlockCase.verifyIrStructLite = some .missingPhiBlock := by
  simp [exampleMissingPhiBlockCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleTaggedNonPhiCase_rejects :
    exampleTaggedNonPhiCase.verifyIrStructLite = some .nonPhiCarriesPhiBlock := by
  simp [exampleTaggedNonPhiCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidPhiOwnerBlockCase_rejects :
    exampleInvalidPhiOwnerBlockCase.verifyIrStructLite = some .invalidPhiOwnerBlock := by
  simp [exampleInvalidPhiOwnerBlockCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidParamIndexCase_rejects :
    exampleInvalidParamIndexCase.verifyIrStructLite = some .invalidParamIndex := by
  simp [exampleInvalidParamIndexCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidCallArgNamesCase_rejects :
    exampleInvalidCallArgNamesCase.verifyIrStructLite = some .invalidCallArgNames := by
  simp [exampleInvalidCallArgNamesCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleSelfReferentialValueCase_rejects :
    exampleSelfReferentialValueCase.verifyIrStructLite = some .selfReferentialValue := by
  simp [exampleSelfReferentialValueCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleNonPhiCycleCase_rejects :
    exampleNonPhiCycleCase.verifyIrStructLite = some .nonPhiValueCycle := by
  simp [exampleNonPhiCycleCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidBodyHeadCase_rejects :
    exampleInvalidBodyHeadCase.verifyIrStructLite = some .invalidBodyHead := by
  simp [exampleInvalidBodyHeadCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidBodyHeadEntryEdgeCase_rejects :
    exampleInvalidBodyHeadEntryEdgeCase.verifyIrStructLite = some .invalidBodyHeadEntryEdge := by
  simp [exampleInvalidBodyHeadEntryEdgeCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidEntryPrologueCase_rejects :
    exampleInvalidEntryPrologueCase.verifyIrStructLite = some .invalidEntryPrologue := by
  simp [exampleInvalidEntryPrologueCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidEntryPredCase_rejects :
    exampleInvalidEntryPredCase.verifyIrStructLite = some .invalidEntryPredecessor := by
  simp [exampleInvalidEntryPredCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidPhiPlacementCase_rejects :
    exampleInvalidPhiPlacementCase.verifyIrStructLite = some .invalidPhiPlacement := by
  simp [exampleInvalidPhiPlacementCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidPhiPredecessorAliasesCase_rejects :
    exampleInvalidPhiPredecessorAliasesCase.verifyIrStructLite = some .invalidPhiPredecessorAliases := by
  simp [exampleInvalidPhiPredecessorAliasesCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidPhiEdgeValueCase_rejects :
    exampleInvalidPhiEdgeValueCase.verifyIrStructLite = some .invalidPhiEdgeValue := by
  simp [exampleInvalidPhiEdgeValueCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidEntryTerminatorCase_rejects :
    exampleInvalidEntryTerminatorCase.verifyIrStructLite = some .invalidEntryTerminator := by
  simp [exampleInvalidEntryTerminatorCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidBranchTargetsCase_rejects :
    exampleInvalidBranchTargetsCase.verifyIrStructLite = some .invalidBranchTargets := by
  simp [exampleInvalidBranchTargetsCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidLoopHeaderSplitCase_rejects :
    exampleInvalidLoopHeaderSplitCase.verifyIrStructLite = some .invalidLoopHeaderSplit := by
  simp [exampleInvalidLoopHeaderSplitCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidLoopHeaderPredsCase_rejects :
    exampleInvalidLoopHeaderPredsCase.verifyIrStructLite = some .invalidLoopHeaderPredecessors := by
  simp [exampleInvalidLoopHeaderPredsCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleInvalidBodyHeadTerminatorCase_rejects :
    exampleInvalidBodyHeadTerminatorCase.verifyIrStructLite = some .invalidBodyHeadTerminator := by
  simp [exampleInvalidBodyHeadTerminatorCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase]

theorem exampleStructCleanCase_accepts :
    exampleStructCleanCase.verifyIrStructLite = none := by
  simp [exampleStructCleanCase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase, VerifyIrLiteCase.verifyIrLite]

end RRProofs
