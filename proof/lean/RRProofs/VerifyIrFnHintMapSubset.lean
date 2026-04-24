import RRProofs.VerifyIrFnParamMetaSubset

namespace RRProofs

inductive CallSemanticsTag where
  | builtin
  | runtimeHelper
  | closureDispatch
  | userDefined
deriving Repr, DecidableEq

inductive MemoryLayoutHintTag where
  | dense1D
  | columnMajor2D
  | columnMajorND
deriving Repr, DecidableEq

abbrev HintMap := List (ConsumerNodeId × String)

structure FnHintMapRecordLite where
  shell : FnParamMetaRecordLite
  callSemantics : List (ConsumerNodeId × CallSemanticsTag)
  memoryLayoutHints : List (ConsumerNodeId × MemoryLayoutHintTag)
deriving Repr, DecidableEq

def fnHintMapToFnParamMeta (fnHintMap : FnHintMapRecordLite) : FnParamMetaRecordLite :=
  fnHintMap.shell

def fnHintMapToFnMeta (fnHintMap : FnHintMapRecordLite) : FnMetaRecordLite :=
  fnParamMetaToFnMeta fnHintMap.shell

def fnHintMapToFnRecord (fnHintMap : FnHintMapRecordLite) : FnRecordLite :=
  fnParamMetaToFnRecord fnHintMap.shell

def fnHintMapLookupValueDeps (fnHintMap : FnHintMapRecordLite) (root : ConsumerNodeId) :
    Option (List ConsumerNodeId) :=
  fnParamMetaLookupValueDeps fnHintMap.shell root

def fnHintMapDependsOnPhiInBlockExceptFuel
    (fuel : Nat) (fnHintMap : FnHintMapRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) : Prop :=
  fnParamMetaDependsOnPhiInBlockExceptFuel fuel fnHintMap.shell seen root phiBlock exempt

theorem fnHintMapLookupValueDeps_eq_fnParamMeta
    (fnHintMap : FnHintMapRecordLite) (root : ConsumerNodeId) :
    fnHintMapLookupValueDeps fnHintMap root =
      fnParamMetaLookupValueDeps (fnHintMapToFnParamMeta fnHintMap) root := rfl

theorem fnHintMapDependsOnPhiInBlockExcept_eq_fnParamMeta
    (fuel : Nat) (fnHintMap : FnHintMapRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) :
    fnHintMapDependsOnPhiInBlockExceptFuel fuel fnHintMap seen root phiBlock exempt =
      fnParamMetaDependsOnPhiInBlockExceptFuel fuel (fnHintMapToFnParamMeta fnHintMap) seen root phiBlock exempt := rfl

theorem fnHintMapLookupValueDeps_eq_fnRecord
    (fnHintMap : FnHintMapRecordLite) (root : ConsumerNodeId) :
    fnHintMapLookupValueDeps fnHintMap root =
      fnRecordLookupValueDeps (fnHintMapToFnRecord fnHintMap) root := by
  exact fnParamMetaLookupValueDeps_eq_fnRecord fnHintMap.shell root

theorem fnHintMapDependsOnPhiInBlockExcept_eq_fnRecord
    (fuel : Nat) (fnHintMap : FnHintMapRecordLite) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock : Nat) (exempt : ConsumerNodeId) :
    fnHintMapDependsOnPhiInBlockExceptFuel fuel fnHintMap seen root phiBlock exempt =
      fnRecordDependsOnPhiInBlockExceptFuel fuel (fnHintMapToFnRecord fnHintMap) seen root phiBlock exempt := by
  exact fnParamMetaDependsOnPhiInBlockExcept_eq_fnRecord fuel fnHintMap.shell seen root phiBlock exempt

def exampleFnHintMapRecord : FnHintMapRecordLite :=
  { shell := exampleFnParamMetaRecord
  , callSemantics := [(2, .builtin), (6, .userDefined)]
  , memoryLayoutHints := [(2, .dense1D), (6, .columnMajorND)]
  }

theorem exampleFnHintMapRecord_toShell :
    fnHintMapToFnParamMeta exampleFnHintMapRecord = exampleFnParamMetaRecord := by
  rfl

theorem exampleFnHintMapRecord_callSemantics :
    exampleFnHintMapRecord.callSemantics = [(2, .builtin), (6, .userDefined)] := by
  rfl

theorem exampleFnHintMapRecord_memoryLayoutHints :
    exampleFnHintMapRecord.memoryLayoutHints = [(2, .dense1D), (6, .columnMajorND)] := by
  rfl

theorem exampleFnHintMapLookup_phi :
    fnHintMapLookupValueDeps exampleFnHintMapRecord 1 = some [3] := by
  rfl

theorem exampleFnHintMapLookup_binary :
    fnHintMapLookupValueDeps exampleFnHintMapRecord 6 = some [6, 1] := by
  rfl

theorem exampleFnHintMapDepends_direct_phi :
    fnHintMapDependsOnPhiInBlockExceptFuel 3 exampleFnHintMapRecord [] 0 7 99 := by
  exact exampleFnParamMetaDepends_direct_phi

theorem exampleFnHintMapDepends_exempt_phi_through_arg :
    fnHintMapDependsOnPhiInBlockExceptFuel 3 exampleFnHintMapRecord [] 1 7 1 := by
  exact exampleFnParamMetaDepends_exempt_phi_through_arg

theorem exampleFnHintMapDepends_other_block_ignored :
    ¬ fnHintMapDependsOnPhiInBlockExceptFuel 3 exampleFnHintMapRecord [] 2 7 99 := by
  exact exampleFnParamMetaDepends_other_block_ignored

end RRProofs
