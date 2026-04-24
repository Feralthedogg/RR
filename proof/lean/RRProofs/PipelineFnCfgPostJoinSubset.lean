import RRProofs.PipelineFnCfgJoinExecSubset

namespace RRProofs

structure SrcFnCfgPostJoinProgram where
  joinExecProg : SrcFnCfgJoinExecProgram
  contName : String
  contBlock : SrcBlockEnvProgram

structure MirFnCfgPostJoinProgram where
  joinExecProg : MirFnCfgJoinExecProgram
  contName : String
  contBlock : MirBlockEnvProgram

structure RFnCfgPostJoinProgram where
  joinExecProg : RFnCfgJoinExecProgram
  contName : String
  contBlock : RBlockEnvProgram

def lowerFnCfgPostJoinProgram (p : SrcFnCfgPostJoinProgram) : MirFnCfgPostJoinProgram :=
  { joinExecProg := lowerFnCfgJoinExecProgram p.joinExecProg
  , contName := p.contName
  , contBlock := lowerBlockEnvProgram p.contBlock
  }

def emitRFnCfgPostJoinProgram (p : MirFnCfgPostJoinProgram) : RFnCfgPostJoinProgram :=
  { joinExecProg := emitRFnCfgJoinExecProgram p.joinExecProg
  , contName := p.contName
  , contBlock := emitRBlockEnvProgram p.contBlock
  }

