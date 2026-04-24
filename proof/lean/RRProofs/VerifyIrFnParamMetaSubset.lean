import RRProofs.VerifyIrFnMetaSubset

namespace RRProofs

structure FnParamMetaRecordLite where
  shell : FnMetaRecordLite
  paramDefaultRExprs : List (Option String)
  paramSpans : List SpanTag
  paramTyHints : List ValueTyTag
  paramTermHints : List ValueTermTag
  paramHintSpans : List (Option SpanTag)
deriving Repr, DecidableEq

def fnParamMetaToFnMeta (fnParamMeta : FnParamMetaRecordLite) : FnMetaRecordLite :=
  fnParamMeta.shell

def fnParamMetaToFnRecord (fnParamMeta : FnParamMetaRecordLite) : FnRecordLite :=
  fnMetaToFnRecord fnParamMeta.shell

def fnParamMetaLookupValueDeps (fnParamMeta : FnParamMetaRecordLite) (root : ConsumerNodeId) :
    Option (List ConsumerNodeId) :=
  fnMetaLookupValueDeps fnParamMeta.shell root

def fnParamMetaDependsOnPhiInBlockExceptFuel
    (fuel : Nat) (fnParamMeta : FnParamMetaRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) : Prop :=
  fnMetaDependsOnPhiInBlockExceptFuel fuel fnParamMeta.shell seen root phiBlock exempt

theorem fnParamMetaLookupValueDeps_eq_fnMeta
    (fnParamMeta : FnParamMetaRecordLite) (root : ConsumerNodeId) :
    fnParamMetaLookupValueDeps fnParamMeta root =
      fnMetaLookupValueDeps (fnParamMetaToFnMeta fnParamMeta) root := rfl

theorem fnParamMetaDependsOnPhiInBlockExcept_eq_fnMeta
    (fuel : Nat) (fnParamMeta : FnParamMetaRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) :
    fnParamMetaDependsOnPhiInBlockExceptFuel fuel fnParamMeta seen root phiBlock exempt =
      fnMetaDependsOnPhiInBlockExceptFuel fuel (fnParamMetaToFnMeta fnParamMeta) seen root phiBlock exempt := rfl

theorem fnParamMetaLookupValueDeps_eq_fnRecord
    (fnParamMeta : FnParamMetaRecordLite) (root : ConsumerNodeId) :
    fnParamMetaLookupValueDeps fnParamMeta root =
      fnRecordLookupValueDeps (fnParamMetaToFnRecord fnParamMeta) root := by
  exact fnMetaLookupValueDeps_eq_fnRecord fnParamMeta.shell root

theorem fnParamMetaDependsOnPhiInBlockExcept_eq_fnRecord
    (fuel : Nat) (fnParamMeta : FnParamMetaRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) :
    fnParamMetaDependsOnPhiInBlockExceptFuel fuel fnParamMeta seen root phiBlock exempt =
      fnRecordDependsOnPhiInBlockExceptFuel fuel (fnParamMetaToFnRecord fnParamMeta) seen root phiBlock exempt := by
  exact fnMetaDependsOnPhiInBlockExcept_eq_fnRecord fuel fnParamMeta.shell seen root phiBlock exempt

def exampleFnParamMetaRecord : FnParamMetaRecordLite :=
  { shell := exampleFnMetaRecord
  , paramDefaultRExprs := [none, some "1L"]
  , paramSpans := [.source, .dummy]
  , paramTyHints := [.intLike, .recordLike]
  , paramTermHints := [.scalar, .record]
  , paramHintSpans := [some .source, none]
  }

theorem exampleFnParamMetaRecord_toShell :
    fnParamMetaToFnMeta exampleFnParamMetaRecord = exampleFnMetaRecord := by
  rfl

theorem exampleFnParamMetaRecord_paramDefaults :
    exampleFnParamMetaRecord.paramDefaultRExprs = [none, some "1L"] := by
  rfl

theorem exampleFnParamMetaRecord_paramHints :
    exampleFnParamMetaRecord.paramTyHints = [.intLike, .recordLike] ∧
    exampleFnParamMetaRecord.paramTermHints = [.scalar, .record] := by
  exact ⟨rfl, rfl⟩

theorem exampleFnParamMetaRecord_paramSpans :
    exampleFnParamMetaRecord.paramSpans = [.source, .dummy] ∧
    exampleFnParamMetaRecord.paramHintSpans = [some .source, none] := by
  exact ⟨rfl, rfl⟩

theorem exampleFnParamMetaLookup_phi :
    fnParamMetaLookupValueDeps exampleFnParamMetaRecord 1 = some [3] := by
  rfl

theorem exampleFnParamMetaLookup_binary :
    fnParamMetaLookupValueDeps exampleFnParamMetaRecord 6 = some [6, 1] := by
  rfl

theorem exampleFnParamMetaDepends_direct_phi :
    fnParamMetaDependsOnPhiInBlockExceptFuel 3 exampleFnParamMetaRecord [] 0 7 99 := by
  exact exampleFnMetaDepends_direct_phi

theorem exampleFnParamMetaDepends_exempt_phi_through_arg :
    fnParamMetaDependsOnPhiInBlockExceptFuel 3 exampleFnParamMetaRecord [] 1 7 1 := by
  exact exampleFnMetaDepends_exempt_phi_through_arg

theorem exampleFnParamMetaDepends_other_block_ignored :
    ¬ fnParamMetaDependsOnPhiInBlockExceptFuel 3 exampleFnParamMetaRecord [] 2 7 99 := by
  exact exampleFnMetaDepends_other_block_ignored

end RRProofs
