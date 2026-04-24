import RRProofs.PipelineFnCfgOpenSearchFrontierSubset

namespace RRProofs

structure SrcFnCfgOpenSearchDynamicFrontierProgram where
  frontierProg : SrcFnCfgOpenSearchFrontierProgram
  discovered : List PriorityTrace
  nextFrontier : List PriorityTrace

structure MirFnCfgOpenSearchDynamicFrontierProgram where
  frontierProg : MirFnCfgOpenSearchFrontierProgram
  discovered : List PriorityTrace
  nextFrontier : List PriorityTrace

structure RFnCfgOpenSearchDynamicFrontierProgram where
  frontierProg : RFnCfgOpenSearchFrontierProgram
  discovered : List PriorityTrace
  nextFrontier : List PriorityTrace

def lowerFnCfgOpenSearchDynamicFrontierProgram
    (p : SrcFnCfgOpenSearchDynamicFrontierProgram) : MirFnCfgOpenSearchDynamicFrontierProgram :=
  { frontierProg := lowerFnCfgOpenSearchFrontierProgram p.frontierProg
  , discovered := p.discovered
  , nextFrontier := p.nextFrontier
  }

def emitRFnCfgOpenSearchDynamicFrontierProgram
    (p : MirFnCfgOpenSearchDynamicFrontierProgram) : RFnCfgOpenSearchDynamicFrontierProgram :=
  { frontierProg := emitRFnCfgOpenSearchFrontierProgram p.frontierProg
  , discovered := p.discovered
  , nextFrontier := p.nextFrontier
  }

