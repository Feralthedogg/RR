import RRProofs.PipelineFnCfgPhiExecSubset
import RRProofs.PipelineAssignPhiSubset

namespace RRProofs

structure SrcFnCfgJoinStateProgram where
  phiProg : SrcFnCfgPhiProgram
  joinEnv : LetEnv
  joinRet : SrcLetExpr

structure MirFnCfgJoinStateProgram where
  phiProg : MirFnCfgPhiProgram
  joinEnv : LetEnv
  joinRet : MirLetExpr

structure RFnCfgJoinStateProgram where
  phiProg : RFnCfgPhiProgram
  joinEnv : LetEnv
  joinRet : RLetExpr

def lowerFnCfgJoinStateProgram (p : SrcFnCfgJoinStateProgram) : MirFnCfgJoinStateProgram :=
  { phiProg := lowerFnCfgPhiProgram p.phiProg
  , joinEnv := p.joinEnv
  , joinRet := lowerLet p.joinRet
  }

def emitRFnCfgJoinStateProgram (p : MirFnCfgJoinStateProgram) : RFnCfgJoinStateProgram :=
  { phiProg := emitRFnCfgPhiProgram p.phiProg
  , joinEnv := p.joinEnv
  , joinRet := emitRLet p.joinRet
  }

def evalSrcFnCfgJoinStateProgram
    (p : SrcFnCfgJoinStateProgram) (choice : BranchChoice) : Option RValue := do
  let merged <- evalSrcFnCfgPhiProgram p.phiProg choice
  evalSrcLet ((p.phiProg.phiName, merged) :: p.joinEnv) p.joinRet

def evalMirFnCfgJoinStateProgram
    (p : MirFnCfgJoinStateProgram) (choice : BranchChoice) : Option RValue := do
  let merged <- evalMirFnCfgPhiProgram p.phiProg choice
  evalMirLet ((p.phiProg.phiName, merged) :: p.joinEnv) p.joinRet

def evalRFnCfgJoinStateProgram
    (p : RFnCfgJoinStateProgram) (choice : BranchChoice) : Option RValue := do
  let merged <- evalRFnCfgPhiProgram p.phiProg choice
  evalRLet ((p.phiProg.phiName, merged) :: p.joinEnv) p.joinRet

theorem lowerFnCfgJoinStateProgram_preserves_meta
    (p : SrcFnCfgJoinStateProgram) :
    (lowerFnCfgJoinStateProgram p).phiProg.joinBid = p.phiProg.joinBid ∧
      (lowerFnCfgJoinStateProgram p).phiProg.phiName = p.phiProg.phiName := by
  constructor
  · rfl
  · rfl

theorem emitRFnCfgJoinStateProgram_preserves_meta
    (p : MirFnCfgJoinStateProgram) :
    (emitRFnCfgJoinStateProgram p).phiProg.joinBid = p.phiProg.joinBid ∧
      (emitRFnCfgJoinStateProgram p).phiProg.phiName = p.phiProg.phiName := by
  constructor
  · rfl
  · rfl

theorem lowerFnCfgJoinStateProgram_preserves_eval
    (p : SrcFnCfgJoinStateProgram) (choice : BranchChoice) :
    evalMirFnCfgJoinStateProgram (lowerFnCfgJoinStateProgram p) choice =
      evalSrcFnCfgJoinStateProgram p choice := by
  have hMeta := lowerFnCfgPhiProgram_preserves_meta p.phiProg
  unfold evalMirFnCfgJoinStateProgram evalSrcFnCfgJoinStateProgram
  simp [lowerFnCfgJoinStateProgram, hMeta.2]
  rw [lowerFnCfgPhiProgram_preserves_eval]
  cases h : evalSrcFnCfgPhiProgram p.phiProg choice <;> simp [lowerLet_preserves_eval]

theorem emitRFnCfgJoinStateProgram_preserves_eval
    (p : MirFnCfgJoinStateProgram) (choice : BranchChoice) :
    evalRFnCfgJoinStateProgram (emitRFnCfgJoinStateProgram p) choice =
      evalMirFnCfgJoinStateProgram p choice := by
  have hMeta := emitRFnCfgPhiProgram_preserves_meta p.phiProg
  unfold evalRFnCfgJoinStateProgram evalMirFnCfgJoinStateProgram
  simp [emitRFnCfgJoinStateProgram, hMeta.2]
  rw [emitRFnCfgPhiProgram_preserves_eval]
  cases h : evalMirFnCfgPhiProgram p.phiProg choice <;> simp [emitRLet_preserves_eval]

