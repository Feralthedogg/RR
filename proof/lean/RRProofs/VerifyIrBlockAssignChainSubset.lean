import RRProofs.VerifyIrBlockAssignFlowSubset

namespace RRProofs

def exampleAssignChainOutDefs : MustDefBlockId -> DefSet
  | 1 => ["y"]
  | 2 => ["y", "tmp"]
  | _ => []

theorem exampleAssignChainJoinContainsY :
    "y" ∈ inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3 := by
  apply mem_inDefsFromPreds_of_forall_pred
  · decide
  · simp [examplePreds]
  · intro pred hPred
    simp [examplePreds] at hPred ⊢
    rcases hPred with rfl | rfl
    · simp [exampleAssignChainOutDefs]
    · simp [exampleAssignChainOutDefs]

def exampleAssignChainBlock : ActualBlockRecordLite :=
  { id := 40
  , instrs :=
      [ .assign "loop" 4 .source
      , .assign "x" 6 .source
      , .eval 3 .source
      ]
  , term := .ret (some 3)
  }

theorem exampleAssignChainBlock_rawRequired :
    rawRequiredVarsOfBlock exampleActualValueFullTable exampleAssignChainBlock = ["y"] := by
  rfl

theorem exampleAssignChainBlock_clean_of_y
    {defs : DefSet}
    (hMem : "y" ∈ defs) :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable defs exampleAssignChainBlock).verifyFlow = none := by
  apply rawBlockFlow_none_of_required_subset
  intro v hv
  rw [exampleAssignChainBlock_rawRequired] at hv
  simp at hv
  simpa [hv] using hMem

theorem exampleAssignChainBlock_clean_from_join :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3)
      exampleAssignChainBlock).verifyFlow = none := by
  exact exampleAssignChainBlock_clean_of_y exampleAssignChainJoinContainsY

def exampleAssignChainFnBlockRecord : FnBlockRecordLite :=
  { shell := exampleFnHintMapRecord
  , blocks := [exampleAssignChainBlock]
  }

def exampleAssignChainFlowLiteCase : VerifyIrFlowLiteCase :=
  flowLiteCaseOfFnBlock exampleFlowBase exampleAssignChainFnBlockRecord
    [inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3]

theorem exampleAssignChainFlowLiteCase_accepts :
    exampleAssignChainFlowLiteCase.verifyIrFlowLite = none := by
  native_decide

end RRProofs
