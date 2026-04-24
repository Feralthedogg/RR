import RRProofs.VerifyIrArgEnvTraversalSubset
import RRProofs.VerifyIrArgListTraversalSubset

namespace RRProofs

structure EnvScanComposeCase where
  argEnvListClean : Prop
  valueKindListClean : Prop
  argEnvFieldClean : Prop
  valueKindFieldClean : Prop

def EnvScanComposeCase.allClean (c : EnvScanComposeCase) : Prop :=
  c.argEnvListClean ∧ c.valueKindListClean ∧ c.argEnvFieldClean ∧ c.valueKindFieldClean

def crossCasesClean (callCase fieldCase : EnvScanComposeCase) : Prop :=
  callCase.argEnvListClean ∧
  callCase.valueKindListClean ∧
  fieldCase.argEnvFieldClean ∧
  fieldCase.valueKindFieldClean

theorem EnvScanComposeCase.allClean_of_components
    (c : EnvScanComposeCase)
    (hArgList : c.argEnvListClean)
    (hVkList : c.valueKindListClean)
    (hArgField : c.argEnvFieldClean)
    (hVkField : c.valueKindFieldClean) :
    c.allClean := by
  exact ⟨hArgList, hVkList, hArgField, hVkField⟩

theorem EnvScanComposeCase.components_of_allClean
    (c : EnvScanComposeCase)
    (h : c.allClean) :
    c.argEnvListClean ∧ c.valueKindListClean ∧ c.argEnvFieldClean ∧ c.valueKindFieldClean := h

theorem crossCasesClean_of_allClean
    (callCase fieldCase : EnvScanComposeCase)
    (hCall : callCase.allClean)
    (hField : fieldCase.allClean) :
    crossCasesClean callCase fieldCase := by
  rcases hCall with ⟨hArgList, hVkList, _, _⟩
  rcases hField with ⟨_, _, hArgField, hVkField⟩
  exact ⟨hArgList, hVkList, hArgField, hVkField⟩

theorem crossCasesClean_components
    (callCase fieldCase : EnvScanComposeCase)
    (h : crossCasesClean callCase fieldCase) :
    callCase.argEnvListClean ∧
    callCase.valueKindListClean ∧
    fieldCase.argEnvFieldClean ∧
    fieldCase.valueKindFieldClean := h

def mkListComposeCase
    (env : ArgValueEnv) (phi arg : ArgValueId) (es : List ArgEnvExpr)
    (defined : DefSet) (vkEs : List VkExpr) : EnvScanComposeCase :=
  { argEnvListClean := firstMissingArgEnvExprList (mergedArgEnv env phi arg) es = none
  , valueKindListClean := firstUndefinedVkList defined vkEs = none
  , argEnvFieldClean := True
  , valueKindFieldClean := True
  }

def mkFieldComposeCase
    (env : ArgValueEnv) (phi arg : ArgValueId) (fs : List ArgEnvField)
    (defined : DefSet) (vkFs : List FieldArg) : EnvScanComposeCase :=
  { argEnvListClean := True
  , valueKindListClean := True
  , argEnvFieldClean := firstMissingArgEnvFields (mergedArgEnv env phi arg) fs = none
  , valueKindFieldClean := firstUndefinedFieldArgs defined vkFs = none
  }

theorem mkListComposeCase_allClean_of_clean
    (env : ArgValueEnv) (phi arg : ArgValueId) (es : List ArgEnvExpr)
    (defined : DefSet) (vkEs : List VkExpr)
    (hArgEnv : firstMissingArgEnvExprList (mergedArgEnv env phi arg) es = none)
    (hVk : firstUndefinedVkList defined vkEs = none) :
    (mkListComposeCase env phi arg es defined vkEs).allClean := by
  apply EnvScanComposeCase.allClean_of_components
  · simpa [mkListComposeCase] using hArgEnv
  · simpa [mkListComposeCase] using hVk
  · simp [mkListComposeCase]
  · simp [mkListComposeCase]

