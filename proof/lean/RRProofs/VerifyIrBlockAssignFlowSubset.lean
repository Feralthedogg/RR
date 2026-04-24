import RRProofs.VerifyIrBlockMustDefComposeSubset

namespace RRProofs

def exampleAssignOutDefs : MustDefBlockId -> DefSet
  | 1 => ["y"]
  | 2 => ["y", "z"]
  | _ => []

theorem exampleAssignJoinContainsY :
    "y" ∈ inDefsFromPreds 0 [] examplePreds exampleAssignOutDefs 3 := by
  apply mem_inDefsFromPreds_of_forall_pred
  · decide
  · simp [examplePreds]
  · intro pred hPred
    simp [examplePreds] at hPred ⊢
    rcases hPred with rfl | rfl
    · simp [exampleAssignOutDefs]
    · simp [exampleAssignOutDefs]

def exampleAssignDrivenBlock : ActualBlockRecordLite :=
  exampleGoodActualBlock

theorem exampleAssignDrivenBlock_rawRequired :
    rawRequiredVarsOfBlock exampleActualValueFullTable exampleAssignDrivenBlock = ["y"] := by
  rfl

theorem exampleAssignDrivenBlock_clean_of_y
    {defs : DefSet}
    (hMem : "y" ∈ defs) :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable defs exampleAssignDrivenBlock).verifyFlow = none := by
  apply rawBlockFlow_none_of_required_subset
  intro v hv
  rw [exampleAssignDrivenBlock_rawRequired] at hv
  simp at hv
  simpa [hv] using hMem

theorem exampleAssignDrivenBlock_clean_from_join :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignOutDefs 3)
      exampleAssignDrivenBlock).verifyFlow = none := by
  exact exampleAssignDrivenBlock_clean_of_y exampleAssignJoinContainsY

def exampleAssignJoinFlowLiteCase : VerifyIrFlowLiteCase :=
  flowLiteCaseOfFnBlock exampleFlowBase exampleGoodFnBlockRecord
    [inDefsFromPreds 0 [] examplePreds exampleAssignOutDefs 3]

theorem exampleAssignJoinFlowLiteCase_accepts :
    exampleAssignJoinFlowLiteCase.verifyIrFlowLite = none := by
  native_decide

end RRProofs