def evalSrcFnCfgOpenSearchDynamicFrontierProgram (p : SrcFnCfgOpenSearchDynamicFrontierProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchFrontierProgram p.frontierProg

def evalMirFnCfgOpenSearchDynamicFrontierProgram (p : MirFnCfgOpenSearchDynamicFrontierProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchFrontierProgram p.frontierProg

def evalRFnCfgOpenSearchDynamicFrontierProgram (p : RFnCfgOpenSearchDynamicFrontierProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchFrontierProgram p.frontierProg

def srcOpenSearchDynamicFrontierWitness (p : SrcFnCfgOpenSearchDynamicFrontierProgram) : Prop :=
  srcOpenSearchFrontierWitness p.frontierProg ∧
    p.nextFrontier = p.frontierProg.frontier ++ p.discovered ∧
    p.frontierProg.haltProg.selected ∈ p.nextFrontier

def mirOpenSearchDynamicFrontierWitness (p : MirFnCfgOpenSearchDynamicFrontierProgram) : Prop :=
  mirOpenSearchFrontierWitness p.frontierProg ∧
    p.nextFrontier = p.frontierProg.frontier ++ p.discovered ∧
    p.frontierProg.haltProg.selected ∈ p.nextFrontier

def rOpenSearchDynamicFrontierWitness (p : RFnCfgOpenSearchDynamicFrontierProgram) : Prop :=
  rOpenSearchFrontierWitness p.frontierProg ∧
    p.nextFrontier = p.frontierProg.frontier ++ p.discovered ∧
    p.frontierProg.haltProg.selected ∈ p.nextFrontier

theorem lowerFnCfgOpenSearchDynamicFrontierProgram_preserves_meta
    (p : SrcFnCfgOpenSearchDynamicFrontierProgram) :
    (lowerFnCfgOpenSearchDynamicFrontierProgram p).frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchDynamicFrontierProgram p).discovered = p.discovered ∧
      (lowerFnCfgOpenSearchDynamicFrontierProgram p).nextFrontier = p.nextFrontier := by
  constructor
  · simpa [lowerFnCfgOpenSearchDynamicFrontierProgram] using
      (lowerFnCfgOpenSearchFrontierProgram_preserves_meta p.frontierProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchDynamicFrontierProgram_preserves_meta
    (p : MirFnCfgOpenSearchDynamicFrontierProgram) :
    (emitRFnCfgOpenSearchDynamicFrontierProgram p).frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchDynamicFrontierProgram p).discovered = p.discovered ∧
      (emitRFnCfgOpenSearchDynamicFrontierProgram p).nextFrontier = p.nextFrontier := by
  constructor
  · simpa [emitRFnCfgOpenSearchDynamicFrontierProgram] using
      (emitRFnCfgOpenSearchFrontierProgram_preserves_meta p.frontierProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchDynamicFrontierProgram_preserves_eval
    (p : SrcFnCfgOpenSearchDynamicFrontierProgram) :
    evalMirFnCfgOpenSearchDynamicFrontierProgram (lowerFnCfgOpenSearchDynamicFrontierProgram p) =
      evalSrcFnCfgOpenSearchDynamicFrontierProgram p := by
  rfl

theorem emitRFnCfgOpenSearchDynamicFrontierProgram_preserves_eval
    (p : MirFnCfgOpenSearchDynamicFrontierProgram) :
    evalRFnCfgOpenSearchDynamicFrontierProgram (emitRFnCfgOpenSearchDynamicFrontierProgram p) =
      evalMirFnCfgOpenSearchDynamicFrontierProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchDynamicFrontierProgram_preserves_eval
    (p : SrcFnCfgOpenSearchDynamicFrontierProgram) :
    evalRFnCfgOpenSearchDynamicFrontierProgram
        (emitRFnCfgOpenSearchDynamicFrontierProgram (lowerFnCfgOpenSearchDynamicFrontierProgram p)) =
      evalSrcFnCfgOpenSearchDynamicFrontierProgram p := by
  rfl

theorem lowerFnCfgOpenSearchDynamicFrontierProgram_preserves_witness
    (p : SrcFnCfgOpenSearchDynamicFrontierProgram) :
    srcOpenSearchDynamicFrontierWitness p →
      mirOpenSearchDynamicFrontierWitness (lowerFnCfgOpenSearchDynamicFrontierProgram p) := by
  intro h
  rcases h with ⟨hOpen, hNext, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierProgram_preserves_witness _ hOpen
  constructor
  · simpa [lowerFnCfgOpenSearchDynamicFrontierProgram] using hNext
  · simpa [lowerFnCfgOpenSearchDynamicFrontierProgram] using hMem

theorem emitRFnCfgOpenSearchDynamicFrontierProgram_preserves_witness
    (p : MirFnCfgOpenSearchDynamicFrontierProgram) :
    mirOpenSearchDynamicFrontierWitness p →
      rOpenSearchDynamicFrontierWitness (emitRFnCfgOpenSearchDynamicFrontierProgram p) := by
  intro h
  rcases h with ⟨hOpen, hNext, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierProgram_preserves_witness _ hOpen
  constructor
  · simpa [emitRFnCfgOpenSearchDynamicFrontierProgram] using hNext
  · simpa [emitRFnCfgOpenSearchDynamicFrontierProgram] using hMem

theorem lowerEmitFnCfgOpenSearchDynamicFrontierProgram_preserves_witness
    (p : SrcFnCfgOpenSearchDynamicFrontierProgram) :
    srcOpenSearchDynamicFrontierWitness p →
      rOpenSearchDynamicFrontierWitness
        (emitRFnCfgOpenSearchDynamicFrontierProgram (lowerFnCfgOpenSearchDynamicFrontierProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchDynamicFrontierProgram_preserves_witness _
    (lowerFnCfgOpenSearchDynamicFrontierProgram_preserves_witness _ h)

def stableFnCfgOpenSearchDynamicFrontierProgram : SrcFnCfgOpenSearchDynamicFrontierProgram :=
  { frontierProg := stableFnCfgOpenSearchFrontierProgram
  , discovered := [[]]
  , nextFrontier := [stableClosedLoopSummary, []]
  }

theorem stableFnCfgOpenSearchDynamicFrontierProgram_meta_preserved :
    (lowerFnCfgOpenSearchDynamicFrontierProgram stableFnCfgOpenSearchDynamicFrontierProgram).frontierProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchDynamicFrontierProgram stableFnCfgOpenSearchDynamicFrontierProgram).discovered = [[]] ∧
      (lowerFnCfgOpenSearchDynamicFrontierProgram stableFnCfgOpenSearchDynamicFrontierProgram).nextFrontier =
        [stableClosedLoopSummary, []] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchDynamicFrontierProgram_src_witness :
    srcOpenSearchDynamicFrontierWitness stableFnCfgOpenSearchDynamicFrontierProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierProgram_src_witness
  constructor
  · rfl
  · simp [stableFnCfgOpenSearchDynamicFrontierProgram,
      stableFnCfgOpenSearchFrontierProgram, stableFnCfgOpenSearchHaltDiscoverProgram]

theorem stableFnCfgOpenSearchDynamicFrontierProgram_eval_preserved :
    evalRFnCfgOpenSearchDynamicFrontierProgram
      (emitRFnCfgOpenSearchDynamicFrontierProgram
        (lowerFnCfgOpenSearchDynamicFrontierProgram stableFnCfgOpenSearchDynamicFrontierProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchDynamicFrontierProgram_preserved :
    rOpenSearchDynamicFrontierWitness
      (emitRFnCfgOpenSearchDynamicFrontierProgram
        (lowerFnCfgOpenSearchDynamicFrontierProgram stableFnCfgOpenSearchDynamicFrontierProgram)) := by
  exact lowerEmitFnCfgOpenSearchDynamicFrontierProgram_preserves_witness _
    stableFnCfgOpenSearchDynamicFrontierProgram_src_witness

end RRProofs