def evalSrcFnCfgPostJoinProgram
    (p : SrcFnCfgPostJoinProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalSrcFnCfgJoinExecProgram p.joinExecProg choice
  evalSrcBlockEnvProgram { p.contBlock with
    inEnv := (p.contName, joined) :: p.contBlock.inEnv }

def evalMirFnCfgPostJoinProgram
    (p : MirFnCfgPostJoinProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalMirFnCfgJoinExecProgram p.joinExecProg choice
  evalMirBlockEnvProgram { p.contBlock with
    inEnv := (p.contName, joined) :: p.contBlock.inEnv }

def evalRFnCfgPostJoinProgram
    (p : RFnCfgPostJoinProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalRFnCfgJoinExecProgram p.joinExecProg choice
  evalRBlockEnvProgram { p.contBlock with
    inEnv := (p.contName, joined) :: p.contBlock.inEnv }

theorem lowerFnCfgPostJoinProgram_preserves_meta
    (p : SrcFnCfgPostJoinProgram) :
    (lowerFnCfgPostJoinProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (lowerFnCfgPostJoinProgram p).contName = p.contName ∧
      (lowerFnCfgPostJoinProgram p).contBlock.bid = p.contBlock.bid := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem emitRFnCfgPostJoinProgram_preserves_meta
    (p : MirFnCfgPostJoinProgram) :
    (emitRFnCfgPostJoinProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (emitRFnCfgPostJoinProgram p).contName = p.contName ∧
      (emitRFnCfgPostJoinProgram p).contBlock.bid = p.contBlock.bid := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem lowerFnCfgPostJoinProgram_preserves_eval
    (p : SrcFnCfgPostJoinProgram) (choice : BranchChoice) :
    evalMirFnCfgPostJoinProgram (lowerFnCfgPostJoinProgram p) choice =
      evalSrcFnCfgPostJoinProgram p choice := by
  unfold evalMirFnCfgPostJoinProgram evalSrcFnCfgPostJoinProgram
  simp [lowerFnCfgPostJoinProgram]
  rw [lowerFnCfgJoinExecProgram_preserves_eval]
  cases h : evalSrcFnCfgJoinExecProgram p.joinExecProg choice with
  | none =>
      simp
  | some joined =>
      simpa [lowerBlockEnvProgram] using
        (lowerBlockEnvProgram_preserves_eval
          { p.contBlock with
            inEnv := (p.contName, joined) :: p.contBlock.inEnv })

theorem emitRFnCfgPostJoinProgram_preserves_eval
    (p : MirFnCfgPostJoinProgram) (choice : BranchChoice) :
    evalRFnCfgPostJoinProgram (emitRFnCfgPostJoinProgram p) choice =
      evalMirFnCfgPostJoinProgram p choice := by
  unfold evalRFnCfgPostJoinProgram evalMirFnCfgPostJoinProgram
  simp [emitRFnCfgPostJoinProgram]
  rw [emitRFnCfgJoinExecProgram_preserves_eval]
  cases h : evalMirFnCfgJoinExecProgram p.joinExecProg choice with
  | none =>
      simp
  | some joined =>
      simpa [emitRBlockEnvProgram] using
        (emitRBlockEnvProgram_preserves_eval
          { p.contBlock with
            inEnv := (p.contName, joined) :: p.contBlock.inEnv })

theorem lowerEmitFnCfgPostJoinProgram_preserves_eval
    (p : SrcFnCfgPostJoinProgram) (choice : BranchChoice) :
    evalRFnCfgPostJoinProgram (emitRFnCfgPostJoinProgram (lowerFnCfgPostJoinProgram p)) choice =
      evalSrcFnCfgPostJoinProgram p choice := by
  rw [emitRFnCfgPostJoinProgram_preserves_eval, lowerFnCfgPostJoinProgram_preserves_eval]

def branchingFnCfgPostJoinProgram : SrcFnCfgPostJoinProgram :=
  { joinExecProg := branchingFnCfgJoinExecProgram
  , contName := "joined"
  , contBlock :=
      { bid := 19
      , inEnv := [("tail", .int 2)]
      , stmts := [.assign "tmp3" (.constInt 5)]
      , ret := .add (.var "joined") (.add (.var "tmp3") (.var "tail"))
      }
  }

theorem branchingFnCfgPostJoinProgram_meta_preserved :
    (lowerFnCfgPostJoinProgram branchingFnCfgPostJoinProgram).joinExecProg.phiProg.joinBid = 17 ∧
      (lowerFnCfgPostJoinProgram branchingFnCfgPostJoinProgram).contName = "joined" ∧
      (lowerFnCfgPostJoinProgram branchingFnCfgPostJoinProgram).contBlock.bid = 19 := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem branchingFnCfgPostJoinProgram_then_preserved :
    evalRFnCfgPostJoinProgram
      (emitRFnCfgPostJoinProgram (lowerFnCfgPostJoinProgram branchingFnCfgPostJoinProgram))
      .thenBranch = some (.int 24) := by
  rw [lowerEmitFnCfgPostJoinProgram_preserves_eval]
  have hPhi :
      evalSrcFnCfgPhiProgram branchingFnCfgJoinExecProgram.phiProg .thenBranch =
        some (.int 12) := by
    simpa [branchingFnCfgJoinExecProgram] using branchingFnCfgPhiProgram_then_src
  simp [branchingFnCfgPostJoinProgram, evalSrcFnCfgPostJoinProgram, evalSrcFnCfgJoinExecProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]
  rw [hPhi]
  rfl

theorem branchingFnCfgPostJoinProgram_else_preserved :
    evalRFnCfgPostJoinProgram
      (emitRFnCfgPostJoinProgram (lowerFnCfgPostJoinProgram branchingFnCfgPostJoinProgram))
      .elseBranch = some (.int 37) := by
  rw [lowerEmitFnCfgPostJoinProgram_preserves_eval]
  have hPhi :
      evalSrcFnCfgPhiProgram branchingFnCfgJoinExecProgram.phiProg .elseBranch =
        some (.int 25) := by
    simpa [branchingFnCfgJoinExecProgram] using branchingFnCfgPhiProgram_else_src
  simp [branchingFnCfgPostJoinProgram, evalSrcFnCfgPostJoinProgram, evalSrcFnCfgJoinExecProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]
  rw [hPhi]
  rfl

end RRProofs
