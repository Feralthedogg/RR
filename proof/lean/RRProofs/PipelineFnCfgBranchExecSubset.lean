import RRProofs.PipelineFnCfgExecSubset

namespace RRProofs

inductive BranchChoice where
  | thenBranch
  | elseBranch
deriving Repr, DecidableEq

structure SrcFnCfgBranchProgram where
  fnCfg : SrcFnCfgProgram
  blockOrder : List Nat
  thenPath : List Nat
  elsePath : List Nat

structure MirFnCfgBranchProgram where
  fnCfg : MirFnCfgProgram
  blockOrder : List Nat
  thenPath : List Nat
  elsePath : List Nat

structure RFnCfgBranchProgram where
  fnCfg : RFnCfgProgram
  blockOrder : List Nat
  thenPath : List Nat
  elsePath : List Nat

def pathForChoice (choice : BranchChoice) (thenPath elsePath : List Nat) : List Nat :=
  match choice with
  | .thenBranch => thenPath
  | .elseBranch => elsePath

def toSrcFnCfgExecProgram (p : SrcFnCfgBranchProgram) (choice : BranchChoice) : SrcFnCfgExecProgram :=
  { fnCfg := p.fnCfg
  , blockOrder := p.blockOrder
  , execPath := pathForChoice choice p.thenPath p.elsePath
  }

def toMirFnCfgExecProgram (p : MirFnCfgBranchProgram) (choice : BranchChoice) : MirFnCfgExecProgram :=
  { fnCfg := p.fnCfg
  , blockOrder := p.blockOrder
  , execPath := pathForChoice choice p.thenPath p.elsePath
  }

def toRFnCfgExecProgram (p : RFnCfgBranchProgram) (choice : BranchChoice) : RFnCfgExecProgram :=
  { fnCfg := p.fnCfg
  , blockOrder := p.blockOrder
  , execPath := pathForChoice choice p.thenPath p.elsePath
  }

def lowerFnCfgBranchProgram (p : SrcFnCfgBranchProgram) : MirFnCfgBranchProgram :=
  { fnCfg := lowerFnCfgProgram p.fnCfg
  , blockOrder := p.blockOrder
  , thenPath := p.thenPath
  , elsePath := p.elsePath
  }

def emitRFnCfgBranchProgram (p : MirFnCfgBranchProgram) : RFnCfgBranchProgram :=
  { fnCfg := emitRFnCfgProgram p.fnCfg
  , blockOrder := p.blockOrder
  , thenPath := p.thenPath
  , elsePath := p.elsePath
  }

def evalSrcFnCfgBranchProgram (p : SrcFnCfgBranchProgram) (choice : BranchChoice) : List FnBlockResult :=
  evalSrcFnCfgExecProgram (toSrcFnCfgExecProgram p choice)

def evalMirFnCfgBranchProgram (p : MirFnCfgBranchProgram) (choice : BranchChoice) : List FnBlockResult :=
  evalMirFnCfgExecProgram (toMirFnCfgExecProgram p choice)

def evalRFnCfgBranchProgram (p : RFnCfgBranchProgram) (choice : BranchChoice) : List FnBlockResult :=
  evalRFnCfgExecProgram (toRFnCfgExecProgram p choice)

theorem lowerFnCfgBranchProgram_preserves_eval
    (p : SrcFnCfgBranchProgram) (choice : BranchChoice) :
    evalMirFnCfgBranchProgram (lowerFnCfgBranchProgram p) choice = evalSrcFnCfgBranchProgram p choice := by
  unfold evalMirFnCfgBranchProgram evalSrcFnCfgBranchProgram
  simp [toMirFnCfgExecProgram, toSrcFnCfgExecProgram, lowerFnCfgBranchProgram]
  exact lowerFnCfgExecProgram_preserves_eval (toSrcFnCfgExecProgram p choice)

theorem emitRFnCfgBranchProgram_preserves_eval
    (p : MirFnCfgBranchProgram) (choice : BranchChoice) :
    evalRFnCfgBranchProgram (emitRFnCfgBranchProgram p) choice = evalMirFnCfgBranchProgram p choice := by
  unfold evalRFnCfgBranchProgram evalMirFnCfgBranchProgram
  simp [toRFnCfgExecProgram, toMirFnCfgExecProgram, emitRFnCfgBranchProgram]
  exact emitRFnCfgExecProgram_preserves_eval (toMirFnCfgExecProgram p choice)

theorem lowerEmitFnCfgBranchProgram_preserves_eval
    (p : SrcFnCfgBranchProgram) (choice : BranchChoice) :
    evalRFnCfgBranchProgram (emitRFnCfgBranchProgram (lowerFnCfgBranchProgram p)) choice =
      evalSrcFnCfgBranchProgram p choice := by
  rw [emitRFnCfgBranchProgram_preserves_eval, lowerFnCfgBranchProgram_preserves_eval]

def incomingElseBlockProgram : SrcBlockEnvProgram :=
  { bid := 13
  , inEnv := [("arg", .record [("base", .int 20)])]
  , stmts := [.assign "tmp" (.constInt 5)]
  , ret := .add (.field (.var "arg") "base") (.var "tmp")
  }

theorem incomingElseBlockProgram_preserved :
    evalRBlockEnvProgram
      (emitRBlockEnvProgram (lowerBlockEnvProgram incomingElseBlockProgram)) =
      some (.int 25) := by
  rw [lowerEmitBlockEnvProgram_preserves_eval]
  simp [incomingElseBlockProgram, evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt,
    evalSrc, evalSrcLet, lookupField]

def branchingFnCfgProgram : SrcFnCfgBranchProgram :=
  { fnCfg :=
      { name := "toy_branch_fn"
      , entry := 7
      , bodyHead := 11
      , preds := fun
          | 7 => []
          | 11 => [7]
          | 13 => [7]
          | _ => []
      , blocks := [incomingFieldBlockProgram, incomingBranchBlockProgram, incomingElseBlockProgram]
      }
  , blockOrder := [7, 11, 13]
  , thenPath := [7, 11]
  , elsePath := [7, 13]
  }

theorem branchingFnCfgProgram_then_preserved :
    evalRFnCfgBranchProgram
      (emitRFnCfgBranchProgram (lowerFnCfgBranchProgram branchingFnCfgProgram)) .thenBranch =
      [(7, some (.int 7)), (11, some (.int 12))] := by
  rw [lowerEmitFnCfgBranchProgram_preserves_eval]
  simp [branchingFnCfgProgram, evalSrcFnCfgBranchProgram, toSrcFnCfgExecProgram, pathForChoice,
    evalSrcFnCfgExecProgram, lookupFnBlockResult, evalSrcFnCfgProgram, evalSrcFnBlocks,
    incomingFieldBlockProgram, incomingBranchBlockProgram, incomingElseBlockProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

theorem branchingFnCfgProgram_else_preserved :
    evalRFnCfgBranchProgram
      (emitRFnCfgBranchProgram (lowerFnCfgBranchProgram branchingFnCfgProgram)) .elseBranch =
      [(7, some (.int 7)), (13, some (.int 25))] := by
  rw [lowerEmitFnCfgBranchProgram_preserves_eval]
  simp [branchingFnCfgProgram, evalSrcFnCfgBranchProgram, toSrcFnCfgExecProgram, pathForChoice,
    evalSrcFnCfgExecProgram, lookupFnBlockResult, evalSrcFnCfgProgram, evalSrcFnBlocks,
    incomingFieldBlockProgram, incomingBranchBlockProgram, incomingElseBlockProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

end RRProofs
