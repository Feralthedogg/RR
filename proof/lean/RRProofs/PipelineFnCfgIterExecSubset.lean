import RRProofs.PipelineFnCfgPostJoinSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

structure SrcPostJoinStep where
  bindName : String
  block : SrcBlockEnvProgram

structure MirPostJoinStep where
  bindName : String
  block : MirBlockEnvProgram

structure RPostJoinStep where
  bindName : String
  block : RBlockEnvProgram

structure SrcFnCfgIterExecProgram where
  joinExecProg : SrcFnCfgJoinExecProgram
  steps : List SrcPostJoinStep

structure MirFnCfgIterExecProgram where
  joinExecProg : MirFnCfgJoinExecProgram
  steps : List MirPostJoinStep

structure RFnCfgIterExecProgram where
  joinExecProg : RFnCfgJoinExecProgram
  steps : List RPostJoinStep

def lowerPostJoinStep (s : SrcPostJoinStep) : MirPostJoinStep :=
  { bindName := s.bindName
  , block := lowerBlockEnvProgram s.block
  }

def emitRPostJoinStep (s : MirPostJoinStep) : RPostJoinStep :=
  { bindName := s.bindName
  , block := emitRBlockEnvProgram s.block
  }

def lowerFnCfgIterExecProgram (p : SrcFnCfgIterExecProgram) : MirFnCfgIterExecProgram :=
  { joinExecProg := lowerFnCfgJoinExecProgram p.joinExecProg
  , steps := p.steps.map lowerPostJoinStep
  }

def emitRFnCfgIterExecProgram (p : MirFnCfgIterExecProgram) : RFnCfgIterExecProgram :=
  { joinExecProg := emitRFnCfgJoinExecProgram p.joinExecProg
  , steps := p.steps.map emitRPostJoinStep
  }

def evalSrcPostJoinStep (step : SrcPostJoinStep) (current : RValue) : Option RValue :=
  evalSrcBlockEnvProgram { step.block with
    inEnv := (step.bindName, current) :: step.block.inEnv }

def evalMirPostJoinStep (step : MirPostJoinStep) (current : RValue) : Option RValue :=
  evalMirBlockEnvProgram { step.block with
    inEnv := (step.bindName, current) :: step.block.inEnv }

def evalRPostJoinStep (step : RPostJoinStep) (current : RValue) : Option RValue :=
  evalRBlockEnvProgram { step.block with
    inEnv := (step.bindName, current) :: step.block.inEnv }

def evalSrcPostJoinSteps : List SrcPostJoinStep → RValue → Option RValue
  | [], current => some current
  | step :: rest, current => do
      let next <- evalSrcPostJoinStep step current
      evalSrcPostJoinSteps rest next

def evalMirPostJoinSteps : List MirPostJoinStep → RValue → Option RValue
  | [], current => some current
  | step :: rest, current => do
      let next <- evalMirPostJoinStep step current
      evalMirPostJoinSteps rest next

def evalRPostJoinSteps : List RPostJoinStep → RValue → Option RValue
  | [], current => some current
  | step :: rest, current => do
      let next <- evalRPostJoinStep step current
      evalRPostJoinSteps rest next

