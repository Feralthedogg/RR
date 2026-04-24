import RRProofs.VerifyIrCfgOrderWorklistSubset

namespace RRProofs

inductive CfgFixedPointErrorLite where
  | struct
  | changed
  | flow
deriving Repr, DecidableEq

def verifyIrCfgFixedPointLite (w : JoinCfgOrderWorklistWitnessLite) :
    Option CfgFixedPointErrorLite :=
  if w.base.base.base.cfg.base.verifyIrStructLite = none then
    if w.anyChanged then
      some CfgFixedPointErrorLite.changed
    else
      match w.base.base.base.cfg.toFlowCase.verifyIrFlowLite with
      | none => none
      | some _ => some CfgFixedPointErrorLite.flow
  else
    some CfgFixedPointErrorLite.struct

theorem verifyIrCfgFixedPointLite_none_of_stable_seedStepInDefs
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
    verifyIrCfgFixedPointLite w = none := by
  have hAccNoChange :=
    w.accepts_and_reports_no_change_of_stable_seedStepInDefs
      hCfgPreds hOrder hReach hPredMap hStable hSeedJoinDefs hBase hLeft hRight hJoinReq
  rcases hAccNoChange with ⟨hAcc, hNoChange⟩
  simp [verifyIrCfgFixedPointLite, hBase, hNoChange, hAcc]

end RRProofs
