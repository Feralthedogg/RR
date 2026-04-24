import RRProofs.PipelineFnCfgLoopDiscoverSubset

namespace RRProofs

structure SrcFnCfgLoopWorklistProgram where
  discoverProg : SrcFnCfgLoopDiscoverProgram
  remaining : List RValue
  done : List RValue

structure MirFnCfgLoopWorklistProgram where
  discoverProg : MirFnCfgLoopDiscoverProgram
  remaining : List RValue
  done : List RValue

structure RFnCfgLoopWorklistProgram where
  discoverProg : RFnCfgLoopDiscoverProgram
  remaining : List RValue
  done : List RValue

def srcLoopWorklistUpdate (p : SrcFnCfgLoopWorklistProgram) : List RValue × List RValue :=
  (p.discoverProg.selected :: p.done, p.remaining)

def mirLoopWorklistUpdate (p : MirFnCfgLoopWorklistProgram) : List RValue × List RValue :=
  (p.discoverProg.selected :: p.done, p.remaining)

def rLoopWorklistUpdate (p : RFnCfgLoopWorklistProgram) : List RValue × List RValue :=
  (p.discoverProg.selected :: p.done, p.remaining)

def lowerFnCfgLoopWorklistProgram (p : SrcFnCfgLoopWorklistProgram) : MirFnCfgLoopWorklistProgram :=
  { discoverProg := lowerFnCfgLoopDiscoverProgram p.discoverProg
  , remaining := p.remaining
  , done := p.done
  }

def emitRFnCfgLoopWorklistProgram (p : MirFnCfgLoopWorklistProgram) : RFnCfgLoopWorklistProgram :=
  { discoverProg := emitRFnCfgLoopDiscoverProgram p.discoverProg
  , remaining := p.remaining
  , done := p.done
  }

def srcLoopWorklistWitness (p : SrcFnCfgLoopWorklistProgram) : Prop :=
  p.discoverProg.worklist = p.discoverProg.selected :: p.remaining ∧
    srcLoopDiscoverWitness p.discoverProg

def mirLoopWorklistWitness (p : MirFnCfgLoopWorklistProgram) : Prop :=
  p.discoverProg.worklist = p.discoverProg.selected :: p.remaining ∧
    mirLoopDiscoverWitness p.discoverProg

def rLoopWorklistWitness (p : RFnCfgLoopWorklistProgram) : Prop :=
  p.discoverProg.worklist = p.discoverProg.selected :: p.remaining ∧
    rLoopDiscoverWitness p.discoverProg

theorem lowerFnCfgLoopWorklistProgram_preserves_meta
    (p : SrcFnCfgLoopWorklistProgram) :
    (lowerFnCfgLoopWorklistProgram p).discoverProg.cycleProg.reentryProg.joinExecProg.phiProg.joinBid =
        p.discoverProg.cycleProg.reentryProg.joinExecProg.phiProg.joinBid ∧
      (lowerFnCfgLoopWorklistProgram p).remaining = p.remaining ∧
      (lowerFnCfgLoopWorklistProgram p).done = p.done := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem emitRFnCfgLoopWorklistProgram_preserves_meta
    (p : MirFnCfgLoopWorklistProgram) :
    (emitRFnCfgLoopWorklistProgram p).discoverProg.cycleProg.reentryProg.joinExecProg.phiProg.joinBid =
        p.discoverProg.cycleProg.reentryProg.joinExecProg.phiProg.joinBid ∧
      (emitRFnCfgLoopWorklistProgram p).remaining = p.remaining ∧
      (emitRFnCfgLoopWorklistProgram p).done = p.done := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem lowerFnCfgLoopWorklistProgram_preserves_update
    (p : SrcFnCfgLoopWorklistProgram) :
    mirLoopWorklistUpdate (lowerFnCfgLoopWorklistProgram p) = srcLoopWorklistUpdate p := by
  rfl

theorem emitRFnCfgLoopWorklistProgram_preserves_update
    (p : MirFnCfgLoopWorklistProgram) :
    rLoopWorklistUpdate (emitRFnCfgLoopWorklistProgram p) = mirLoopWorklistUpdate p := by
  rfl

