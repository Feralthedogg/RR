import RRProofs.PipelineStmtSubset

namespace RRProofs

structure SrcCfgProgram where
  cond : SrcExpr
  thenStmts : List SrcStmt
  elseStmts : List SrcStmt
  ret : SrcLetExpr
deriving Repr

structure MirCfgProgram where
  cond : MirExpr
  thenStmts : List MirStmt
  elseStmts : List MirStmt
  ret : MirLetExpr
deriving Repr

structure RCfgProgram where
  cond : RExpr
  thenStmts : List RStmt
  elseStmts : List RStmt
  ret : RLetExpr
deriving Repr

def lowerCfgProgram (p : SrcCfgProgram) : MirCfgProgram :=
  { cond := lower p.cond
    thenStmts := lowerStmts p.thenStmts
    elseStmts := lowerStmts p.elseStmts
    ret := lowerLet p.ret }

def emitRCfgProgram (p : MirCfgProgram) : RCfgProgram :=
  { cond := emitR p.cond
    thenStmts := emitRStmts p.thenStmts
    elseStmts := emitRStmts p.elseStmts
    ret := emitRLet p.ret }

def evalSrcCfgProgram (p : SrcCfgProgram) : Option RValue := do
  let cv <- evalSrc p.cond
  let env <-
    match cv with
    | .bool true => execSrcStmts [] p.thenStmts
    | .bool false => execSrcStmts [] p.elseStmts
    | _ => none
  evalSrcLet env p.ret

def evalMirCfgProgram (p : MirCfgProgram) : Option RValue := do
  let cv <- evalMir p.cond
  let env <-
    match cv with
    | .bool true => execMirStmts [] p.thenStmts
    | .bool false => execMirStmts [] p.elseStmts
    | _ => none
  evalMirLet env p.ret

def evalRCfgProgram (p : RCfgProgram) : Option RValue := do
  let cv <- evalRExpr p.cond
  let env <-
    match cv with
    | .bool true => execRStmts [] p.thenStmts
    | .bool false => execRStmts [] p.elseStmts
    | _ => none
  evalRLet env p.ret

theorem lowerCfgProgram_preserves_eval
    (p : SrcCfgProgram) :
    evalMirCfgProgram (lowerCfgProgram p) = evalSrcCfgProgram p := by
  cases p with
  | mk cond thenStmts elseStmts ret =>
      simp [evalMirCfgProgram, evalSrcCfgProgram, lowerCfgProgram, lower_preserves_eval]
      cases h : evalSrc cond <;> simp
      case some rv =>
        cases rv <;> simp [lowerStmts_preserves_exec, lowerLet_preserves_eval]

theorem emitRCfgProgram_preserves_eval
    (p : MirCfgProgram) :
    evalRCfgProgram (emitRCfgProgram p) = evalMirCfgProgram p := by
  cases p with
  | mk cond thenStmts elseStmts ret =>
      simp [evalRCfgProgram, evalMirCfgProgram, emitRCfgProgram, emitR_preserves_eval]
      cases h : evalMir cond <;> simp
      case some rv =>
        cases rv <;> simp [emitRStmts_preserves_exec, emitRLet_preserves_eval]

theorem lowerEmitCfgProgram_preserves_eval
    (p : SrcCfgProgram) :
    evalRCfgProgram (emitRCfgProgram (lowerCfgProgram p)) = evalSrcCfgProgram p := by
  rw [emitRCfgProgram_preserves_eval, lowerCfgProgram_preserves_eval]

def branchCfgNestedRecordProgram : SrcCfgProgram :=
  { cond := .constBool true
    thenStmts := [.assign "rec" (.record [("inner", .record [("x", .constInt 1)])])]
    elseStmts := [.assign "rec" (.record [("inner", .record [("x", .constInt 2)])])]
    ret := .add (.field (.field (.var "rec") "inner") "x") (.pure (.constInt 3)) }

theorem branchCfgNestedRecordProgram_preserved :
    evalRCfgProgram (emitRCfgProgram (lowerCfgProgram branchCfgNestedRecordProgram)) = some (.int 4) := by
  rw [lowerEmitCfgProgram_preserves_eval]
  simp [branchCfgNestedRecordProgram, evalSrcCfgProgram, execSrcStmts, execSrcStmt,
    evalSrc, evalSrcFields, evalSrcLet, lookupField]

end RRProofs
