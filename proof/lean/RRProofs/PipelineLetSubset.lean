import RRProofs.LoweringLetSubset
import RRProofs.PipelineAssignPhiSubset

namespace RRProofs

theorem lowerEmitLet_preserves_eval
    (env : LetEnv) (expr : SrcLetExpr) :
    evalRLet env (emitRLet (lowerLet expr)) = evalSrcLet env expr := by
  rw [emitRLet_preserves_eval, lowerLet_preserves_eval]

theorem simpleLetAddSrc_pipeline_preserved :
    evalRLet [] (emitRLet (lowerLet simpleLetAddSrc)) = some (.int 7) := by
  rw [lowerEmitLet_preserves_eval]
  simp [simpleLetAddSrc, evalSrcLet, evalSrc, lookupField]

theorem letRecordFieldSrc_pipeline_preserved :
    evalRLet [] (emitRLet (lowerLet letRecordFieldSrc)) = some (.int 7) := by
  rw [lowerEmitLet_preserves_eval]
  simp [letRecordFieldSrc, evalSrcLet, evalSrc, evalSrcFields, lookupField]

theorem letNestedRecordFieldSrc_pipeline_preserved :
    evalRLet [] (emitRLet (lowerLet letNestedRecordFieldSrc)) = some (.int 7) := by
  rw [lowerEmitLet_preserves_eval]
  simp [letNestedRecordFieldSrc, evalSrcLet, evalSrc, evalSrcFields, lookupField]

end RRProofs
