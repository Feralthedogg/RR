import RRProofs.PipelineFnCfgConvergenceProtocolSubset

namespace RRProofs

structure SrcFnCfgHaltDiscoverProgram where
  protocolProg : SrcFnCfgConvergenceProtocolProgram
  searchSpace : List PriorityTrace
  selected : PriorityTrace

structure MirFnCfgHaltDiscoverProgram where
  protocolProg : MirFnCfgConvergenceProtocolProgram
  searchSpace : List PriorityTrace
  selected : PriorityTrace

structure RFnCfgHaltDiscoverProgram where
  protocolProg : RFnCfgConvergenceProtocolProgram
  searchSpace : List PriorityTrace
  selected : PriorityTrace

def lowerFnCfgHaltDiscoverProgram (p : SrcFnCfgHaltDiscoverProgram) : MirFnCfgHaltDiscoverProgram :=
  { protocolProg := lowerFnCfgConvergenceProtocolProgram p.protocolProg
  , searchSpace := p.searchSpace
  , selected := p.selected
  }

def emitRFnCfgHaltDiscoverProgram (p : MirFnCfgHaltDiscoverProgram) : RFnCfgHaltDiscoverProgram :=
  { protocolProg := emitRFnCfgConvergenceProtocolProgram p.protocolProg
  , searchSpace := p.searchSpace
  , selected := p.selected
  }

def evalSrcFnCfgHaltDiscoverProgram (p : SrcFnCfgHaltDiscoverProgram) : PriorityTrace :=
  p.protocolProg.haltSummary

def evalMirFnCfgHaltDiscoverProgram (p : MirFnCfgHaltDiscoverProgram) : PriorityTrace :=
  p.protocolProg.haltSummary

def evalRFnCfgHaltDiscoverProgram (p : RFnCfgHaltDiscoverProgram) : PriorityTrace :=
  p.protocolProg.haltSummary

def srcHaltDiscoverWitness (p : SrcFnCfgHaltDiscoverProgram) : Prop :=
  srcConvergenceProtocolWitness p.protocolProg ∧
    p.selected ∈ p.searchSpace ∧
    p.selected = p.protocolProg.haltSummary

def mirHaltDiscoverWitness (p : MirFnCfgHaltDiscoverProgram) : Prop :=
  mirConvergenceProtocolWitness p.protocolProg ∧
    p.selected ∈ p.searchSpace ∧
    p.selected = p.protocolProg.haltSummary

def rHaltDiscoverWitness (p : RFnCfgHaltDiscoverProgram) : Prop :=
  rConvergenceProtocolWitness p.protocolProg ∧
    p.selected ∈ p.searchSpace ∧
    p.selected = p.protocolProg.haltSummary

theorem lowerFnCfgHaltDiscoverProgram_preserves_meta
    (p : SrcFnCfgHaltDiscoverProgram) :
    (lowerFnCfgHaltDiscoverProgram p).protocolProg.summaryProg.rounds.length = p.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgHaltDiscoverProgram p).searchSpace = p.searchSpace ∧
      (lowerFnCfgHaltDiscoverProgram p).selected = p.selected := by
  constructor
  · simp [lowerFnCfgHaltDiscoverProgram, lowerFnCfgConvergenceProtocolProgram_preserves_meta]
  constructor
  · rfl
  · rfl

theorem emitRFnCfgHaltDiscoverProgram_preserves_meta
    (p : MirFnCfgHaltDiscoverProgram) :
    (emitRFnCfgHaltDiscoverProgram p).protocolProg.summaryProg.rounds.length = p.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgHaltDiscoverProgram p).searchSpace = p.searchSpace ∧
      (emitRFnCfgHaltDiscoverProgram p).selected = p.selected := by
  constructor
  · simp [emitRFnCfgHaltDiscoverProgram, emitRFnCfgConvergenceProtocolProgram_preserves_meta]
  constructor
  · rfl
  · rfl

theorem lowerFnCfgHaltDiscoverProgram_preserves_eval
    (p : SrcFnCfgHaltDiscoverProgram) :
    evalMirFnCfgHaltDiscoverProgram (lowerFnCfgHaltDiscoverProgram p) =
      evalSrcFnCfgHaltDiscoverProgram p := by
  rfl

