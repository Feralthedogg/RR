import RRProofs.VerifyIrBlockAssignChainSubset

namespace RRProofs

def exampleAssignBranchOutDefs : MustDefBlockId -> DefSet
  | 1 => ["y"]
  | 2 => ["y", "guard"]
  | _ => []

theorem exampleAssignBranchJoinContainsY :
    "y" ∈ inDefsFromPreds 0 [] examplePreds exampleAssignBranchOutDefs 3 := by
  apply mem_inDefsFromPreds_of_forall_pred
  · decide
  · simp [examplePreds]
  · intro pred hPred
    simp [examplePreds] at hPred ⊢
    rcases hPred with rfl | rfl
    · simp [exampleAssignBranchOutDefs]
    · simp [exampleAssignBranchOutDefs]

def exampleAssignBranchBlock : ActualBlockRecordLite :=
  { id := 50
  , instrs :=
      [ .assign "loop" 4 .source
      , .assign "x" 6 .source
      ]
  , term := .branch 3 1 2
  }

theorem exampleAssignBranchBlock_rawRequired :
    rawRequiredVarsOfBlock exampleActualValueFullTable exampleAssignBranchBlock = ["y"] := by
  rfl

theorem exampleAssignBranchBlock_clean_of_y
    {defs : DefSet}
    (hMem : "y" ∈ defs) :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable defs exampleAssignBranchBlock).verifyFlow = none := by
  apply rawBlockFlow_none_of_required_subset
  intro v hv
  rw [exampleAssignBranchBlock_rawRequired] at hv
  simp at hv
  simpa [hv] using hMem

theorem exampleAssignBranchBlock_clean_from_join :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignBranchOutDefs 3)
      exampleAssignBranchBlock).verifyFlow = none := by
  exact exampleAssignBranchBlock_clean_of_y exampleAssignBranchJoinContainsY

def exampleAssignBranchFnBlockRecord : FnBlockRecordLite :=
  { shell := exampleFnHintMapRecord
  , blocks := [exampleAssignBranchBlock]
  }

def exampleAssignBranchFlowLiteCase : VerifyIrFlowLiteCase :=
  flowLiteCaseOfFnBlock exampleFlowBase exampleAssignBranchFnBlockRecord
    [inDefsFromPreds 0 [] examplePreds exampleAssignBranchOutDefs 3]

theorem exampleAssignBranchFlowLiteCase_accepts :
    exampleAssignBranchFlowLiteCase.verifyIrFlowLite = none := by
  native_decide

end RRProofs
