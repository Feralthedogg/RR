import RRProofs.VerifyIrBlockDefinedHereSubset
import RRProofs.VerifyIrBlockMustDefComposeSubset

namespace RRProofs

def flowLiteSingleBlockCase
    (base : VerifyIrStructLiteCase)
    (table : ActualValueFullTableLite)
    (defs : DefSet)
    (bb : ActualBlockRecordLite) : VerifyIrFlowLiteCase :=
  { base := base
  , blocks := [rawFlowCaseOfActualBlock table defs bb]
  }

theorem flowLiteSingleBlockCase_accepts_of_rawBlockFlow_none
    {base : VerifyIrStructLiteCase}
    {table : ActualValueFullTableLite}
    {defs : DefSet}
    {bb : ActualBlockRecordLite}
    (hBase : base.verifyIrStructLite = none)
    (hBlock : (rawFlowCaseOfActualBlock table defs bb).verifyFlow = none) :
    (flowLiteSingleBlockCase base table defs bb).verifyIrFlowLite = none := by
  simp [flowLiteSingleBlockCase, VerifyIrFlowLiteCase.verifyIrFlowLite,
    verifyFlowBlocks, hBlock, hBase]

theorem flowLiteSingleBlockCase_accepts_of_required_subset
    {base : VerifyIrStructLiteCase}
    {table : ActualValueFullTableLite}
    {defs : DefSet}
    {bb : ActualBlockRecordLite}
    (hBase : base.verifyIrStructLite = none)
    (hReq : ∀ v, v ∈ rawRequiredVarsOfBlock table bb -> v ∈ defs) :
    (flowLiteSingleBlockCase base table defs bb).verifyIrFlowLite = none := by
  apply flowLiteSingleBlockCase_accepts_of_rawBlockFlow_none hBase
  exact rawBlockFlow_none_of_required_subset hReq

theorem flowLiteSingleBlockCase_accepts_and_preserves_init
    {base : VerifyIrStructLiteCase}
    {table : ActualValueFullTableLite}
    {defs : DefSet}
    {bb : ActualBlockRecordLite}
    {v : Var}
    (hBase : base.verifyIrStructLite = none)
    (hBlock : (rawFlowCaseOfActualBlock table defs bb).verifyFlow = none)
    (hMem : v ∈ defs) :
    let c := flowLiteSingleBlockCase base table defs bb
    c.verifyIrFlowLite = none ∧ v ∈ finalDefinedVars defs bb := by
  simp [flowLiteSingleBlockCase_accepts_of_rawBlockFlow_none hBase hBlock,
    mem_finalDefinedVars_of_mem_init hMem]

theorem exampleFlowBase_struct_clean :
    exampleFlowBase.verifyIrStructLite = none := by
  simp [exampleFlowBase, VerifyIrStructLiteCase.verifyIrStructLite, verifyValueStructs,
    ValueStructCase.verifyStruct, exampleCleanStructBase, VerifyIrLiteCase.verifyIrLite]

theorem exampleAssignChainExecutable_accepts :
    (flowLiteSingleBlockCase exampleFlowBase exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3)
      exampleAssignChainBlock).verifyIrFlowLite = none := by
  exact flowLiteSingleBlockCase_accepts_of_rawBlockFlow_none
    exampleFlowBase_struct_clean
    exampleAssignChainBlock_clean_from_join

theorem exampleAssignBranchExecutable_accepts :
    (flowLiteSingleBlockCase exampleFlowBase exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignBranchOutDefs 3)
      exampleAssignBranchBlock).verifyIrFlowLite = none := by
  exact flowLiteSingleBlockCase_accepts_of_rawBlockFlow_none
    exampleFlowBase_struct_clean
    exampleAssignBranchBlock_clean_from_join

theorem exampleAssignStore3DExecutable_accepts :
    (flowLiteSingleBlockCase exampleFlowBase exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3)
      exampleAssignStore3DBlock).verifyIrFlowLite = none := by
  exact flowLiteSingleBlockCase_accepts_of_rawBlockFlow_none
    exampleFlowBase_struct_clean
    exampleAssignStore3DBlock_clean_from_join

theorem exampleAssignChainExecutable_preserves_incomingY :
    let defs := inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3
    let c := flowLiteSingleBlockCase exampleFlowBase exampleActualValueFullTable defs exampleAssignChainBlock
    c.verifyIrFlowLite = none ∧ "y" ∈ finalDefinedVars defs exampleAssignChainBlock := by
  apply flowLiteSingleBlockCase_accepts_and_preserves_init
  · exact exampleFlowBase_struct_clean
  · exact exampleAssignChainBlock_clean_from_join
  · exact exampleAssignChainJoinContainsY

end RRProofs
