import RRProofs.PipelineFnCfgLoopFixpointSubset

namespace RRProofs

structure SrcFnCfgLoopDiscoverProgram where
  cycleProg : SrcFnCfgLoopCycleProgram
  witnessChoice : BranchChoice
  worklist : List RValue
  selected : RValue

structure MirFnCfgLoopDiscoverProgram where
  cycleProg : MirFnCfgLoopCycleProgram
  witnessChoice : BranchChoice
  worklist : List RValue
  selected : RValue

structure RFnCfgLoopDiscoverProgram where
  cycleProg : RFnCfgLoopCycleProgram
  witnessChoice : BranchChoice
  worklist : List RValue
  selected : RValue

def toSrcFnCfgLoopFixpointProgram (p : SrcFnCfgLoopDiscoverProgram) : SrcFnCfgLoopFixpointProgram :=
  { cycleProg := p.cycleProg
  , witnessChoice := p.witnessChoice
  , stable := p.selected
  }

def toMirFnCfgLoopFixpointProgram (p : MirFnCfgLoopDiscoverProgram) : MirFnCfgLoopFixpointProgram :=
  { cycleProg := p.cycleProg
  , witnessChoice := p.witnessChoice
  , stable := p.selected
  }

def toRFnCfgLoopFixpointProgram (p : RFnCfgLoopDiscoverProgram) : RFnCfgLoopFixpointProgram :=
  { cycleProg := p.cycleProg
  , witnessChoice := p.witnessChoice
  , stable := p.selected
  }

def lowerFnCfgLoopDiscoverProgram (p : SrcFnCfgLoopDiscoverProgram) : MirFnCfgLoopDiscoverProgram :=
  { cycleProg := lowerFnCfgLoopCycleProgram p.cycleProg
  , witnessChoice := p.witnessChoice
  , worklist := p.worklist
  , selected := p.selected
  }

def emitRFnCfgLoopDiscoverProgram (p : MirFnCfgLoopDiscoverProgram) : RFnCfgLoopDiscoverProgram :=
  { cycleProg := emitRFnCfgLoopCycleProgram p.cycleProg
  , witnessChoice := p.witnessChoice
  , worklist := p.worklist
  , selected := p.selected
  }

def srcLoopDiscoverWitness (p : SrcFnCfgLoopDiscoverProgram) : Prop :=
  p.selected ∈ p.worklist ∧ srcLoopFixpointWitness (toSrcFnCfgLoopFixpointProgram p)

def mirLoopDiscoverWitness (p : MirFnCfgLoopDiscoverProgram) : Prop :=
  p.selected ∈ p.worklist ∧ mirLoopFixpointWitness (toMirFnCfgLoopFixpointProgram p)

def rLoopDiscoverWitness (p : RFnCfgLoopDiscoverProgram) : Prop :=
  p.selected ∈ p.worklist ∧ rLoopFixpointWitness (toRFnCfgLoopFixpointProgram p)

theorem lowerFnCfgLoopDiscoverProgram_preserves_meta
    (p : SrcFnCfgLoopDiscoverProgram) :
    (lowerFnCfgLoopDiscoverProgram p).cycleProg.reentryProg.joinExecProg.phiProg.joinBid =
        p.cycleProg.reentryProg.joinExecProg.phiProg.joinBid ∧
      (lowerFnCfgLoopDiscoverProgram p).witnessChoice = p.witnessChoice ∧
      (lowerFnCfgLoopDiscoverProgram p).worklist = p.worklist ∧
      (lowerFnCfgLoopDiscoverProgram p).selected = p.selected := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem emitRFnCfgLoopDiscoverProgram_preserves_meta
    (p : MirFnCfgLoopDiscoverProgram) :
    (emitRFnCfgLoopDiscoverProgram p).cycleProg.reentryProg.joinExecProg.phiProg.joinBid =
        p.cycleProg.reentryProg.joinExecProg.phiProg.joinBid ∧
      (emitRFnCfgLoopDiscoverProgram p).witnessChoice = p.witnessChoice ∧
      (emitRFnCfgLoopDiscoverProgram p).worklist = p.worklist ∧
      (emitRFnCfgLoopDiscoverProgram p).selected = p.selected := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem lowerFnCfgLoopDiscoverProgram_preserves_witness
    (p : SrcFnCfgLoopDiscoverProgram) :
    srcLoopDiscoverWitness p →
      mirLoopDiscoverWitness (lowerFnCfgLoopDiscoverProgram p) := by
  intro h
  rcases h with ⟨hMem, hFix⟩
  constructor
  · simpa [srcLoopDiscoverWitness, mirLoopDiscoverWitness, lowerFnCfgLoopDiscoverProgram]
      using hMem
  · simpa [srcLoopDiscoverWitness, mirLoopDiscoverWitness, toSrcFnCfgLoopFixpointProgram,
      toMirFnCfgLoopFixpointProgram, lowerFnCfgLoopDiscoverProgram, lowerFnCfgLoopFixpointProgram]
      using lowerFnCfgLoopFixpointProgram_preserves_witness (toSrcFnCfgLoopFixpointProgram p) hFix

