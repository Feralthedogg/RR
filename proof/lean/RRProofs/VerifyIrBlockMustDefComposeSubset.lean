import RRProofs.VerifyIrBlockFlowSubset
import RRProofs.VerifyIrMustDefSubset

namespace RRProofs

def rawRequiredVarsOfBlock
    (table : ActualValueFullTableLite) (bb : ActualBlockRecordLite) : List Var :=
  blockRequiredVars table [] bb

def rawFlowCaseOfActualBlock
    (table : ActualValueFullTableLite) (defs : DefSet) (bb : ActualBlockRecordLite) : FlowBlockCase :=
  { defined := defs
  , required := rawRequiredVarsOfBlock table bb
  }

theorem firstMissingVar_none_of_required_subset
    {defs required : DefSet}
    (hAll : ∀ v, v ∈ required -> v ∈ defs) :
    firstMissingVar defs required = none := by
  induction required with
  | nil =>
      rfl
  | cons v rest ih =>
      have hv : v ∈ defs := hAll v (by simp)
      have hrest : ∀ u, u ∈ rest -> u ∈ defs := by
        intro u hu
        exact hAll u (by simp [hu])
      simpa [firstMissingVar, hv] using ih hrest

theorem verifyFlow_none_of_required_subset
    {defs required : DefSet}
    (hAll : ∀ v, v ∈ required -> v ∈ defs) :
    ({ defined := defs, required := required } : FlowBlockCase).verifyFlow = none := by
  simp [FlowBlockCase.verifyFlow, firstMissingVar_none_of_required_subset hAll]

theorem rawBlockFlow_none_of_required_subset
    {table : ActualValueFullTableLite} {defs : DefSet} {bb : ActualBlockRecordLite}
    (hAll : ∀ v, v ∈ rawRequiredVarsOfBlock table bb -> v ∈ defs) :
    (rawFlowCaseOfActualBlock table defs bb).verifyFlow = none := by
  exact verifyFlow_none_of_required_subset hAll

def exampleTwoReadOutDefs : MustDefBlockId -> DefSet
  | 1 => ["x", "y"]
  | 2 => ["x", "y", "z"]
  | _ => []

theorem exampleTwoReadJoinContainsX :
    "x" ∈ inDefsFromPreds 0 [] examplePreds exampleTwoReadOutDefs 3 := by
  apply mem_inDefsFromPreds_of_forall_pred
  · decide
  · simp [examplePreds]
  · intro pred hPred
    simp [examplePreds] at hPred ⊢
    rcases hPred with rfl | rfl
    · simp [exampleTwoReadOutDefs]
    · simp [exampleTwoReadOutDefs]

theorem exampleTwoReadJoinContainsY :
    "y" ∈ inDefsFromPreds 0 [] examplePreds exampleTwoReadOutDefs 3 := by
  apply mem_inDefsFromPreds_of_forall_pred
  · decide
  · simp [examplePreds]
  · intro pred hPred
    simp [examplePreds] at hPred ⊢
    rcases hPred with rfl | rfl
    · simp [exampleTwoReadOutDefs]
    · simp [exampleTwoReadOutDefs]

def exampleMultiReadBlock : ActualBlockRecordLite :=
  { id := 30
  , instrs := [.eval 3 .source, .eval 4 .source]
  , term := .ret (some 3)
  }

theorem exampleMultiReadBlock_rawRequired :
    rawRequiredVarsOfBlock exampleActualValueFullTable exampleMultiReadBlock = ["x", "y", "x"] := by
  rfl

theorem exampleMultiReadBlock_clean_from_join :
    (rawFlowCaseOfActualBlock exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleTwoReadOutDefs 3)
      exampleMultiReadBlock).verifyFlow = none := by
  apply rawBlockFlow_none_of_required_subset
  intro v hv
  rw [exampleMultiReadBlock_rawRequired] at hv
  simp at hv
  rcases hv with rfl | rfl | rfl
  · exact exampleTwoReadJoinContainsX
  · exact exampleTwoReadJoinContainsY
  · exact exampleTwoReadJoinContainsX

end RRProofs
