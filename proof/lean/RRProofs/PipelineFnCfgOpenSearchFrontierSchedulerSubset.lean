import RRProofs.PipelineFnCfgOpenSearchDynamicFrontierSubset

namespace RRProofs

structure SrcFnCfgOpenSearchFrontierSchedulerProgram where
  dynProg : SrcFnCfgOpenSearchDynamicFrontierProgram
  futureRounds : List (List PriorityTrace)
  scheduledRounds : List (List PriorityTrace)

structure MirFnCfgOpenSearchFrontierSchedulerProgram where
  dynProg : MirFnCfgOpenSearchDynamicFrontierProgram
  futureRounds : List (List PriorityTrace)
  scheduledRounds : List (List PriorityTrace)

structure RFnCfgOpenSearchFrontierSchedulerProgram where
  dynProg : RFnCfgOpenSearchDynamicFrontierProgram
  futureRounds : List (List PriorityTrace)
  scheduledRounds : List (List PriorityTrace)

def lowerFnCfgOpenSearchFrontierSchedulerProgram
    (p : SrcFnCfgOpenSearchFrontierSchedulerProgram) : MirFnCfgOpenSearchFrontierSchedulerProgram :=
  { dynProg := lowerFnCfgOpenSearchDynamicFrontierProgram p.dynProg
  , futureRounds := p.futureRounds
  , scheduledRounds := p.scheduledRounds
  }

def emitRFnCfgOpenSearchFrontierSchedulerProgram
    (p : MirFnCfgOpenSearchFrontierSchedulerProgram) : RFnCfgOpenSearchFrontierSchedulerProgram :=
  { dynProg := emitRFnCfgOpenSearchDynamicFrontierProgram p.dynProg
  , futureRounds := p.futureRounds
  , scheduledRounds := p.scheduledRounds
  }

def evalSrcFnCfgOpenSearchFrontierSchedulerProgram (p : SrcFnCfgOpenSearchFrontierSchedulerProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchDynamicFrontierProgram p.dynProg

def evalMirFnCfgOpenSearchFrontierSchedulerProgram (p : MirFnCfgOpenSearchFrontierSchedulerProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchDynamicFrontierProgram p.dynProg

def evalRFnCfgOpenSearchFrontierSchedulerProgram (p : RFnCfgOpenSearchFrontierSchedulerProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchDynamicFrontierProgram p.dynProg

def srcOpenSearchFrontierSchedulerWitness (p : SrcFnCfgOpenSearchFrontierSchedulerProgram) : Prop :=
  srcOpenSearchDynamicFrontierWitness p.dynProg ∧
    p.scheduledRounds = p.dynProg.nextFrontier :: p.futureRounds ∧
    p.dynProg.frontierProg.haltProg.selected ∈ p.dynProg.nextFrontier

def mirOpenSearchFrontierSchedulerWitness (p : MirFnCfgOpenSearchFrontierSchedulerProgram) : Prop :=
  mirOpenSearchDynamicFrontierWitness p.dynProg ∧
    p.scheduledRounds = p.dynProg.nextFrontier :: p.futureRounds ∧
    p.dynProg.frontierProg.haltProg.selected ∈ p.dynProg.nextFrontier

def rOpenSearchFrontierSchedulerWitness (p : RFnCfgOpenSearchFrontierSchedulerProgram) : Prop :=
  rOpenSearchDynamicFrontierWitness p.dynProg ∧
    p.scheduledRounds = p.dynProg.nextFrontier :: p.futureRounds ∧
    p.dynProg.frontierProg.haltProg.selected ∈ p.dynProg.nextFrontier

theorem lowerFnCfgOpenSearchFrontierSchedulerProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierSchedulerProgram) :
    (lowerFnCfgOpenSearchFrontierSchedulerProgram p).dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierSchedulerProgram p).futureRounds = p.futureRounds ∧
      (lowerFnCfgOpenSearchFrontierSchedulerProgram p).scheduledRounds = p.scheduledRounds := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierSchedulerProgram] using
      (lowerFnCfgOpenSearchDynamicFrontierProgram_preserves_meta p.dynProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierSchedulerProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierSchedulerProgram) :
    (emitRFnCfgOpenSearchFrontierSchedulerProgram p).dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierSchedulerProgram p).futureRounds = p.futureRounds ∧
      (emitRFnCfgOpenSearchFrontierSchedulerProgram p).scheduledRounds = p.scheduledRounds := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierSchedulerProgram] using
      (emitRFnCfgOpenSearchDynamicFrontierProgram_preserves_meta p.dynProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierSchedulerProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierSchedulerProgram) :
    evalMirFnCfgOpenSearchFrontierSchedulerProgram (lowerFnCfgOpenSearchFrontierSchedulerProgram p) =
      evalSrcFnCfgOpenSearchFrontierSchedulerProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierSchedulerProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierSchedulerProgram) :
    evalRFnCfgOpenSearchFrontierSchedulerProgram (emitRFnCfgOpenSearchFrontierSchedulerProgram p) =
      evalMirFnCfgOpenSearchFrontierSchedulerProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierSchedulerProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierSchedulerProgram) :
    evalRFnCfgOpenSearchFrontierSchedulerProgram
        (emitRFnCfgOpenSearchFrontierSchedulerProgram (lowerFnCfgOpenSearchFrontierSchedulerProgram p)) =
      evalSrcFnCfgOpenSearchFrontierSchedulerProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierSchedulerProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierSchedulerProgram) :
    srcOpenSearchFrontierSchedulerWitness p →
      mirOpenSearchFrontierSchedulerWitness (lowerFnCfgOpenSearchFrontierSchedulerProgram p) := by
  intro h
  rcases h with ⟨hDyn, hSched, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchDynamicFrontierProgram_preserves_witness _ hDyn
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierSchedulerProgram] using hSched
  · simpa [lowerFnCfgOpenSearchFrontierSchedulerProgram] using hMem

