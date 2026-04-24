import RRProofs.PipelineFnCfgReentrySubset

set_option linter.unusedSimpArgs false

namespace RRProofs

structure SrcFnCfgLoopCycleProgram where
  reentryProg : SrcFnCfgReentryProgram
  accName : String
  cycleName : String
  init : RValue
  choices : List BranchChoice
  cycleBlock : SrcBlockEnvProgram

structure MirFnCfgLoopCycleProgram where
  reentryProg : MirFnCfgReentryProgram
  accName : String
  cycleName : String
  init : RValue
  choices : List BranchChoice
  cycleBlock : MirBlockEnvProgram

structure RFnCfgLoopCycleProgram where
  reentryProg : RFnCfgReentryProgram
  accName : String
  cycleName : String
  init : RValue
  choices : List BranchChoice
  cycleBlock : RBlockEnvProgram

def lowerFnCfgLoopCycleProgram (p : SrcFnCfgLoopCycleProgram) : MirFnCfgLoopCycleProgram :=
  { reentryProg := lowerFnCfgReentryProgram p.reentryProg
  , accName := p.accName
  , cycleName := p.cycleName
  , init := p.init
  , choices := p.choices
  , cycleBlock := lowerBlockEnvProgram p.cycleBlock
  }

def emitRFnCfgLoopCycleProgram (p : MirFnCfgLoopCycleProgram) : RFnCfgLoopCycleProgram :=
  { reentryProg := emitRFnCfgReentryProgram p.reentryProg
  , accName := p.accName
  , cycleName := p.cycleName
  , init := p.init
  , choices := p.choices
  , cycleBlock := emitRBlockEnvProgram p.cycleBlock
  }

def evalSrcCycleStep (p : SrcFnCfgLoopCycleProgram) (acc cycleVal : RValue) : Option RValue :=
  evalSrcBlockEnvProgram { p.cycleBlock with
    inEnv := (p.accName, acc) :: (p.cycleName, cycleVal) :: p.cycleBlock.inEnv }

def evalMirCycleStep (p : MirFnCfgLoopCycleProgram) (acc cycleVal : RValue) : Option RValue :=
  evalMirBlockEnvProgram { p.cycleBlock with
    inEnv := (p.accName, acc) :: (p.cycleName, cycleVal) :: p.cycleBlock.inEnv }

def evalRCycleStep (p : RFnCfgLoopCycleProgram) (acc cycleVal : RValue) : Option RValue :=
  evalRBlockEnvProgram { p.cycleBlock with
    inEnv := (p.accName, acc) :: (p.cycleName, cycleVal) :: p.cycleBlock.inEnv }

def evalSrcLoopChoices (p : SrcFnCfgLoopCycleProgram) : List BranchChoice → RValue → Option RValue
  | [], current => some current
  | choice :: rest, current => do
      let cycleVal <- evalSrcFnCfgReentryProgram p.reentryProg choice
      let next <- evalSrcCycleStep p current cycleVal
      evalSrcLoopChoices p rest next

def evalMirLoopChoices (p : MirFnCfgLoopCycleProgram) : List BranchChoice → RValue → Option RValue
  | [], current => some current
  | choice :: rest, current => do
      let cycleVal <- evalMirFnCfgReentryProgram p.reentryProg choice
      let next <- evalMirCycleStep p current cycleVal
      evalMirLoopChoices p rest next

def evalRLoopChoices (p : RFnCfgLoopCycleProgram) : List BranchChoice → RValue → Option RValue
  | [], current => some current
  | choice :: rest, current => do
      let cycleVal <- evalRFnCfgReentryProgram p.reentryProg choice
      let next <- evalRCycleStep p current cycleVal
      evalRLoopChoices p rest next

def evalSrcFnCfgLoopCycleProgram (p : SrcFnCfgLoopCycleProgram) : Option RValue :=
  evalSrcLoopChoices p p.choices p.init

def evalMirFnCfgLoopCycleProgram (p : MirFnCfgLoopCycleProgram) : Option RValue :=
  evalMirLoopChoices p p.choices p.init

def evalRFnCfgLoopCycleProgram (p : RFnCfgLoopCycleProgram) : Option RValue :=
  evalRLoopChoices p p.choices p.init

