import RRProofs.PipelineFnCfgOpenSearchFrontierReopenSubset

namespace RRProofs

structure SrcFnCfgOpenSearchFrontierReopenDynamicProgram where
  reopenProg : SrcFnCfgOpenSearchFrontierReopenProgram
  discovered : List PriorityTrace
  nextFrontier : List PriorityTrace

structure MirFnCfgOpenSearchFrontierReopenDynamicProgram where
  reopenProg : MirFnCfgOpenSearchFrontierReopenProgram
  discovered : List PriorityTrace
  nextFrontier : List PriorityTrace

structure RFnCfgOpenSearchFrontierReopenDynamicProgram where
  reopenProg : RFnCfgOpenSearchFrontierReopenProgram
  discovered : List PriorityTrace
  nextFrontier : List PriorityTrace

def lowerFnCfgOpenSearchFrontierReopenDynamicProgram
    (p : SrcFnCfgOpenSearchFrontierReopenDynamicProgram) :
    MirFnCfgOpenSearchFrontierReopenDynamicProgram :=
  { reopenProg := lowerFnCfgOpenSearchFrontierReopenProgram p.reopenProg
  , discovered := p.discovered
  , nextFrontier := p.nextFrontier
  }

def emitRFnCfgOpenSearchFrontierReopenDynamicProgram
    (p : MirFnCfgOpenSearchFrontierReopenDynamicProgram) :
    RFnCfgOpenSearchFrontierReopenDynamicProgram :=
  { reopenProg := emitRFnCfgOpenSearchFrontierReopenProgram p.reopenProg
  , discovered := p.discovered
  , nextFrontier := p.nextFrontier
  }

def evalSrcFnCfgOpenSearchFrontierReopenDynamicProgram
    (p : SrcFnCfgOpenSearchFrontierReopenDynamicProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchFrontierReopenProgram p.reopenProg

def evalMirFnCfgOpenSearchFrontierReopenDynamicProgram
    (p : MirFnCfgOpenSearchFrontierReopenDynamicProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchFrontierReopenProgram p.reopenProg

def evalRFnCfgOpenSearchFrontierReopenDynamicProgram
    (p : RFnCfgOpenSearchFrontierReopenDynamicProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchFrontierReopenProgram p.reopenProg

def srcOpenSearchFrontierReopenDynamicWitness
    (p : SrcFnCfgOpenSearchFrontierReopenDynamicProgram) : Prop :=
  srcOpenSearchFrontierReopenWitness p.reopenProg ∧
    p.nextFrontier = p.reopenProg.frontier ++ p.discovered ∧
    p.reopenProg.haltProg.selected ∈ p.nextFrontier

def mirOpenSearchFrontierReopenDynamicWitness
    (p : MirFnCfgOpenSearchFrontierReopenDynamicProgram) : Prop :=
  mirOpenSearchFrontierReopenWitness p.reopenProg ∧
    p.nextFrontier = p.reopenProg.frontier ++ p.discovered ∧
    p.reopenProg.haltProg.selected ∈ p.nextFrontier

def rOpenSearchFrontierReopenDynamicWitness
    (p : RFnCfgOpenSearchFrontierReopenDynamicProgram) : Prop :=
  rOpenSearchFrontierReopenWitness p.reopenProg ∧
    p.nextFrontier = p.reopenProg.frontier ++ p.discovered ∧
    p.reopenProg.haltProg.selected ∈ p.nextFrontier

theorem lowerFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierReopenDynamicProgram) :
    (lowerFnCfgOpenSearchFrontierReopenDynamicProgram p).reopenProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.reopenProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierReopenDynamicProgram p).discovered = p.discovered ∧
      (lowerFnCfgOpenSearchFrontierReopenDynamicProgram p).nextFrontier = p.nextFrontier := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierReopenDynamicProgram] using
      (lowerFnCfgOpenSearchFrontierReopenProgram_preserves_meta p.reopenProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierReopenDynamicProgram) :
    (emitRFnCfgOpenSearchFrontierReopenDynamicProgram p).reopenProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.reopenProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierReopenDynamicProgram p).discovered = p.discovered ∧
      (emitRFnCfgOpenSearchFrontierReopenDynamicProgram p).nextFrontier = p.nextFrontier := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierReopenDynamicProgram] using
      (emitRFnCfgOpenSearchFrontierReopenProgram_preserves_meta p.reopenProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenDynamicProgram) :
    evalMirFnCfgOpenSearchFrontierReopenDynamicProgram (lowerFnCfgOpenSearchFrontierReopenDynamicProgram p) =
      evalSrcFnCfgOpenSearchFrontierReopenDynamicProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierReopenDynamicProgram) :
    evalRFnCfgOpenSearchFrontierReopenDynamicProgram (emitRFnCfgOpenSearchFrontierReopenDynamicProgram p) =
      evalMirFnCfgOpenSearchFrontierReopenDynamicProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenDynamicProgram) :
    evalRFnCfgOpenSearchFrontierReopenDynamicProgram
        (emitRFnCfgOpenSearchFrontierReopenDynamicProgram
          (lowerFnCfgOpenSearchFrontierReopenDynamicProgram p)) =
      evalSrcFnCfgOpenSearchFrontierReopenDynamicProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenDynamicProgram) :
    srcOpenSearchFrontierReopenDynamicWitness p →
      mirOpenSearchFrontierReopenDynamicWitness (lowerFnCfgOpenSearchFrontierReopenDynamicProgram p) := by
  intro h
  rcases h with ⟨hOpen, hNext, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierReopenProgram_preserves_witness _ hOpen
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierReopenDynamicProgram] using hNext
  · simpa [lowerFnCfgOpenSearchFrontierReopenDynamicProgram] using hMem

