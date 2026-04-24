import RRProofs.LoweringIfPhiSubset
import RRProofs.CodegenSubset

namespace RRProofs

inductive RIfPhiExpr where
  | pure : RExpr -> RIfPhiExpr
  | ifPhi : RExpr -> RExpr -> RExpr -> RIfPhiExpr
deriving Repr

def evalRIfPhi : RIfPhiExpr -> Option RValue
  | .pure e => evalRExpr e
  | .ifPhi cond thenVal elseVal => do
      let cv <- evalRExpr cond
      match cv with
      | .bool true => evalRExpr thenVal
      | .bool false => evalRExpr elseVal
      | _ => none

def emitRIfPhi : MirIfPhiExpr -> RIfPhiExpr
  | .pure e => .pure (emitR e)
  | .ifPhi cond thenVal elseVal => .ifPhi (emitR cond) (emitR thenVal) (emitR elseVal)

theorem emitRIfPhi_preserves_eval
    (expr : MirIfPhiExpr) :
    evalRIfPhi (emitRIfPhi expr) = evalMirIfPhi expr := by
  cases expr with
  | pure e =>
      simp [emitRIfPhi, evalRIfPhi, evalMirIfPhi, emitR_preserves_eval]
  | ifPhi cond thenVal elseVal =>
      simp [emitRIfPhi, evalRIfPhi, evalMirIfPhi,
        emitR_preserves_eval cond, emitR_preserves_eval thenVal, emitR_preserves_eval elseVal]
      rfl

theorem lowerEmitIfPhi_preserves_eval
    (expr : SrcIfExpr) :
    evalRIfPhi (emitRIfPhi (lowerIfPhi expr)) = evalSrcIf expr := by
  rw [emitRIfPhi_preserves_eval, lowerIfPhi_preserves_eval]

theorem branchRecordFieldSrc_pipeline_preserved :
    evalRIfPhi (emitRIfPhi (lowerIfPhi branchRecordFieldSrc)) = some (.int 1) := by
  rw [lowerEmitIfPhi_preserves_eval]
  simp [branchRecordFieldSrc, evalSrcIf, evalSrc, evalSrcFields, lookupField]

end RRProofs
