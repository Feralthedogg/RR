import RRProofs.VerifyIrFnRecordSubset

namespace RRProofs

structure FnMetaRecordLite where
  shell : FnRecordLite
  userName : Option String
  fnSpan : SpanTag
  retTyHint : Option ValueTyTag
  retTermHint : Option ValueTermTag
  retHintSpan : Option SpanTag
  inferredRetTy : ValueTyTag
  inferredRetTerm : ValueTermTag
  unsupportedDynamic : Bool
  fallbackReasons : List String
  hybridInteropReasons : List String
  opaqueInterop : Bool
  opaqueReasons : List String
  opaqueInteropReasons : List String
deriving Repr, DecidableEq

def fnMetaToFnRecord (fnMeta : FnMetaRecordLite) : FnRecordLite :=
  fnMeta.shell

def fnMetaLookupValueDeps (fnMeta : FnMetaRecordLite) (root : ConsumerNodeId) :
    Option (List ConsumerNodeId) :=
  fnRecordLookupValueDeps fnMeta.shell root

def fnMetaDependsOnPhiInBlockExceptFuel
    (fuel : Nat) (fnMeta : FnMetaRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) : Prop :=
  fnRecordDependsOnPhiInBlockExceptFuel fuel fnMeta.shell seen root phiBlock exempt

theorem fnMetaLookupValueDeps_eq_fnRecord
    (fnMeta : FnMetaRecordLite) (root : ConsumerNodeId) :
    fnMetaLookupValueDeps fnMeta root =
      fnRecordLookupValueDeps (fnMetaToFnRecord fnMeta) root := rfl

theorem fnMetaDependsOnPhiInBlockExcept_eq_fnRecord
    (fuel : Nat) (fnMeta : FnMetaRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) :
    fnMetaDependsOnPhiInBlockExceptFuel fuel fnMeta seen root phiBlock exempt =
      fnRecordDependsOnPhiInBlockExceptFuel fuel (fnMetaToFnRecord fnMeta) seen root phiBlock exempt := rfl

theorem fnMetaLookupValueDeps_eq_valueTable
    (fnMeta : FnMetaRecordLite) (root : ConsumerNodeId) :
    fnMetaLookupValueDeps fnMeta root =
      lookupActualValueFullDeps (fnMeta.shell.values) root := by
  exact fnRecordLookupValueDeps_eq_valueTable fnMeta.shell root

theorem fnMetaDependsOnPhiInBlockExcept_eq_valueTable
    (fuel : Nat) (fnMeta : FnMetaRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) :
    fnMetaDependsOnPhiInBlockExceptFuel fuel fnMeta seen root phiBlock exempt =
      dependsOnPhiInBlockExceptActualValueFullTableFuel fuel fnMeta.shell.values seen root phiBlock exempt := by
  exact fnRecordDependsOnPhiInBlockExcept_eq_valueTable fuel fnMeta.shell seen root phiBlock exempt

def exampleFnMetaRecord : FnMetaRecordLite :=
  { shell := exampleFnRecord
  , userName := some "exampleUser"
  , fnSpan := .source
  , retTyHint := some .intLike
  , retTermHint := some .scalar
  , retHintSpan := some .source
  , inferredRetTy := .intLike
  , inferredRetTerm := .scalar
  , unsupportedDynamic := true
  , fallbackReasons := ["dynamic builtin"]
  , hybridInteropReasons := ["package::foo"]
  , opaqueInterop := true
  , opaqueReasons := ["opaque runtime"]
  , opaqueInteropReasons := ["ffi::bar"]
  }

theorem exampleFnMetaRecord_toShell :
    fnMetaToFnRecord exampleFnMetaRecord = exampleFnRecord := by
  rfl

theorem exampleFnMetaRecord_userName :
    exampleFnMetaRecord.userName = some "exampleUser" := by
  rfl

theorem exampleFnMetaRecord_retHints :
    exampleFnMetaRecord.retTyHint = some .intLike ∧
    exampleFnMetaRecord.retTermHint = some .scalar := by
  exact ⟨rfl, rfl⟩

theorem exampleFnMetaRecord_interopFlags :
    exampleFnMetaRecord.unsupportedDynamic = true ∧
    exampleFnMetaRecord.opaqueInterop = true := by
  exact ⟨rfl, rfl⟩

theorem exampleFnMetaLookup_phi :
    fnMetaLookupValueDeps exampleFnMetaRecord 1 = some [3] := by
  rfl

theorem exampleFnMetaLookup_binary :
    fnMetaLookupValueDeps exampleFnMetaRecord 6 = some [6, 1] := by
  rfl

theorem exampleFnMetaDepends_direct_phi :
    fnMetaDependsOnPhiInBlockExceptFuel 3 exampleFnMetaRecord [] 0 7 99 := by
  exact exampleFnRecordDepends_direct_phi

theorem exampleFnMetaDepends_exempt_phi_through_arg :
    fnMetaDependsOnPhiInBlockExceptFuel 3 exampleFnMetaRecord [] 1 7 1 := by
  exact exampleFnRecordDepends_exempt_phi_through_arg

theorem exampleFnMetaDepends_other_block_ignored :
    ¬ fnMetaDependsOnPhiInBlockExceptFuel 3 exampleFnMetaRecord [] 2 7 99 := by
  exact exampleFnRecordDepends_other_block_ignored

end RRProofs
