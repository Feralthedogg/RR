import RRProofs.LoweringSubset

namespace RRProofs

inductive SrcIfExpr where
  | pure : SrcExpr -> SrcIfExpr
  | ite : SrcExpr -> SrcExpr -> SrcExpr -> SrcIfExpr
deriving Repr

inductive MirIfPhiExpr where
  | pure : MirExpr -> MirIfPhiExpr
  | ifPhi : MirExpr -> MirExpr -> MirExpr -> MirIfPhiExpr
deriving Repr

def evalSrcIf : SrcIfExpr -> Option RValue
  | .pure e => evalSrc e
  | .ite cond thenExpr elseExpr => do
      let cv <- evalSrc cond
      match cv with
      | .bool true => evalSrc thenExpr
      | .bool false => evalSrc elseExpr
      | _ => none

def evalMirIfPhi : MirIfPhiExpr -> Option RValue
  | .pure e => evalMir e
  | .ifPhi cond thenVal elseVal => do
      let cv <- evalMir cond
      match cv with
      | .bool true => evalMir thenVal
      | .bool false => evalMir elseVal
      | _ => none

def lowerIfPhi : SrcIfExpr -> MirIfPhiExpr
  | .pure e => .pure (lower e)
  | .ite cond thenExpr elseExpr => .ifPhi (lower cond) (lower thenExpr) (lower elseExpr)

theorem lowerIfPhi_preserves_eval
    (expr : SrcIfExpr) :
    evalMirIfPhi (lowerIfPhi expr) = evalSrcIf expr := by
  cases expr with
  | pure e =>
      simp [lowerIfPhi, evalMirIfPhi, evalSrcIf, lower_preserves_eval]
  | ite cond thenExpr elseExpr =>
      simp [lowerIfPhi, evalMirIfPhi, evalSrcIf, lower_preserves_eval]

def branchRecordFieldSrc : SrcIfExpr :=
  .ite
    (.constBool true)
    (.field (.record [("x", .constInt 1)]) "x")
    (.field (.record [("x", .constInt 2)]) "x")

theorem branchRecordFieldSrc_preserved :
    evalMirIfPhi (lowerIfPhi branchRecordFieldSrc) = some (.int 1) := by
  rw [lowerIfPhi_preserves_eval]
  simp [branchRecordFieldSrc, evalSrcIf, evalSrc, evalSrcFields, lookupField]

end RRProofs