theorem lowerCycleStep_preserves_eval
    (p : SrcFnCfgLoopCycleProgram) (acc cycleVal : RValue) :
    evalMirCycleStep (lowerFnCfgLoopCycleProgram p) acc cycleVal =
      evalSrcCycleStep p acc cycleVal := by
  simpa [evalMirCycleStep, evalSrcCycleStep, lowerFnCfgLoopCycleProgram, lowerBlockEnvProgram] using
    (lowerBlockEnvProgram_preserves_eval
      { p.cycleBlock with
        inEnv := (p.accName, acc) :: (p.cycleName, cycleVal) :: p.cycleBlock.inEnv })

theorem emitRCycleStep_preserves_eval
    (p : MirFnCfgLoopCycleProgram) (acc cycleVal : RValue) :
    evalRCycleStep (emitRFnCfgLoopCycleProgram p) acc cycleVal =
      evalMirCycleStep p acc cycleVal := by
  simpa [evalRCycleStep, evalMirCycleStep, emitRFnCfgLoopCycleProgram, emitRBlockEnvProgram] using
    (emitRBlockEnvProgram_preserves_eval
      { p.cycleBlock with
        inEnv := (p.accName, acc) :: (p.cycleName, cycleVal) :: p.cycleBlock.inEnv })

theorem lowerLoopChoices_preserves_eval
    (p : SrcFnCfgLoopCycleProgram) :
    ∀ (choices : List BranchChoice) (current : RValue),
      evalMirLoopChoices (lowerFnCfgLoopCycleProgram p) choices current =
        evalSrcLoopChoices p choices current := by
  intro choices current
  induction choices generalizing current with
  | nil =>
      rfl
  | cons choice rest ih =>
      simp [evalMirLoopChoices, evalSrcLoopChoices, lowerFnCfgLoopCycleProgram]
      rw [lowerFnCfgReentryProgram_preserves_eval]
      cases h : evalSrcFnCfgReentryProgram p.reentryProg choice with
      | none =>
          simp [h]
      | some cycleVal =>
          simp [h]
          have hStep :
              evalMirCycleStep
                  { reentryProg := lowerFnCfgReentryProgram p.reentryProg
                  , accName := p.accName
                  , cycleName := p.cycleName
                  , init := p.init
                  , choices := p.choices
                  , cycleBlock := lowerBlockEnvProgram p.cycleBlock
                  }
                  current cycleVal =
                evalSrcCycleStep p current cycleVal := by
            simpa [lowerFnCfgLoopCycleProgram] using lowerCycleStep_preserves_eval p current cycleVal
          rw [hStep]
          cases hs : evalSrcCycleStep p current cycleVal with
          | none =>
              simp [hs]
          | some val =>
              simpa [hs] using ih val

theorem emitRLoopChoices_preserves_eval
    (p : MirFnCfgLoopCycleProgram) :
    ∀ (choices : List BranchChoice) (current : RValue),
      evalRLoopChoices (emitRFnCfgLoopCycleProgram p) choices current =
        evalMirLoopChoices p choices current := by
  intro choices current
  induction choices generalizing current with
  | nil =>
      rfl
  | cons choice rest ih =>
      simp [evalRLoopChoices, evalMirLoopChoices, emitRFnCfgLoopCycleProgram]
      rw [emitRFnCfgReentryProgram_preserves_eval]
      cases h : evalMirFnCfgReentryProgram p.reentryProg choice with
      | none =>
          simp [h]
      | some cycleVal =>
          simp [h]
          have hStep :
              evalRCycleStep
                  { reentryProg := emitRFnCfgReentryProgram p.reentryProg
                  , accName := p.accName
                  , cycleName := p.cycleName
                  , init := p.init
                  , choices := p.choices
                  , cycleBlock := emitRBlockEnvProgram p.cycleBlock
                  }
                  current cycleVal =
                evalMirCycleStep p current cycleVal := by
            simpa [emitRFnCfgLoopCycleProgram] using emitRCycleStep_preserves_eval p current cycleVal
          rw [hStep]
          cases hs : evalMirCycleStep p current cycleVal with
          | none =>
              simp [hs]
          | some val =>
              simpa [hs] using ih val

