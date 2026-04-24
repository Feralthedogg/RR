import RRProofs.LoweringLetSubset

namespace RRProofs

inductive SrcAssignPhiExpr where
  | ifAssign : String -> SrcExpr -> SrcExpr -> SrcExpr -> SrcLetExpr -> SrcAssignPhiExpr
deriving Repr

inductive MirAssignPhiExpr where
  | ifAssignPhi : String -> MirExpr -> MirExpr -> MirExpr -> MirLetExpr -> MirAssignPhiExpr
deriving Repr

def evalSrcAssignPhi : LetEnv -> SrcAssignPhiExpr -> Option RValue
  | env, .ifAssign name cond thenBind elseBind body => do
      let cv <- evalSrc cond
      let merged <-
        match cv with
        | .bool true => evalSrc thenBind
        | .bool false => evalSrc elseBind
        | _ => none
      evalSrcLet ((name, merged) :: env) body

def evalMirAssignPhi : LetEnv -> MirAssignPhiExpr -> Option RValue
  | env, .ifAssignPhi name cond thenVal elseVal body => do
      let cv <- evalMir cond
      let merged <-
        match cv with
        | .bool true => evalMir thenVal
        | .bool false => evalMir elseVal
        | _ => none
      evalMirLet ((name, merged) :: env) body

def lowerAssignPhi : SrcAssignPhiExpr -> MirAssignPhiExpr
  | .ifAssign name cond thenBind elseBind body =>
      .ifAssignPhi name (lower cond) (lower thenBind) (lower elseBind) (lowerLet body)

theorem lowerAssignPhi_preserves_eval
    (env : LetEnv) (expr : SrcAssignPhiExpr) :
    evalMirAssignPhi env (lowerAssignPhi expr) = evalSrcAssignPhi env expr := by
  cases expr with
  | ifAssign name cond thenBind elseBind body =>
      simp [lowerAssignPhi, evalMirAssignPhi, evalSrcAssignPhi,
        lower_preserves_eval, lowerLet_preserves_eval]

def branchAssignedLocalSrc : SrcAssignPhiExpr :=
  .ifAssign "x" (.constBool true) (.constInt 1) (.constInt 2)
    (.add (.var "x") (.pure (.constInt 3)))

theorem branchAssignedLocalSrc_preserved :
    evalMirAssignPhi [] (lowerAssignPhi branchAssignedLocalSrc) = some (.int 4) := by
  rw [lowerAssignPhi_preserves_eval]
  simp [branchAssignedLocalSrc, evalSrcAssignPhi, evalSrc, evalSrcLet, lookupField]

def branchAssignedRecordFieldSrc : SrcAssignPhiExpr :=
  .ifAssign "rec" (.constBool true)
    (.record [("x", .constInt 1)])
    (.record [("x", .constInt 2)])
    (.add (.field (.var "rec") "x") (.pure (.constInt 3)))

theorem branchAssignedRecordFieldSrc_preserved :
    evalMirAssignPhi [] (lowerAssignPhi branchAssignedRecordFieldSrc) = some (.int 4) := by
  rw [lowerAssignPhi_preserves_eval]
  simp [branchAssignedRecordFieldSrc, evalSrcAssignPhi, evalSrc, evalSrcFields, evalSrcLet, lookupField]

def branchAssignedNestedRecordFieldSrc : SrcAssignPhiExpr :=
  .ifAssign "rec" (.constBool true)
    (.record [("inner", .record [("x", .constInt 1)])])
    (.record [("inner", .record [("x", .constInt 2)])])
    (.add (.field (.field (.var "rec") "inner") "x") (.pure (.constInt 3)))

theorem branchAssignedNestedRecordFieldSrc_preserved :
    evalMirAssignPhi [] (lowerAssignPhi branchAssignedNestedRecordFieldSrc) = some (.int 4) := by
  rw [lowerAssignPhi_preserves_eval]
  simp [branchAssignedNestedRecordFieldSrc, evalSrcAssignPhi, evalSrc, evalSrcFields, evalSrcLet, lookupField]

end RRProofs
