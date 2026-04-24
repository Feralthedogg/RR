import RRProofs.PipelineFnCfgOpenSearchConvergenceProtocolSubset

namespace RRProofs

structure SrcFnCfgOpenSearchHaltDiscoverProgram where
  protocolProg : SrcFnCfgOpenSearchConvergenceProtocolProgram
  searchSpace : List PriorityTrace
  selected : PriorityTrace

structure MirFnCfgOpenSearchHaltDiscoverProgram where
  protocolProg : MirFnCfgOpenSearchConvergenceProtocolProgram
  searchSpace : List PriorityTrace
  selected : PriorityTrace

structure RFnCfgOpenSearchHaltDiscoverProgram where
  protocolProg : RFnCfgOpenSearchConvergenceProtocolProgram
  searchSpace : List PriorityTrace
  selected : PriorityTrace

def lowerFnCfgOpenSearchHaltDiscoverProgram
    (p : SrcFnCfgOpenSearchHaltDiscoverProgram) : MirFnCfgOpenSearchHaltDiscoverProgram :=
  { protocolProg := lowerFnCfgOpenSearchConvergenceProtocolProgram p.protocolProg
  , searchSpace := p.searchSpace
  , selected := p.selected
  }

def emitRFnCfgOpenSearchHaltDiscoverProgram
    (p : MirFnCfgOpenSearchHaltDiscoverProgram) : RFnCfgOpenSearchHaltDiscoverProgram :=
  { protocolProg := emitRFnCfgOpenSearchConvergenceProtocolProgram p.protocolProg
  , searchSpace := p.searchSpace
  , selected := p.selected
  }

def evalSrcFnCfgOpenSearchHaltDiscoverProgram (p : SrcFnCfgOpenSearchHaltDiscoverProgram) : PriorityTrace :=
  p.protocolProg.haltSummary

def evalMirFnCfgOpenSearchHaltDiscoverProgram (p : MirFnCfgOpenSearchHaltDiscoverProgram) : PriorityTrace :=
  p.protocolProg.haltSummary

def evalRFnCfgOpenSearchHaltDiscoverProgram (p : RFnCfgOpenSearchHaltDiscoverProgram) : PriorityTrace :=
  p.protocolProg.haltSummary

def srcOpenSearchHaltDiscoverWitness (p : SrcFnCfgOpenSearchHaltDiscoverProgram) : Prop :=
  srcOpenSearchConvergenceProtocolWitness p.protocolProg ∧
    p.selected ∈ p.searchSpace ∧
    p.selected = p.protocolProg.haltSummary

def mirOpenSearchHaltDiscoverWitness (p : MirFnCfgOpenSearchHaltDiscoverProgram) : Prop :=
  mirOpenSearchConvergenceProtocolWitness p.protocolProg ∧
    p.selected ∈ p.searchSpace ∧
    p.selected = p.protocolProg.haltSummary

def rOpenSearchHaltDiscoverWitness (p : RFnCfgOpenSearchHaltDiscoverProgram) : Prop :=
  rOpenSearchConvergenceProtocolWitness p.protocolProg ∧
    p.selected ∈ p.searchSpace ∧
    p.selected = p.protocolProg.haltSummary

theorem lowerFnCfgOpenSearchHaltDiscoverProgram_preserves_meta
    (p : SrcFnCfgOpenSearchHaltDiscoverProgram) :
    (lowerFnCfgOpenSearchHaltDiscoverProgram p).protocolProg.summaryProg.rounds.length =
        p.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchHaltDiscoverProgram p).searchSpace = p.searchSpace ∧
      (lowerFnCfgOpenSearchHaltDiscoverProgram p).selected = p.selected := by
  constructor
  · simp [lowerFnCfgOpenSearchHaltDiscoverProgram,
      lowerFnCfgOpenSearchConvergenceProtocolProgram_preserves_meta]
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchHaltDiscoverProgram_preserves_meta
    (p : MirFnCfgOpenSearchHaltDiscoverProgram) :
    (emitRFnCfgOpenSearchHaltDiscoverProgram p).protocolProg.summaryProg.rounds.length =
        p.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchHaltDiscoverProgram p).searchSpace = p.searchSpace ∧
      (emitRFnCfgOpenSearchHaltDiscoverProgram p).selected = p.selected := by
  constructor
  · simp [emitRFnCfgOpenSearchHaltDiscoverProgram,
      emitRFnCfgOpenSearchConvergenceProtocolProgram_preserves_meta]
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchHaltDiscoverProgram_preserves_eval
    (p : SrcFnCfgOpenSearchHaltDiscoverProgram) :
    evalMirFnCfgOpenSearchHaltDiscoverProgram (lowerFnCfgOpenSearchHaltDiscoverProgram p) =
      evalSrcFnCfgOpenSearchHaltDiscoverProgram p := by
  rfl

