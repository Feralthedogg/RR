import RRProofs.VerifyIrValueTableWalkSubset

namespace RRProofs

inductive ValueTableKind where
  | constLike
  | paramLike
  | loadLike
  | rSymbolLike
  | len (base : ConsumerNodeId)
  | indices (base : ConsumerNodeId)
  | unary (base : ConsumerNodeId)
  | fieldGet (base : ConsumerNodeId)
  | range (start finish : ConsumerNodeId)
  | binary (lhs rhs : ConsumerNodeId)
  | phi (args : List ConsumerNodeId)
  | call (args : List ConsumerNodeId)
  | intrinsic (args : List ConsumerNodeId)
  | recordLit (fields : List (String × ConsumerNodeId))
  | fieldSet (base value : ConsumerNodeId)
  | index1d (base idx : ConsumerNodeId)
  | index2d (base r c : ConsumerNodeId)
  | index3d (base i j k : ConsumerNodeId)
deriving Repr, DecidableEq

def valueTableKindToDepsKind : ValueTableKind -> ValueDepsKind
  | .constLike => .constLike
  | .paramLike => .paramLike
  | .loadLike => .loadLike
  | .rSymbolLike => .rSymbolLike
  | .len base => .len base
  | .indices base => .indices base
  | .unary base => .unary base
  | .fieldGet base => .fieldGet base
  | .range start finish => .range start finish
  | .binary lhs rhs => .binary lhs rhs
  | .phi args => .phi args
  | .call args => .call args
  | .intrinsic args => .intrinsic args
  | .recordLit fields => .recordLit fields
  | .fieldSet base value => .fieldSet base value
  | .index1d base idx => .index1d base idx
  | .index2d base r c => .index2d base r c
  | .index3d base i j k => .index3d base i j k

def valueTableKindDeps (kind : ValueTableKind) : List ConsumerNodeId :=
  valueDeps (valueTableKindToDepsKind kind)

theorem valueTableKindDeps_eq_valueDeps (kind : ValueTableKind) :
    valueTableKindDeps kind = valueDeps (valueTableKindToDepsKind kind) := rfl

structure FnIrValueRowLite where
  phiBlock : Option Nat
  kind : ValueTableKind
deriving Repr, DecidableEq

abbrev FnIrValueTableLite := List FnIrValueRowLite

def fnIrRowToTableValue (row : FnIrValueRowLite) : TableValue :=
  { phiBlock := row.phiBlock
  , depsKind := valueTableKindToDepsKind row.kind
  }

def fnIrValueTableToValueTable (table : FnIrValueTableLite) : ValueTable :=
  table.map fnIrRowToTableValue

def lookupFnIrValueRow (table : FnIrValueTableLite) (root : ConsumerNodeId) : Option FnIrValueRowLite :=
  match table, root with
  | [], _ => none
  | row :: _, 0 => some row
  | _ :: rest, n + 1 => lookupFnIrValueRow rest n

def lookupFnIrValueDeps
    (table : FnIrValueTableLite) (root : ConsumerNodeId) : Option (List ConsumerNodeId) :=
  match lookupFnIrValueRow table root with
  | some row => some (valueTableKindDeps row.kind)
  | none => none

theorem lookupFnIrValueDeps_eq_lookupValueDeps :
    ∀ table root,
      lookupFnIrValueDeps table root =
        lookupValueDeps (fnIrValueTableToValueTable table) root := by
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
          simpa [lookupFnIrValueDeps, lookupFnIrValueRow, lookupValueDeps,
            fnIrValueTableToValueTable, lookupTableValue] using ih n

def dependsOnPhiInBlockExceptFnIrTableFuel
    (fuel : Nat) (table : FnIrValueTableLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) : Prop :=
  dependsOnPhiInBlockExceptTableFuel fuel (fnIrValueTableToValueTable table) seen root phiBlock exempt

def exampleFnIrValueTable : FnIrValueTableLite :=
  [ { phiBlock := none, kind := .binary 1 2 }
  , { phiBlock := some 7, kind := .phi [3] }
  , { phiBlock := none, kind := .call [4, 5] }
  , { phiBlock := some 7, kind := .constLike }
  , { phiBlock := none, kind := .constLike }
  , { phiBlock := some 8, kind := .constLike }
  , { phiBlock := none, kind := .binary 6 1 }
  ]

theorem exampleLookupFnIrValueDeps_phi :
    lookupFnIrValueDeps exampleFnIrValueTable 1 = some [3] := by
  rfl

theorem exampleLookupFnIrValueDeps_binary :
    lookupFnIrValueDeps exampleFnIrValueTable 6 = some [6, 1] := by
  rfl

theorem exampleLookupFnIrValueDeps_oob :
    lookupFnIrValueDeps exampleFnIrValueTable 99 = none := by
  rfl

theorem exampleLookupFnIrValueDeps_matches_table_lookup :
    lookupFnIrValueDeps exampleFnIrValueTable 2 =
      lookupValueDeps (fnIrValueTableToValueTable exampleFnIrValueTable) 2 := by
  simpa using lookupFnIrValueDeps_eq_lookupValueDeps exampleFnIrValueTable 2

theorem exampleFnIrTableDepends_direct_phi :
    dependsOnPhiInBlockExceptFnIrTableFuel 3 exampleFnIrValueTable [] 0 7 99 := by
  simpa [dependsOnPhiInBlockExceptFnIrTableFuel, exampleFnIrValueTable,
    fnIrValueTableToValueTable, fnIrRowToTableValue] using exampleTableDepends_direct_phi

theorem exampleFnIrTableDepends_exempt_phi_through_arg :
    dependsOnPhiInBlockExceptFnIrTableFuel 3 exampleFnIrValueTable [] 1 7 1 := by
  simpa [dependsOnPhiInBlockExceptFnIrTableFuel, exampleFnIrValueTable,
    fnIrValueTableToValueTable, fnIrRowToTableValue] using
    exampleTableDepends_exempt_phi_through_arg

theorem exampleFnIrTableDepends_other_block_ignored :
    ¬ dependsOnPhiInBlockExceptFnIrTableFuel 3 exampleFnIrValueTable [] 2 7 99 := by
  simpa [dependsOnPhiInBlockExceptFnIrTableFuel, exampleFnIrValueTable,
    fnIrValueTableToValueTable, fnIrRowToTableValue] using exampleTableDepends_other_block_ignored

theorem exampleFnIrTableDepends_self_loop_skips_seen_but_finds_phi :
    dependsOnPhiInBlockExceptFnIrTableFuel 4 exampleFnIrValueTable [] 6 7 99 := by
  simpa [dependsOnPhiInBlockExceptFnIrTableFuel, exampleFnIrValueTable,
    fnIrValueTableToValueTable, fnIrRowToTableValue] using
    exampleTableDepends_self_loop_skips_seen_but_finds_phi

end RRProofs
