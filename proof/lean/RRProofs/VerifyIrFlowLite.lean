import RRProofs.VerifyIrStructLite

namespace RRProofs

inductive VerifyErrorFlowLite where
  | base : VerifyErrorStructLite -> VerifyErrorFlowLite
  | useBeforeDef : Var -> VerifyErrorFlowLite
deriving DecidableEq, Repr

structure FlowBlockCase where
  defined : List Var
  required : List Var
deriving Repr

def firstMissingVar (defined required : List Var) : Option Var :=
  required.find? (fun v => !(v ∈ defined))

def FlowBlockCase.verifyFlow (b : FlowBlockCase) : Option VerifyErrorFlowLite :=
  (firstMissingVar b.defined b.required).map VerifyErrorFlowLite.useBeforeDef

def verifyFlowBlocks : List FlowBlockCase -> Option VerifyErrorFlowLite
  | [] => none
  | b :: rest =>
      match b.verifyFlow with
      | some err => some err
      | none => verifyFlowBlocks rest

structure VerifyIrFlowLiteCase where
  base : VerifyIrStructLiteCase
  blocks : List FlowBlockCase

def VerifyIrFlowLiteCase.verifyIrFlowLite
    (c : VerifyIrFlowLiteCase) : Option VerifyErrorFlowLite :=
  match verifyFlowBlocks c.blocks with
  | some err => some err
  | none => c.base.verifyIrStructLite.map VerifyErrorFlowLite.base

theorem verifyIrFlowLite_none_implies_struct_clean
    (c : VerifyIrFlowLiteCase)
    (h : c.verifyIrFlowLite = none) :
    c.base.verifyIrStructLite = none := by
  unfold VerifyIrFlowLiteCase.verifyIrFlowLite at h
  cases hF : verifyFlowBlocks c.blocks with
  | some err =>
      simp [hF] at h
  | none =>
      simpa [hF] using h

theorem verifyIrFlowLite_ok_zero_trip_sound
    (c : VerifyIrFlowLiteCase)
    (hVerify : c.verifyIrFlowLite = none)
    (hWf : c.base.base.base.wf)
    (hSafe : c.base.base.base.licm.safeCandidate)
    (entry locals : State) :
    (runOriginalMachine c.base.base.base.licm.fnir false entry locals).result? =
      (runHoistedMachine c.base.base.base.licm.fnir false entry locals).result? := by
  exact verifyIrStructLite_ok_zero_trip_sound
    c.base
    (verifyIrFlowLite_none_implies_struct_clean c hVerify)
    hWf
    hSafe
    entry
    locals

theorem verifyIrFlowLite_ok_one_trip_sound
    (c : VerifyIrFlowLiteCase)
    (hVerify : c.verifyIrFlowLite = none)
    (hWf : c.base.base.base.wf)
    (hSafe : c.base.base.base.licm.safeCandidate)
    (entry locals : State) :
    (runOriginalMachine c.base.base.base.licm.fnir true entry locals).result? =
      (runHoistedMachine c.base.base.base.licm.fnir true entry locals).result? := by
  exact verifyIrStructLite_ok_one_trip_sound
    c.base
    (verifyIrFlowLite_none_implies_struct_clean c hVerify)
    hWf
    hSafe
    entry
    locals

def exampleFlowBase : VerifyIrStructLiteCase :=
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
  , values := [{ isPhi := false, phiBlock? := none, ownerBlockValid := true, ownerBlockHasPreds := true, ownerBlockHasDistinctPreds := true, phiArgsEdgeAvailable := true, paramIndexValid := true, callNamesValid := true, selfReferenceFree := true, nonPhiAcyclic := true }]
  }

def exampleUseBeforeDefCase : VerifyIrFlowLiteCase :=
  { base := exampleFlowBase
  , blocks := [{ defined := ["y"], required := ["x", "y"] }]
  }

def exampleFlowCleanCase : VerifyIrFlowLiteCase :=
  { base := exampleFlowBase
  , blocks :=
      [ { defined := ["x", "y"], required := ["x"] }
      , { defined := ["x", "y"], required := ["y"] }
      ]
  }

theorem exampleUseBeforeDefCase_rejects :
    exampleUseBeforeDefCase.verifyIrFlowLite = some (.useBeforeDef "x") := by
  simp [exampleUseBeforeDefCase, VerifyIrFlowLiteCase.verifyIrFlowLite, verifyFlowBlocks,
    FlowBlockCase.verifyFlow, firstMissingVar, exampleFlowBase, exampleCleanStructBase]

theorem exampleFlowCleanCase_accepts :
    exampleFlowCleanCase.verifyIrFlowLite = none := by
  simp [exampleFlowCleanCase, VerifyIrFlowLiteCase.verifyIrFlowLite, verifyFlowBlocks,
    FlowBlockCase.verifyFlow, firstMissingVar, exampleFlowBase, exampleCleanStructBase,
    VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, VerifyIrLiteCase.verifyIrLite]

end RRProofs
