import RRProofs.VerifyIrCfgExecutableSubset
import RRProofs.VerifyIrMustDefFixedPointSubset

set_option linter.unusedVariables false

namespace RRProofs

structure JoinCfgReachabilityWitnessLite where
  cfg : JoinCfgWitnessLite
  reachable : ReachableMap
  preds : PredMap
  outDefs : DefMap
  entry : MustDefBlockId
  entryDefs : DefSet

def JoinCfgReachabilityWitnessLite.joinReachableOk (w : JoinCfgReachabilityWitnessLite) : Prop :=
  w.reachable w.cfg.join.id = true

def JoinCfgReachabilityWitnessLite.joinPredsOk (w : JoinCfgReachabilityWitnessLite) : Prop :=
  reachablePreds w.reachable w.preds w.cfg.join.id = w.cfg.joinPreds

def JoinCfgReachabilityWitnessLite.joinStepInDefsOk (w : JoinCfgReachabilityWitnessLite) : Prop :=
  stepInDefs w.entry w.entryDefs w.reachable w.preds w.outDefs w.cfg.join.id = w.cfg.defsJoin

theorem JoinCfgReachabilityWitnessLite.accepts_of_join_stepInDefs
    {w : JoinCfgReachabilityWitnessLite}
    (hCfgPreds : w.cfg.predsOk)
    (hOrder : w.cfg.orderOk)
    (hReach : w.joinReachableOk)
    (hPredMap : w.joinPredsOk)
    (hJoinDefs : w.joinStepInDefsOk)
    (hBase : w.cfg.base.verifyIrStructLite = none)
    (hLeft : (rawFlowCaseOfActualBlock w.cfg.table w.cfg.defsLeft w.cfg.left).verifyFlow = none)
    (hRight : (rawFlowCaseOfActualBlock w.cfg.table w.cfg.defsRight w.cfg.right).verifyFlow = none)
    (hJoinReq : ∀ v, v ∈ rawRequiredVarsOfBlock w.cfg.table w.cfg.join ->
      v ∈ stepInDefs w.entry w.entryDefs w.reachable w.preds w.outDefs w.cfg.join.id) :
    w.cfg.toFlowCase.verifyIrFlowLite = none := by
  have hJoin : (rawFlowCaseOfActualBlock w.cfg.table w.cfg.defsJoin w.cfg.join).verifyFlow = none := by
    rw [← hJoinDefs]
    exact rawBlockFlow_none_of_required_subset hJoinReq
  exact w.cfg.accepts_of_rawBlocks_none hCfgPreds hOrder hBase hLeft hRight hJoin

def exampleCfgReachable : ReachableMap
  | 40 => true
  | 50 => true
  | 30 => true
  | _ => false

def exampleCfgPredMap : PredMap
  | 30 => [40, 50]
  | _ => []

def exampleCfgOutDefs : DefMap
  | 40 => ["x", "y"]
  | 50 => ["x", "y"]
  | _ => []

def exampleJoinCfgReachabilityWitness : JoinCfgReachabilityWitnessLite :=
  { cfg := exampleJoinCfgWitness
  , reachable := exampleCfgReachable
  , preds := exampleCfgPredMap
  , outDefs := exampleCfgOutDefs
  , entry := 0
  , entryDefs := []
  }

theorem exampleJoinCfgReachabilityWitness_joinReachableOk :
    exampleJoinCfgReachabilityWitness.joinReachableOk := by
  rfl

theorem exampleJoinCfgReachabilityWitness_joinPredsOk :
    exampleJoinCfgReachabilityWitness.joinPredsOk := by
  rfl

theorem exampleJoinCfgReachabilityWitness_joinStepInDefsOk :
    exampleJoinCfgReachabilityWitness.joinStepInDefsOk := by
  rfl

theorem exampleJoinCfgReachabilityWitness_joinReachablePredsNonempty :
    reachablePreds exampleJoinCfgReachabilityWitness.reachable
      exampleJoinCfgReachabilityWitness.preds
      exampleJoinCfgReachabilityWitness.cfg.join.id ≠ [] := by
  native_decide