theorem emitRFnCfgOpenSearchHaltDiscoverProgram_preserves_eval
    (p : MirFnCfgOpenSearchHaltDiscoverProgram) :
    evalRFnCfgOpenSearchHaltDiscoverProgram (emitRFnCfgOpenSearchHaltDiscoverProgram p) =
      evalMirFnCfgOpenSearchHaltDiscoverProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchHaltDiscoverProgram_preserves_eval
    (p : SrcFnCfgOpenSearchHaltDiscoverProgram) :
    evalRFnCfgOpenSearchHaltDiscoverProgram
        (emitRFnCfgOpenSearchHaltDiscoverProgram (lowerFnCfgOpenSearchHaltDiscoverProgram p)) =
      evalSrcFnCfgOpenSearchHaltDiscoverProgram p := by
  rfl

theorem lowerFnCfgOpenSearchHaltDiscoverProgram_preserves_witness
    (p : SrcFnCfgOpenSearchHaltDiscoverProgram) :
    srcOpenSearchHaltDiscoverWitness p →
      mirOpenSearchHaltDiscoverWitness (lowerFnCfgOpenSearchHaltDiscoverProgram p) := by
  intro h
  rcases h with ⟨hConv, hMem, hSel⟩
  constructor
  · exact lowerFnCfgOpenSearchConvergenceProtocolProgram_preserves_witness _ hConv
  constructor
  · simpa [lowerFnCfgOpenSearchHaltDiscoverProgram] using hMem
  · simpa [lowerFnCfgOpenSearchHaltDiscoverProgram] using hSel

theorem emitRFnCfgOpenSearchHaltDiscoverProgram_preserves_witness
    (p : MirFnCfgOpenSearchHaltDiscoverProgram) :
    mirOpenSearchHaltDiscoverWitness p →
      rOpenSearchHaltDiscoverWitness (emitRFnCfgOpenSearchHaltDiscoverProgram p) := by
  intro h
  rcases h with ⟨hConv, hMem, hSel⟩
  constructor
  · exact emitRFnCfgOpenSearchConvergenceProtocolProgram_preserves_witness _ hConv
  constructor
  · simpa [emitRFnCfgOpenSearchHaltDiscoverProgram] using hMem
  · simpa [emitRFnCfgOpenSearchHaltDiscoverProgram] using hSel

theorem lowerEmitFnCfgOpenSearchHaltDiscoverProgram_preserves_witness
    (p : SrcFnCfgOpenSearchHaltDiscoverProgram) :
    srcOpenSearchHaltDiscoverWitness p →
      rOpenSearchHaltDiscoverWitness
        (emitRFnCfgOpenSearchHaltDiscoverProgram (lowerFnCfgOpenSearchHaltDiscoverProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchHaltDiscoverProgram_preserves_witness _
    (lowerFnCfgOpenSearchHaltDiscoverProgram_preserves_witness _ h)

def stableFnCfgOpenSearchHaltDiscoverProgram : SrcFnCfgOpenSearchHaltDiscoverProgram :=
  { protocolProg := stableFnCfgOpenSearchConvergenceProtocolProgram
  , searchSpace := [[], stableClosedLoopSummary]
  , selected := stableClosedLoopSummary
  }

theorem stableFnCfgOpenSearchHaltDiscoverProgram_meta_preserved :
    (lowerFnCfgOpenSearchHaltDiscoverProgram stableFnCfgOpenSearchHaltDiscoverProgram).protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchHaltDiscoverProgram stableFnCfgOpenSearchHaltDiscoverProgram).searchSpace =
        [[], stableClosedLoopSummary] ∧
      (lowerFnCfgOpenSearchHaltDiscoverProgram stableFnCfgOpenSearchHaltDiscoverProgram).selected =
        stableClosedLoopSummary := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchHaltDiscoverProgram_src_witness :
    srcOpenSearchHaltDiscoverWitness stableFnCfgOpenSearchHaltDiscoverProgram := by
  constructor
  · exact stableFnCfgOpenSearchConvergenceProtocolProgram_src_witness
  constructor
  · simp [stableFnCfgOpenSearchHaltDiscoverProgram]
  · rfl

theorem stableFnCfgOpenSearchHaltDiscoverProgram_eval_preserved :
    evalRFnCfgOpenSearchHaltDiscoverProgram
      (emitRFnCfgOpenSearchHaltDiscoverProgram
        (lowerFnCfgOpenSearchHaltDiscoverProgram stableFnCfgOpenSearchHaltDiscoverProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchHaltDiscoverProgram_preserved :
    rOpenSearchHaltDiscoverWitness
      (emitRFnCfgOpenSearchHaltDiscoverProgram
        (lowerFnCfgOpenSearchHaltDiscoverProgram stableFnCfgOpenSearchHaltDiscoverProgram)) := by
  exact lowerEmitFnCfgOpenSearchHaltDiscoverProgram_preserves_witness _
    stableFnCfgOpenSearchHaltDiscoverProgram_src_witness

end RRProofs
