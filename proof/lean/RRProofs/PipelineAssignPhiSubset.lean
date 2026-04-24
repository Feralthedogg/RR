import RRProofs.LoweringAssignPhiSubset
import RRProofs.CodegenSubset

namespace RRProofs

inductive RLetExpr where
  | pure : RExpr -> RLetExpr
  | var : String -> RLetExpr
  | add : RLetExpr -> RLetExpr -> RLetExpr
  | field : RLetExpr -> String -> RLetExpr
  | let1 : String -> RExpr -> RLetExpr -> RLetExpr
deriving Repr

def evalRLet : LetEnv -> RLetExpr -> Option RValue
  | _, .pure e => evalRExpr e
  | env, .var name => lookupField env name
  | env, .add lhs rhs => do
      let lv <- evalRLet env lhs
      let rv <- evalRLet env rhs
      match lv, rv with
      | .int l, .int r => some (.int (l + r))
      | _, _ => none
  | env, .field base name => do
      let v <- evalRLet env base
      match v with
      | .record fields => lookupField fields name
      | _ => none
  | env, .let1 name bind body => do
      let v <- evalRExpr bind
      evalRLet ((name, v) :: env) body

def emitRLet : MirLetExpr -> RLetExpr
  | .pure e => .pure (emitR e)
  | .var name => .var name
  | .add lhs rhs => .add (emitRLet lhs) (emitRLet rhs)
  | .field base name => .field (emitRLet base) name
  | .let1 name bind body => .let1 name (emitR bind) (emitRLet body)

theorem emitRLet_preserves_eval
    (env : LetEnv) (expr : MirLetExpr) :
    evalRLet env (emitRLet expr) = evalMirLet env expr := by
  induction expr generalizing env with
  | pure e =>
      simp [emitRLet, evalRLet, evalMirLet, emitR_preserves_eval]
  | var name =>
      simp [emitRLet, evalRLet, evalMirLet]
  | add lhs rhs ihL ihR =>
      simp [emitRLet, evalRLet, evalMirLet, ihL, ihR]
      rfl
  | field base name ih =>
      simp [emitRLet, evalRLet, evalMirLet, ih]
      rfl
  | let1 name bind body ih =>
      simp [emitRLet, evalRLet, evalMirLet, emitR_preserves_eval]
      cases h : evalMir bind <;> simp [ih]

inductive RAssignPhiExpr where
  | ifAssignPhi : String -> RExpr -> RExpr -> RExpr -> RLetExpr -> RAssignPhiExpr
deriving Repr

def evalRAssignPhi : LetEnv -> RAssignPhiExpr -> Option RValue
  | env, .ifAssignPhi name cond thenVal elseVal body => do
      let cv <- evalRExpr cond
      let merged <-
        match cv with
        | .bool true => evalRExpr thenVal
        | .bool false => evalRExpr elseVal
        | _ => none
      evalRLet ((name, merged) :: env) body

def emitRAssignPhi : MirAssignPhiExpr -> RAssignPhiExpr
  | .ifAssignPhi name cond thenVal elseVal body =>
      .ifAssignPhi name (emitR cond) (emitR thenVal) (emitR elseVal) (emitRLet body)

theorem emitRAssignPhi_preserves_eval
    (env : LetEnv) (expr : MirAssignPhiExpr) :
    evalRAssignPhi env (emitRAssignPhi expr) = evalMirAssignPhi env expr := by
  cases expr with
  | ifAssignPhi name cond thenVal elseVal body =>
      simp [emitRAssignPhi, evalRAssignPhi, evalMirAssignPhi,
        emitR_preserves_eval cond, emitR_preserves_eval thenVal,
        emitR_preserves_eval elseVal, emitRLet_preserves_eval]
      rfl

theorem lowerEmitAssignPhi_preserves_eval
    (expr : SrcAssignPhiExpr) :
    evalRAssignPhi [] (emitRAssignPhi (lowerAssignPhi expr)) = evalSrcAssignPhi [] expr := by
  rw [emitRAssignPhi_preserves_eval, lowerAssignPhi_preserves_eval]

theorem branchAssignedLocalSrc_pipeline_preserved :
    evalRAssignPhi [] (emitRAssignPhi (lowerAssignPhi branchAssignedLocalSrc)) = some (.int 4) := by
  rw [lowerEmitAssignPhi_preserves_eval]
  simp [branchAssignedLocalSrc, evalSrcAssignPhi, evalSrc, evalSrcLet, lookupField]

theorem branchAssignedRecordFieldSrc_pipeline_preserved :
    evalRAssignPhi [] (emitRAssignPhi (lowerAssignPhi branchAssignedRecordFieldSrc)) = some (.int 4) := by
  rw [lowerEmitAssignPhi_preserves_eval]
  simp [branchAssignedRecordFieldSrc, evalSrcAssignPhi, evalSrc, evalSrcFields, evalSrcLet, lookupField]

theorem branchAssignedNestedRecordFieldSrc_pipeline_preserved :
    evalRAssignPhi [] (emitRAssignPhi (lowerAssignPhi branchAssignedNestedRecordFieldSrc)) = some (.int 4) := by
  rw [lowerEmitAssignPhi_preserves_eval]
  simp [branchAssignedNestedRecordFieldSrc, evalSrcAssignPhi, evalSrc, evalSrcFields, evalSrcLet, lookupField]

end RRProofs
