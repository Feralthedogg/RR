import RRProofs.VerifyIrValueDepsWalkSubset

set_option linter.unusedVariables false

namespace RRProofs

structure TableValue where
  phiBlock : Option Nat
  depsKind : ValueDepsKind
deriving Repr, DecidableEq

abbrev ValueTable := List TableValue

def lookupTableValue (table : ValueTable) (root : ConsumerNodeId) : Option TableValue :=
  match table, root with
  | [], _ => none
  | node :: _, 0 => some node
  | _ :: rest, n + 1 => lookupTableValue rest n

def lookupValueDeps (table : ValueTable) (root : ConsumerNodeId) : Option (List ConsumerNodeId) :=
  match lookupTableValue table root with
  | some node => some (valueDeps node.depsKind)
  | none => none

mutual
  def dependsOnPhiInBlockExceptTableFuel :
      Nat -> ValueTable -> List ConsumerNodeId -> ConsumerNodeId -> Nat -> ConsumerNodeId -> Prop
    | 0, _, _, _, _, _ => False
    | fuel + 1, table, seen, root, phiBlock, exempt =>
        if root ∈ seen then
          False
        else
          match lookupTableValue table root with
          | none => False
          | some node =>
              ((root ≠ exempt) ∧ node.phiBlock = some phiBlock) ∨
                depListDependsOnPhiTableFuel fuel table (root :: seen) (valueDeps node.depsKind)
                  phiBlock exempt

  def depListDependsOnPhiTableFuel :
      Nat -> ValueTable -> List ConsumerNodeId -> List ConsumerNodeId -> Nat -> ConsumerNodeId -> Prop
    | 0, _, _, _, _, _ => False
    | fuel + 1, table, seen, [], _, _ => False
    | fuel + 1, table, seen, root :: rest, phiBlock, exempt =>
        dependsOnPhiInBlockExceptTableFuel fuel table seen root phiBlock exempt ∨
          depListDependsOnPhiTableFuel fuel table seen rest phiBlock exempt
end

theorem lookupValueDeps_some
    (table : ValueTable) (root : ConsumerNodeId) (node : TableValue)
    (h : lookupTableValue table root = some node) :
    lookupValueDeps table root = some (valueDeps node.depsKind) := by
  simp [lookupValueDeps, h]

theorem lookupValueDeps_none
    (table : ValueTable) (root : ConsumerNodeId)
    (h : lookupTableValue table root = none) :
    lookupValueDeps table root = none := by
  simp [lookupValueDeps, h]

theorem dependsOnPhiInBlockExceptTableFuel_here
    (fuel : Nat) (table : ValueTable) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock exempt : Nat) (node : TableValue)
    (hLookup : lookupTableValue table root = some node)
    (hPhi : node.phiBlock = some phiBlock)
    (hFresh : root ∉ seen)
    (hNe : root ≠ exempt) :
    dependsOnPhiInBlockExceptTableFuel (fuel + 1) table seen root phiBlock exempt := by
  simp [dependsOnPhiInBlockExceptTableFuel, hFresh, hLookup, hPhi, hNe]

theorem depListDependsOnPhiTableFuel_head
    (fuel : Nat) (table : ValueTable) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (rest : List ConsumerNodeId)
    (phiBlock exempt : Nat)
    (hRoot : dependsOnPhiInBlockExceptTableFuel fuel table seen root phiBlock exempt) :
    depListDependsOnPhiTableFuel (fuel + 1) table seen (root :: rest) phiBlock exempt := by
  simp [depListDependsOnPhiTableFuel, hRoot]

theorem depListDependsOnPhiTableFuel_tail
    (fuel : Nat) (table : ValueTable) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (rest : List ConsumerNodeId)
    (phiBlock exempt : Nat)
    (hRest : depListDependsOnPhiTableFuel fuel table seen rest phiBlock exempt) :
    depListDependsOnPhiTableFuel (fuel + 1) table seen (root :: rest) phiBlock exempt := by
  simp [depListDependsOnPhiTableFuel, hRest]