theorem emitRFnCfgHaltDiscoverProgram_preserves_eval
    (p : MirFnCfgHaltDiscoverProgram) :
    evalRFnCfgHaltDiscoverProgram (emitRFnCfgHaltDiscoverProgram p) =
      evalMirFnCfgHaltDiscoverProgram p := by
  rfl

theorem lowerEmitFnCfgHaltDiscoverProgram_preserves_eval
    (p : SrcFnCfgHaltDiscoverProgram) :
    evalRFnCfgHaltDiscoverProgram (emitRFnCfgHaltDiscoverProgram (lowerFnCfgHaltDiscoverProgram p)) =
      evalSrcFnCfgHaltDiscoverProgram p := by
  rfl

theorem lowerFnCfgHaltDiscoverProgram_preserves_witness
    (p : SrcFnCfgHaltDiscoverProgram) :
    srcHaltDiscoverWitness p →
      mirHaltDiscoverWitness (lowerFnCfgHaltDiscoverProgram p) := by
  intro h
  rcases h with ⟨hConv, hMem, hSel⟩
  constructor
  · exact lowerFnCfgConvergenceProtocolProgram_preserves_witness _ hConv
  constructor
  · simpa [lowerFnCfgHaltDiscoverProgram] using hMem
  · simpa [lowerFnCfgHaltDiscoverProgram] using hSel

theorem emitRFnCfgHaltDiscoverProgram_preserves_witness
    (p : MirFnCfgHaltDiscoverProgram) :
    mirHaltDiscoverWitness p →
      rHaltDiscoverWitness (emitRFnCfgHaltDiscoverProgram p) := by
  intro h
  rcases h with ⟨hConv, hMem, hSel⟩
  constructor
  · exact emitRFnCfgConvergenceProtocolProgram_preserves_witness _ hConv
  constructor
  · simpa [emitRFnCfgHaltDiscoverProgram] using hMem
  · simpa [emitRFnCfgHaltDiscoverProgram] using hSel

theorem lowerEmitFnCfgHaltDiscoverProgram_preserves_witness
    (p : SrcFnCfgHaltDiscoverProgram) :
    srcHaltDiscoverWitness p →
      rHaltDiscoverWitness (emitRFnCfgHaltDiscoverProgram (lowerFnCfgHaltDiscoverProgram p)) := by
  intro h
  exact emitRFnCfgHaltDiscoverProgram_preserves_witness _
    (lowerFnCfgHaltDiscoverProgram_preserves_witness _ h)

def stableFnCfgHaltDiscoverProgram : SrcFnCfgHaltDiscoverProgram :=
  { protocolProg := stableFnCfgConvergenceProtocolProgram
  , searchSpace := [[], stableClosedLoopSummary]
  , selected := stableClosedLoopSummary
  }

theorem stableFnCfgHaltDiscoverProgram_meta_preserved :
    (lowerFnCfgHaltDiscoverProgram stableFnCfgHaltDiscoverProgram).protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgHaltDiscoverProgram stableFnCfgHaltDiscoverProgram).searchSpace = [[], stableClosedLoopSummary] ∧
      (lowerFnCfgHaltDiscoverProgram stableFnCfgHaltDiscoverProgram).selected = stableClosedLoopSummary := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgHaltDiscoverProgram_src_witness :
    srcHaltDiscoverWitness stableFnCfgHaltDiscoverProgram := by
  constructor
  · exact stableFnCfgConvergenceProtocolProgram_src_witness
  constructor
  · simp [stableFnCfgHaltDiscoverProgram]
  · rfl

theorem stableFnCfgHaltDiscoverProgram_eval_preserved :
    evalRFnCfgHaltDiscoverProgram
      (emitRFnCfgHaltDiscoverProgram (lowerFnCfgHaltDiscoverProgram stableFnCfgHaltDiscoverProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgHaltDiscoverProgram_preserved :
    rHaltDiscoverWitness
      (emitRFnCfgHaltDiscoverProgram (lowerFnCfgHaltDiscoverProgram stableFnCfgHaltDiscoverProgram)) := by
  exact lowerEmitFnCfgHaltDiscoverProgram_preserves_witness _
    stableFnCfgHaltDiscoverProgram_src_witness

end RRProofs
