import RRProofs.PipelineFnCfgHaltDiscoverSubset

namespace RRProofs

structure SrcFnCfgOpenSearchProgram where
  haltProg : SrcFnCfgHaltDiscoverProgram
  completed : List PriorityTrace
  frontier : List PriorityTrace

structure MirFnCfgOpenSearchProgram where
  haltProg : MirFnCfgHaltDiscoverProgram
  completed : List PriorityTrace
  frontier : List PriorityTrace

structure RFnCfgOpenSearchProgram where
  haltProg : RFnCfgHaltDiscoverProgram
  completed : List PriorityTrace
  frontier : List PriorityTrace

def lowerFnCfgOpenSearchProgram (p : SrcFnCfgOpenSearchProgram) : MirFnCfgOpenSearchProgram :=
  { haltProg := lowerFnCfgHaltDiscoverProgram p.haltProg
  , completed := p.completed
  , frontier := p.frontier
  }

def emitRFnCfgOpenSearchProgram (p : MirFnCfgOpenSearchProgram) : RFnCfgOpenSearchProgram :=
  { haltProg := emitRFnCfgHaltDiscoverProgram p.haltProg
  , completed := p.completed
  , frontier := p.frontier
  }

def evalSrcFnCfgOpenSearchProgram (p : SrcFnCfgOpenSearchProgram) : PriorityTrace :=
  evalSrcFnCfgHaltDiscoverProgram p.haltProg

def evalMirFnCfgOpenSearchProgram (p : MirFnCfgOpenSearchProgram) : PriorityTrace :=
  evalMirFnCfgHaltDiscoverProgram p.haltProg

def evalRFnCfgOpenSearchProgram (p : RFnCfgOpenSearchProgram) : PriorityTrace :=
  evalRFnCfgHaltDiscoverProgram p.haltProg

def srcOpenSearchWitness (p : SrcFnCfgOpenSearchProgram) : Prop :=
  srcHaltDiscoverWitness p.haltProg ∧
    p.haltProg.searchSpace = p.completed ++ p.frontier ∧
    p.haltProg.selected ∈ p.frontier

def mirOpenSearchWitness (p : MirFnCfgOpenSearchProgram) : Prop :=
  mirHaltDiscoverWitness p.haltProg ∧
    p.haltProg.searchSpace = p.completed ++ p.frontier ∧
    p.haltProg.selected ∈ p.frontier

def rOpenSearchWitness (p : RFnCfgOpenSearchProgram) : Prop :=
  rHaltDiscoverWitness p.haltProg ∧
    p.haltProg.searchSpace = p.completed ++ p.frontier ∧
    p.haltProg.selected ∈ p.frontier

theorem lowerFnCfgOpenSearchProgram_preserves_meta
    (p : SrcFnCfgOpenSearchProgram) :
    (lowerFnCfgOpenSearchProgram p).haltProg.protocolProg.summaryProg.rounds.length =
        p.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchProgram p).completed = p.completed ∧
      (lowerFnCfgOpenSearchProgram p).frontier = p.frontier := by
  constructor
  · simpa [lowerFnCfgOpenSearchProgram] using
      (lowerFnCfgHaltDiscoverProgram_preserves_meta p.haltProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchProgram_preserves_meta
    (p : MirFnCfgOpenSearchProgram) :
    (emitRFnCfgOpenSearchProgram p).haltProg.protocolProg.summaryProg.rounds.length =
        p.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchProgram p).completed = p.completed ∧
      (emitRFnCfgOpenSearchProgram p).frontier = p.frontier := by
  constructor
  · simpa [emitRFnCfgOpenSearchProgram] using
      (emitRFnCfgHaltDiscoverProgram_preserves_meta p.haltProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchProgram_preserves_eval
    (p : SrcFnCfgOpenSearchProgram) :
    evalMirFnCfgOpenSearchProgram (lowerFnCfgOpenSearchProgram p) =
      evalSrcFnCfgOpenSearchProgram p := by
  rfl

theorem emitRFnCfgOpenSearchProgram_preserves_eval
    (p : MirFnCfgOpenSearchProgram) :
    evalRFnCfgOpenSearchProgram (emitRFnCfgOpenSearchProgram p) =
      evalMirFnCfgOpenSearchProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchProgram_preserves_eval
    (p : SrcFnCfgOpenSearchProgram) :
    evalRFnCfgOpenSearchProgram (emitRFnCfgOpenSearchProgram (lowerFnCfgOpenSearchProgram p)) =
      evalSrcFnCfgOpenSearchProgram p := by
  rfl

theorem lowerFnCfgOpenSearchProgram_preserves_witness
    (p : SrcFnCfgOpenSearchProgram) :
    srcOpenSearchWitness p →
      mirOpenSearchWitness (lowerFnCfgOpenSearchProgram p) := by
  intro h
  rcases h with ⟨hHalt, hSplit, hMem⟩
  constructor
  · exact lowerFnCfgHaltDiscoverProgram_preserves_witness _ hHalt
  constructor
  · simpa [lowerFnCfgOpenSearchProgram] using hSplit
  · simpa [lowerFnCfgOpenSearchProgram] using hMem

theorem emitRFnCfgOpenSearchProgram_preserves_witness
    (p : MirFnCfgOpenSearchProgram) :
    mirOpenSearchWitness p →
      rOpenSearchWitness (emitRFnCfgOpenSearchProgram p) := by
  intro h
  rcases h with ⟨hHalt, hSplit, hMem⟩
  constructor
  · exact emitRFnCfgHaltDiscoverProgram_preserves_witness _ hHalt
  constructor
  · simpa [emitRFnCfgOpenSearchProgram] using hSplit
  · simpa [emitRFnCfgOpenSearchProgram] using hMem

theorem lowerEmitFnCfgOpenSearchProgram_preserves_witness
    (p : SrcFnCfgOpenSearchProgram) :
    srcOpenSearchWitness p →
      rOpenSearchWitness (emitRFnCfgOpenSearchProgram (lowerFnCfgOpenSearchProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchProgram_preserves_witness _
    (lowerFnCfgOpenSearchProgram_preserves_witness _ h)

def stableFnCfgOpenSearchProgram : SrcFnCfgOpenSearchProgram :=
  { haltProg := stableFnCfgHaltDiscoverProgram
  , completed := [[]]
  , frontier := [stableClosedLoopSummary]
  }

theorem stableFnCfgOpenSearchProgram_meta_preserved :
    (lowerFnCfgOpenSearchProgram stableFnCfgOpenSearchProgram).haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchProgram stableFnCfgOpenSearchProgram).completed = [[]] ∧
      (lowerFnCfgOpenSearchProgram stableFnCfgOpenSearchProgram).frontier = [stableClosedLoopSummary] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchProgram_src_witness :
    srcOpenSearchWitness stableFnCfgOpenSearchProgram := by
  constructor
  · exact stableFnCfgHaltDiscoverProgram_src_witness
  constructor
  · rfl
  · simp [stableFnCfgOpenSearchProgram, stableFnCfgHaltDiscoverProgram]

theorem stableFnCfgOpenSearchProgram_eval_preserved :
    evalRFnCfgOpenSearchProgram
      (emitRFnCfgOpenSearchProgram (lowerFnCfgOpenSearchProgram stableFnCfgOpenSearchProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchProgram_preserved :
    rOpenSearchWitness
      (emitRFnCfgOpenSearchProgram (lowerFnCfgOpenSearchProgram stableFnCfgOpenSearchProgram)) := by
  exact lowerEmitFnCfgOpenSearchProgram_preserves_witness _
    stableFnCfgOpenSearchProgram_src_witness

end RRProofs
