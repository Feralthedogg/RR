import RRProofs.PipelineFnCfgOpenSearchFrontierHaltDiscoverSubset

namespace RRProofs

structure SrcFnCfgOpenSearchFrontierReopenProgram where
  haltProg : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram
  completed : List PriorityTrace
  frontier : List PriorityTrace

structure MirFnCfgOpenSearchFrontierReopenProgram where
  haltProg : MirFnCfgOpenSearchFrontierHaltDiscoverProgram
  completed : List PriorityTrace
  frontier : List PriorityTrace

structure RFnCfgOpenSearchFrontierReopenProgram where
  haltProg : RFnCfgOpenSearchFrontierHaltDiscoverProgram
  completed : List PriorityTrace
  frontier : List PriorityTrace

def lowerFnCfgOpenSearchFrontierReopenProgram
    (p : SrcFnCfgOpenSearchFrontierReopenProgram) : MirFnCfgOpenSearchFrontierReopenProgram :=
  { haltProg := lowerFnCfgOpenSearchFrontierHaltDiscoverProgram p.haltProg
  , completed := p.completed
  , frontier := p.frontier
  }

def emitRFnCfgOpenSearchFrontierReopenProgram
    (p : MirFnCfgOpenSearchFrontierReopenProgram) : RFnCfgOpenSearchFrontierReopenProgram :=
  { haltProg := emitRFnCfgOpenSearchFrontierHaltDiscoverProgram p.haltProg
  , completed := p.completed
  , frontier := p.frontier
  }

def evalSrcFnCfgOpenSearchFrontierReopenProgram (p : SrcFnCfgOpenSearchFrontierReopenProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchFrontierHaltDiscoverProgram p.haltProg

def evalMirFnCfgOpenSearchFrontierReopenProgram (p : MirFnCfgOpenSearchFrontierReopenProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchFrontierHaltDiscoverProgram p.haltProg

def evalRFnCfgOpenSearchFrontierReopenProgram (p : RFnCfgOpenSearchFrontierReopenProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchFrontierHaltDiscoverProgram p.haltProg

def srcOpenSearchFrontierReopenWitness (p : SrcFnCfgOpenSearchFrontierReopenProgram) : Prop :=
  srcOpenSearchFrontierHaltDiscoverWitness p.haltProg ∧
    p.haltProg.searchSpace = p.completed ++ p.frontier ∧
    p.haltProg.selected ∈ p.frontier

def mirOpenSearchFrontierReopenWitness (p : MirFnCfgOpenSearchFrontierReopenProgram) : Prop :=
  mirOpenSearchFrontierHaltDiscoverWitness p.haltProg ∧
    p.haltProg.searchSpace = p.completed ++ p.frontier ∧
    p.haltProg.selected ∈ p.frontier

def rOpenSearchFrontierReopenWitness (p : RFnCfgOpenSearchFrontierReopenProgram) : Prop :=
  rOpenSearchFrontierHaltDiscoverWitness p.haltProg ∧
    p.haltProg.searchSpace = p.completed ++ p.frontier ∧
    p.haltProg.selected ∈ p.frontier

theorem lowerFnCfgOpenSearchFrontierReopenProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierReopenProgram) :
    (lowerFnCfgOpenSearchFrontierReopenProgram p).haltProg.protocolProg.summaryProg.rounds.length =
        p.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierReopenProgram p).completed = p.completed ∧
      (lowerFnCfgOpenSearchFrontierReopenProgram p).frontier = p.frontier := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierReopenProgram] using
      (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_meta p.haltProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierReopenProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierReopenProgram) :
    (emitRFnCfgOpenSearchFrontierReopenProgram p).haltProg.protocolProg.summaryProg.rounds.length =
        p.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierReopenProgram p).completed = p.completed ∧
      (emitRFnCfgOpenSearchFrontierReopenProgram p).frontier = p.frontier := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierReopenProgram] using
      (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_meta p.haltProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierReopenProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenProgram) :
    evalMirFnCfgOpenSearchFrontierReopenProgram (lowerFnCfgOpenSearchFrontierReopenProgram p) =
      evalSrcFnCfgOpenSearchFrontierReopenProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierReopenProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierReopenProgram) :
    evalRFnCfgOpenSearchFrontierReopenProgram (emitRFnCfgOpenSearchFrontierReopenProgram p) =
      evalMirFnCfgOpenSearchFrontierReopenProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierReopenProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenProgram) :
    evalRFnCfgOpenSearchFrontierReopenProgram
        (emitRFnCfgOpenSearchFrontierReopenProgram (lowerFnCfgOpenSearchFrontierReopenProgram p)) =
      evalSrcFnCfgOpenSearchFrontierReopenProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierReopenProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenProgram) :
    srcOpenSearchFrontierReopenWitness p →
      mirOpenSearchFrontierReopenWitness (lowerFnCfgOpenSearchFrontierReopenProgram p) := by
  intro h
  rcases h with ⟨hHalt, hSplit, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_witness _ hHalt
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierReopenProgram] using hSplit
  · simpa [lowerFnCfgOpenSearchFrontierReopenProgram] using hMem

theorem emitRFnCfgOpenSearchFrontierReopenProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierReopenProgram) :
    mirOpenSearchFrontierReopenWitness p →
      rOpenSearchFrontierReopenWitness (emitRFnCfgOpenSearchFrontierReopenProgram p) := by
  intro h
  rcases h with ⟨hHalt, hSplit, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_witness _ hHalt
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierReopenProgram] using hSplit
  · simpa [emitRFnCfgOpenSearchFrontierReopenProgram] using hMem

theorem lowerEmitFnCfgOpenSearchFrontierReopenProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenProgram) :
    srcOpenSearchFrontierReopenWitness p →
      rOpenSearchFrontierReopenWitness
        (emitRFnCfgOpenSearchFrontierReopenProgram (lowerFnCfgOpenSearchFrontierReopenProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierReopenProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierReopenProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierReopenProgram : SrcFnCfgOpenSearchFrontierReopenProgram :=
  { haltProg := stableFnCfgOpenSearchFrontierHaltDiscoverProgram
  , completed := [[]]
  , frontier := [stableClosedLoopSummary]
  }

theorem stableFnCfgOpenSearchFrontierReopenProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierReopenProgram stableFnCfgOpenSearchFrontierReopenProgram).haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierReopenProgram stableFnCfgOpenSearchFrontierReopenProgram).completed = [[]] ∧
      (lowerFnCfgOpenSearchFrontierReopenProgram stableFnCfgOpenSearchFrontierReopenProgram).frontier =
        [stableClosedLoopSummary] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierReopenProgram_src_witness :
    srcOpenSearchFrontierReopenWitness stableFnCfgOpenSearchFrontierReopenProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierHaltDiscoverProgram_src_witness
  constructor
  · rfl
  · simp [stableFnCfgOpenSearchFrontierReopenProgram,
      stableFnCfgOpenSearchFrontierHaltDiscoverProgram]

theorem stableFnCfgOpenSearchFrontierReopenProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierReopenProgram
      (emitRFnCfgOpenSearchFrontierReopenProgram
        (lowerFnCfgOpenSearchFrontierReopenProgram stableFnCfgOpenSearchFrontierReopenProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierReopenProgram_preserved :
    rOpenSearchFrontierReopenWitness
      (emitRFnCfgOpenSearchFrontierReopenProgram
        (lowerFnCfgOpenSearchFrontierReopenProgram stableFnCfgOpenSearchFrontierReopenProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierReopenProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierReopenProgram_src_witness

end RRProofs
