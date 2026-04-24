import RRProofs.VerifyIrCfgWorklistSubset

namespace RRProofs

structure JoinCfgOrderWorklistWitnessLite where
  base : JoinCfgWorklistWitnessLite

def JoinCfgOrderWorklistWitnessLite.nextLeftOutDefs (w : JoinCfgOrderWorklistWitnessLite) : DefSet :=
  stepOutDefs w.base.base.base.entry w.base.base.base.entryDefs
    w.base.base.base.reachable w.base.base.base.preds w.base.base.assigned
    w.base.base.seed w.base.base.base.cfg.left.id

def JoinCfgOrderWorklistWitnessLite.nextRightOutDefs (w : JoinCfgOrderWorklistWitnessLite) : DefSet :=
  stepOutDefs w.base.base.base.entry w.base.base.base.entryDefs
    w.base.base.base.reachable w.base.base.base.preds w.base.base.assigned
    w.base.base.seed w.base.base.base.cfg.right.id

def JoinCfgOrderWorklistWitnessLite.leftChanged (w : JoinCfgOrderWorklistWitnessLite) : Bool :=
  decide (w.nextLeftOutDefs ≠ w.base.base.seed w.base.base.base.cfg.left.id)

def JoinCfgOrderWorklistWitnessLite.rightChanged (w : JoinCfgOrderWorklistWitnessLite) : Bool :=
  decide (w.nextRightOutDefs ≠ w.base.base.seed w.base.base.base.cfg.right.id)

def JoinCfgOrderWorklistWitnessLite.changedFlags (w : JoinCfgOrderWorklistWitnessLite) : List Bool :=
  [w.leftChanged, w.rightChanged, w.base.joinChanged]

def JoinCfgOrderWorklistWitnessLite.anyChanged (w : JoinCfgOrderWorklistWitnessLite) : Bool :=
  w.changedFlags.any id

theorem JoinCfgOrderWorklistWitnessLite.nextLeftOutDefs_eq_seedLeft_of_stable
    {w : JoinCfgOrderWorklistWitnessLite}
    (hStable : w.base.base.seedStable) :
    w.nextLeftOutDefs = w.base.base.seed w.base.base.base.cfg.left.id := by
  have hEq := congrArg (fun m => m w.base.base.base.cfg.left.id) hStable
  simpa [JoinCfgOrderWorklistWitnessLite.nextLeftOutDefs,
    JoinCfgConvergenceWitnessLite.seedStable, outMapStable, stepOutMap] using hEq

theorem JoinCfgOrderWorklistWitnessLite.nextRightOutDefs_eq_seedRight_of_stable
    {w : JoinCfgOrderWorklistWitnessLite}
    (hStable : w.base.base.seedStable) :
    w.nextRightOutDefs = w.base.base.seed w.base.base.base.cfg.right.id := by
  have hEq := congrArg (fun m => m w.base.base.base.cfg.right.id) hStable
  simpa [JoinCfgOrderWorklistWitnessLite.nextRightOutDefs,
    JoinCfgConvergenceWitnessLite.seedStable, outMapStable, stepOutMap] using hEq

theorem JoinCfgOrderWorklistWitnessLite.leftChanged_eq_false_of_stable
    {w : JoinCfgOrderWorklistWitnessLite}
    (hStable : w.base.base.seedStable) :
    w.leftChanged = false := by
  unfold JoinCfgOrderWorklistWitnessLite.leftChanged
  simp [w.nextLeftOutDefs_eq_seedLeft_of_stable hStable]

theorem JoinCfgOrderWorklistWitnessLite.rightChanged_eq_false_of_stable
    {w : JoinCfgOrderWorklistWitnessLite}
    (hStable : w.base.base.seedStable) :
    w.rightChanged = false := by
  unfold JoinCfgOrderWorklistWitnessLite.rightChanged
  simp [w.nextRightOutDefs_eq_seedRight_of_stable hStable]

theorem JoinCfgOrderWorklistWitnessLite.anyChanged_eq_false_of_stable
    {w : JoinCfgOrderWorklistWitnessLite}
    (hStable : w.base.base.seedStable) :
    w.anyChanged = false := by
  simp [JoinCfgOrderWorklistWitnessLite.anyChanged,
    JoinCfgOrderWorklistWitnessLite.changedFlags,
    w.leftChanged_eq_false_of_stable hStable,
    w.rightChanged_eq_false_of_stable hStable,
    w.base.joinChanged_eq_false_of_stable hStable]

theorem JoinCfgOrderWorklistWitnessLite.accepts_and_reports_no_change_of_stable_seedStepInDefs
    {w : JoinCfgOrderWorklistWitnessLite}
    (hCfgPreds : w.base.base.base.cfg.predsOk)
    (hOrder : w.base.base.base.cfg.orderOk)
    (hReach : w.base.base.base.joinReachableOk)
    (hPredMap : w.base.base.base.joinPredsOk)
    (hStable : w.base.base.seedStable)
    (hSeedJoinDefs : stepInDefs w.base.base.base.entry w.base.base.base.entryDefs
      w.base.base.base.reachable w.base.base.base.preds w.base.base.seed
      w.base.base.base.cfg.join.id = w.base.base.base.cfg.defsJoin)
    (hBase : w.base.base.base.cfg.base.verifyIrStructLite = none)
    (hLeft : (rawFlowCaseOfActualBlock w.base.base.base.cfg.table
      w.base.base.base.cfg.defsLeft w.base.base.base.cfg.left).verifyFlow = none)
    (hRight : (rawFlowCaseOfActualBlock w.base.base.base.cfg.table
      w.base.base.base.cfg.defsRight w.base.base.base.cfg.right).verifyFlow = none)
    (hJoinReq : ∀ v, v ∈ rawRequiredVarsOfBlock w.base.base.base.cfg.table
      w.base.base.base.cfg.join ->
      v ∈ stepInDefs w.base.base.base.entry w.base.base.base.entryDefs
        w.base.base.base.reachable w.base.base.base.preds w.base.base.seed
        w.base.base.base.cfg.join.id) :
    w.base.base.base.cfg.toFlowCase.verifyIrFlowLite = none ∧
      w.anyChanged = false := by
  constructor
  · exact w.base.accepts_and_reports_no_join_change_of_stable_seedStepInDefs
      hCfgPreds hOrder hReach hPredMap hStable hSeedJoinDefs hBase hLeft hRight hJoinReq |>.1
  · exact w.anyChanged_eq_false_of_stable hStable

end RRProofs