theorem dependsOnPhiInBlockExceptTableFuel_of_depList
    (fuel : Nat) (table : ValueTable) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock exempt : Nat) (node : TableValue)
    (hLookup : lookupTableValue table root = some node)
    (hFresh : root ∉ seen)
    (hDeps : depListDependsOnPhiTableFuel fuel table (root :: seen) (valueDeps node.depsKind)
      phiBlock exempt) :
    dependsOnPhiInBlockExceptTableFuel (fuel + 1) table seen root phiBlock exempt := by
  simp [dependsOnPhiInBlockExceptTableFuel, hFresh, hLookup, hDeps]

def examplePhiWalkTable : ValueTable :=
  [ { phiBlock := none, depsKind := .binary 1 2 }
  , { phiBlock := some 7, depsKind := .phi [3] }
  , { phiBlock := none, depsKind := .call [4, 5] }
  , { phiBlock := some 7, depsKind := .constLike }
  , { phiBlock := none, depsKind := .constLike }
  , { phiBlock := some 8, depsKind := .constLike }
  , { phiBlock := none, depsKind := .binary 6 1 }
  ]

theorem exampleLookupValueDeps_phi :
    lookupValueDeps examplePhiWalkTable 1 = some [3] := by
  rfl

theorem exampleLookupValueDeps_binary :
    lookupValueDeps examplePhiWalkTable 6 = some [6, 1] := by
  rfl

theorem exampleLookupValueDeps_oob :
    lookupValueDeps examplePhiWalkTable 99 = none := by
  rfl

theorem exampleTableDepends_direct_phi :
    dependsOnPhiInBlockExceptTableFuel 3 examplePhiWalkTable [] 0 7 99 := by
  apply dependsOnPhiInBlockExceptTableFuel_of_depList
    (fuel := 2) (table := examplePhiWalkTable) (seen := []) (root := 0)
    (phiBlock := 7) (exempt := 99)
    (node := { phiBlock := none, depsKind := .binary 1 2 })
  · rfl
  · simp
  · apply depListDependsOnPhiTableFuel_head
    exact dependsOnPhiInBlockExceptTableFuel_here
      (fuel := 0) (table := examplePhiWalkTable) (seen := [0]) (root := 1)
      (phiBlock := 7) (exempt := 99)
      (node := { phiBlock := some 7, depsKind := .phi [3] })
      rfl rfl (by simp) (by decide)

theorem exampleTableDepends_exempt_phi_through_arg :
    dependsOnPhiInBlockExceptTableFuel 3 examplePhiWalkTable [] 1 7 1 := by
  apply dependsOnPhiInBlockExceptTableFuel_of_depList
    (fuel := 2) (table := examplePhiWalkTable) (seen := []) (root := 1)
    (phiBlock := 7) (exempt := 1)
    (node := { phiBlock := some 7, depsKind := .phi [3] })
  · rfl
  · simp
  · apply depListDependsOnPhiTableFuel_head
    exact dependsOnPhiInBlockExceptTableFuel_here
      (fuel := 0) (table := examplePhiWalkTable) (seen := [1]) (root := 3)
      (phiBlock := 7) (exempt := 1)
      (node := { phiBlock := some 7, depsKind := .constLike })
      rfl rfl (by simp) (by decide)

theorem exampleTableDepends_other_block_ignored :
    ¬ dependsOnPhiInBlockExceptTableFuel 3 examplePhiWalkTable [] 2 7 99 := by
  simp [dependsOnPhiInBlockExceptTableFuel, depListDependsOnPhiTableFuel,
    examplePhiWalkTable, lookupTableValue, valueDeps]

theorem exampleTableDepends_self_loop_skips_seen_but_finds_phi :
    dependsOnPhiInBlockExceptTableFuel 4 examplePhiWalkTable [] 6 7 99 := by
  apply dependsOnPhiInBlockExceptTableFuel_of_depList
    (fuel := 3) (table := examplePhiWalkTable) (seen := []) (root := 6)
    (phiBlock := 7) (exempt := 99)
    (node := { phiBlock := none, depsKind := .binary 6 1 })
  · rfl
  · simp
  · apply depListDependsOnPhiTableFuel_tail
    apply depListDependsOnPhiTableFuel_head
    exact dependsOnPhiInBlockExceptTableFuel_here
      (fuel := 0) (table := examplePhiWalkTable) (seen := [6]) (root := 1)
      (phiBlock := 7) (exempt := 99)
      (node := { phiBlock := some 7, depsKind := .phi [3] })
      rfl rfl (by simp) (by decide)

end RRProofs
