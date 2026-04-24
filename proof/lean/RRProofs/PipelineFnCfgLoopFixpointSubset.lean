import RRProofs.PipelineFnCfgLoopCycleSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

structure SrcFnCfgLoopFixpointProgram where
  cycleProg : SrcFnCfgLoopCycleProgram
  witnessChoice : BranchChoice
  stable : RValue

structure MirFnCfgLoopFixpointProgram where
  cycleProg : MirFnCfgLoopCycleProgram
  witnessChoice : BranchChoice
  stable : RValue

structure RFnCfgLoopFixpointProgram where
  cycleProg : RFnCfgLoopCycleProgram
  witnessChoice : BranchChoice
  stable : RValue

def lowerFnCfgLoopFixpointProgram (p : SrcFnCfgLoopFixpointProgram) : MirFnCfgLoopFixpointProgram :=
  { cycleProg := lowerFnCfgLoopCycleProgram p.cycleProg
  , witnessChoice := p.witnessChoice
  , stable := p.stable
  }

def emitRFnCfgLoopFixpointProgram (p : MirFnCfgLoopFixpointProgram) : RFnCfgLoopFixpointProgram :=
  { cycleProg := emitRFnCfgLoopCycleProgram p.cycleProg
  , witnessChoice := p.witnessChoice
  , stable := p.stable
  }

def srcLoopFixpointWitness (p : SrcFnCfgLoopFixpointProgram) : Prop :=
  evalSrcFnCfgLoopCycleProgram p.cycleProg = some p.stable ∧
    evalSrcLoopChoices p.cycleProg [p.witnessChoice] p.stable = some p.stable

def mirLoopFixpointWitness (p : MirFnCfgLoopFixpointProgram) : Prop :=
  evalMirFnCfgLoopCycleProgram p.cycleProg = some p.stable ∧
    evalMirLoopChoices p.cycleProg [p.witnessChoice] p.stable = some p.stable

def rLoopFixpointWitness (p : RFnCfgLoopFixpointProgram) : Prop :=
  evalRFnCfgLoopCycleProgram p.cycleProg = some p.stable ∧
    evalRLoopChoices p.cycleProg [p.witnessChoice] p.stable = some p.stable

theorem lowerFnCfgLoopFixpointProgram_preserves_meta
    (p : SrcFnCfgLoopFixpointProgram) :
    (lowerFnCfgLoopFixpointProgram p).cycleProg.reentryProg.joinExecProg.phiProg.joinBid =
        p.cycleProg.reentryProg.joinExecProg.phiProg.joinBid ∧
      (lowerFnCfgLoopFixpointProgram p).witnessChoice = p.witnessChoice ∧
      (lowerFnCfgLoopFixpointProgram p).stable = p.stable := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem emitRFnCfgLoopFixpointProgram_preserves_meta
    (p : MirFnCfgLoopFixpointProgram) :
    (emitRFnCfgLoopFixpointProgram p).cycleProg.reentryProg.joinExecProg.phiProg.joinBid =
        p.cycleProg.reentryProg.joinExecProg.phiProg.joinBid ∧
      (emitRFnCfgLoopFixpointProgram p).witnessChoice = p.witnessChoice ∧
      (emitRFnCfgLoopFixpointProgram p).stable = p.stable := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem lowerFnCfgLoopFixpointProgram_preserves_witness
    (p : SrcFnCfgLoopFixpointProgram) :
    srcLoopFixpointWitness p →
      mirLoopFixpointWitness (lowerFnCfgLoopFixpointProgram p) := by
  intro h
  rcases h with ⟨hCycle, hStep⟩
  constructor
  · simpa [srcLoopFixpointWitness, mirLoopFixpointWitness, lowerFnCfgLoopFixpointProgram] using
      (congrArg id ((lowerFnCfgLoopCycleProgram_preserves_eval p.cycleProg).trans hCycle))
  · simpa [srcLoopFixpointWitness, mirLoopFixpointWitness, lowerFnCfgLoopFixpointProgram] using
      (congrArg id ((lowerLoopChoices_preserves_eval p.cycleProg [p.witnessChoice] p.stable).trans hStep))

