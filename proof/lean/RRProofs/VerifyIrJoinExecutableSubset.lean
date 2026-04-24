import RRProofs.VerifyIrTwoBlockExecutableSubset

namespace RRProofs

def flowLiteJoinCase
    (base : VerifyIrStructLiteCase)
    (table : ActualValueFullTableLite)
    (defsLeft : DefSet) (left : ActualBlockRecordLite)
    (defsRight : DefSet) (right : ActualBlockRecordLite)
    (defsJoin : DefSet) (join : ActualBlockRecordLite) : VerifyIrFlowLiteCase :=
  { base := base
  , blocks :=
      [ rawFlowCaseOfActualBlock table defsLeft left
      , rawFlowCaseOfActualBlock table defsRight right
      , rawFlowCaseOfActualBlock table defsJoin join
      ]
  }

theorem flowLiteJoinCase_accepts_of_rawBlocks_none
    {base : VerifyIrStructLiteCase}
    {table : ActualValueFullTableLite}
    {defsLeft defsRight defsJoin : DefSet}
    {left right join : ActualBlockRecordLite}
    (hBase : base.verifyIrStructLite = none)
    (hLeft : (rawFlowCaseOfActualBlock table defsLeft left).verifyFlow = none)
    (hRight : (rawFlowCaseOfActualBlock table defsRight right).verifyFlow = none)
    (hJoin : (rawFlowCaseOfActualBlock table defsJoin join).verifyFlow = none) :
    (flowLiteJoinCase base table defsLeft left defsRight right defsJoin join).verifyIrFlowLite = none := by
  simp [flowLiteJoinCase, VerifyIrFlowLiteCase.verifyIrFlowLite, verifyFlowBlocks,
    hLeft, hRight, hJoin, hBase]

theorem flowLiteJoinCase_accepts_of_required_subset
    {base : VerifyIrStructLiteCase}
    {table : ActualValueFullTableLite}
    {defsLeft defsRight defsJoin : DefSet}
    {left right join : ActualBlockRecordLite}
    (hBase : base.verifyIrStructLite = none)
    (hReqLeft : ∀ v, v ∈ rawRequiredVarsOfBlock table left -> v ∈ defsLeft)
    (hReqRight : ∀ v, v ∈ rawRequiredVarsOfBlock table right -> v ∈ defsRight)
    (hReqJoin : ∀ v, v ∈ rawRequiredVarsOfBlock table join -> v ∈ defsJoin) :
    (flowLiteJoinCase base table defsLeft left defsRight right defsJoin join).verifyIrFlowLite = none := by
  apply flowLiteJoinCase_accepts_of_rawBlocks_none hBase
  · exact rawBlockFlow_none_of_required_subset hReqLeft
  · exact rawBlockFlow_none_of_required_subset hReqRight
  · exact rawBlockFlow_none_of_required_subset hReqJoin

theorem flowLiteJoinCase_accepts_and_preserves_init
    {base : VerifyIrStructLiteCase}
    {table : ActualValueFullTableLite}
    {defsLeft defsRight defsJoin : DefSet}
    {left right join : ActualBlockRecordLite}
    {vLeft vRight vJoin : Var}
    (hBase : base.verifyIrStructLite = none)
    (hLeft : (rawFlowCaseOfActualBlock table defsLeft left).verifyFlow = none)
    (hRight : (rawFlowCaseOfActualBlock table defsRight right).verifyFlow = none)
    (hJoin : (rawFlowCaseOfActualBlock table defsJoin join).verifyFlow = none)
    (hMemLeft : vLeft ∈ defsLeft)
    (hMemRight : vRight ∈ defsRight)
    (hMemJoin : vJoin ∈ defsJoin) :
    let c := flowLiteJoinCase base table defsLeft left defsRight right defsJoin join
    c.verifyIrFlowLite = none ∧
      vLeft ∈ finalDefinedVars defsLeft left ∧
      vRight ∈ finalDefinedVars defsRight right ∧
      vJoin ∈ finalDefinedVars defsJoin join := by
  simp [flowLiteJoinCase_accepts_of_rawBlocks_none hBase hLeft hRight hJoin,
    mem_finalDefinedVars_of_mem_init hMemLeft,
    mem_finalDefinedVars_of_mem_init hMemRight,
    mem_finalDefinedVars_of_mem_init hMemJoin]

theorem exampleJoinExecutable_accepts :
    (flowLiteJoinCase exampleFlowBase exampleActualValueFullTable
      (inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3) exampleAssignChainBlock
      (inDefsFromPreds 0 [] examplePreds exampleAssignBranchOutDefs 3) exampleAssignBranchBlock
      (inDefsFromPreds 0 [] examplePreds exampleTwoReadOutDefs 3) exampleMultiReadBlock
    ).verifyIrFlowLite = none := by
  apply flowLiteJoinCase_accepts_of_rawBlocks_none
  · exact exampleFlowBase_struct_clean
  · exact exampleAssignChainBlock_clean_from_join
  · exact exampleAssignBranchBlock_clean_from_join
  · exact exampleMultiReadBlock_clean_from_join

theorem exampleJoinExecutable_preserves_incomingY :
    let defsLeft := inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3
    let defsRight := inDefsFromPreds 0 [] examplePreds exampleAssignBranchOutDefs 3
    let defsJoin := inDefsFromPreds 0 [] examplePreds exampleTwoReadOutDefs 3
    let c := flowLiteJoinCase exampleFlowBase exampleActualValueFullTable
      defsLeft exampleAssignChainBlock defsRight exampleAssignBranchBlock defsJoin exampleMultiReadBlock
    c.verifyIrFlowLite = none ∧
      "y" ∈ finalDefinedVars defsLeft exampleAssignChainBlock ∧
      "y" ∈ finalDefinedVars defsRight exampleAssignBranchBlock ∧
      "y" ∈ finalDefinedVars defsJoin exampleMultiReadBlock := by
  apply flowLiteJoinCase_accepts_and_preserves_init
  · exact exampleFlowBase_struct_clean
  · exact exampleAssignChainBlock_clean_from_join
  · exact exampleAssignBranchBlock_clean_from_join
  · exact exampleMultiReadBlock_clean_from_join
  · exact exampleAssignChainJoinContainsY
  · exact exampleAssignBranchJoinContainsY
  · exact exampleTwoReadJoinContainsY

end RRProofs
