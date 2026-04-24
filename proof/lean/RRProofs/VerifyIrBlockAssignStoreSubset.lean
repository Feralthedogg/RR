import RRProofs.VerifyIrBlockAssignChainSubset

namespace RRProofs

def exampleAssignStoreOutDefs : MustDefBlockId -> DefSet
  | 1 => ["y"]
  | 2 => ["y", "tmp"]
  | _ => []

theorem exampleAssignStoreJoinContainsY :
    "y" ∈ inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3 := by
  apply mem_inDefsFromPreds_of_forall_pred
  · decide
  · simp [examplePreds]
  · intro pred hPred
    simp [examplePreds] at hPred ⊢
    rcases hPred with rfl | rfl
    · simp [exampleAssignStoreOutDefs]
    · simp [exampleAssignStoreOutDefs]

def exampleAssignStore1DBlock : ActualBlockRecordLite :=
  { id := 60
  , instrs :=
      [ .assign "loop" 4 .source
      , .assign "x" 6 .source
      , .storeIndex1D 3 4 3 .source
      ]
  , term := .unreachable
  }

def exampleAssignStore2DBlock : ActualBlockRecordLite :=
  { id := 61
  , instrs :=
      [ .assign "loop" 4 .source
      , .assign "x" 6 .source
      , .storeIndex2D 3 4 4 3 .source
      ]
  , term := .unreachable
  }

def exampleAssignStore3DBlock : ActualBlockRecordLite :=
  { id := 62
  , instrs :=
      [ .assign "loop" 4 .source
      , .assign "x" 6 .source
      , .storeIndex3D 3 4 4 4 3 .source
      ]
  , term := .unreachable
  }

theorem exampleAssignStore1DBlock_rawRequired :
    rawRequiredVarsOfBlock exampleActualValueFullTable exampleAssignStore1DBlock = ["y", "y"] := by
  rfl

theorem exampleAssignStore2DBlock_rawRequired :
    rawRequiredVarsOfBlock exampleActualValueFullTable exampleAssignStore2DBlock = ["y", "y", "y"] := by
  rfl

theorem exampleAssignStore3DBlock_rawRequired :
    rawRequiredVarsOfBlock exampleActualValueFullTable exampleAssignStore3DBlock = ["y", "y", "y", "y"] := by
  rfl

theorem exampleAssignStore1DBlock_clean_of_y
    {defs : DefSet}
    (hMem : "y" ∈ defs) :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable defs exampleAssignStore1DBlock).verifyFlow = none := by
  apply rawBlockFlow_none_of_required_subset
  intro v hv
  rw [exampleAssignStore1DBlock_rawRequired] at hv
  simp at hv
  simpa [hv] using hMem

theorem exampleAssignStore2DBlock_clean_of_y
    {defs : DefSet}
    (hMem : "y" ∈ defs) :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable defs exampleAssignStore2DBlock).verifyFlow = none := by
  apply rawBlockFlow_none_of_required_subset
  intro v hv
  rw [exampleAssignStore2DBlock_rawRequired] at hv
  simp at hv
  simpa [hv] using hMem

theorem exampleAssignStore3DBlock_clean_of_y
    {defs : DefSet}
    (hMem : "y" ∈ defs) :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable defs exampleAssignStore3DBlock).verifyFlow = none := by
  apply rawBlockFlow_none_of_required_subset
  intro v hv
  rw [exampleAssignStore3DBlock_rawRequired] at hv
  simp at hv
  simpa [hv] using hMem

theorem exampleAssignStore1DBlock_clean_from_join :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3)
      exampleAssignStore1DBlock).verifyFlow = none := by
  exact exampleAssignStore1DBlock_clean_of_y exampleAssignStoreJoinContainsY

theorem exampleAssignStore2DBlock_clean_from_join :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3)
      exampleAssignStore2DBlock).verifyFlow = none := by
  exact exampleAssignStore2DBlock_clean_of_y exampleAssignStoreJoinContainsY

theorem exampleAssignStore3DBlock_clean_from_join :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3)
      exampleAssignStore3DBlock).verifyFlow = none := by
  exact exampleAssignStore3DBlock_clean_of_y exampleAssignStoreJoinContainsY

def exampleAssignStore1DFnBlockRecord : FnBlockRecordLite :=
  { shell := exampleFnHintMapRecord
  , blocks := [exampleAssignStore1DBlock]
  }

def exampleAssignStore2DFnBlockRecord : FnBlockRecordLite :=
  { shell := exampleFnHintMapRecord
  , blocks := [exampleAssignStore2DBlock]
  }

def exampleAssignStore3DFnBlockRecord : FnBlockRecordLite :=
  { shell := exampleFnHintMapRecord
  , blocks := [exampleAssignStore3DBlock]
  }

def exampleAssignStore1DFlowLiteCase : VerifyIrFlowLiteCase :=
  flowLiteCaseOfFnBlock exampleFlowBase exampleAssignStore1DFnBlockRecord
    [inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3]

def exampleAssignStore2DFlowLiteCase : VerifyIrFlowLiteCase :=
  flowLiteCaseOfFnBlock exampleFlowBase exampleAssignStore2DFnBlockRecord
    [inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3]

def exampleAssignStore3DFlowLiteCase : VerifyIrFlowLiteCase :=
  flowLiteCaseOfFnBlock exampleFlowBase exampleAssignStore3DFnBlockRecord
    [inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3]

theorem exampleAssignStore1DFlowLiteCase_accepts :
    exampleAssignStore1DFlowLiteCase.verifyIrFlowLite = none := by
  native_decide

theorem exampleAssignStore2DFlowLiteCase_accepts :
    exampleAssignStore2DFlowLiteCase.verifyIrFlowLite = none := by
  native_decide

theorem exampleAssignStore3DFlowLiteCase_accepts :
    exampleAssignStore3DFlowLiteCase.verifyIrFlowLite = none := by
  native_decide

end RRProofs