theorem emitRFnCfgLoopFixpointProgram_preserves_witness
    (p : MirFnCfgLoopFixpointProgram) :
    mirLoopFixpointWitness p →
      rLoopFixpointWitness (emitRFnCfgLoopFixpointProgram p) := by
  intro h
  rcases h with ⟨hCycle, hStep⟩
  constructor
  · simpa [mirLoopFixpointWitness, rLoopFixpointWitness, emitRFnCfgLoopFixpointProgram] using
      (congrArg id ((emitRFnCfgLoopCycleProgram_preserves_eval p.cycleProg).trans hCycle))
  · simpa [mirLoopFixpointWitness, rLoopFixpointWitness, emitRFnCfgLoopFixpointProgram] using
      (congrArg id ((emitRLoopChoices_preserves_eval p.cycleProg [p.witnessChoice] p.stable).trans hStep))

theorem lowerEmitFnCfgLoopFixpointProgram_preserves_witness
    (p : SrcFnCfgLoopFixpointProgram) :
    srcLoopFixpointWitness p →
      rLoopFixpointWitness (emitRFnCfgLoopFixpointProgram (lowerFnCfgLoopFixpointProgram p)) := by
  intro h
  exact emitRFnCfgLoopFixpointProgram_preserves_witness _ (lowerFnCfgLoopFixpointProgram_preserves_witness _ h)

def stableFnCfgLoopCycleProgram : SrcFnCfgLoopCycleProgram :=
  { reentryProg := branchingFnCfgReentryProgram
  , accName := "acc"
  , cycleName := "cycle"
  , init := .int 10
  , choices := [.thenBranch, .elseBranch]
  , cycleBlock :=
      { bid := 37
      , inEnv := []
      , stmts := []
      , ret := .var "acc"
      }
  }

def stableFnCfgLoopFixpointProgram : SrcFnCfgLoopFixpointProgram :=
  { cycleProg := stableFnCfgLoopCycleProgram
  , witnessChoice := .thenBranch
  , stable := .int 10
  }

theorem stableFnCfgLoopFixpointProgram_meta_preserved :
    (lowerFnCfgLoopFixpointProgram stableFnCfgLoopFixpointProgram).cycleProg.reentryProg.joinExecProg.phiProg.joinBid = 17 ∧
      (lowerFnCfgLoopFixpointProgram stableFnCfgLoopFixpointProgram).witnessChoice = .thenBranch ∧
      (lowerFnCfgLoopFixpointProgram stableFnCfgLoopFixpointProgram).stable = .int 10 := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgLoopFixpointProgram_src_witness :
    srcLoopFixpointWitness stableFnCfgLoopFixpointProgram := by
  constructor
  · simp [srcLoopFixpointWitness, stableFnCfgLoopFixpointProgram, stableFnCfgLoopCycleProgram,
      evalSrcFnCfgLoopCycleProgram, evalSrcLoopChoices, evalSrcCycleStep,
      evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]
    have hThen :
        evalSrcFnCfgReentryProgram branchingFnCfgReentryProgram .thenBranch = some (.int 35) := by
      rw [← lowerEmitFnCfgReentryProgram_preserves_eval]
      simpa using branchingFnCfgReentryProgram_then_preserved
    have hElse :
        evalSrcFnCfgReentryProgram branchingFnCfgReentryProgram .elseBranch = some (.int 48) := by
      rw [← lowerEmitFnCfgReentryProgram_preserves_eval]
      simpa using branchingFnCfgReentryProgram_else_preserved
    rw [hThen, hElse]
    rfl
  · simp [srcLoopFixpointWitness, stableFnCfgLoopFixpointProgram, stableFnCfgLoopCycleProgram,
      evalSrcLoopChoices, evalSrcCycleStep, evalSrcBlockEnvProgram, execSrcStmts,
      execSrcStmt, evalSrc, evalSrcLet, lookupField]
    have hThen :
        evalSrcFnCfgReentryProgram branchingFnCfgReentryProgram .thenBranch = some (.int 35) := by
      rw [← lowerEmitFnCfgReentryProgram_preserves_eval]
      simpa using branchingFnCfgReentryProgram_then_preserved
    rw [hThen]
    rfl

theorem stableFnCfgLoopFixpointProgram_preserved :
    rLoopFixpointWitness
      (emitRFnCfgLoopFixpointProgram (lowerFnCfgLoopFixpointProgram stableFnCfgLoopFixpointProgram)) := by
  exact lowerEmitFnCfgLoopFixpointProgram_preserves_witness _ stableFnCfgLoopFixpointProgram_src_witness

end RRProofs
