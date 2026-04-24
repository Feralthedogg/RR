import RRProofs.PipelineAssignPhiSubset

namespace RRProofs

inductive SrcStmt where
  | assign : String -> SrcExpr -> SrcStmt
  | ifAssign : String -> SrcExpr -> SrcExpr -> SrcExpr -> SrcStmt
deriving Repr

inductive MirStmt where
  | assign : String -> MirExpr -> MirStmt
  | ifAssign : String -> MirExpr -> MirExpr -> MirExpr -> MirStmt
deriving Repr

inductive RStmt where
  | assign : String -> RExpr -> RStmt
  | ifAssign : String -> RExpr -> RExpr -> RExpr -> RStmt
deriving Repr

def execSrcStmt : LetEnv -> SrcStmt -> Option LetEnv
  | env, .assign name rhs => do
      let v <- evalSrc rhs
      some ((name, v) :: env)
  | env, .ifAssign name cond thenRhs elseRhs => do
      let cv <- evalSrc cond
      let merged <-
        match cv with
        | .bool true => evalSrc thenRhs
        | .bool false => evalSrc elseRhs
        | _ => none
      some ((name, merged) :: env)

def execMirStmt : LetEnv -> MirStmt -> Option LetEnv
  | env, .assign name rhs => do
      let v <- evalMir rhs
      some ((name, v) :: env)
  | env, .ifAssign name cond thenRhs elseRhs => do
      let cv <- evalMir cond
      let merged <-
        match cv with
        | .bool true => evalMir thenRhs
        | .bool false => evalMir elseRhs
        | _ => none
      some ((name, merged) :: env)

def execRStmt : LetEnv -> RStmt -> Option LetEnv
  | env, .assign name rhs => do
      let v <- evalRExpr rhs
      some ((name, v) :: env)
  | env, .ifAssign name cond thenRhs elseRhs => do
      let cv <- evalRExpr cond
      let merged <-
        match cv with
        | .bool true => evalRExpr thenRhs
        | .bool false => evalRExpr elseRhs
        | _ => none
      some ((name, merged) :: env)

def lowerStmt : SrcStmt -> MirStmt
  | .assign name rhs => .assign name (lower rhs)
  | .ifAssign name cond thenRhs elseRhs =>
      .ifAssign name (lower cond) (lower thenRhs) (lower elseRhs)

def emitRStmt : MirStmt -> RStmt
  | .assign name rhs => .assign name (emitR rhs)
  | .ifAssign name cond thenRhs elseRhs =>
      .ifAssign name (emitR cond) (emitR thenRhs) (emitR elseRhs)

def execSrcStmts : LetEnv -> List SrcStmt -> Option LetEnv
  | env, [] => some env
  | env, stmt :: rest => do
      let env' <- execSrcStmt env stmt
      execSrcStmts env' rest

def execMirStmts : LetEnv -> List MirStmt -> Option LetEnv
  | env, [] => some env
  | env, stmt :: rest => do
      let env' <- execMirStmt env stmt
      execMirStmts env' rest

def execRStmts : LetEnv -> List RStmt -> Option LetEnv
  | env, [] => some env
  | env, stmt :: rest => do
      let env' <- execRStmt env stmt
      execRStmts env' rest

def lowerStmts : List SrcStmt -> List MirStmt
  | [] => []
  | stmt :: rest => lowerStmt stmt :: lowerStmts rest

def emitRStmts : List MirStmt -> List RStmt
  | [] => []
  | stmt :: rest => emitRStmt stmt :: emitRStmts rest

theorem lowerStmt_preserves_exec
    (env : LetEnv) (stmt : SrcStmt) :
    execMirStmt env (lowerStmt stmt) = execSrcStmt env stmt := by
  cases stmt with
  | assign name rhs =>
      simp [lowerStmt, execMirStmt, execSrcStmt, lower_preserves_eval]
  | ifAssign name cond thenRhs elseRhs =>
      simp [lowerStmt, execMirStmt, execSrcStmt,
        lower_preserves_eval cond, lower_preserves_eval thenRhs, lower_preserves_eval elseRhs]

theorem emitRStmt_preserves_exec
    (env : LetEnv) (stmt : MirStmt) :
    execRStmt env (emitRStmt stmt) = execMirStmt env stmt := by
  cases stmt with
  | assign name rhs =>
      simp [emitRStmt, execRStmt, execMirStmt, emitR_preserves_eval]
  | ifAssign name cond thenRhs elseRhs =>
      simp [emitRStmt, execRStmt, execMirStmt,
        emitR_preserves_eval cond, emitR_preserves_eval thenRhs, emitR_preserves_eval elseRhs]

