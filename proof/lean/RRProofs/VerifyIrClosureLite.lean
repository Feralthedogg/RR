import RRProofs.VerifyIrFlowLite

set_option linter.unusedSimpArgs false

namespace RRProofs

inductive WrapperKind where
  | leaf
  | phi
  | intrinsic
  | record
  | fieldGet
  | fieldSet
deriving DecidableEq, Repr

structure WrapperNode where
  kind : WrapperKind
  deps : List Nat
deriving Repr

abbrev WrapperGraph := List WrapperNode

def lookupNode : WrapperGraph -> Nat -> Option WrapperNode
  | [], _ => none
  | node :: _, 0 => some node
  | _ :: rest, idx + 1 => lookupNode rest idx

inductive UsedFrom (g : WrapperGraph) : Nat -> Nat -> Prop where
  | self (root : Nat) : UsedFrom g root root
  | step (root src to : Nat) (node : WrapperNode) :
      UsedFrom g root src ->
      lookupNode g src = some node ->
      to ∈ node.deps ->
      UsedFrom g root to

def nodeIsPhi (g : WrapperGraph) (idx : Nat) : Prop :=
  match lookupNode g idx with
  | some node => node.kind = WrapperKind.phi
  | none => False

def intrinsicPhiGraph : WrapperGraph :=
  [ { kind := .intrinsic, deps := [1] }
  , { kind := .phi, deps := [] }
  ]

def recordPhiGraph : WrapperGraph :=
  [ { kind := .record, deps := [1] }
  , { kind := .phi, deps := [] }
  ]

def recordIntrinsicPhiGraph : WrapperGraph :=
  [ { kind := .record, deps := [1] }
  , { kind := .intrinsic, deps := [2] }
  , { kind := .phi, deps := [] }
  ]

def fieldGetRecordPhiGraph : WrapperGraph :=
  [ { kind := .fieldGet, deps := [1] }
  , { kind := .record, deps := [2] }
  , { kind := .phi, deps := [] }
  ]

def nestedFieldGetRecordPhiGraph : WrapperGraph :=
  [ { kind := .fieldGet, deps := [1] }
  , { kind := .fieldGet, deps := [2] }
  , { kind := .record, deps := [3] }
  , { kind := .phi, deps := [] }
  ]

theorem intrinsicPhiGraph_reaches_nested_phi :
    UsedFrom intrinsicPhiGraph 0 1 := by
  apply UsedFrom.step 0 0 1 { kind := .intrinsic, deps := [1] }
  · exact UsedFrom.self 0
  · rfl
  · simp

theorem recordPhiGraph_reaches_nested_phi :
    UsedFrom recordPhiGraph 0 1 := by
  apply UsedFrom.step 0 0 1 { kind := .record, deps := [1] }
  · exact UsedFrom.self 0
  · rfl
  · simp

theorem intrinsicPhiGraph_nested_target_is_phi :
    nodeIsPhi intrinsicPhiGraph 1 := by
  simp [nodeIsPhi, lookupNode, intrinsicPhiGraph]

theorem recordPhiGraph_nested_target_is_phi :
    nodeIsPhi recordPhiGraph 1 := by
  simp [nodeIsPhi, lookupNode, recordPhiGraph]

theorem recordIntrinsicPhiGraph_reaches_nested_phi :
    UsedFrom recordIntrinsicPhiGraph 0 2 := by
  apply UsedFrom.step 0 1 2 { kind := .intrinsic, deps := [2] }
  · apply UsedFrom.step 0 0 1 { kind := .record, deps := [1] }
    · exact UsedFrom.self 0
    · rfl
    · simp
  · rfl
  · simp

theorem recordIntrinsicPhiGraph_nested_target_is_phi :
    nodeIsPhi recordIntrinsicPhiGraph 2 := by
  simp [nodeIsPhi, lookupNode, recordIntrinsicPhiGraph]

theorem fieldGetRecordPhiGraph_reaches_nested_phi :
    UsedFrom fieldGetRecordPhiGraph 0 2 := by
  apply UsedFrom.step 0 1 2 { kind := .record, deps := [2] }
  · apply UsedFrom.step 0 0 1 { kind := .fieldGet, deps := [1] }
    · exact UsedFrom.self 0
    · rfl
    · simp
  · rfl
  · simp

theorem fieldGetRecordPhiGraph_nested_target_is_phi :
    nodeIsPhi fieldGetRecordPhiGraph 2 := by
  simp [nodeIsPhi, lookupNode, fieldGetRecordPhiGraph]

theorem nestedFieldGetRecordPhiGraph_reaches_nested_phi :
    UsedFrom nestedFieldGetRecordPhiGraph 0 3 := by
  apply UsedFrom.step 0 2 3 { kind := .record, deps := [3] }
  · apply UsedFrom.step 0 1 2 { kind := .fieldGet, deps := [2] }
    · apply UsedFrom.step 0 0 1 { kind := .fieldGet, deps := [1] }
      · exact UsedFrom.self 0
      · rfl
      · simp
    · rfl
    · simp
  · rfl
  · simp

theorem nestedFieldGetRecordPhiGraph_nested_target_is_phi :
    nodeIsPhi nestedFieldGetRecordPhiGraph 3 := by
  simp [nodeIsPhi, lookupNode, nestedFieldGetRecordPhiGraph]

end RRProofs