theorem lowerEmitFnCfgJoinStateProgram_preserves_eval
    (p : SrcFnCfgJoinStateProgram) (choice : BranchChoice) :
    evalRFnCfgJoinStateProgram (emitRFnCfgJoinStateProgram (lowerFnCfgJoinStateProgram p)) choice =
      evalSrcFnCfgJoinStateProgram p choice := by
  rw [emitRFnCfgJoinStateProgram_preserves_eval, lowerFnCfgJoinStateProgram_preserves_eval]

def branchingFnCfgJoinStateProgram : SrcFnCfgJoinStateProgram :=
  { phiProg := branchingFnCfgPhiProgram
  , joinEnv := [("bonus", .int 1)]
  , joinRet := .add (.var "out") (.var "bonus")
  }

theorem branchingFnCfgJoinStateProgram_meta_preserved :
    (lowerFnCfgJoinStateProgram branchingFnCfgJoinStateProgram).phiProg.joinBid = 17 ∧
      (lowerFnCfgJoinStateProgram branchingFnCfgJoinStateProgram).phiProg.phiName = "out" ∧
      (emitRFnCfgJoinStateProgram
        (lowerFnCfgJoinStateProgram branchingFnCfgJoinStateProgram)).phiProg.joinBid = 17 := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem branchingFnCfgProgram_then_src_results_for_join :
    evalSrcFnCfgBranchProgram branchingFnCfgProgram .thenBranch =
      [(7, some (.int 7)), (11, some (.int 12))] := by
  simp [branchingFnCfgProgram, evalSrcFnCfgBranchProgram, toSrcFnCfgExecProgram, pathForChoice,
    evalSrcFnCfgExecProgram, lookupFnBlockResult, evalSrcFnCfgProgram, evalSrcFnBlocks,
    incomingFieldBlockProgram, incomingBranchBlockProgram, incomingElseBlockProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

theorem branchingFnCfgProgram_else_src_results_for_join :
    evalSrcFnCfgBranchProgram branchingFnCfgProgram .elseBranch =
      [(7, some (.int 7)), (13, some (.int 25))] := by
  simp [branchingFnCfgProgram, evalSrcFnCfgBranchProgram, toSrcFnCfgExecProgram, pathForChoice,
    evalSrcFnCfgExecProgram, lookupFnBlockResult, evalSrcFnCfgProgram, evalSrcFnBlocks,
    incomingFieldBlockProgram, incomingBranchBlockProgram, incomingElseBlockProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

theorem branchingFnCfgPhiProgram_then_src :
    evalSrcFnCfgPhiProgram branchingFnCfgPhiProgram .thenBranch = some (.int 12) := by
  simp [branchingFnCfgPhiProgram, evalSrcFnCfgPhiProgram, phiMergeResult, branchExitResult]
  rw [branchingFnCfgProgram_then_src_results_for_join]
  rfl

theorem branchingFnCfgPhiProgram_else_src :
    evalSrcFnCfgPhiProgram branchingFnCfgPhiProgram .elseBranch = some (.int 25) := by
  simp [branchingFnCfgPhiProgram, evalSrcFnCfgPhiProgram, phiMergeResult, branchExitResult]
  rw [branchingFnCfgProgram_else_src_results_for_join]
  rfl

theorem branchingFnCfgJoinStateProgram_then_preserved :
    evalRFnCfgJoinStateProgram
      (emitRFnCfgJoinStateProgram (lowerFnCfgJoinStateProgram branchingFnCfgJoinStateProgram))
      .thenBranch = some (.int 13) := by
  rw [lowerEmitFnCfgJoinStateProgram_preserves_eval]
  simp [branchingFnCfgJoinStateProgram, evalSrcFnCfgJoinStateProgram, evalSrcLet, lookupField]
  rw [branchingFnCfgPhiProgram_then_src]
  rfl

theorem branchingFnCfgJoinStateProgram_else_preserved :
    evalRFnCfgJoinStateProgram
      (emitRFnCfgJoinStateProgram (lowerFnCfgJoinStateProgram branchingFnCfgJoinStateProgram))
      .elseBranch = some (.int 26) := by
  rw [lowerEmitFnCfgJoinStateProgram_preserves_eval]
  simp [branchingFnCfgJoinStateProgram, evalSrcFnCfgJoinStateProgram, evalSrcLet, lookupField]
  rw [branchingFnCfgPhiProgram_else_src]
  rfl

end RRProofs