theorem lowerStmts_preserves_exec
    (env : LetEnv) (stmts : List SrcStmt) :
    execMirStmts env (lowerStmts stmts) = execSrcStmts env stmts := by
  induction stmts generalizing env with
  | nil =>
      simp [execMirStmts, execSrcStmts, lowerStmts]
  | cons stmt rest ih =>
      simp [execMirStmts, execSrcStmts, lowerStmts]
      rw [lowerStmt_preserves_exec]
      cases h : execSrcStmt env stmt <;> simp [ih]

theorem emitRStmts_preserves_exec
    (env : LetEnv) (stmts : List MirStmt) :
    execRStmts env (emitRStmts stmts) = execMirStmts env stmts := by
  induction stmts generalizing env with
  | nil =>
      simp [execRStmts, execMirStmts, emitRStmts]
  | cons stmt rest ih =>
      simp [execRStmts, execMirStmts, emitRStmts]
      rw [emitRStmt_preserves_exec]
      cases h : execMirStmt env stmt <;> simp [ih]

structure SrcProgram where
  stmts : List SrcStmt
  ret : SrcLetExpr
deriving Repr

structure MirProgram where
  stmts : List MirStmt
  ret : MirLetExpr
deriving Repr

structure RProgram where
  stmts : List RStmt
  ret : RLetExpr
deriving Repr

def lowerProgram (p : SrcProgram) : MirProgram :=
  { stmts := lowerStmts p.stmts, ret := lowerLet p.ret }

def emitRProgram (p : MirProgram) : RProgram :=
  { stmts := emitRStmts p.stmts, ret := emitRLet p.ret }

def evalSrcProgram (p : SrcProgram) : Option RValue := do
  let env <- execSrcStmts [] p.stmts
  evalSrcLet env p.ret

def evalMirProgram (p : MirProgram) : Option RValue := do
  let env <- execMirStmts [] p.stmts
  evalMirLet env p.ret

def evalRProgram (p : RProgram) : Option RValue := do
  let env <- execRStmts [] p.stmts
  evalRLet env p.ret

theorem lowerProgram_preserves_eval
    (p : SrcProgram) :
    evalMirProgram (lowerProgram p) = evalSrcProgram p := by
  cases p with
  | mk stmts ret =>
      simp [evalMirProgram, evalSrcProgram, lowerProgram]
      rw [lowerStmts_preserves_exec]
      cases h : execSrcStmts [] stmts <;> simp [lowerLet_preserves_eval]

theorem emitRProgram_preserves_eval
    (p : MirProgram) :
    evalRProgram (emitRProgram p) = evalMirProgram p := by
  cases p with
  | mk stmts ret =>
      simp [evalRProgram, evalMirProgram, emitRProgram]
      rw [emitRStmts_preserves_exec]
      cases h : execMirStmts [] stmts <;> simp [emitRLet_preserves_eval]

theorem lowerEmitProgram_preserves_eval
    (p : SrcProgram) :
    evalRProgram (emitRProgram (lowerProgram p)) = evalSrcProgram p := by
  rw [emitRProgram_preserves_eval, lowerProgram_preserves_eval]

def straightLineProgram : SrcProgram :=
  { stmts := [.assign "x" (.constInt 4)]
    ret := .add (.var "x") (.pure (.constInt 3)) }

theorem straightLineProgram_preserved :
    evalRProgram (emitRProgram (lowerProgram straightLineProgram)) = some (.int 7) := by
  rw [lowerEmitProgram_preserves_eval]
  simp [straightLineProgram, evalSrcProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

def branchNestedRecordProgram : SrcProgram :=
  { stmts := [.ifAssign "rec" (.constBool true)
      (.record [("inner", .record [("x", .constInt 1)])])
      (.record [("inner", .record [("x", .constInt 2)])])]
    ret := .add (.field (.field (.var "rec") "inner") "x") (.pure (.constInt 3)) }

theorem branchNestedRecordProgram_preserved :
    evalRProgram (emitRProgram (lowerProgram branchNestedRecordProgram)) = some (.int 4) := by
  rw [lowerEmitProgram_preserves_eval]
  simp [branchNestedRecordProgram, evalSrcProgram, execSrcStmts, execSrcStmt,
    evalSrc, evalSrcFields, evalSrcLet, lookupField]

end RRProofs