theorem emitRFnCfgOpenSearchFrontierSchedulerProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierSchedulerProgram) :
    mirOpenSearchFrontierSchedulerWitness p →
      rOpenSearchFrontierSchedulerWitness (emitRFnCfgOpenSearchFrontierSchedulerProgram p) := by
  intro h
  rcases h with ⟨hDyn, hSched, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchDynamicFrontierProgram_preserves_witness _ hDyn
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierSchedulerProgram] using hSched
  · simpa [emitRFnCfgOpenSearchFrontierSchedulerProgram] using hMem

theorem lowerEmitFnCfgOpenSearchFrontierSchedulerProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierSchedulerProgram) :
    srcOpenSearchFrontierSchedulerWitness p →
      rOpenSearchFrontierSchedulerWitness
        (emitRFnCfgOpenSearchFrontierSchedulerProgram (lowerFnCfgOpenSearchFrontierSchedulerProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierSchedulerProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierSchedulerProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierSchedulerProgram : SrcFnCfgOpenSearchFrontierSchedulerProgram :=
  { dynProg := stableFnCfgOpenSearchDynamicFrontierProgram
  , futureRounds := [[stableClosedLoopSummary]]
  , scheduledRounds := [[stableClosedLoopSummary, []], [stableClosedLoopSummary]]
  }

theorem stableFnCfgOpenSearchFrontierSchedulerProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierSchedulerProgram stableFnCfgOpenSearchFrontierSchedulerProgram).dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierSchedulerProgram stableFnCfgOpenSearchFrontierSchedulerProgram).futureRounds =
        [[stableClosedLoopSummary]] ∧
      (lowerFnCfgOpenSearchFrontierSchedulerProgram stableFnCfgOpenSearchFrontierSchedulerProgram).scheduledRounds =
        [[stableClosedLoopSummary, []], [stableClosedLoopSummary]] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierSchedulerProgram_src_witness :
    srcOpenSearchFrontierSchedulerWitness stableFnCfgOpenSearchFrontierSchedulerProgram := by
  constructor
  · exact stableFnCfgOpenSearchDynamicFrontierProgram_src_witness
  constructor
  · rfl
  · exact stableFnCfgOpenSearchDynamicFrontierProgram_src_witness.2.2

theorem stableFnCfgOpenSearchFrontierSchedulerProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierSchedulerProgram
      (emitRFnCfgOpenSearchFrontierSchedulerProgram
        (lowerFnCfgOpenSearchFrontierSchedulerProgram stableFnCfgOpenSearchFrontierSchedulerProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierSchedulerProgram_preserved :
    rOpenSearchFrontierSchedulerWitness
      (emitRFnCfgOpenSearchFrontierSchedulerProgram
        (lowerFnCfgOpenSearchFrontierSchedulerProgram stableFnCfgOpenSearchFrontierSchedulerProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierSchedulerProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierSchedulerProgram_src_witness

end RRProofs