theorem lowerFnCfgLoopWorklistProgram_preserves_witness
    (p : SrcFnCfgLoopWorklistProgram) :
    srcLoopWorklistWitness p →
      mirLoopWorklistWitness (lowerFnCfgLoopWorklistProgram p) := by
  intro h
  rcases h with ⟨hHead, hDisc⟩
  constructor
  · simpa [lowerFnCfgLoopWorklistProgram] using hHead
  · simpa [lowerFnCfgLoopWorklistProgram] using
      lowerFnCfgLoopDiscoverProgram_preserves_witness p.discoverProg hDisc

theorem emitRFnCfgLoopWorklistProgram_preserves_witness
    (p : MirFnCfgLoopWorklistProgram) :
    mirLoopWorklistWitness p →
      rLoopWorklistWitness (emitRFnCfgLoopWorklistProgram p) := by
  intro h
  rcases h with ⟨hHead, hDisc⟩
  constructor
  · simpa [emitRFnCfgLoopWorklistProgram] using hHead
  · simpa [emitRFnCfgLoopWorklistProgram] using
      emitRFnCfgLoopDiscoverProgram_preserves_witness p.discoverProg hDisc

theorem lowerEmitFnCfgLoopWorklistProgram_preserves_witness
    (p : SrcFnCfgLoopWorklistProgram) :
    srcLoopWorklistWitness p →
      rLoopWorklistWitness (emitRFnCfgLoopWorklistProgram (lowerFnCfgLoopWorklistProgram p)) := by
  intro h
  exact emitRFnCfgLoopWorklistProgram_preserves_witness _ (lowerFnCfgLoopWorklistProgram_preserves_witness _ h)

def stableHeadFnCfgLoopDiscoverProgram : SrcFnCfgLoopDiscoverProgram :=
  { cycleProg := stableFnCfgLoopCycleProgram
  , witnessChoice := .thenBranch
  , worklist := [.int 10, .int 12]
  , selected := .int 10
  }

theorem stableHeadFnCfgLoopDiscoverProgram_src_witness :
    srcLoopDiscoverWitness stableHeadFnCfgLoopDiscoverProgram := by
  constructor
  · simp [stableHeadFnCfgLoopDiscoverProgram]
  · simpa [stableHeadFnCfgLoopDiscoverProgram, toSrcFnCfgLoopFixpointProgram] using
      stableFnCfgLoopFixpointProgram_src_witness

def stableFnCfgLoopWorklistProgram : SrcFnCfgLoopWorklistProgram :=
  { discoverProg := stableHeadFnCfgLoopDiscoverProgram
  , remaining := [.int 12]
  , done := [.int 5]
  }

theorem stableFnCfgLoopWorklistProgram_meta_preserved :
    (lowerFnCfgLoopWorklistProgram stableFnCfgLoopWorklistProgram).discoverProg.cycleProg.reentryProg.joinExecProg.phiProg.joinBid = 17 ∧
      (lowerFnCfgLoopWorklistProgram stableFnCfgLoopWorklistProgram).remaining = [.int 12] ∧
      (lowerFnCfgLoopWorklistProgram stableFnCfgLoopWorklistProgram).done = [.int 5] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgLoopWorklistProgram_src_witness :
    srcLoopWorklistWitness stableFnCfgLoopWorklistProgram := by
  constructor
  · rfl
  · exact stableHeadFnCfgLoopDiscoverProgram_src_witness

theorem stableFnCfgLoopWorklistProgram_update_preserved :
    rLoopWorklistUpdate
      (emitRFnCfgLoopWorklistProgram (lowerFnCfgLoopWorklistProgram stableFnCfgLoopWorklistProgram)) =
      ([.int 10, .int 5], [.int 12]) := by
  rfl

theorem stableFnCfgLoopWorklistProgram_preserved :
    rLoopWorklistWitness
      (emitRFnCfgLoopWorklistProgram (lowerFnCfgLoopWorklistProgram stableFnCfgLoopWorklistProgram)) := by
  exact lowerEmitFnCfgLoopWorklistProgram_preserves_witness _ stableFnCfgLoopWorklistProgram_src_witness

end RRProofs