theorem lowerFnCfgLoopCycleProgram_preserves_meta
    (p : SrcFnCfgLoopCycleProgram) :
    (lowerFnCfgLoopCycleProgram p).reentryProg.joinExecProg.phiProg.joinBid = p.reentryProg.joinExecProg.phiProg.joinBid ∧
      (lowerFnCfgLoopCycleProgram p).choices = p.choices ∧
      (lowerFnCfgLoopCycleProgram p).cycleBlock.bid = p.cycleBlock.bid := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem emitRFnCfgLoopCycleProgram_preserves_meta
    (p : MirFnCfgLoopCycleProgram) :
    (emitRFnCfgLoopCycleProgram p).reentryProg.joinExecProg.phiProg.joinBid = p.reentryProg.joinExecProg.phiProg.joinBid ∧
      (emitRFnCfgLoopCycleProgram p).choices = p.choices ∧
      (emitRFnCfgLoopCycleProgram p).cycleBlock.bid = p.cycleBlock.bid := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem lowerFnCfgLoopCycleProgram_preserves_eval
    (p : SrcFnCfgLoopCycleProgram) :
    evalMirFnCfgLoopCycleProgram (lowerFnCfgLoopCycleProgram p) =
      evalSrcFnCfgLoopCycleProgram p := by
  unfold evalMirFnCfgLoopCycleProgram evalSrcFnCfgLoopCycleProgram
  simpa using lowerLoopChoices_preserves_eval p p.choices p.init

theorem emitRFnCfgLoopCycleProgram_preserves_eval
    (p : MirFnCfgLoopCycleProgram) :
    evalRFnCfgLoopCycleProgram (emitRFnCfgLoopCycleProgram p) =
      evalMirFnCfgLoopCycleProgram p := by
  unfold evalRFnCfgLoopCycleProgram evalMirFnCfgLoopCycleProgram
  simpa using emitRLoopChoices_preserves_eval p p.choices p.init

theorem lowerEmitFnCfgLoopCycleProgram_preserves_eval
    (p : SrcFnCfgLoopCycleProgram) :
    evalRFnCfgLoopCycleProgram (emitRFnCfgLoopCycleProgram (lowerFnCfgLoopCycleProgram p)) =
      evalSrcFnCfgLoopCycleProgram p := by
  rw [emitRFnCfgLoopCycleProgram_preserves_eval, lowerFnCfgLoopCycleProgram_preserves_eval]

def branchingFnCfgLoopCycleProgram : SrcFnCfgLoopCycleProgram :=
  { reentryProg := branchingFnCfgReentryProgram
  , accName := "acc"
  , cycleName := "cycle"
  , init := .int 1
  , choices := [.thenBranch, .elseBranch, .thenBranch]
  , cycleBlock :=
      { bid := 31
      , inEnv := []
      , stmts := [.assign "bonus" (.constInt 1)]
      , ret := .add (.var "acc") (.add (.var "cycle") (.var "bonus"))
      }
  }

theorem branchingFnCfgLoopCycleProgram_meta_preserved :
    (lowerFnCfgLoopCycleProgram branchingFnCfgLoopCycleProgram).reentryProg.joinExecProg.phiProg.joinBid = 17 ∧
      (lowerFnCfgLoopCycleProgram branchingFnCfgLoopCycleProgram).choices = [.thenBranch, .elseBranch, .thenBranch] ∧
      (lowerFnCfgLoopCycleProgram branchingFnCfgLoopCycleProgram).cycleBlock.bid = 31 := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem branchingFnCfgLoopCycleProgram_preserved :
    evalRFnCfgLoopCycleProgram
      (emitRFnCfgLoopCycleProgram (lowerFnCfgLoopCycleProgram branchingFnCfgLoopCycleProgram)) =
      some (.int 122) := by
  rw [lowerEmitFnCfgLoopCycleProgram_preserves_eval]
  have hThen :
      evalSrcFnCfgReentryProgram branchingFnCfgReentryProgram .thenBranch = some (.int 35) := by
    rw [← lowerEmitFnCfgReentryProgram_preserves_eval]
    simpa using branchingFnCfgReentryProgram_then_preserved
  have hElse :
      evalSrcFnCfgReentryProgram branchingFnCfgReentryProgram .elseBranch = some (.int 48) := by
    rw [← lowerEmitFnCfgReentryProgram_preserves_eval]
    simpa using branchingFnCfgReentryProgram_else_preserved
  simp [branchingFnCfgLoopCycleProgram, evalSrcFnCfgLoopCycleProgram, evalSrcLoopChoices,
    evalSrcCycleStep, evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet,
    lookupField, hThen, hElse]

end RRProofs
