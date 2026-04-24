import RRProofs.PipelineStmtSubset

namespace RRProofs

structure SrcBlockEnvProgram where
  bid : Nat
  inEnv : LetEnv
  stmts : List SrcStmt
  ret : SrcLetExpr
deriving Repr

structure MirBlockEnvProgram where
  bid : Nat
  inEnv : LetEnv
  stmts : List MirStmt
  ret : MirLetExpr
deriving Repr

structure RBlockEnvProgram where
  bid : Nat
  inEnv : LetEnv
  stmts : List RStmt
  ret : RLetExpr
deriving Repr

def lowerBlockEnvProgram (p : SrcBlockEnvProgram) : MirBlockEnvProgram :=
  { bid := p.bid
  , inEnv := p.inEnv
  , stmts := lowerStmts p.stmts
  , ret := lowerLet p.ret
  }

def emitRBlockEnvProgram (p : MirBlockEnvProgram) : RBlockEnvProgram :=
  { bid := p.bid
  , inEnv := p.inEnv
  , stmts := emitRStmts p.stmts
  , ret := emitRLet p.ret
  }

def evalSrcBlockEnvProgram (p : SrcBlockEnvProgram) : Option RValue := do
  let env <- execSrcStmts p.inEnv p.stmts
  evalSrcLet env p.ret

def evalMirBlockEnvProgram (p : MirBlockEnvProgram) : Option RValue := do
  let env <- execMirStmts p.inEnv p.stmts
  evalMirLet env p.ret

def evalRBlockEnvProgram (p : RBlockEnvProgram) : Option RValue := do
  let env <- execRStmts p.inEnv p.stmts
  evalRLet env p.ret

theorem lowerBlockEnvProgram_preserves_block_id
    (p : SrcBlockEnvProgram) :
    (lowerBlockEnvProgram p).bid = p.bid := by
  rfl

theorem emitRBlockEnvProgram_preserves_block_id
    (p : MirBlockEnvProgram) :
    (emitRBlockEnvProgram p).bid = p.bid := by
  rfl

theorem lowerBlockEnvProgram_preserves_eval
    (p : SrcBlockEnvProgram) :
    evalMirBlockEnvProgram (lowerBlockEnvProgram p) = evalSrcBlockEnvProgram p := by
  cases p with
  | mk bid inEnv stmts ret =>
      simp [evalMirBlockEnvProgram, evalSrcBlockEnvProgram, lowerBlockEnvProgram]
      rw [lowerStmts_preserves_exec]
      cases h : execSrcStmts inEnv stmts <;> simp [lowerLet_preserves_eval]

theorem emitRBlockEnvProgram_preserves_eval
    (p : MirBlockEnvProgram) :
    evalRBlockEnvProgram (emitRBlockEnvProgram p) = evalMirBlockEnvProgram p := by
  cases p with
  | mk bid inEnv stmts ret =>
      simp [evalRBlockEnvProgram, evalMirBlockEnvProgram, emitRBlockEnvProgram]
      rw [emitRStmts_preserves_exec]
      cases h : execMirStmts inEnv stmts <;> simp [emitRLet_preserves_eval]

theorem lowerEmitBlockEnvProgram_preserves_eval
    (p : SrcBlockEnvProgram) :
    evalRBlockEnvProgram (emitRBlockEnvProgram (lowerBlockEnvProgram p)) =
      evalSrcBlockEnvProgram p := by
  rw [emitRBlockEnvProgram_preserves_eval, lowerBlockEnvProgram_preserves_eval]

def incomingFieldBlockProgram : SrcBlockEnvProgram :=
  { bid := 7
  , inEnv := [("arg", .record [("x", .int 4)])]
  , stmts := [.assign "tmp" (.constInt 3)]
  , ret := .add (.field (.var "arg") "x") (.var "tmp")
  }

theorem incomingFieldBlockProgram_block_id_preserved :
    (lowerBlockEnvProgram incomingFieldBlockProgram).bid = 7 ∧
      (emitRBlockEnvProgram (lowerBlockEnvProgram incomingFieldBlockProgram)).bid = 7 := by
  simp [incomingFieldBlockProgram, lowerBlockEnvProgram, emitRBlockEnvProgram]

theorem incomingFieldBlockProgram_preserved :
    evalRBlockEnvProgram
      (emitRBlockEnvProgram (lowerBlockEnvProgram incomingFieldBlockProgram)) =
      some (.int 7) := by
  rw [lowerEmitBlockEnvProgram_preserves_eval]
  simp [incomingFieldBlockProgram, evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt,
    evalSrc, evalSrcLet, lookupField]

def incomingBranchBlockProgram : SrcBlockEnvProgram :=
  { bid := 11
  , inEnv := [("arg", .record [("base", .int 10)])]
  , stmts := [.ifAssign "tmp" (.constBool true) (.constInt 2) (.constInt 5)]
  , ret := .add (.field (.var "arg") "base") (.var "tmp")
  }

theorem incomingBranchBlockProgram_preserved :
    evalRBlockEnvProgram
      (emitRBlockEnvProgram (lowerBlockEnvProgram incomingBranchBlockProgram)) =
      some (.int 12) := by
  rw [lowerEmitBlockEnvProgram_preserves_eval]
  simp [incomingBranchBlockProgram, evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt,
    evalSrc, evalSrcLet, lookupField]

end RRProofs