def evalSrcFnCfgIterExecProgram
    (p : SrcFnCfgIterExecProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalSrcFnCfgJoinExecProgram p.joinExecProg choice
  evalSrcPostJoinSteps p.steps joined

def evalMirFnCfgIterExecProgram
    (p : MirFnCfgIterExecProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalMirFnCfgJoinExecProgram p.joinExecProg choice
  evalMirPostJoinSteps p.steps joined

def evalRFnCfgIterExecProgram
    (p : RFnCfgIterExecProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalRFnCfgJoinExecProgram p.joinExecProg choice
  evalRPostJoinSteps p.steps joined

theorem lowerPostJoinSteps_preserves_eval
    (steps : List SrcPostJoinStep) (current : RValue) :
    evalMirPostJoinSteps (steps.map lowerPostJoinStep) current =
      evalSrcPostJoinSteps steps current := by
  have hStep :
      ∀ (step : SrcPostJoinStep) (current : RValue),
        evalMirPostJoinStep (lowerPostJoinStep step) current =
          evalSrcPostJoinStep step current := by
    intro step current
    simpa [evalMirPostJoinStep, evalSrcPostJoinStep, lowerPostJoinStep, lowerBlockEnvProgram] using
      (lowerBlockEnvProgram_preserves_eval
        { step.block with inEnv := (step.bindName, current) :: step.block.inEnv })
  induction steps generalizing current with
  | nil =>
      rfl
  | cons step rest ih =>
      simp [evalMirPostJoinSteps, evalSrcPostJoinSteps, hStep step current, ih]

theorem emitRPostJoinSteps_preserves_eval
    (steps : List MirPostJoinStep) (current : RValue) :
    evalRPostJoinSteps (steps.map emitRPostJoinStep) current =
      evalMirPostJoinSteps steps current := by
  have hStep :
      ∀ (step : MirPostJoinStep) (current : RValue),
        evalRPostJoinStep (emitRPostJoinStep step) current =
          evalMirPostJoinStep step current := by
    intro step current
    simpa [evalRPostJoinStep, evalMirPostJoinStep, emitRPostJoinStep, emitRBlockEnvProgram] using
      (emitRBlockEnvProgram_preserves_eval
        { step.block with inEnv := (step.bindName, current) :: step.block.inEnv })
  induction steps generalizing current with
  | nil =>
      rfl
  | cons step rest ih =>
      simp [evalRPostJoinSteps, evalMirPostJoinSteps, hStep step current, ih]

theorem lowerFnCfgIterExecProgram_preserves_meta
    (p : SrcFnCfgIterExecProgram) :
    (lowerFnCfgIterExecProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (lowerFnCfgIterExecProgram p).steps.length = p.steps.length := by
  constructor
  · rfl
  · simp [lowerFnCfgIterExecProgram]

theorem emitRFnCfgIterExecProgram_preserves_meta
    (p : MirFnCfgIterExecProgram) :
    (emitRFnCfgIterExecProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (emitRFnCfgIterExecProgram p).steps.length = p.steps.length := by
  constructor
  · rfl
  · simp [emitRFnCfgIterExecProgram]

theorem lowerFnCfgIterExecProgram_preserves_eval
    (p : SrcFnCfgIterExecProgram) (choice : BranchChoice) :
    evalMirFnCfgIterExecProgram (lowerFnCfgIterExecProgram p) choice =
      evalSrcFnCfgIterExecProgram p choice := by
  unfold evalMirFnCfgIterExecProgram evalSrcFnCfgIterExecProgram
  simp [lowerFnCfgIterExecProgram]
  rw [lowerFnCfgJoinExecProgram_preserves_eval]
  cases h : evalSrcFnCfgJoinExecProgram p.joinExecProg choice with
  | none =>
      simp
  | some joined =>
      simpa using lowerPostJoinSteps_preserves_eval p.steps joined

theorem emitRFnCfgIterExecProgram_preserves_eval
    (p : MirFnCfgIterExecProgram) (choice : BranchChoice) :
    evalRFnCfgIterExecProgram (emitRFnCfgIterExecProgram p) choice =
      evalMirFnCfgIterExecProgram p choice := by
  unfold evalRFnCfgIterExecProgram evalMirFnCfgIterExecProgram
  simp [emitRFnCfgIterExecProgram]
  rw [emitRFnCfgJoinExecProgram_preserves_eval]
  cases h : evalMirFnCfgJoinExecProgram p.joinExecProg choice with
  | none =>
      simp
  | some joined =>
      simpa using emitRPostJoinSteps_preserves_eval p.steps joined

theorem lowerEmitFnCfgIterExecProgram_preserves_eval
    (p : SrcFnCfgIterExecProgram) (choice : BranchChoice) :
    evalRFnCfgIterExecProgram (emitRFnCfgIterExecProgram (lowerFnCfgIterExecProgram p)) choice =
      evalSrcFnCfgIterExecProgram p choice := by
  rw [emitRFnCfgIterExecProgram_preserves_eval, lowerFnCfgIterExecProgram_preserves_eval]

def branchingFnCfgIterExecProgram : SrcFnCfgIterExecProgram :=
  { joinExecProg := branchingFnCfgJoinExecProgram
  , steps :=
      [ { bindName := "joined"
        , block :=
            { bid := 19
            , inEnv := [("tail", .int 2)]
            , stmts := [.assign "tmp3" (.constInt 5)]
            , ret := .add (.var "joined") (.add (.var "tmp3") (.var "tail"))
            }
        }
      , { bindName := "after"
        , block :=
            { bid := 23
            , inEnv := [("delta", .int 3)]
            , stmts := [.assign "tmp4" (.constInt 1)]
            , ret := .add (.var "after") (.add (.var "tmp4") (.var "delta"))
            }
        }
      ]
  }

theorem branchingFnCfgIterExecProgram_meta_preserved :
    (lowerFnCfgIterExecProgram branchingFnCfgIterExecProgram).joinExecProg.phiProg.joinBid = 17 ∧
      (lowerFnCfgIterExecProgram branchingFnCfgIterExecProgram).steps.length = 2 := by
  constructor
  · rfl
  · rfl

theorem branchingFnCfgIterExecProgram_then_preserved :
    evalRFnCfgIterExecProgram
      (emitRFnCfgIterExecProgram (lowerFnCfgIterExecProgram branchingFnCfgIterExecProgram))
      .thenBranch = some (.int 28) := by
  rw [lowerEmitFnCfgIterExecProgram_preserves_eval]
  have hPhi :
      evalSrcFnCfgPhiProgram branchingFnCfgJoinExecProgram.phiProg .thenBranch =
        some (.int 12) := by
    simpa [branchingFnCfgJoinExecProgram] using branchingFnCfgPhiProgram_then_src
  simp [branchingFnCfgIterExecProgram, evalSrcFnCfgIterExecProgram, evalSrcPostJoinSteps,
    evalSrcFnCfgJoinExecProgram, evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt,
    evalSrc, evalSrcLet, lookupField]
  rw [hPhi]
  rfl

theorem branchingFnCfgIterExecProgram_else_preserved :
    evalRFnCfgIterExecProgram
      (emitRFnCfgIterExecProgram (lowerFnCfgIterExecProgram branchingFnCfgIterExecProgram))
      .elseBranch = some (.int 41) := by
  rw [lowerEmitFnCfgIterExecProgram_preserves_eval]
  have hPhi :
      evalSrcFnCfgPhiProgram branchingFnCfgJoinExecProgram.phiProg .elseBranch =
        some (.int 25) := by
    simpa [branchingFnCfgJoinExecProgram] using branchingFnCfgPhiProgram_else_src
  simp [branchingFnCfgIterExecProgram, evalSrcFnCfgIterExecProgram, evalSrcPostJoinSteps,
    evalSrcFnCfgJoinExecProgram, evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt,
    evalSrc, evalSrcLet, lookupField]
  rw [hPhi]
  rfl

end RRProofs
