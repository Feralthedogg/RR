import RRProofs.VerifyIrFnHintMapSubset

namespace RRProofs

inductive InstrRecordLite where
  | assign (dst : String) (src : ConsumerNodeId) (span : SpanTag)
  | eval (val : ConsumerNodeId) (span : SpanTag)
  | storeIndex1D (base idx val : ConsumerNodeId) (span : SpanTag)
  | storeIndex2D (base r c val : ConsumerNodeId) (span : SpanTag)
  | storeIndex3D (base i j k val : ConsumerNodeId) (span : SpanTag)
deriving Repr, DecidableEq

inductive TerminatorRecordLite where
  | goto (target : Nat)
  | branch (cond : ConsumerNodeId) (thenBb elseBb : Nat)
  | ret (val : Option ConsumerNodeId)
  | unreachable
deriving Repr, DecidableEq

structure ActualBlockRecordLite where
  id : Nat
  instrs : List InstrRecordLite
  term : TerminatorRecordLite
deriving Repr, DecidableEq

def terminatorRecordToTermTag (term : TerminatorRecordLite) : TermTag :=
  match term with
  | .goto target => .goto target
  | .branch _ thenBb elseBb => .branch thenBb elseBb
  | .ret _ => .ret
  | .unreachable => .unreachable

def actualBlockRecordToBlockLite (bb : ActualBlockRecordLite) : BlockLite :=
  { id := bb.id
  , term := terminatorRecordToTermTag bb.term
  }

structure FnBlockRecordLite where
  shell : FnHintMapRecordLite
  blocks : List ActualBlockRecordLite
deriving Repr, DecidableEq

def fnBlockRecordToFnHintMap (fnBlock : FnBlockRecordLite) : FnHintMapRecordLite :=
  fnBlock.shell

def fnBlockRecordToFnRecord (fnBlock : FnBlockRecordLite) : FnRecordLite :=
  { fnHintMapToFnRecord fnBlock.shell with
    blocks := fnBlock.blocks.map actualBlockRecordToBlockLite
  }

def fnBlockRecordLookupValueDeps (fnBlock : FnBlockRecordLite) (root : ConsumerNodeId) :
    Option (List ConsumerNodeId) :=
  fnHintMapLookupValueDeps fnBlock.shell root

def fnBlockRecordDependsOnPhiInBlockExceptFuel
    (fuel : Nat) (fnBlock : FnBlockRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) : Prop :=
  fnHintMapDependsOnPhiInBlockExceptFuel fuel fnBlock.shell seen root phiBlock exempt

theorem fnBlockRecordLookupValueDeps_eq_fnHintMap
    (fnBlock : FnBlockRecordLite) (root : ConsumerNodeId) :
    fnBlockRecordLookupValueDeps fnBlock root =
      fnHintMapLookupValueDeps (fnBlockRecordToFnHintMap fnBlock) root := rfl

theorem fnBlockRecordDependsOnPhiInBlockExcept_eq_fnHintMap
    (fuel : Nat) (fnBlock : FnBlockRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) :
    fnBlockRecordDependsOnPhiInBlockExceptFuel fuel fnBlock seen root phiBlock exempt =
      fnHintMapDependsOnPhiInBlockExceptFuel fuel (fnBlockRecordToFnHintMap fnBlock) seen root phiBlock exempt := rfl

theorem fnBlockRecordLookupValueDeps_eq_fnRecord
    (fnBlock : FnBlockRecordLite) (root : ConsumerNodeId) :
    fnBlockRecordLookupValueDeps fnBlock root =
      fnRecordLookupValueDeps (fnBlockRecordToFnRecord fnBlock) root := rfl

theorem fnBlockRecordDependsOnPhiInBlockExcept_eq_fnRecord
    (fuel : Nat) (fnBlock : FnBlockRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) :
    fnBlockRecordDependsOnPhiInBlockExceptFuel fuel fnBlock seen root phiBlock exempt =
      fnRecordDependsOnPhiInBlockExceptFuel fuel (fnBlockRecordToFnRecord fnBlock) seen root phiBlock exempt := rfl

def exampleActualBlocks : List ActualBlockRecordLite :=
  [ { id := 0
    , instrs := [.assign "tmp0" 0 .source]
    , term := .goto 1
    }
  , { id := 1
    , instrs := [.eval 2 .source]
    , term := .branch 2 2 3
    }
  , { id := 2
    , instrs := []
    , term := .ret (some 4)
    }
  , { id := 3
    , instrs := []
    , term := .unreachable
    }
  ]

def exampleFnBlockRecord : FnBlockRecordLite :=
  { shell := exampleFnHintMapRecord
  , blocks := exampleActualBlocks
  }

theorem exampleActualBlocks_project :
    exampleActualBlocks.map actualBlockRecordToBlockLite = exampleFnRecord.blocks := by
  rfl

theorem exampleFnBlockRecord_toShell :
    fnBlockRecordToFnHintMap exampleFnBlockRecord = exampleFnHintMapRecord := by
  rfl

theorem exampleFnBlockRecord_toFnRecord :
    fnBlockRecordToFnRecord exampleFnBlockRecord = exampleFnRecord := by
  rfl

theorem exampleFnBlockRecord_lookup_phi :
    fnBlockRecordLookupValueDeps exampleFnBlockRecord 1 = some [3] := by
  rfl

theorem exampleFnBlockRecord_lookup_binary :
    fnBlockRecordLookupValueDeps exampleFnBlockRecord 6 = some [6, 1] := by
  rfl

theorem exampleFnBlockRecord_depends_direct_phi :
    fnBlockRecordDependsOnPhiInBlockExceptFuel 3 exampleFnBlockRecord [] 0 7 99 := by
  exact exampleFnHintMapDepends_direct_phi

theorem exampleFnBlockRecord_depends_exempt_phi_through_arg :
    fnBlockRecordDependsOnPhiInBlockExceptFuel 3 exampleFnBlockRecord [] 1 7 1 := by
  exact exampleFnHintMapDepends_exempt_phi_through_arg

theorem exampleFnBlockRecord_depends_other_block_ignored :
    ¬ fnBlockRecordDependsOnPhiInBlockExceptFuel 3 exampleFnBlockRecord [] 2 7 99 := by
  exact exampleFnHintMapDepends_other_block_ignored

end RRProofs
