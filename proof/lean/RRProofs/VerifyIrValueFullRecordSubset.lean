import RRProofs.VerifyIrValueRecordSubset

namespace RRProofs

inductive SpanTag where
  | dummy
  | source
deriving Repr, DecidableEq

inductive FactsTag where
  | unknown
  | nonneg
  | bounded
deriving Repr, DecidableEq

inductive ValueTyTag where
  | unknown
  | intLike
  | recordLike
deriving Repr, DecidableEq

inductive ValueTermTag where
  | any
  | scalar
  | record
deriving Repr, DecidableEq

structure ActualValueFullRecordLite where
  id : ConsumerNodeId
  kind : ValueTableKind
  span : SpanTag
  facts : FactsTag
  valueTy : ValueTyTag
  valueTerm : ValueTermTag
  originVar : Option String
  phiBlock : Option Nat
  escape : EscapeTag
deriving Repr, DecidableEq

abbrev ActualValueFullTableLite := List ActualValueFullRecordLite

def actualValueFullRecordToRecord (row : ActualValueFullRecordLite) : ActualValueRecordLite :=
  { id := row.id
  , kind := row.kind
  , originVar := row.originVar
  , phiBlock := row.phiBlock
  , escape := row.escape
  }

def actualValueFullTableToRecordTable (table : ActualValueFullTableLite) : ActualValueTableLite :=
  table.map actualValueFullRecordToRecord

def lookupActualValueFullRow (table : ActualValueFullTableLite) (root : ConsumerNodeId) :
    Option ActualValueFullRecordLite :=
  match table, root with
  | [], _ => none
  | row :: _, 0 => some row
  | _ :: rest, n + 1 => lookupActualValueFullRow rest n

def lookupActualValueFullDeps
    (table : ActualValueFullTableLite) (root : ConsumerNodeId) : Option (List ConsumerNodeId) :=
  match lookupActualValueFullRow table root with
  | some row => some (valueTableKindDeps row.kind)
  | none => none

theorem lookupActualValueFullDeps_eq_lookupActualValueDeps :
    ∀ table root,
      lookupActualValueFullDeps table root =
        lookupActualValueDeps (actualValueFullTableToRecordTable table) root := by
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
          simpa [lookupActualValueFullDeps, lookupActualValueFullRow, lookupActualValueDeps,
            lookupActualValueRow, actualValueFullTableToRecordTable, actualValueFullRecordToRecord] using ih n

def dependsOnPhiInBlockExceptActualValueFullTableFuel
    (fuel : Nat) (table : ActualValueFullTableLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) : Prop :=
  dependsOnPhiInBlockExceptActualValueTableFuel fuel (actualValueFullTableToRecordTable table) seen root phiBlock exempt

def exampleActualValueFullTable : ActualValueFullTableLite :=
  [ { id := 0, kind := .binary 1 2, span := .source, facts := .bounded,
      valueTy := .intLike, valueTerm := .scalar,
      originVar := some "tmp0", phiBlock := none, escape := .unknown }
  , { id := 1, kind := .phi [3], span := .source, facts := .unknown,
      valueTy := .intLike, valueTerm := .scalar,
      originVar := some "phi1", phiBlock := some 7, escape := .unknown }
  , { id := 2, kind := .call [4, 5], span := .source, facts := .unknown,
      valueTy := .unknown, valueTerm := .any,
      originVar := none, phiBlock := none, escape := .escaped }
  , { id := 3, kind := .constLike, span := .dummy, facts := .nonneg,
      valueTy := .intLike, valueTerm := .scalar,
      originVar := some "x", phiBlock := some 7, escape := .local }
  , { id := 4, kind := .constLike, span := .dummy, facts := .nonneg,
      valueTy := .intLike, valueTerm := .scalar,
      originVar := some "y", phiBlock := none, escape := .local }
  , { id := 5, kind := .constLike, span := .dummy, facts := .unknown,
      valueTy := .intLike, valueTerm := .scalar,
      originVar := none, phiBlock := some 8, escape := .unknown }
  , { id := 6, kind := .binary 6 1, span := .source, facts := .unknown,
      valueTy := .intLike, valueTerm := .scalar,
      originVar := some "loop", phiBlock := none, escape := .escaped }
  ]

theorem exampleLookupActualValueFullDeps_phi :
    lookupActualValueFullDeps exampleActualValueFullTable 1 = some [3] := by
  rfl

theorem exampleLookupActualValueFullDeps_binary :
    lookupActualValueFullDeps exampleActualValueFullTable 6 = some [6, 1] := by
  rfl

theorem exampleLookupActualValueFullDeps_oob :
    lookupActualValueFullDeps exampleActualValueFullTable 99 = none := by
  rfl

theorem exampleLookupActualValueFullDeps_matches_recordLookup :
    lookupActualValueFullDeps exampleActualValueFullTable 2 =
      lookupActualValueDeps (actualValueFullTableToRecordTable exampleActualValueFullTable) 2 := by
  simpa using lookupActualValueFullDeps_eq_lookupActualValueDeps exampleActualValueFullTable 2

theorem exampleActualValueFullTableDepends_direct_phi :
    dependsOnPhiInBlockExceptActualValueFullTableFuel 3 exampleActualValueFullTable [] 0 7 99 := by
  simpa [dependsOnPhiInBlockExceptActualValueFullTableFuel, exampleActualValueFullTable,
    actualValueFullTableToRecordTable, actualValueFullRecordToRecord] using
    exampleActualValueTableDepends_direct_phi

theorem exampleActualValueFullTableDepends_exempt_phi_through_arg :
    dependsOnPhiInBlockExceptActualValueFullTableFuel 3 exampleActualValueFullTable [] 1 7 1 := by
  simpa [dependsOnPhiInBlockExceptActualValueFullTableFuel, exampleActualValueFullTable,
    actualValueFullTableToRecordTable, actualValueFullRecordToRecord] using
    exampleActualValueTableDepends_exempt_phi_through_arg

theorem exampleActualValueFullTableDepends_other_block_ignored :
    ¬ dependsOnPhiInBlockExceptActualValueFullTableFuel 3 exampleActualValueFullTable [] 2 7 99 := by
  simpa [dependsOnPhiInBlockExceptActualValueFullTableFuel, exampleActualValueFullTable,
    actualValueFullTableToRecordTable, actualValueFullRecordToRecord] using
    exampleActualValueTableDepends_other_block_ignored

theorem exampleActualValueFullTableDepends_self_loop_skips_seen_but_finds_phi :
    dependsOnPhiInBlockExceptActualValueFullTableFuel 4 exampleActualValueFullTable [] 6 7 99 := by
  simpa [dependsOnPhiInBlockExceptActualValueFullTableFuel, exampleActualValueFullTable,
    actualValueFullTableToRecordTable, actualValueFullRecordToRecord] using
    exampleActualValueTableDepends_self_loop_skips_seen_but_finds_phi

end RRProofs
