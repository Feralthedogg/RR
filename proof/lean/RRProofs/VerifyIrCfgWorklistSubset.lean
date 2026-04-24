import RRProofs.VerifyIrCfgConvergenceSubset

namespace RRProofs

structure JoinCfgWorklistWitnessLite where
  base : JoinCfgConvergenceWitnessLite

def JoinCfgWorklistWitnessLite.nextJoinOutDefs (w : JoinCfgWorklistWitnessLite) : DefSet :=
  stepOutDefs w.base.base.entry w.base.base.entryDefs
    w.base.base.reachable w.base.base.preds w.base.assigned w.base.seed
    w.base.base.cfg.join.id

def JoinCfgWorklistWitnessLite.joinChanged (w : JoinCfgWorklistWitnessLite) : Bool :=
  decide (w.nextJoinOutDefs ≠ w.base.seed w.base.base.cfg.join.id)

theorem JoinCfgWorklistWitnessLite.nextJoinOutDefs_eq_seedJoin_of_stable
    {w : JoinCfgWorklistWitnessLite}
    (hStable : w.base.seedStable) :
    w.nextJoinOutDefs = w.base.seed w.base.base.cfg.join.id := by
  have hEq :=
    congrArg (fun m => m w.base.base.cfg.join.id) hStable
  simpa [JoinCfgWorklistWitnessLite.nextJoinOutDefs,
    JoinCfgConvergenceWitnessLite.seedStable, outMapStable, stepOutMap] using hEq

theorem JoinCfgWorklistWitnessLite.joinChanged_eq_false_of_stable
    {w : JoinCfgWorklistWitnessLite}
    (hStable : w.base.seedStable) :
    w.joinChanged = false := by
  unfold JoinCfgWorklistWitnessLite.joinChanged
  simp [w.nextJoinOutDefs_eq_seedJoin_of_stable hStable]

theorem JoinCfgWorklistWitnessLite.accepts_and_reports_no_join_change_of_stable_seedStepInDefs
    {w : JoinCfgWorklistWitnessLite}
    (hCfgPreds : w.base.base.cfg.predsOk)
    (hOrder : w.base.base.cfg.orderOk)
    (hReach : w.base.base.joinReachableOk)
    (hPredMap : w.base.base.joinPredsOk)
    (hStable : w.base.seedStable)
    (hSeedJoinDefs : stepInDefs w.base.base.entry w.base.base.entryDefs
      w.base.base.reachable w.base.base.preds w.base.seed
      w.base.base.cfg.join.id = w.base.base.cfg.defsJoin)
    (hBase : w.base.base.cfg.base.verifyIrStructLite = none)
    (hLeft : (rawFlowCaseOfActualBlock w.base.base.cfg.table
      w.base.base.cfg.defsLeft w.base.base.cfg.left).verifyFlow = none)
    (hRight : (rawFlowCaseOfActualBlock w.base.base.cfg.table
      w.base.base.cfg.defsRight w.base.base.cfg.right).verifyFlow = none)
    (hJoinReq : ∀ v, v ∈ rawRequiredVarsOfBlock w.base.base.cfg.table
      w.base.base.cfg.join ->
      v ∈ stepInDefs w.base.base.entry w.base.base.entryDefs
        w.base.base.reachable w.base.base.preds w.base.seed
        w.base.base.cfg.join.id) :
    w.base.base.cfg.toFlowCase.verifyIrFlowLite = none ∧
      w.joinChanged = false := by
  constructor
  · exact w.base.accepts_of_stable_seedStepInDefs
      hCfgPreds hOrder hReach hPredMap hStable hSeedJoinDefs hBase hLeft hRight hJoinReq
  · exact w.joinChanged_eq_false_of_stable hStable

end RRProofs
