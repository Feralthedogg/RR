import RRProofs.VerifyIrEnvScanComposeSubset

namespace RRProofs

inductive ConsumerMeta where
  | call
      (env : ArgValueEnv) (phi arg : ArgValueId) (es : List ArgEnvExpr)
      (defined : DefSet) (vkEs : List VkExpr)
  | intrinsic
      (env : ArgValueEnv) (phi arg : ArgValueId) (es : List ArgEnvExpr)
      (defined : DefSet) (vkEs : List VkExpr)
  | recordLit
      (env : ArgValueEnv) (phi arg : ArgValueId) (fs : List ArgEnvField)
      (defined : DefSet) (vkFs : List FieldArg)

def ConsumerMeta.clean : ConsumerMeta -> Prop
  | .call env phi arg es defined vkEs =>
      (mkListComposeCase env phi arg es defined vkEs).allClean
  | .intrinsic env phi arg es defined vkEs =>
      (mkListComposeCase env phi arg es defined vkEs).allClean
  | .recordLit env phi arg fs defined vkFs =>
      (mkFieldComposeCase env phi arg fs defined vkFs).allClean

theorem ConsumerMeta.clean_call_of_clean
    (env : ArgValueEnv) (phi arg : ArgValueId) (es : List ArgEnvExpr)
    (defined : DefSet) (vkEs : List VkExpr)
    (hArgEnv : firstMissingArgEnvExprList (mergedArgEnv env phi arg) es = none)
    (hVk : firstUndefinedVkList defined vkEs = none) :
    ConsumerMeta.clean (.call env phi arg es defined vkEs) := by
  simpa [ConsumerMeta.clean] using
    mkListComposeCase_allClean_of_clean env phi arg es defined vkEs hArgEnv hVk

theorem ConsumerMeta.clean_call_of_defined
    (env : ArgValueEnv) (phi arg : ArgValueId) (es : List ArgEnvExpr)
    (defined : DefSet) (vkEs : List VkExpr)
    (hArgEnv : firstMissingArgEnvExprList (mergedArgEnv env phi arg) es = none)
    (hVk : loadsDefinedVkList defined vkEs) :
    ConsumerMeta.clean (.call env phi arg es defined vkEs) := by
  simpa [ConsumerMeta.clean] using
    mkListComposeCase_allClean_of_defined env phi arg es defined vkEs hArgEnv hVk

theorem ConsumerMeta.clean_intrinsic_of_clean
    (env : ArgValueEnv) (phi arg : ArgValueId) (es : List ArgEnvExpr)
    (defined : DefSet) (vkEs : List VkExpr)
    (hArgEnv : firstMissingArgEnvExprList (mergedArgEnv env phi arg) es = none)
    (hVk : firstUndefinedVkList defined vkEs = none) :
    ConsumerMeta.clean (.intrinsic env phi arg es defined vkEs) := by
  simpa [ConsumerMeta.clean] using
    mkListComposeCase_allClean_of_clean env phi arg es defined vkEs hArgEnv hVk

theorem ConsumerMeta.clean_intrinsic_of_defined
    (env : ArgValueEnv) (phi arg : ArgValueId) (es : List ArgEnvExpr)
    (defined : DefSet) (vkEs : List VkExpr)
    (hArgEnv : firstMissingArgEnvExprList (mergedArgEnv env phi arg) es = none)
    (hVk : loadsDefinedVkList defined vkEs) :
    ConsumerMeta.clean (.intrinsic env phi arg es defined vkEs) := by
  simpa [ConsumerMeta.clean] using
    mkListComposeCase_allClean_of_defined env phi arg es defined vkEs hArgEnv hVk

theorem ConsumerMeta.clean_recordLit_of_clean
    (env : ArgValueEnv) (phi arg : ArgValueId) (fs : List ArgEnvField)
    (defined : DefSet) (vkFs : List FieldArg)
    (hArgEnv : firstMissingArgEnvFields (mergedArgEnv env phi arg) fs = none)
    (hVk : firstUndefinedFieldArgs defined vkFs = none) :
    ConsumerMeta.clean (.recordLit env phi arg fs defined vkFs) := by
  simpa [ConsumerMeta.clean] using
    mkFieldComposeCase_allClean_of_clean env phi arg fs defined vkFs hArgEnv hVk

theorem ConsumerMeta.clean_recordLit_of_defined
    (env : ArgValueEnv) (phi arg : ArgValueId) (fs : List ArgEnvField)
    (defined : DefSet) (vkFs : List FieldArg)
    (hArgEnv : firstMissingArgEnvFields (mergedArgEnv env phi arg) fs = none)
    (hVk : fieldsDefined defined vkFs) :
    ConsumerMeta.clean (.recordLit env phi arg fs defined vkFs) := by
  simpa [ConsumerMeta.clean] using
    mkFieldComposeCase_allClean_of_defined env phi arg fs defined vkFs hArgEnv hVk

def exampleCallConsumer : ConsumerMeta :=
  .call
    exampleArgEnv 9 1 exampleCallArgEnvExprs
    (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
      5 exampleStableSeed 3)
    exampleCallArgs

def exampleIntrinsicConsumer : ConsumerMeta :=
  .intrinsic
    exampleArgEnv 9 1 exampleCallArgEnvExprs
    (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
      5 exampleStableSeed 3)
    exampleIntrinsicArgs

def exampleRecordConsumer : ConsumerMeta :=
  .recordLit
    exampleArgEnv 12 7 exampleRecordArgEnvFields
    (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
      5 exampleStableSeed 3)
    exampleRecordFields

theorem exampleCallConsumer_clean :
    exampleCallConsumer.clean := by
  simpa [exampleCallConsumer] using
    ConsumerMeta.clean_call_of_clean
      exampleArgEnv 9 1 exampleCallArgEnvExprs
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleCallArgs
      exampleCallArgEnvExprs_scan_clean_from_selected_eval
      exampleCallArgs_scan_clean

theorem exampleIntrinsicConsumer_clean :
    exampleIntrinsicConsumer.clean := by
  simpa [exampleIntrinsicConsumer] using
    ConsumerMeta.clean_intrinsic_of_clean
      exampleArgEnv 9 1 exampleCallArgEnvExprs
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleIntrinsicArgs
      exampleCallArgEnvExprs_scan_clean_from_selected_eval
      exampleIntrinsicArgs_scan_clean

theorem exampleRecordConsumer_clean :
    exampleRecordConsumer.clean := by
  simpa [exampleRecordConsumer] using
    ConsumerMeta.clean_recordLit_of_clean
      exampleArgEnv 12 7 exampleRecordArgEnvFields
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleRecordFields
      exampleRecordArgEnvFields_scan_clean_from_selected_eval
      exampleRecordFields_scan_clean

end RRProofs