theorem mkFieldComposeCase_allClean_of_clean
    (env : ArgValueEnv) (phi arg : ArgValueId) (fs : List ArgEnvField)
    (defined : DefSet) (vkFs : List FieldArg)
    (hArgEnv : firstMissingArgEnvFields (mergedArgEnv env phi arg) fs = none)
    (hVk : firstUndefinedFieldArgs defined vkFs = none) :
    (mkFieldComposeCase env phi arg fs defined vkFs).allClean := by
  apply EnvScanComposeCase.allClean_of_components
  · simp [mkFieldComposeCase]
  · simp [mkFieldComposeCase]
  · simpa [mkFieldComposeCase] using hArgEnv
  · simpa [mkFieldComposeCase] using hVk

theorem mkListComposeCase_allClean_of_defined
    (env : ArgValueEnv) (phi arg : ArgValueId) (es : List ArgEnvExpr)
    (defined : DefSet) (vkEs : List VkExpr)
    (hArgEnv : firstMissingArgEnvExprList (mergedArgEnv env phi arg) es = none)
    (hVk : loadsDefinedVkList defined vkEs) :
    (mkListComposeCase env phi arg es defined vkEs).allClean := by
  apply mkListComposeCase_allClean_of_clean
  · exact hArgEnv
  · exact firstUndefinedVkList_none_of_loadsDefined defined vkEs hVk

theorem mkFieldComposeCase_allClean_of_defined
    (env : ArgValueEnv) (phi arg : ArgValueId) (fs : List ArgEnvField)
    (defined : DefSet) (vkFs : List FieldArg)
    (hArgEnv : firstMissingArgEnvFields (mergedArgEnv env phi arg) fs = none)
    (hVk : fieldsDefined defined vkFs) :
    (mkFieldComposeCase env phi arg fs defined vkFs).allClean := by
  apply mkFieldComposeCase_allClean_of_clean
  · exact hArgEnv
  · exact firstUndefinedFieldArgs_none_of_fieldsDefined defined vkFs hVk

theorem mkCrossComposeClean_of_clean
    (callEnv : ArgValueEnv) (callPhi callArg : ArgValueId) (callEs : List ArgEnvExpr)
    (callDefined : DefSet) (callVkEs : List VkExpr)
    (fieldEnv : ArgValueEnv) (fieldPhi fieldArg : ArgValueId) (fieldFs : List ArgEnvField)
    (fieldDefined : DefSet) (fieldVkFs : List FieldArg)
    (hCallArgEnv :
      firstMissingArgEnvExprList (mergedArgEnv callEnv callPhi callArg) callEs = none)
    (hCallVk : firstUndefinedVkList callDefined callVkEs = none)
    (hFieldArgEnv :
      firstMissingArgEnvFields (mergedArgEnv fieldEnv fieldPhi fieldArg) fieldFs = none)
    (hFieldVk : firstUndefinedFieldArgs fieldDefined fieldVkFs = none) :
    crossCasesClean
      (mkListComposeCase callEnv callPhi callArg callEs callDefined callVkEs)
      (mkFieldComposeCase fieldEnv fieldPhi fieldArg fieldFs fieldDefined fieldVkFs) := by
  apply crossCasesClean_of_allClean
  · exact
      mkListComposeCase_allClean_of_clean
        callEnv callPhi callArg callEs callDefined callVkEs hCallArgEnv hCallVk
  · exact
      mkFieldComposeCase_allClean_of_clean
        fieldEnv fieldPhi fieldArg fieldFs fieldDefined fieldVkFs hFieldArgEnv hFieldVk

