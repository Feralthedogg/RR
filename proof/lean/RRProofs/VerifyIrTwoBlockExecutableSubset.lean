import RRProofs.VerifyIrBlockExecutableSubset

namespace RRProofs

def flowLiteTwoBlockCase
    (base : VerifyIrStructLiteCase)
    (table : ActualValueFullTableLite)
    (defs1 : DefSet) (bb1 : ActualBlockRecordLite)
    (defs2 : DefSet) (bb2 : ActualBlockRecordLite) : VerifyIrFlowLiteCase :=
  { base := base
  , blocks :=
      [ rawFlowCaseOfActualBlock table defs1 bb1
      , rawFlowCaseOfActualBlock table defs2 bb2
      ]
  }

theorem flowLiteTwoBlockCase_accepts_of_rawBlocks_none
    {base : VerifyIrStructLiteCase}
    {table : ActualValueFullTableLite}
    {defs1 defs2 : DefSet}
    {bb1 bb2 : ActualBlockRecordLite}
    (hBase : base.verifyIrStructLite = none)
    (h1 : (rawFlowCaseOfActualBlock table defs1 bb1).verifyFlow = none)
    (h2 : (rawFlowCaseOfActualBlock table defs2 bb2).verifyFlow = none) :
    (flowLiteTwoBlockCase base table defs1 bb1 defs2 bb2).verifyIrFlowLite = none := by
  simp [flowLiteTwoBlockCase, VerifyIrFlowLiteCase.verifyIrFlowLite, verifyFlowBlocks,
    h1, h2, hBase]

theorem flowLiteTwoBlockCase_accepts_of_required_subset
    {base : VerifyIrStructLiteCase}
    {table : ActualValueFullTableLite}
    {defs1 defs2 : DefSet}
    {bb1 bb2 : ActualBlockRecordLite}
    (hBase : base.verifyIrStructLite = none)
    (hReq1 : ∀ v, v ∈ rawRequiredVarsOfBlock table bb1 -> v ∈ defs1)
    (hReq2 : ∀ v, v ∈ rawRequiredVarsOfBlock table bb2 -> v ∈ defs2) :
    (flowLiteTwoBlockCase base table defs1 bb1 defs2 bb2).verifyIrFlowLite = none := by
  apply flowLiteTwoBlockCase_accepts_of_rawBlocks_none hBase
  · exact rawBlockFlow_none_of_required_subset hReq1
  · exact rawBlockFlow_none_of_required_subset hReq2

theorem flowLiteTwoBlockCase_accepts_and_preserves_init
    {base : VerifyIrStructLiteCase}
    {table : ActualValueFullTableLite}
    {defs1 defs2 : DefSet}
    {bb1 bb2 : ActualBlockRecordLite}
    {v1 v2 : Var}
    (hBase : base.verifyIrStructLite = none)
    (h1 : (rawFlowCaseOfActualBlock table defs1 bb1).verifyFlow = none)
    (h2 : (rawFlowCaseOfActualBlock table defs2 bb2).verifyFlow = none)
    (hMem1 : v1 ∈ defs1)
    (hMem2 : v2 ∈ defs2) :
    let c := flowLiteTwoBlockCase base table defs1 bb1 defs2 bb2
    c.verifyIrFlowLite = none ∧
      v1 ∈ finalDefinedVars defs1 bb1 ∧
      v2 ∈ finalDefinedVars defs2 bb2 := by
  simp [flowLiteTwoBlockCase_accepts_of_rawBlocks_none hBase h1 h2,
    mem_finalDefinedVars_of_mem_init hMem1,
    mem_finalDefinedVars_of_mem_init hMem2]

theorem exampleTwoBlockExecutable_accepts :
    (flowLiteTwoBlockCase exampleFlowBase exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3) exampleAssignChainBlock
      (inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3) exampleAssignStore3DBlock
    ).verifyIrFlowLite = none := by
  apply flowLiteTwoBlockCase_accepts_of_rawBlocks_none
  · exact exampleFlowBase_struct_clean
  · exact exampleAssignChainBlock_clean_from_join
  · exact exampleAssignStore3DBlock_clean_from_join

theorem exampleTwoBlockExecutable_preserves_incomingY :
    let defs1 := inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3
    let defs2 := inDefsFromPreds 0 [] examplePreds exampleAssignStoreOutDefs 3
    let c := flowLiteTwoBlockCase exampleFlowBase exampleActualValueFullTable
      defs1 exampleAssignChainBlock defs2 exampleAssignStore3DBlock
    c.verifyIrFlowLite = none ∧
      "y" ∈ finalDefinedVars defs1 exampleAssignChainBlock ∧
      "y" ∈ finalDefinedVars defs2 exampleAssignStore3DBlock := by
  apply flowLiteTwoBlockCase_accepts_and_preserves_init
  · exact exampleFlowBase_struct_clean
  · exact exampleAssignChainBlock_clean_from_join
  · exact exampleAssignStore3DBlock_clean_from_join
  · exact exampleAssignChainJoinContainsY
  · exact exampleAssignStoreJoinContainsY

end RRProofs
