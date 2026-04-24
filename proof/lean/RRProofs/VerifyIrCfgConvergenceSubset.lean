import RRProofs.VerifyIrCfgReachabilitySubset
import RRProofs.VerifyIrMustDefConvergenceSubset

namespace RRProofs

structure JoinCfgConvergenceWitnessLite where
  base : JoinCfgReachabilityWitnessLite
  assigned : AssignMap
  seed : DefMap
  fuel : Nat

def JoinCfgConvergenceWitnessLite.seedStable (w : JoinCfgConvergenceWitnessLite) : Prop :=
  outMapStable w.base.entry w.base.entryDefs w.base.reachable w.base.preds w.assigned w.seed

def JoinCfgConvergenceWitnessLite.iteratedOutDefs (w : JoinCfgConvergenceWitnessLite) : DefMap :=
  iterateOutMap w.base.entry w.base.entryDefs w.base.reachable w.base.preds w.assigned w.fuel w.seed

def JoinCfgConvergenceWitnessLite.toReachabilityWitness
    (w : JoinCfgConvergenceWitnessLite) : JoinCfgReachabilityWitnessLite :=
  { cfg := w.base.cfg
  , reachable := w.base.reachable
  , preds := w.base.preds
  , outDefs := w.iteratedOutDefs
  , entry := w.base.entry
  , entryDefs := w.base.entryDefs
  }

theorem JoinCfgConvergenceWitnessLite.iteratedOutDefs_eq_seed_of_stable
    {w : JoinCfgConvergenceWitnessLite}
    (hStable : w.seedStable) :
    w.iteratedOutDefs = w.seed := by
  simpa [JoinCfgConvergenceWitnessLite.iteratedOutDefs] using
    (iterateOutMap_of_stable w.base.entry w.base.entryDefs
      w.base.reachable w.base.preds w.assigned w.seed hStable w.fuel)

theorem JoinCfgConvergenceWitnessLite.accepts_of_stable_seedStepInDefs
    {w : JoinCfgConvergenceWitnessLite}
    (hCfgPreds : w.base.cfg.predsOk)
    (hOrder : w.base.cfg.orderOk)
    (hReach : w.base.joinReachableOk)
    (hPredMap : w.base.joinPredsOk)
    (hStable : w.seedStable)
    (hSeedJoinDefs : stepInDefs w.base.entry w.base.entryDefs
      w.base.reachable w.base.preds w.seed w.base.cfg.join.id = w.base.cfg.defsJoin)
    (hBase : w.base.cfg.base.verifyIrStructLite = none)
    (hLeft : (rawFlowCaseOfActualBlock w.base.cfg.table
      w.base.cfg.defsLeft w.base.cfg.left).verifyFlow = none)
    (hRight : (rawFlowCaseOfActualBlock w.base.cfg.table
      w.base.cfg.defsRight w.base.cfg.right).verifyFlow = none)
    (hJoinReq : ∀ v, v ∈ rawRequiredVarsOfBlock w.base.cfg.table w.base.cfg.join ->
      v ∈ stepInDefs w.base.entry w.base.entryDefs
        w.base.reachable w.base.preds w.seed w.base.cfg.join.id) :
    w.base.cfg.toFlowCase.verifyIrFlowLite = none := by
  have hIterJoinDefs :
      stepInDefs w.base.entry w.base.entryDefs
        w.base.reachable w.base.preds w.iteratedOutDefs w.base.cfg.join.id =
        w.base.cfg.defsJoin := by
    rw [w.iteratedOutDefs_eq_seed_of_stable hStable]
    exact hSeedJoinDefs
  have hIterJoinReq :
      ∀ v, v ∈ rawRequiredVarsOfBlock w.base.cfg.table w.base.cfg.join ->
        v ∈ stepInDefs w.base.entry w.base.entryDefs
          w.base.reachable w.base.preds w.iteratedOutDefs w.base.cfg.join.id := by
    intro v hv
    rw [w.iteratedOutDefs_eq_seed_of_stable hStable]
    exact hJoinReq v hv
  simpa [JoinCfgConvergenceWitnessLite.toReachabilityWitness] using
    (JoinCfgReachabilityWitnessLite.accepts_of_join_stepInDefs
      (w := w.toReachabilityWitness)
      hCfgPreds hOrder hReach hPredMap hIterJoinDefs hBase hLeft hRight hIterJoinReq)

end RRProofs
