import RRProofs.VerifyIrBlockFlowSubset
import RRProofs.VerifyIrMustDefSubset

namespace RRProofs

def singletonEvalBlock (bid root : ConsumerNodeId) : ActualBlockRecordLite :=
  { id := bid
  , instrs := [.eval root .source]
  , term := .unreachable
  }

theorem singletonEvalReads_x :
    instrRecordReads exampleActualValueFullTable (.eval 3 .source) = ["x"] := by
  rfl

theorem singletonEvalBlock_rejects_without_defs :
    (flowCaseOfActualBlock exampleActualValueFullTable [] (singletonEvalBlock 20 3)).verifyFlow =
      some (.useBeforeDef "x") := by
  native_decide

theorem singletonEvalBlock_accepts_x_of_must_def
    {defs : DefSet}
    (hMem : "x" ∈ defs) :
    (flowCaseOfActualBlock exampleActualValueFullTable defs (singletonEvalBlock 20 3)).verifyFlow = none := by
  have hReq :
      blockRequiredVars exampleActualValueFullTable defs (singletonEvalBlock 20 3) = [] := by
    simp [singletonEvalBlock, blockRequiredVars, stepInstrFlow, singletonEvalReads_x,
      missingVars, terminatorRecordReads, hMem]
  rw [flowCaseOfActualBlock, FlowBlockCase.verifyFlow, hReq]
  simp [firstMissingVar]

theorem exampleJoinMustDef_singletonEval_clean :
    (flowCaseOfActualBlock exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleOutDefs 3)
      (singletonEvalBlock 20 3)).verifyFlow = none := by
  exact singletonEvalBlock_accepts_x_of_must_def example_join_contains_x

end RRProofs
