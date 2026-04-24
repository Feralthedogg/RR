import RRProofs.PipelineFnCfgBranchExecSubset

namespace RRProofs

def branchExitResult (results : List FnBlockResult) : Option RValue :=
  match results.reverse.head? with
  | some (_, value) => value
  | none => none

def phiMergeResult (choice : BranchChoice)
    (thenResults elseResults : List FnBlockResult) : Option RValue :=
  match choice with
  | .thenBranch => branchExitResult thenResults
  | .elseBranch => branchExitResult elseResults

structure SrcFnCfgPhiProgram where
  branchProg : SrcFnCfgBranchProgram
  joinBid : Nat
  phiName : String

structure MirFnCfgPhiProgram where
  branchProg : MirFnCfgBranchProgram
  joinBid : Nat
  phiName : String

structure RFnCfgPhiProgram where
  branchProg : RFnCfgBranchProgram
  joinBid : Nat
  phiName : String

def lowerFnCfgPhiProgram (p : SrcFnCfgPhiProgram) : MirFnCfgPhiProgram :=
  { branchProg := lowerFnCfgBranchProgram p.branchProg
  , joinBid := p.joinBid
  , phiName := p.phiName
  }

def emitRFnCfgPhiProgram (p : MirFnCfgPhiProgram) : RFnCfgPhiProgram :=
  { branchProg := emitRFnCfgBranchProgram p.branchProg
  , joinBid := p.joinBid
  , phiName := p.phiName
  }

def evalSrcFnCfgPhiProgram (p : SrcFnCfgPhiProgram) (choice : BranchChoice) : Option RValue :=
  phiMergeResult choice
    (evalSrcFnCfgBranchProgram p.branchProg .thenBranch)
    (evalSrcFnCfgBranchProgram p.branchProg .elseBranch)

def evalMirFnCfgPhiProgram (p : MirFnCfgPhiProgram) (choice : BranchChoice) : Option RValue :=
  phiMergeResult choice
    (evalMirFnCfgBranchProgram p.branchProg .thenBranch)
    (evalMirFnCfgBranchProgram p.branchProg .elseBranch)

def evalRFnCfgPhiProgram (p : RFnCfgPhiProgram) (choice : BranchChoice) : Option RValue :=
  phiMergeResult choice
    (evalRFnCfgBranchProgram p.branchProg .thenBranch)
    (evalRFnCfgBranchProgram p.branchProg .elseBranch)

theorem lowerFnCfgPhiProgram_preserves_meta
    (p : SrcFnCfgPhiProgram) :
    (lowerFnCfgPhiProgram p).joinBid = p.joinBid ∧
      (lowerFnCfgPhiProgram p).phiName = p.phiName := by
  constructor
  · rfl
  · rfl

theorem emitRFnCfgPhiProgram_preserves_meta
    (p : MirFnCfgPhiProgram) :
    (emitRFnCfgPhiProgram p).joinBid = p.joinBid ∧
      (emitRFnCfgPhiProgram p).phiName = p.phiName := by
  constructor
  · rfl
  · rfl

theorem lowerFnCfgPhiProgram_preserves_eval
    (p : SrcFnCfgPhiProgram) (choice : BranchChoice) :
    evalMirFnCfgPhiProgram (lowerFnCfgPhiProgram p) choice = evalSrcFnCfgPhiProgram p choice := by
  unfold evalMirFnCfgPhiProgram evalSrcFnCfgPhiProgram
  simp [lowerFnCfgPhiProgram]
  rw [lowerFnCfgBranchProgram_preserves_eval, lowerFnCfgBranchProgram_preserves_eval]

theorem emitRFnCfgPhiProgram_preserves_eval
    (p : MirFnCfgPhiProgram) (choice : BranchChoice) :
    evalRFnCfgPhiProgram (emitRFnCfgPhiProgram p) choice = evalMirFnCfgPhiProgram p choice := by
  unfold evalRFnCfgPhiProgram evalMirFnCfgPhiProgram
  simp [emitRFnCfgPhiProgram]
  rw [emitRFnCfgBranchProgram_preserves_eval, emitRFnCfgBranchProgram_preserves_eval]

theorem lowerEmitFnCfgPhiProgram_preserves_eval
    (p : SrcFnCfgPhiProgram) (choice : BranchChoice) :
    evalRFnCfgPhiProgram (emitRFnCfgPhiProgram (lowerFnCfgPhiProgram p)) choice =
      evalSrcFnCfgPhiProgram p choice := by
  rw [emitRFnCfgPhiProgram_preserves_eval, lowerFnCfgPhiProgram_preserves_eval]

def branchingFnCfgPhiProgram : SrcFnCfgPhiProgram :=
  { branchProg := branchingFnCfgProgram
  , joinBid := 17
  , phiName := "out"
  }

theorem branchingFnCfgPhiProgram_meta_preserved :
    (lowerFnCfgPhiProgram branchingFnCfgPhiProgram).joinBid = 17 ∧
      (lowerFnCfgPhiProgram branchingFnCfgPhiProgram).phiName = "out" ∧
      (emitRFnCfgPhiProgram (lowerFnCfgPhiProgram branchingFnCfgPhiProgram)).joinBid = 17 := by
  simp [branchingFnCfgPhiProgram, lowerFnCfgPhiProgram, emitRFnCfgPhiProgram]

theorem branchingFnCfgProgram_then_src_results :
    evalSrcFnCfgBranchProgram branchingFnCfgProgram .thenBranch =
      [(7, some (.int 7)), (11, some (.int 12))] := by
  simp [branchingFnCfgProgram, evalSrcFnCfgBranchProgram, toSrcFnCfgExecProgram, pathForChoice,
    evalSrcFnCfgExecProgram, lookupFnBlockResult, evalSrcFnCfgProgram, evalSrcFnBlocks,
    incomingFieldBlockProgram, incomingBranchBlockProgram, incomingElseBlockProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

theorem branchingFnCfgProgram_else_src_results :
    evalSrcFnCfgBranchProgram branchingFnCfgProgram .elseBranch =
      [(7, some (.int 7)), (13, some (.int 25))] := by
  simp [branchingFnCfgProgram, evalSrcFnCfgBranchProgram, toSrcFnCfgExecProgram, pathForChoice,
    evalSrcFnCfgExecProgram, lookupFnBlockResult, evalSrcFnCfgProgram, evalSrcFnBlocks,
    incomingFieldBlockProgram, incomingBranchBlockProgram, incomingElseBlockProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

theorem branchingFnCfgPhiProgram_then_preserved :
    evalRFnCfgPhiProgram
      (emitRFnCfgPhiProgram (lowerFnCfgPhiProgram branchingFnCfgPhiProgram)) .thenBranch =
      some (.int 12) := by
  rw [lowerEmitFnCfgPhiProgram_preserves_eval]
  simp [branchingFnCfgPhiProgram, evalSrcFnCfgPhiProgram, phiMergeResult, branchExitResult]
  rfl

theorem branchingFnCfgPhiProgram_else_preserved :
    evalRFnCfgPhiProgram
      (emitRFnCfgPhiProgram (lowerFnCfgPhiProgram branchingFnCfgPhiProgram)) .elseBranch =
      some (.int 25) := by
  rw [lowerEmitFnCfgPhiProgram_preserves_eval]
  simp [branchingFnCfgPhiProgram, evalSrcFnCfgPhiProgram, phiMergeResult, branchExitResult]
  rw [branchingFnCfgProgram_else_src_results]
  rfl

end RRProofs
