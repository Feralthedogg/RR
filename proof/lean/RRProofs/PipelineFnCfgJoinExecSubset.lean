import RRProofs.PipelineFnCfgJoinStateSubset

namespace RRProofs

structure SrcFnCfgJoinExecProgram where
  phiProg : SrcFnCfgPhiProgram
  joinBlock : SrcBlockEnvProgram

structure MirFnCfgJoinExecProgram where
  phiProg : MirFnCfgPhiProgram
  joinBlock : MirBlockEnvProgram

structure RFnCfgJoinExecProgram where
  phiProg : RFnCfgPhiProgram
  joinBlock : RBlockEnvProgram

def lowerFnCfgJoinExecProgram (p : SrcFnCfgJoinExecProgram) : MirFnCfgJoinExecProgram :=
  { phiProg := lowerFnCfgPhiProgram p.phiProg
  , joinBlock := lowerBlockEnvProgram p.joinBlock
  }

def emitRFnCfgJoinExecProgram (p : MirFnCfgJoinExecProgram) : RFnCfgJoinExecProgram :=
  { phiProg := emitRFnCfgPhiProgram p.phiProg
  , joinBlock := emitRBlockEnvProgram p.joinBlock
  }

def evalSrcFnCfgJoinExecProgram
    (p : SrcFnCfgJoinExecProgram) (choice : BranchChoice) : Option RValue := do
  let merged <- evalSrcFnCfgPhiProgram p.phiProg choice
  evalSrcBlockEnvProgram { p.joinBlock with
    inEnv := (p.phiProg.phiName, merged) :: p.joinBlock.inEnv }

def evalMirFnCfgJoinExecProgram
    (p : MirFnCfgJoinExecProgram) (choice : BranchChoice) : Option RValue := do
  let merged <- evalMirFnCfgPhiProgram p.phiProg choice
  evalMirBlockEnvProgram { p.joinBlock with
    inEnv := (p.phiProg.phiName, merged) :: p.joinBlock.inEnv }

def evalRFnCfgJoinExecProgram
    (p : RFnCfgJoinExecProgram) (choice : BranchChoice) : Option RValue := do
  let merged <- evalRFnCfgPhiProgram p.phiProg choice
  evalRBlockEnvProgram { p.joinBlock with
    inEnv := (p.phiProg.phiName, merged) :: p.joinBlock.inEnv }

theorem lowerFnCfgJoinExecProgram_preserves_meta
    (p : SrcFnCfgJoinExecProgram) :
    (lowerFnCfgJoinExecProgram p).phiProg.joinBid = p.phiProg.joinBid ∧
      (lowerFnCfgJoinExecProgram p).phiProg.phiName = p.phiProg.phiName ∧
      (lowerFnCfgJoinExecProgram p).joinBlock.bid = p.joinBlock.bid := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem emitRFnCfgJoinExecProgram_preserves_meta
    (p : MirFnCfgJoinExecProgram) :
    (emitRFnCfgJoinExecProgram p).phiProg.joinBid = p.phiProg.joinBid ∧
      (emitRFnCfgJoinExecProgram p).phiProg.phiName = p.phiProg.phiName ∧
      (emitRFnCfgJoinExecProgram p).joinBlock.bid = p.joinBlock.bid := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem lowerFnCfgJoinExecProgram_preserves_eval
    (p : SrcFnCfgJoinExecProgram) (choice : BranchChoice) :
    evalMirFnCfgJoinExecProgram (lowerFnCfgJoinExecProgram p) choice =
      evalSrcFnCfgJoinExecProgram p choice := by
  have hPhiMeta := lowerFnCfgPhiProgram_preserves_meta p.phiProg
  unfold evalMirFnCfgJoinExecProgram evalSrcFnCfgJoinExecProgram
  simp [lowerFnCfgJoinExecProgram, hPhiMeta.2]
  rw [lowerFnCfgPhiProgram_preserves_eval]
  cases h : evalSrcFnCfgPhiProgram p.phiProg choice with
  | none =>
      simp
  | some merged =>
      simpa [lowerBlockEnvProgram] using
        (lowerBlockEnvProgram_preserves_eval
          { p.joinBlock with
            inEnv := (p.phiProg.phiName, merged) :: p.joinBlock.inEnv })

theorem emitRFnCfgJoinExecProgram_preserves_eval
    (p : MirFnCfgJoinExecProgram) (choice : BranchChoice) :
    evalRFnCfgJoinExecProgram (emitRFnCfgJoinExecProgram p) choice =
      evalMirFnCfgJoinExecProgram p choice := by
  have hPhiMeta := emitRFnCfgPhiProgram_preserves_meta p.phiProg
  unfold evalRFnCfgJoinExecProgram evalMirFnCfgJoinExecProgram
  simp [emitRFnCfgJoinExecProgram, hPhiMeta.2]
  rw [emitRFnCfgPhiProgram_preserves_eval]
  cases h : evalMirFnCfgPhiProgram p.phiProg choice with
  | none =>
      simp
  | some merged =>
      simpa [emitRBlockEnvProgram] using
        (emitRBlockEnvProgram_preserves_eval
          { p.joinBlock with
            inEnv := (p.phiProg.phiName, merged) :: p.joinBlock.inEnv })

theorem lowerEmitFnCfgJoinExecProgram_preserves_eval
    (p : SrcFnCfgJoinExecProgram) (choice : BranchChoice) :
    evalRFnCfgJoinExecProgram (emitRFnCfgJoinExecProgram (lowerFnCfgJoinExecProgram p)) choice =
      evalSrcFnCfgJoinExecProgram p choice := by
  rw [emitRFnCfgJoinExecProgram_preserves_eval, lowerFnCfgJoinExecProgram_preserves_eval]

def branchingFnCfgJoinExecProgram : SrcFnCfgJoinExecProgram :=
  { phiProg := branchingFnCfgPhiProgram
  , joinBlock :=
      { bid := 17
      , inEnv := [("bonus", .int 1)]
      , stmts := [.assign "tmp2" (.constInt 4)]
      , ret := .add (.var "out") (.add (.var "tmp2") (.var "bonus"))
      }
  }

theorem branchingFnCfgJoinExecProgram_meta_preserved :
    (lowerFnCfgJoinExecProgram branchingFnCfgJoinExecProgram).phiProg.joinBid = 17 ∧
      (lowerFnCfgJoinExecProgram branchingFnCfgJoinExecProgram).phiProg.phiName = "out" ∧
      (lowerFnCfgJoinExecProgram branchingFnCfgJoinExecProgram).joinBlock.bid = 17 := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem branchingFnCfgJoinExecProgram_then_preserved :
    evalRFnCfgJoinExecProgram
      (emitRFnCfgJoinExecProgram (lowerFnCfgJoinExecProgram branchingFnCfgJoinExecProgram))
      .thenBranch = some (.int 17) := by
  rw [lowerEmitFnCfgJoinExecProgram_preserves_eval]
  simp [branchingFnCfgJoinExecProgram, evalSrcFnCfgJoinExecProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]
  rw [branchingFnCfgPhiProgram_then_src]
  rfl

theorem branchingFnCfgJoinExecProgram_else_preserved :
    evalRFnCfgJoinExecProgram
      (emitRFnCfgJoinExecProgram (lowerFnCfgJoinExecProgram branchingFnCfgJoinExecProgram))
      .elseBranch = some (.int 30) := by
  rw [lowerEmitFnCfgJoinExecProgram_preserves_eval]
  simp [branchingFnCfgJoinExecProgram, evalSrcFnCfgJoinExecProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]
  rw [branchingFnCfgPhiProgram_else_src]
  rfl

end RRProofs
