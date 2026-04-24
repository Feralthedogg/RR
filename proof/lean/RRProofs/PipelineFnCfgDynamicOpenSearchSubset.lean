import RRProofs.PipelineFnCfgOpenSearchSubset

namespace RRProofs

structure SrcFnCfgDynamicOpenSearchProgram where
  openProg : SrcFnCfgOpenSearchProgram
  discovered : List PriorityTrace
  nextFrontier : List PriorityTrace

structure MirFnCfgDynamicOpenSearchProgram where
  openProg : MirFnCfgOpenSearchProgram
  discovered : List PriorityTrace
  nextFrontier : List PriorityTrace

structure RFnCfgDynamicOpenSearchProgram where
  openProg : RFnCfgOpenSearchProgram
  discovered : List PriorityTrace
  nextFrontier : List PriorityTrace

def lowerFnCfgDynamicOpenSearchProgram
    (p : SrcFnCfgDynamicOpenSearchProgram) : MirFnCfgDynamicOpenSearchProgram :=
  { openProg := lowerFnCfgOpenSearchProgram p.openProg
  , discovered := p.discovered
  , nextFrontier := p.nextFrontier
  }

def emitRFnCfgDynamicOpenSearchProgram
    (p : MirFnCfgDynamicOpenSearchProgram) : RFnCfgDynamicOpenSearchProgram :=
  { openProg := emitRFnCfgOpenSearchProgram p.openProg
  , discovered := p.discovered
  , nextFrontier := p.nextFrontier
  }

