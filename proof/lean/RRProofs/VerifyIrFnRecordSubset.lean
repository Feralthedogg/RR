import RRProofs.VerifyIrValueFullRecordSubset

namespace RRProofs

inductive TermTag where
  | goto (target : Nat)
  | branch (thenBb elseBb : Nat)
  | ret
  | unreachable
deriving Repr, DecidableEq

structure BlockLite where
  id : Nat
  term : TermTag
deriving Repr, DecidableEq

structure FnRecordLite where
  name : String
  params : List String
  values : ActualValueFullTableLite
  blocks : List BlockLite
  entry : Nat
  bodyHead : Nat
deriving Repr, DecidableEq

def fnRecordValueTable (fnRec : FnRecordLite) : ActualValueFullTableLite :=
  fnRec.values

def fnRecordLookupValueDeps (fnRec : FnRecordLite) (root : ConsumerNodeId) :
    Option (List ConsumerNodeId) :=
  lookupActualValueFullDeps fnRec.values root

def fnRecordDependsOnPhiInBlockExceptFuel
    (fuel : Nat) (fnRec : FnRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) : Prop :=
  dependsOnPhiInBlockExceptActualValueFullTableFuel fuel fnRec.values seen root phiBlock exempt

theorem fnRecordLookupValueDeps_eq_valueTable
    (fnRec : FnRecordLite) (root : ConsumerNodeId) :
    fnRecordLookupValueDeps fnRec root =
      lookupActualValueFullDeps (fnRecordValueTable fnRec) root := rfl

theorem fnRecordDependsOnPhiInBlockExcept_eq_valueTable
    (fuel : Nat) (fnRec : FnRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) :
    fnRecordDependsOnPhiInBlockExceptFuel fuel fnRec seen root phiBlock exempt =
      dependsOnPhiInBlockExceptActualValueFullTableFuel fuel (fnRecordValueTable fnRec) seen root phiBlock exempt := rfl

def exampleFnRecord : FnRecordLite :=
  { name := "example"
  , params := ["p0", "p1"]
  , values := exampleActualValueFullTable
  , blocks :=
      [ { id := 0, term := .goto 1 }
      , { id := 1, term := .branch 2 3 }
      , { id := 2, term := .ret }
      , { id := 3, term := .unreachable }
      ]
  , entry := 0
  , bodyHead := 1
  }

theorem exampleFnRecordLookup_phi :
    fnRecordLookupValueDeps exampleFnRecord 1 = some [3] := by
  rfl

theorem exampleFnRecordLookup_binary :
    fnRecordLookupValueDeps exampleFnRecord 6 = some [6, 1] := by
  rfl

theorem exampleFnRecordLookup_oob :
    fnRecordLookupValueDeps exampleFnRecord 99 = none := by
  rfl

theorem exampleFnRecordDepends_direct_phi :
    fnRecordDependsOnPhiInBlockExceptFuel 3 exampleFnRecord [] 0 7 99 := by
  exact exampleActualValueFullTableDepends_direct_phi

theorem exampleFnRecordDepends_exempt_phi_through_arg :
    fnRecordDependsOnPhiInBlockExceptFuel 3 exampleFnRecord [] 1 7 1 := by
  exact exampleActualValueFullTableDepends_exempt_phi_through_arg

theorem exampleFnRecordDepends_other_block_ignored :
    ¬ fnRecordDependsOnPhiInBlockExceptFuel 3 exampleFnRecord [] 2 7 99 := by
  exact exampleActualValueFullTableDepends_other_block_ignored

theorem exampleFnRecordDepends_self_loop_skips_seen_but_finds_phi :
    fnRecordDependsOnPhiInBlockExceptFuel 4 exampleFnRecord [] 6 7 99 := by
  exact exampleActualValueFullTableDepends_self_loop_skips_seen_but_finds_phi

end RRProofs
