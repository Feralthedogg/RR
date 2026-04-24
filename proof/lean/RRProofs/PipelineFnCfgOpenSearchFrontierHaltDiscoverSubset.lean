import RRProofs.PipelineFnCfgOpenSearchFrontierConvergenceProtocolSubset

namespace RRProofs

structure SrcFnCfgOpenSearchFrontierHaltDiscoverProgram where
  protocolProg : SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram
  searchSpace : List PriorityTrace
  selected : PriorityTrace

structure MirFnCfgOpenSearchFrontierHaltDiscoverProgram where
  protocolProg : MirFnCfgOpenSearchFrontierConvergenceProtocolProgram
  searchSpace : List PriorityTrace
  selected : PriorityTrace

structure RFnCfgOpenSearchFrontierHaltDiscoverProgram where
  protocolProg : RFnCfgOpenSearchFrontierConvergenceProtocolProgram
  searchSpace : List PriorityTrace
  selected : PriorityTrace

def lowerFnCfgOpenSearchFrontierHaltDiscoverProgram
    (p : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram) : MirFnCfgOpenSearchFrontierHaltDiscoverProgram :=
  { protocolProg := lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram p.protocolProg
  , searchSpace := p.searchSpace
  , selected := p.selected
  }

def emitRFnCfgOpenSearchFrontierHaltDiscoverProgram
    (p : MirFnCfgOpenSearchFrontierHaltDiscoverProgram) : RFnCfgOpenSearchFrontierHaltDiscoverProgram :=
  { protocolProg := emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram p.protocolProg
  , searchSpace := p.searchSpace
  , selected := p.selected
  }

def evalSrcFnCfgOpenSearchFrontierHaltDiscoverProgram (p : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram) : PriorityTrace :=
  p.protocolProg.haltSummary

def evalMirFnCfgOpenSearchFrontierHaltDiscoverProgram (p : MirFnCfgOpenSearchFrontierHaltDiscoverProgram) : PriorityTrace :=
  p.protocolProg.haltSummary

def evalRFnCfgOpenSearchFrontierHaltDiscoverProgram (p : RFnCfgOpenSearchFrontierHaltDiscoverProgram) : PriorityTrace :=
  p.protocolProg.haltSummary

def srcOpenSearchFrontierHaltDiscoverWitness (p : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram) : Prop :=
  srcOpenSearchFrontierConvergenceProtocolWitness p.protocolProg ∧
    p.selected ∈ p.searchSpace ∧
    p.selected = p.protocolProg.haltSummary

def mirOpenSearchFrontierHaltDiscoverWitness (p : MirFnCfgOpenSearchFrontierHaltDiscoverProgram) : Prop :=
  mirOpenSearchFrontierConvergenceProtocolWitness p.protocolProg ∧
    p.selected ∈ p.searchSpace ∧
    p.selected = p.protocolProg.haltSummary

def rOpenSearchFrontierHaltDiscoverWitness (p : RFnCfgOpenSearchFrontierHaltDiscoverProgram) : Prop :=
  rOpenSearchFrontierConvergenceProtocolWitness p.protocolProg ∧
    p.selected ∈ p.searchSpace ∧
    p.selected = p.protocolProg.haltSummary

theorem lowerFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram) :
    (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram p).protocolProg.summaryProg.rounds.length =
        p.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram p).searchSpace = p.searchSpace ∧
      (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram p).selected = p.selected := by
  constructor
  · simp [lowerFnCfgOpenSearchFrontierHaltDiscoverProgram,
      lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_meta]
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierHaltDiscoverProgram) :
    (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram p).protocolProg.summaryProg.rounds.length =
        p.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram p).searchSpace = p.searchSpace ∧
      (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram p).selected = p.selected := by
  constructor
  · simp [emitRFnCfgOpenSearchFrontierHaltDiscoverProgram,
      emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_meta]
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram) :
    evalMirFnCfgOpenSearchFrontierHaltDiscoverProgram (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram p) =
      evalSrcFnCfgOpenSearchFrontierHaltDiscoverProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierHaltDiscoverProgram) :
    evalRFnCfgOpenSearchFrontierHaltDiscoverProgram (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram p) =
      evalMirFnCfgOpenSearchFrontierHaltDiscoverProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram) :
    evalRFnCfgOpenSearchFrontierHaltDiscoverProgram
        (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram p)) =
      evalSrcFnCfgOpenSearchFrontierHaltDiscoverProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram) :
    srcOpenSearchFrontierHaltDiscoverWitness p →
      mirOpenSearchFrontierHaltDiscoverWitness (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram p) := by
  intro h
  rcases h with ⟨hConv, hMem, hSel⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_witness _ hConv
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierHaltDiscoverProgram] using hMem
  · simpa [lowerFnCfgOpenSearchFrontierHaltDiscoverProgram] using hSel

theorem emitRFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierHaltDiscoverProgram) :
    mirOpenSearchFrontierHaltDiscoverWitness p →
      rOpenSearchFrontierHaltDiscoverWitness (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram p) := by
  intro h
  rcases h with ⟨hConv, hMem, hSel⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_witness _ hConv
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierHaltDiscoverProgram] using hMem
  · simpa [emitRFnCfgOpenSearchFrontierHaltDiscoverProgram] using hSel

theorem lowerEmitFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram) :
    srcOpenSearchFrontierHaltDiscoverWitness p →
      rOpenSearchFrontierHaltDiscoverWitness
        (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierHaltDiscoverProgram : SrcFnCfgOpenSearchFrontierHaltDiscoverProgram :=
  { protocolProg := stableFnCfgOpenSearchFrontierConvergenceProtocolProgram
  , searchSpace := [[], stableClosedLoopSummary]
  , selected := stableClosedLoopSummary
  }

theorem stableFnCfgOpenSearchFrontierHaltDiscoverProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram stableFnCfgOpenSearchFrontierHaltDiscoverProgram).protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram stableFnCfgOpenSearchFrontierHaltDiscoverProgram).searchSpace =
        [[], stableClosedLoopSummary] ∧
      (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram stableFnCfgOpenSearchFrontierHaltDiscoverProgram).selected =
        stableClosedLoopSummary := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierHaltDiscoverProgram_src_witness :
    srcOpenSearchFrontierHaltDiscoverWitness stableFnCfgOpenSearchFrontierHaltDiscoverProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierConvergenceProtocolProgram_src_witness
  constructor
  · simp [stableFnCfgOpenSearchFrontierHaltDiscoverProgram]
  · rfl

theorem stableFnCfgOpenSearchFrontierHaltDiscoverProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierHaltDiscoverProgram
      (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram
        (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram stableFnCfgOpenSearchFrontierHaltDiscoverProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierHaltDiscoverProgram_preserved :
    rOpenSearchFrontierHaltDiscoverWitness
      (emitRFnCfgOpenSearchFrontierHaltDiscoverProgram
        (lowerFnCfgOpenSearchFrontierHaltDiscoverProgram stableFnCfgOpenSearchFrontierHaltDiscoverProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierHaltDiscoverProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierHaltDiscoverProgram_src_witness

end RRProofs
