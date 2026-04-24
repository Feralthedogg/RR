import RRProofs.VerifyIrValueKindTableSubset

namespace RRProofs

inductive EscapeTag where
  | local
  | escaped
  | unknown
deriving Repr, DecidableEq

structure ActualValueRecordLite where
  id : ConsumerNodeId
  kind : ValueTableKind
  originVar : Option String
  phiBlock : Option Nat
  escape : EscapeTag
deriving Repr, DecidableEq

abbrev ActualValueTableLite := List ActualValueRecordLite

def actualValueRecordToFnIrRow (row : ActualValueRecordLite) : FnIrValueRowLite :=
  { phiBlock := row.phiBlock
  , kind := row.kind
  }

def actualValueTableToFnIrTable (table : ActualValueTableLite) : FnIrValueTableLite :=
  table.map actualValueRecordToFnIrRow

def lookupActualValueRow (table : ActualValueTableLite) (root : ConsumerNodeId) :
    Option ActualValueRecordLite :=
  match table, root with
  | [], _ => none
  | row :: _, 0 => some row
  | _ :: rest, n + 1 => lookupActualValueRow rest n

def lookupActualValueDeps
    (table : ActualValueTableLite) (root : ConsumerNodeId) : Option (List ConsumerNodeId) :=
  match lookupActualValueRow table root with
  | some row => some (valueTableKindDeps row.kind)
  | none => none

theorem lookupActualValueDeps_eq_lookupFnIrValueDeps :
    ∀ table root,
      lookupActualValueDeps table root =
        lookupFnIrValueDeps (actualValueTableToFnIrTable table) root := by
  intro table
  induction table with
  | nil =>
      intro root
      cases root <;> rfl
  | cons row rest ih =>
      intro root
      cases root with
      | zero =>
          rfl
      | succ n =>
          simpa [lookupActualValueDeps, lookupActualValueRow, lookupFnIrValueDeps,
            lookupFnIrValueRow, actualValueTableToFnIrTable, actualValueRecordToFnIrRow] using ih n

def dependsOnPhiInBlockExceptActualValueTableFuel
    (fuel : Nat) (table : ActualValueTableLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) : Prop :=
  dependsOnPhiInBlockExceptFnIrTableFuel fuel (actualValueTableToFnIrTable table) seen root phiBlock exempt

def exampleActualValueTable : ActualValueTableLite :=
  [ { id := 0, kind := .binary 1 2, originVar := some "tmp0", phiBlock := none, escape := .unknown }
  , { id := 1, kind := .phi [3], originVar := some "phi1", phiBlock := some 7, escape := .unknown }
  , { id := 2, kind := .call [4, 5], originVar := none, phiBlock := none, escape := .escaped }
  , { id := 3, kind := .constLike, originVar := some "x", phiBlock := some 7, escape := .local }
  , { id := 4, kind := .constLike, originVar := some "y", phiBlock := none, escape := .local }
  , { id := 5, kind := .constLike, originVar := none, phiBlock := some 8, escape := .unknown }
  , { id := 6, kind := .binary 6 1, originVar := some "loop", phiBlock := none, escape := .escaped }
  ]

theorem exampleLookupActualValueDeps_phi :
    lookupActualValueDeps exampleActualValueTable 1 = some [3] := by
  rfl

theorem exampleLookupActualValueDeps_binary :
    lookupActualValueDeps exampleActualValueTable 6 = some [6, 1] := by
  rfl

theorem exampleLookupActualValueDeps_oob :
    lookupActualValueDeps exampleActualValueTable 99 = none := by
  rfl

theorem exampleLookupActualValueDeps_matches_fnIrLookup :
    lookupActualValueDeps exampleActualValueTable 2 =
      lookupFnIrValueDeps (actualValueTableToFnIrTable exampleActualValueTable) 2 := by
  simpa using lookupActualValueDeps_eq_lookupFnIrValueDeps exampleActualValueTable 2

theorem exampleActualValueTableDepends_direct_phi :
    dependsOnPhiInBlockExceptActualValueTableFuel 3 exampleActualValueTable [] 0 7 99 := by
  simpa [dependsOnPhiInBlockExceptActualValueTableFuel, exampleActualValueTable,
    actualValueTableToFnIrTable, actualValueRecordToFnIrRow] using exampleFnIrTableDepends_direct_phi

theorem exampleActualValueTableDepends_exempt_phi_through_arg :
    dependsOnPhiInBlockExceptActualValueTableFuel 3 exampleActualValueTable [] 1 7 1 := by
  simpa [dependsOnPhiInBlockExceptActualValueTableFuel, exampleActualValueTable,
    actualValueTableToFnIrTable, actualValueRecordToFnIrRow] using
    exampleFnIrTableDepends_exempt_phi_through_arg

theorem exampleActualValueTableDepends_other_block_ignored :
    ¬ dependsOnPhiInBlockExceptActualValueTableFuel 3 exampleActualValueTable [] 2 7 99 := by
  simpa [dependsOnPhiInBlockExceptActualValueTableFuel, exampleActualValueTable,
    actualValueTableToFnIrTable, actualValueRecordToFnIrRow] using
    exampleFnIrTableDepends_other_block_ignored

theorem exampleActualValueTableDepends_self_loop_skips_seen_but_finds_phi :
    dependsOnPhiInBlockExceptActualValueTableFuel 4 exampleActualValueTable [] 6 7 99 := by
  simpa [dependsOnPhiInBlockExceptActualValueTableFuel, exampleActualValueTable,
    actualValueTableToFnIrTable, actualValueRecordToFnIrRow] using
    exampleFnIrTableDepends_self_loop_skips_seen_but_finds_phi

end RRProofs
