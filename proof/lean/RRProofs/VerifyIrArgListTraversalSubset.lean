import RRProofs.VerifyIrValueKindTraversalSubset

namespace RRProofs

abbrev FieldArg := String × VkExpr

def firstUndefinedVkList (defined : DefSet) : List VkExpr -> Option Var
  | [] => none
  | e :: rest =>
      match firstUndefinedVk defined e with
      | some v => some v
      | none => firstUndefinedVkList defined rest

def loadsDefinedVkList (defined : DefSet) : List VkExpr -> Prop
  | [] => True
  | e :: rest => loadsDefinedVk defined e ∧ loadsDefinedVkList defined rest

def firstUndefinedFieldArgs (defined : DefSet) : List FieldArg -> Option Var
  | [] => none
  | (_, e) :: rest =>
      match firstUndefinedVk defined e with
      | some v => some v
      | none => firstUndefinedFieldArgs defined rest

def fieldsDefined (defined : DefSet) : List FieldArg -> Prop
  | [] => True
  | (_, e) :: rest => loadsDefinedVk defined e ∧ fieldsDefined defined rest

theorem firstUndefinedVkList_none_of_loadsDefined (defined : DefSet) :
    ∀ es, loadsDefinedVkList defined es -> firstUndefinedVkList defined es = none
  | [], _ => rfl
  | e :: rest, h => by
      rcases h with ⟨hHead, hRest⟩
      simp [firstUndefinedVkList, firstUndefinedVk_none_of_loadsDefined defined e hHead,
        firstUndefinedVkList_none_of_loadsDefined defined rest hRest]

theorem firstUndefinedFieldArgs_none_of_fieldsDefined (defined : DefSet) :
    ∀ fs, fieldsDefined defined fs -> firstUndefinedFieldArgs defined fs = none
  | [], _ => rfl
  | (_, e) :: rest, h => by
      rcases h with ⟨hHead, hRest⟩
      simp [firstUndefinedFieldArgs, firstUndefinedVk_none_of_loadsDefined defined e hHead,
        firstUndefinedFieldArgs_none_of_fieldsDefined defined rest hRest]

def exampleCallArgs : List VkExpr :=
  [ .load "x"
  , .binary (.load "tmp") (.load "x")
  , .fieldGet (.load "tmp")
  ]

def exampleIntrinsicArgs : List VkExpr :=
  [ .load "x"
  , .unary (.load "tmp")
  , .range (.load "x") (.load "tmp")
  ]

def exampleRecordFields : List FieldArg :=
  [ ("a", .load "x")
  , ("b", .fieldGet (.load "tmp"))
  , ("c", .binary (.load "x") (.load "tmp"))
  ]

theorem exampleCallArgs_scan_clean :
    firstUndefinedVkList
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleCallArgs = none := by
  rw [exampleStableSeed_iterate_five_block3]
  exact firstUndefinedVkList_none_of_loadsDefined _ _ (by
    simp [exampleCallArgs, loadsDefinedVkList, loadsDefinedVk])

theorem exampleIntrinsicArgs_scan_clean :
    firstUndefinedVkList
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleIntrinsicArgs = none := by
  rw [exampleStableSeed_iterate_five_block3]
  exact firstUndefinedVkList_none_of_loadsDefined _ _ (by
    simp [exampleIntrinsicArgs, loadsDefinedVkList, loadsDefinedVk])

theorem exampleRecordFields_scan_clean :
    firstUndefinedFieldArgs
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      exampleRecordFields = none := by
  rw [exampleStableSeed_iterate_five_block3]
  exact firstUndefinedFieldArgs_none_of_fieldsDefined _ _ (by
    simp [exampleRecordFields, fieldsDefined, loadsDefinedVk])

end RRProofs