theorem emitRFnCfgLoopDiscoverProgram_preserves_witness
    (p : MirFnCfgLoopDiscoverProgram) :
    mirLoopDiscoverWitness p →
      rLoopDiscoverWitness (emitRFnCfgLoopDiscoverProgram p) := by
  intro h
  rcases h with ⟨hMem, hFix⟩
  constructor
  · simpa [mirLoopDiscoverWitness, rLoopDiscoverWitness, emitRFnCfgLoopDiscoverProgram]
      using hMem
  · simpa [mirLoopDiscoverWitness, rLoopDiscoverWitness, toMirFnCfgLoopFixpointProgram,
      toRFnCfgLoopFixpointProgram, emitRFnCfgLoopDiscoverProgram, emitRFnCfgLoopFixpointProgram]
      using emitRFnCfgLoopFixpointProgram_preserves_witness (toMirFnCfgLoopFixpointProgram p) hFix

theorem lowerEmitFnCfgLoopDiscoverProgram_preserves_witness
    (p : SrcFnCfgLoopDiscoverProgram) :
    srcLoopDiscoverWitness p →
      rLoopDiscoverWitness (emitRFnCfgLoopDiscoverProgram (lowerFnCfgLoopDiscoverProgram p)) := by
  intro h
  exact emitRFnCfgLoopDiscoverProgram_preserves_witness _ (lowerFnCfgLoopDiscoverProgram_preserves_witness _ h)

def stableFnCfgLoopDiscoverProgram : SrcFnCfgLoopDiscoverProgram :=
  { cycleProg := stableFnCfgLoopCycleProgram
  , witnessChoice := .thenBranch
  , worklist := [.int 7, .int 10, .int 12]
  , selected := .int 10
  }

theorem stableFnCfgLoopDiscoverProgram_meta_preserved :
    (lowerFnCfgLoopDiscoverProgram stableFnCfgLoopDiscoverProgram).cycleProg.reentryProg.joinExecProg.phiProg.joinBid = 17 ∧
      (lowerFnCfgLoopDiscoverProgram stableFnCfgLoopDiscoverProgram).witnessChoice = .thenBranch ∧
      (lowerFnCfgLoopDiscoverProgram stableFnCfgLoopDiscoverProgram).worklist = [.int 7, .int 10, .int 12] ∧
      (lowerFnCfgLoopDiscoverProgram stableFnCfgLoopDiscoverProgram).selected = .int 10 := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgLoopDiscoverProgram_src_witness :
    srcLoopDiscoverWitness stableFnCfgLoopDiscoverProgram := by
  constructor
  · simp [stableFnCfgLoopDiscoverProgram]
  · simpa [stableFnCfgLoopDiscoverProgram, toSrcFnCfgLoopFixpointProgram] using
      stableFnCfgLoopFixpointProgram_src_witness

theorem stableFnCfgLoopDiscoverProgram_preserved :
    rLoopDiscoverWitness
      (emitRFnCfgLoopDiscoverProgram (lowerFnCfgLoopDiscoverProgram stableFnCfgLoopDiscoverProgram)) := by
  exact lowerEmitFnCfgLoopDiscoverProgram_preserves_witness _ stableFnCfgLoopDiscoverProgram_src_witness

end RRProofs
