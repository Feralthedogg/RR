import RRProofs.VerifyIrJoinExecutableSubset

set_option linter.unusedVariables false

namespace RRProofs

structure JoinCfgWitnessLite where
  base : VerifyIrStructLiteCase
  table : ActualValueFullTableLite
  defsLeft : DefSet
  left : ActualBlockRecordLite
  defsRight : DefSet
  right : ActualBlockRecordLite
  defsJoin : DefSet
  join : ActualBlockRecordLite
  joinPreds : List Nat
  blockOrder : List Nat

def JoinCfgWitnessLite.toFlowCase (w : JoinCfgWitnessLite) : VerifyIrFlowLiteCase :=
  flowLiteJoinCase w.base w.table w.defsLeft w.left w.defsRight w.right w.defsJoin w.join

def JoinCfgWitnessLite.predsOk (w : JoinCfgWitnessLite) : Prop :=
  w.joinPreds = [w.left.id, w.right.id]

def JoinCfgWitnessLite.orderOk (w : JoinCfgWitnessLite) : Prop :=
  w.blockOrder = [w.left.id, w.right.id, w.join.id]

theorem JoinCfgWitnessLite.accepts_of_rawBlocks_none
    {w : JoinCfgWitnessLite}
    (hPreds : w.predsOk)
    (hOrder : w.orderOk)
    (hBase : w.base.verifyIrStructLite = none)
    (hLeft : (rawFlowCaseOfActualBlock w.table w.defsLeft w.left).verifyFlow = none)
    (hRight : (rawFlowCaseOfActualBlock w.table w.defsRight w.right).verifyFlow = none)
    (hJoin : (rawFlowCaseOfActualBlock w.table w.defsJoin w.join).verifyFlow = none) :
    w.toFlowCase.verifyIrFlowLite = none := by
  exact flowLiteJoinCase_accepts_of_rawBlocks_none hBase hLeft hRight hJoin

theorem JoinCfgWitnessLite.accepts_and_preserves_init
    {w : JoinCfgWitnessLite}
    {vLeft vRight vJoin : Var}
    (hPreds : w.predsOk)
    (hOrder : w.orderOk)
    (hBase : w.base.verifyIrStructLite = none)
    (hLeft : (rawFlowCaseOfActualBlock w.table w.defsLeft w.left).verifyFlow = none)
    (hRight : (rawFlowCaseOfActualBlock w.table w.defsRight w.right).verifyFlow = none)
    (hJoin : (rawFlowCaseOfActualBlock w.table w.defsJoin w.join).verifyFlow = none)
    (hMemLeft : vLeft ∈ w.defsLeft)
    (hMemRight : vRight ∈ w.defsRight)
    (hMemJoin : vJoin ∈ w.defsJoin) :
    w.toFlowCase.verifyIrFlowLite = none ∧
      vLeft ∈ finalDefinedVars w.defsLeft w.left ∧
      vRight ∈ finalDefinedVars w.defsRight w.right ∧
      vJoin ∈ finalDefinedVars w.defsJoin w.join := by
  exact flowLiteJoinCase_accepts_and_preserves_init hBase hLeft hRight hJoin hMemLeft hMemRight hMemJoin

def exampleJoinCfgWitness : JoinCfgWitnessLite :=
  { base := exampleFlowBase
  , table := exampleActualValueFullTable
  , defsLeft := inDefsFromPreds 0 [] examplePreds exampleAssignChainOutDefs 3
  , left := exampleAssignChainBlock
  , defsRight := inDefsFromPreds 0 [] examplePreds exampleAssignBranchOutDefs 3
  , right := exampleAssignBranchBlock
  , defsJoin := inDefsFromPreds 0 [] examplePreds exampleTwoReadOutDefs 3
  , join := exampleMultiReadBlock
  , joinPreds := [40, 50]
  , blockOrder := [40, 50, 30]
  }

theorem exampleJoinCfgWitness_predsOk :
    exampleJoinCfgWitness.predsOk := by
  rfl

theorem exampleJoinCfgWitness_orderOk :
    exampleJoinCfgWitness.orderOk := by
  rfl

theorem exampleJoinCfgWitness_accepts :
    exampleJoinCfgWitness.toFlowCase.verifyIrFlowLite = none := by
  apply JoinCfgWitnessLite.accepts_of_rawBlocks_none
  · exact exampleJoinCfgWitness_predsOk
  · exact exampleJoinCfgWitness_orderOk
  · exact exampleFlowBase_struct_clean
  · exact exampleAssignChainBlock_clean_from_join
  · exact exampleAssignBranchBlock_clean_from_join
  · exact exampleMultiReadBlock_clean_from_join

theorem exampleJoinCfgWitness_preserves_incomingY :
    exampleJoinCfgWitness.toFlowCase.verifyIrFlowLite = none ∧
      "y" ∈ finalDefinedVars exampleJoinCfgWitness.defsLeft exampleJoinCfgWitness.left ∧
      "y" ∈ finalDefinedVars exampleJoinCfgWitness.defsRight exampleJoinCfgWitness.right ∧
      "y" ∈ finalDefinedVars exampleJoinCfgWitness.defsJoin exampleJoinCfgWitness.join := by
  apply JoinCfgWitnessLite.accepts_and_preserves_init
  · exact exampleJoinCfgWitness_predsOk
  · exact exampleJoinCfgWitness_orderOk
  · exact exampleFlowBase_struct_clean
  · exact exampleAssignChainBlock_clean_from_join
  · exact exampleAssignBranchBlock_clean_from_join
  · exact exampleMultiReadBlock_clean_from_join
  · exact exampleAssignChainJoinContainsY
  · exact exampleAssignBranchJoinContainsY
  · exact exampleTwoReadJoinContainsY

end RRProofs