theorem exampleJoinCfgReachabilityWitness_joinReq :
    ∀ v, v ∈ rawRequiredVarsOfBlock exampleJoinCfgReachabilityWitness.cfg.table
      exampleJoinCfgReachabilityWitness.cfg.join ->
      v ∈ stepInDefs exampleJoinCfgReachabilityWitness.entry
        exampleJoinCfgReachabilityWitness.entryDefs
        exampleJoinCfgReachabilityWitness.reachable
        exampleJoinCfgReachabilityWitness.preds
        exampleJoinCfgReachabilityWitness.outDefs
        exampleJoinCfgReachabilityWitness.cfg.join.id := by
  intro v hv
  change v ∈ rawRequiredVarsOfBlock exampleActualValueFullTable exampleMultiReadBlock at hv
  rw [exampleMultiReadBlock_rawRequired] at hv
  simp at hv
  rcases hv with rfl | rfl | rfl
  · apply mem_stepInDefs_of_forall_reachable_pred
    · exact exampleJoinCfgReachabilityWitness_joinReachableOk
    · decide
    · exact exampleJoinCfgReachabilityWitness_joinReachablePredsNonempty
    · intro pred hPred
      have hPred'' := hPred
      simp [exampleJoinCfgReachabilityWitness, exampleCfgReachable, exampleCfgPredMap,
        reachablePreds] at hPred''
      have hPred' : pred = 40 ∨ pred = 50 := by
        simpa [exampleJoinCfgWitness, exampleMultiReadBlock] using hPred''.1
      rcases hPred' with rfl | rfl
      · native_decide
      · native_decide
  · apply mem_stepInDefs_of_forall_reachable_pred
    · exact exampleJoinCfgReachabilityWitness_joinReachableOk
    · decide
    · exact exampleJoinCfgReachabilityWitness_joinReachablePredsNonempty
    · intro pred hPred
      have hPred'' := hPred
      simp [exampleJoinCfgReachabilityWitness, exampleCfgReachable, exampleCfgPredMap,
        reachablePreds] at hPred''
      have hPred' : pred = 40 ∨ pred = 50 := by
        simpa [exampleJoinCfgWitness, exampleMultiReadBlock] using hPred''.1
      rcases hPred' with rfl | rfl
      · native_decide
      · native_decide
  · apply mem_stepInDefs_of_forall_reachable_pred
    · exact exampleJoinCfgReachabilityWitness_joinReachableOk
    · decide
    · exact exampleJoinCfgReachabilityWitness_joinReachablePredsNonempty
    · intro pred hPred
      have hPred'' := hPred
      simp [exampleJoinCfgReachabilityWitness, exampleCfgReachable, exampleCfgPredMap,
        reachablePreds] at hPred''
      have hPred' : pred = 40 ∨ pred = 50 := by
        simpa [exampleJoinCfgWitness, exampleMultiReadBlock] using hPred''.1
      rcases hPred' with rfl | rfl
      · native_decide
      · native_decide

theorem exampleJoinCfgReachabilityWitness_accepts :
    exampleJoinCfgReachabilityWitness.cfg.toFlowCase.verifyIrFlowLite = none := by
  apply JoinCfgReachabilityWitnessLite.accepts_of_join_stepInDefs
  · exact exampleJoinCfgWitness_predsOk
  · exact exampleJoinCfgWitness_orderOk
  · exact exampleJoinCfgReachabilityWitness_joinReachableOk
  · exact exampleJoinCfgReachabilityWitness_joinPredsOk
  · exact exampleJoinCfgReachabilityWitness_joinStepInDefsOk
  · exact exampleFlowBase_struct_clean
  · exact exampleAssignChainBlock_clean_from_join
  · exact exampleAssignBranchBlock_clean_from_join
  · exact exampleJoinCfgReachabilityWitness_joinReq

end RRProofs