theorem mkCrossComposeClean_of_defined
    (callEnv : ArgValueEnv) (callPhi callArg : ArgValueId) (callEs : List ArgEnvExpr)
    (callDefined : DefSet) (callVkEs : List VkExpr)
    (fieldEnv : ArgValueEnv) (fieldPhi fieldArg : ArgValueId) (fieldFs : List ArgEnvField)
    (fieldDefined : DefSet) (fieldVkFs : List FieldArg)
    (hCallArgEnv :
      firstMissingArgEnvExprList (mergedArgEnv callEnv callPhi callArg) callEs = none)
    (hCallVk : loadsDefinedVkList callDefined callVkEs)
    (hFieldArgEnv :
      firstMissingArgEnvFields (mergedArgEnv fieldEnv fieldPhi fieldArg) fieldFs = none)
    (hFieldVk : fieldsDefined fieldDefined fieldVkFs) :
    crossCasesClean
      (mkListComposeCase callEnv callPhi callArg callEs callDefined callVkEs)
      (mkFieldComposeCase fieldEnv fieldPhi fieldArg fieldFs fieldDefined fieldVkFs) := by
  apply crossCasesClean_of_allClean
  · exact
      mkListComposeCase_allClean_of_defined
        callEnv callPhi callArg callEs callDefined callVkEs hCallArgEnv hCallVk
  · exact
      mkFieldComposeCase_allClean_of_defined
        fieldEnv fieldPhi fieldArg fieldFs fieldDefined fieldVkFs hFieldArgEnv hFieldVk

def callComposeCase : EnvScanComposeCase :=
  mkListComposeCase
    exampleArgEnv 9 1 exampleCallArgEnvExprs
    (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
      5 exampleStableSeed 3)
    exampleCallArgs

def fieldComposeCase : EnvScanComposeCase :=
  mkFieldComposeCase
    exampleArgEnv 12 7 exampleRecordArgEnvFields
    (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
      5 exampleStableSeed 3)
    exampleRecordFields

theorem callComposeCase_clean :
    callComposeCase.argEnvListClean ∧ callComposeCase.valueKindListClean := by
  constructor
  · simpa [callComposeCase] using exampleCallArgEnvExprs_scan_clean_from_selected_eval
  · simpa [callComposeCase] using exampleCallArgs_scan_clean

theorem fieldComposeCase_clean :
    fieldComposeCase.argEnvFieldClean ∧ fieldComposeCase.valueKindFieldClean := by
  constructor
  · simpa [fieldComposeCase] using exampleRecordArgEnvFields_scan_clean_from_selected_eval
  · simpa [fieldComposeCase] using exampleRecordFields_scan_clean

theorem composeCases_all_clean :
    callComposeCase.argEnvListClean ∧
    callComposeCase.valueKindListClean ∧
    fieldComposeCase.argEnvFieldClean ∧
    fieldComposeCase.valueKindFieldClean := by
  rcases callComposeCase_clean with ⟨hCallEnv, hCallVk⟩
  rcases fieldComposeCase_clean with ⟨hFieldEnv, hFieldVk⟩
  exact ⟨hCallEnv, hCallVk, hFieldEnv, hFieldVk⟩

theorem crossComposeCases_all_clean :
    crossCasesClean callComposeCase fieldComposeCase := by
  exact mkCrossComposeClean_of_clean
    exampleArgEnv 9 1 exampleCallArgEnvExprs
    (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
      5 exampleStableSeed 3)
    exampleCallArgs
    exampleArgEnv 12 7 exampleRecordArgEnvFields
    (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
      5 exampleStableSeed 3)
    exampleRecordFields
    exampleCallArgEnvExprs_scan_clean_from_selected_eval
    exampleCallArgs_scan_clean
    exampleRecordArgEnvFields_scan_clean_from_selected_eval
    exampleRecordFields_scan_clean

theorem callComposeCase_allClean :
    callComposeCase.allClean := by
  simpa [callComposeCase] using
    mkListComposeCase_allClean_of_clean
      exampleArgEnv 9 1 exampleCallArgEnvExprs
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleCallArgs
      exampleCallArgEnvExprs_scan_clean_from_selected_eval
      exampleCallArgs_scan_clean

theorem fieldComposeCase_allClean :
    fieldComposeCase.allClean := by
  simpa [fieldComposeCase] using
    mkFieldComposeCase_allClean_of_clean
      exampleArgEnv 12 7 exampleRecordArgEnvFields
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleRecordFields
      exampleRecordArgEnvFields_scan_clean_from_selected_eval
      exampleRecordFields_scan_clean

end RRProofs
