import RRProofs.PipelineFnCfgOpenSearchHaltDiscoverSubset

namespace RRProofs

structure SrcFnCfgOpenSearchFrontierProgram where
  haltProg : SrcFnCfgOpenSearchHaltDiscoverProgram
  completed : List PriorityTrace
  frontier : List PriorityTrace

structure MirFnCfgOpenSearchFrontierProgram where
  haltProg : MirFnCfgOpenSearchHaltDiscoverProgram
  completed : List PriorityTrace
  frontier : List PriorityTrace

structure RFnCfgOpenSearchFrontierProgram where
  haltProg : RFnCfgOpenSearchHaltDiscoverProgram
  completed : List PriorityTrace
  frontier : List PriorityTrace

def lowerFnCfgOpenSearchFrontierProgram
    (p : SrcFnCfgOpenSearchFrontierProgram) : MirFnCfgOpenSearchFrontierProgram :=
  { haltProg := lowerFnCfgOpenSearchHaltDiscoverProgram p.haltProg
  , completed := p.completed
  , frontier := p.frontier
  }

def emitRFnCfgOpenSearchFrontierProgram
    (p : MirFnCfgOpenSearchFrontierProgram) : RFnCfgOpenSearchFrontierProgram :=
  { haltProg := emitRFnCfgOpenSearchHaltDiscoverProgram p.haltProg
  , completed := p.completed
  , frontier := p.frontier
  }

def evalSrcFnCfgOpenSearchFrontierProgram (p : SrcFnCfgOpenSearchFrontierProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchHaltDiscoverProgram p.haltProg

def evalMirFnCfgOpenSearchFrontierProgram (p : MirFnCfgOpenSearchFrontierProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchHaltDiscoverProgram p.haltProg

def evalRFnCfgOpenSearchFrontierProgram (p : RFnCfgOpenSearchFrontierProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchHaltDiscoverProgram p.haltProg

def srcOpenSearchFrontierWitness (p : SrcFnCfgOpenSearchFrontierProgram) : Prop :=
  srcOpenSearchHaltDiscoverWitness p.haltProg ∧
    p.haltProg.searchSpace = p.completed ++ p.frontier ∧
    p.haltProg.selected ∈ p.frontier

def mirOpenSearchFrontierWitness (p : MirFnCfgOpenSearchFrontierProgram) : Prop :=
  mirOpenSearchHaltDiscoverWitness p.haltProg ∧
    p.haltProg.searchSpace = p.completed ++ p.frontier ∧
    p.haltProg.selected ∈ p.frontier

def rOpenSearchFrontierWitness (p : RFnCfgOpenSearchFrontierProgram) : Prop :=
  rOpenSearchHaltDiscoverWitness p.haltProg ∧
    p.haltProg.searchSpace = p.completed ++ p.frontier ∧
    p.haltProg.selected ∈ p.frontier

theorem lowerFnCfgOpenSearchFrontierProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierProgram) :
    (lowerFnCfgOpenSearchFrontierProgram p).haltProg.protocolProg.summaryProg.rounds.length =
        p.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierProgram p).completed = p.completed ∧
      (lowerFnCfgOpenSearchFrontierProgram p).frontier = p.frontier := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierProgram] using
      (lowerFnCfgOpenSearchHaltDiscoverProgram_preserves_meta p.haltProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierProgram) :
    (emitRFnCfgOpenSearchFrontierProgram p).haltProg.protocolProg.summaryProg.rounds.length =
        p.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierProgram p).completed = p.completed ∧
      (emitRFnCfgOpenSearchFrontierProgram p).frontier = p.frontier := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierProgram] using
      (emitRFnCfgOpenSearchHaltDiscoverProgram_preserves_meta p.haltProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierProgram) :
    evalMirFnCfgOpenSearchFrontierProgram (lowerFnCfgOpenSearchFrontierProgram p) =
      evalSrcFnCfgOpenSearchFrontierProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierProgram) :
    evalRFnCfgOpenSearchFrontierProgram (emitRFnCfgOpenSearchFrontierProgram p) =
      evalMirFnCfgOpenSearchFrontierProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierProgram) :
    evalRFnCfgOpenSearchFrontierProgram
        (emitRFnCfgOpenSearchFrontierProgram (lowerFnCfgOpenSearchFrontierProgram p)) =
      evalSrcFnCfgOpenSearchFrontierProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierProgram) :
    srcOpenSearchFrontierWitness p →
      mirOpenSearchFrontierWitness (lowerFnCfgOpenSearchFrontierProgram p) := by
  intro h
  rcases h with ⟨hHalt, hSplit, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchHaltDiscoverProgram_preserves_witness _ hHalt
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierProgram] using hSplit
  · simpa [lowerFnCfgOpenSearchFrontierProgram] using hMem

theorem emitRFnCfgOpenSearchFrontierProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierProgram) :
    mirOpenSearchFrontierWitness p →
      rOpenSearchFrontierWitness (emitRFnCfgOpenSearchFrontierProgram p) := by
  intro h
  rcases h with ⟨hHalt, hSplit, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchHaltDiscoverProgram_preserves_witness _ hHalt
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierProgram] using hSplit
  · simpa [emitRFnCfgOpenSearchFrontierProgram] using hMem

theorem lowerEmitFnCfgOpenSearchFrontierProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierProgram) :
    srcOpenSearchFrontierWitness p →
      rOpenSearchFrontierWitness
        (emitRFnCfgOpenSearchFrontierProgram (lowerFnCfgOpenSearchFrontierProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierProgram : SrcFnCfgOpenSearchFrontierProgram :=
  { haltProg := stableFnCfgOpenSearchHaltDiscoverProgram
  , completed := [[]]
  , frontier := [stableClosedLoopSummary]
  }

theorem stableFnCfgOpenSearchFrontierProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierProgram stableFnCfgOpenSearchFrontierProgram).haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierProgram stableFnCfgOpenSearchFrontierProgram).completed = [[]] ∧
      (lowerFnCfgOpenSearchFrontierProgram stableFnCfgOpenSearchFrontierProgram).frontier =
        [stableClosedLoopSummary] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierProgram_src_witness :
    srcOpenSearchFrontierWitness stableFnCfgOpenSearchFrontierProgram := by
  constructor
  · exact stableFnCfgOpenSearchHaltDiscoverProgram_src_witness
  constructor
  · rfl
  · simp [stableFnCfgOpenSearchFrontierProgram, stableFnCfgOpenSearchHaltDiscoverProgram]

theorem stableFnCfgOpenSearchFrontierProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierProgram
      (emitRFnCfgOpenSearchFrontierProgram
        (lowerFnCfgOpenSearchFrontierProgram stableFnCfgOpenSearchFrontierProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierProgram_preserved :
    rOpenSearchFrontierWitness
      (emitRFnCfgOpenSearchFrontierProgram
        (lowerFnCfgOpenSearchFrontierProgram stableFnCfgOpenSearchFrontierProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierProgram_src_witness

end RRProofs