def evalSrcFnCfgDynamicOpenSearchProgram (p : SrcFnCfgDynamicOpenSearchProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchProgram p.openProg

def evalMirFnCfgDynamicOpenSearchProgram (p : MirFnCfgDynamicOpenSearchProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchProgram p.openProg

def evalRFnCfgDynamicOpenSearchProgram (p : RFnCfgDynamicOpenSearchProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchProgram p.openProg

def srcDynamicOpenSearchWitness (p : SrcFnCfgDynamicOpenSearchProgram) : Prop :=
  srcOpenSearchWitness p.openProg ∧
    p.nextFrontier = p.openProg.frontier ++ p.discovered ∧
    p.openProg.haltProg.selected ∈ p.nextFrontier

def mirDynamicOpenSearchWitness (p : MirFnCfgDynamicOpenSearchProgram) : Prop :=
  mirOpenSearchWitness p.openProg ∧
    p.nextFrontier = p.openProg.frontier ++ p.discovered ∧
    p.openProg.haltProg.selected ∈ p.nextFrontier

def rDynamicOpenSearchWitness (p : RFnCfgDynamicOpenSearchProgram) : Prop :=
  rOpenSearchWitness p.openProg ∧
    p.nextFrontier = p.openProg.frontier ++ p.discovered ∧
    p.openProg.haltProg.selected ∈ p.nextFrontier

theorem lowerFnCfgDynamicOpenSearchProgram_preserves_meta
    (p : SrcFnCfgDynamicOpenSearchProgram) :
    (lowerFnCfgDynamicOpenSearchProgram p).openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgDynamicOpenSearchProgram p).discovered = p.discovered ∧
      (lowerFnCfgDynamicOpenSearchProgram p).nextFrontier = p.nextFrontier := by
  constructor
  · simpa [lowerFnCfgDynamicOpenSearchProgram] using
      (lowerFnCfgOpenSearchProgram_preserves_meta p.openProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgDynamicOpenSearchProgram_preserves_meta
    (p : MirFnCfgDynamicOpenSearchProgram) :
    (emitRFnCfgDynamicOpenSearchProgram p).openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgDynamicOpenSearchProgram p).discovered = p.discovered ∧
      (emitRFnCfgDynamicOpenSearchProgram p).nextFrontier = p.nextFrontier := by
  constructor
  · simpa [emitRFnCfgDynamicOpenSearchProgram] using
      (emitRFnCfgOpenSearchProgram_preserves_meta p.openProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgDynamicOpenSearchProgram_preserves_eval
    (p : SrcFnCfgDynamicOpenSearchProgram) :
    evalMirFnCfgDynamicOpenSearchProgram (lowerFnCfgDynamicOpenSearchProgram p) =
      evalSrcFnCfgDynamicOpenSearchProgram p := by
  rfl

theorem emitRFnCfgDynamicOpenSearchProgram_preserves_eval
    (p : MirFnCfgDynamicOpenSearchProgram) :
    evalRFnCfgDynamicOpenSearchProgram (emitRFnCfgDynamicOpenSearchProgram p) =
      evalMirFnCfgDynamicOpenSearchProgram p := by
  rfl

theorem lowerEmitFnCfgDynamicOpenSearchProgram_preserves_eval
    (p : SrcFnCfgDynamicOpenSearchProgram) :
    evalRFnCfgDynamicOpenSearchProgram
        (emitRFnCfgDynamicOpenSearchProgram (lowerFnCfgDynamicOpenSearchProgram p)) =
      evalSrcFnCfgDynamicOpenSearchProgram p := by
  rfl

theorem lowerFnCfgDynamicOpenSearchProgram_preserves_witness
    (p : SrcFnCfgDynamicOpenSearchProgram) :
    srcDynamicOpenSearchWitness p →
      mirDynamicOpenSearchWitness (lowerFnCfgDynamicOpenSearchProgram p) := by
  intro h
  rcases h with ⟨hOpen, hNext, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchProgram_preserves_witness _ hOpen
  constructor
  · simpa [lowerFnCfgDynamicOpenSearchProgram] using hNext
  · simpa [lowerFnCfgDynamicOpenSearchProgram] using hMem

theorem emitRFnCfgDynamicOpenSearchProgram_preserves_witness
    (p : MirFnCfgDynamicOpenSearchProgram) :
    mirDynamicOpenSearchWitness p →
      rDynamicOpenSearchWitness (emitRFnCfgDynamicOpenSearchProgram p) := by
  intro h
  rcases h with ⟨hOpen, hNext, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchProgram_preserves_witness _ hOpen
  constructor
  · simpa [emitRFnCfgDynamicOpenSearchProgram] using hNext
  · simpa [emitRFnCfgDynamicOpenSearchProgram] using hMem

theorem lowerEmitFnCfgDynamicOpenSearchProgram_preserves_witness
    (p : SrcFnCfgDynamicOpenSearchProgram) :
    srcDynamicOpenSearchWitness p →
      rDynamicOpenSearchWitness
        (emitRFnCfgDynamicOpenSearchProgram (lowerFnCfgDynamicOpenSearchProgram p)) := by
  intro h
  exact emitRFnCfgDynamicOpenSearchProgram_preserves_witness _
    (lowerFnCfgDynamicOpenSearchProgram_preserves_witness _ h)

def stableFnCfgDynamicOpenSearchProgram : SrcFnCfgDynamicOpenSearchProgram :=
  { openProg := stableFnCfgOpenSearchProgram
  , discovered := [[]]
  , nextFrontier := [stableClosedLoopSummary, []]
  }

theorem stableFnCfgDynamicOpenSearchProgram_meta_preserved :
    (lowerFnCfgDynamicOpenSearchProgram stableFnCfgDynamicOpenSearchProgram).openProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgDynamicOpenSearchProgram stableFnCfgDynamicOpenSearchProgram).discovered = [[]] ∧
      (lowerFnCfgDynamicOpenSearchProgram stableFnCfgDynamicOpenSearchProgram).nextFrontier =
        [stableClosedLoopSummary, []] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgDynamicOpenSearchProgram_src_witness :
    srcDynamicOpenSearchWitness stableFnCfgDynamicOpenSearchProgram := by
  constructor
  · exact stableFnCfgOpenSearchProgram_src_witness
  constructor
  · rfl
  · simp [stableFnCfgDynamicOpenSearchProgram, stableFnCfgOpenSearchProgram, stableFnCfgHaltDiscoverProgram]

theorem stableFnCfgDynamicOpenSearchProgram_eval_preserved :
    evalRFnCfgDynamicOpenSearchProgram
      (emitRFnCfgDynamicOpenSearchProgram
        (lowerFnCfgDynamicOpenSearchProgram stableFnCfgDynamicOpenSearchProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgDynamicOpenSearchProgram_preserved :
    rDynamicOpenSearchWitness
      (emitRFnCfgDynamicOpenSearchProgram
        (lowerFnCfgDynamicOpenSearchProgram stableFnCfgDynamicOpenSearchProgram)) := by
  exact lowerEmitFnCfgDynamicOpenSearchProgram_preserves_witness _
    stableFnCfgDynamicOpenSearchProgram_src_witness

end RRProofs