theorem emitRFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierReopenDynamicProgram) :
    mirOpenSearchFrontierReopenDynamicWitness p →
      rOpenSearchFrontierReopenDynamicWitness (emitRFnCfgOpenSearchFrontierReopenDynamicProgram p) := by
  intro h
  rcases h with ⟨hOpen, hNext, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierReopenProgram_preserves_witness _ hOpen
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierReopenDynamicProgram] using hNext
  · simpa [emitRFnCfgOpenSearchFrontierReopenDynamicProgram] using hMem

theorem lowerEmitFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenDynamicProgram) :
    srcOpenSearchFrontierReopenDynamicWitness p →
      rOpenSearchFrontierReopenDynamicWitness
        (emitRFnCfgOpenSearchFrontierReopenDynamicProgram
          (lowerFnCfgOpenSearchFrontierReopenDynamicProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierReopenDynamicProgram : SrcFnCfgOpenSearchFrontierReopenDynamicProgram :=
  { reopenProg := stableFnCfgOpenSearchFrontierReopenProgram
  , discovered := [[]]
  , nextFrontier := [stableClosedLoopSummary, []]
  }

theorem stableFnCfgOpenSearchFrontierReopenDynamicProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierReopenDynamicProgram stableFnCfgOpenSearchFrontierReopenDynamicProgram).reopenProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierReopenDynamicProgram stableFnCfgOpenSearchFrontierReopenDynamicProgram).discovered = [[]] ∧
      (lowerFnCfgOpenSearchFrontierReopenDynamicProgram stableFnCfgOpenSearchFrontierReopenDynamicProgram).nextFrontier =
        [stableClosedLoopSummary, []] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierReopenDynamicProgram_src_witness :
    srcOpenSearchFrontierReopenDynamicWitness stableFnCfgOpenSearchFrontierReopenDynamicProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierReopenProgram_src_witness
  constructor
  · rfl
  · simp [stableFnCfgOpenSearchFrontierReopenDynamicProgram,
      stableFnCfgOpenSearchFrontierReopenProgram, stableFnCfgOpenSearchFrontierHaltDiscoverProgram]

theorem stableFnCfgOpenSearchFrontierReopenDynamicProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierReopenDynamicProgram
      (emitRFnCfgOpenSearchFrontierReopenDynamicProgram
        (lowerFnCfgOpenSearchFrontierReopenDynamicProgram stableFnCfgOpenSearchFrontierReopenDynamicProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierReopenDynamicProgram_preserved :
    rOpenSearchFrontierReopenDynamicWitness
      (emitRFnCfgOpenSearchFrontierReopenDynamicProgram
        (lowerFnCfgOpenSearchFrontierReopenDynamicProgram stableFnCfgOpenSearchFrontierReopenDynamicProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierReopenDynamicProgram_src_witness

end RRProofs
