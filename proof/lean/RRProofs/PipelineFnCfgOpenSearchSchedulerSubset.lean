import RRProofs.PipelineFnCfgDynamicOpenSearchSubset

namespace RRProofs

structure SrcFnCfgOpenSearchSchedulerProgram where
  dynProg : SrcFnCfgDynamicOpenSearchProgram
  futureRounds : List (List PriorityTrace)
  scheduledRounds : List (List PriorityTrace)

structure MirFnCfgOpenSearchSchedulerProgram where
  dynProg : MirFnCfgDynamicOpenSearchProgram
  futureRounds : List (List PriorityTrace)
  scheduledRounds : List (List PriorityTrace)

structure RFnCfgOpenSearchSchedulerProgram where
  dynProg : RFnCfgDynamicOpenSearchProgram
  futureRounds : List (List PriorityTrace)
  scheduledRounds : List (List PriorityTrace)

def lowerFnCfgOpenSearchSchedulerProgram
    (p : SrcFnCfgOpenSearchSchedulerProgram) : MirFnCfgOpenSearchSchedulerProgram :=
  { dynProg := lowerFnCfgDynamicOpenSearchProgram p.dynProg
  , futureRounds := p.futureRounds
  , scheduledRounds := p.scheduledRounds
  }

def emitRFnCfgOpenSearchSchedulerProgram
    (p : MirFnCfgOpenSearchSchedulerProgram) : RFnCfgOpenSearchSchedulerProgram :=
  { dynProg := emitRFnCfgDynamicOpenSearchProgram p.dynProg
  , futureRounds := p.futureRounds
  , scheduledRounds := p.scheduledRounds
  }

def evalSrcFnCfgOpenSearchSchedulerProgram (p : SrcFnCfgOpenSearchSchedulerProgram) : PriorityTrace :=
  evalSrcFnCfgDynamicOpenSearchProgram p.dynProg

def evalMirFnCfgOpenSearchSchedulerProgram (p : MirFnCfgOpenSearchSchedulerProgram) : PriorityTrace :=
  evalMirFnCfgDynamicOpenSearchProgram p.dynProg

def evalRFnCfgOpenSearchSchedulerProgram (p : RFnCfgOpenSearchSchedulerProgram) : PriorityTrace :=
  evalRFnCfgDynamicOpenSearchProgram p.dynProg

def srcOpenSearchSchedulerWitness (p : SrcFnCfgOpenSearchSchedulerProgram) : Prop :=
  srcDynamicOpenSearchWitness p.dynProg ∧
    p.scheduledRounds = p.dynProg.nextFrontier :: p.futureRounds ∧
    p.dynProg.openProg.haltProg.selected ∈ p.dynProg.nextFrontier

def mirOpenSearchSchedulerWitness (p : MirFnCfgOpenSearchSchedulerProgram) : Prop :=
  mirDynamicOpenSearchWitness p.dynProg ∧
    p.scheduledRounds = p.dynProg.nextFrontier :: p.futureRounds ∧
    p.dynProg.openProg.haltProg.selected ∈ p.dynProg.nextFrontier

def rOpenSearchSchedulerWitness (p : RFnCfgOpenSearchSchedulerProgram) : Prop :=
  rDynamicOpenSearchWitness p.dynProg ∧
    p.scheduledRounds = p.dynProg.nextFrontier :: p.futureRounds ∧
    p.dynProg.openProg.haltProg.selected ∈ p.dynProg.nextFrontier

theorem lowerFnCfgOpenSearchSchedulerProgram_preserves_meta
    (p : SrcFnCfgOpenSearchSchedulerProgram) :
    (lowerFnCfgOpenSearchSchedulerProgram p).dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchSchedulerProgram p).futureRounds = p.futureRounds ∧
      (lowerFnCfgOpenSearchSchedulerProgram p).scheduledRounds = p.scheduledRounds := by
  constructor
  · simpa [lowerFnCfgOpenSearchSchedulerProgram] using
      (lowerFnCfgDynamicOpenSearchProgram_preserves_meta p.dynProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchSchedulerProgram_preserves_meta
    (p : MirFnCfgOpenSearchSchedulerProgram) :
    (emitRFnCfgOpenSearchSchedulerProgram p).dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchSchedulerProgram p).futureRounds = p.futureRounds ∧
      (emitRFnCfgOpenSearchSchedulerProgram p).scheduledRounds = p.scheduledRounds := by
  constructor
  · simpa [emitRFnCfgOpenSearchSchedulerProgram] using
      (emitRFnCfgDynamicOpenSearchProgram_preserves_meta p.dynProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchSchedulerProgram_preserves_eval
    (p : SrcFnCfgOpenSearchSchedulerProgram) :
    evalMirFnCfgOpenSearchSchedulerProgram (lowerFnCfgOpenSearchSchedulerProgram p) =
      evalSrcFnCfgOpenSearchSchedulerProgram p := by
  rfl

theorem emitRFnCfgOpenSearchSchedulerProgram_preserves_eval
    (p : MirFnCfgOpenSearchSchedulerProgram) :
    evalRFnCfgOpenSearchSchedulerProgram (emitRFnCfgOpenSearchSchedulerProgram p) =
      evalMirFnCfgOpenSearchSchedulerProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchSchedulerProgram_preserves_eval
    (p : SrcFnCfgOpenSearchSchedulerProgram) :
    evalRFnCfgOpenSearchSchedulerProgram
        (emitRFnCfgOpenSearchSchedulerProgram (lowerFnCfgOpenSearchSchedulerProgram p)) =
      evalSrcFnCfgOpenSearchSchedulerProgram p := by
  rfl

theorem lowerFnCfgOpenSearchSchedulerProgram_preserves_witness
    (p : SrcFnCfgOpenSearchSchedulerProgram) :
    srcOpenSearchSchedulerWitness p →
      mirOpenSearchSchedulerWitness (lowerFnCfgOpenSearchSchedulerProgram p) := by
  intro h
  rcases h with ⟨hDyn, hSched, hMem⟩
  constructor
  · exact lowerFnCfgDynamicOpenSearchProgram_preserves_witness _ hDyn
  constructor
  · simpa [lowerFnCfgOpenSearchSchedulerProgram] using hSched
  · simpa [lowerFnCfgOpenSearchSchedulerProgram] using hMem

theorem emitRFnCfgOpenSearchSchedulerProgram_preserves_witness
    (p : MirFnCfgOpenSearchSchedulerProgram) :
    mirOpenSearchSchedulerWitness p →
      rOpenSearchSchedulerWitness (emitRFnCfgOpenSearchSchedulerProgram p) := by
  intro h
  rcases h with ⟨hDyn, hSched, hMem⟩
  constructor
  · exact emitRFnCfgDynamicOpenSearchProgram_preserves_witness _ hDyn
  constructor
  · simpa [emitRFnCfgOpenSearchSchedulerProgram] using hSched
  · simpa [emitRFnCfgOpenSearchSchedulerProgram] using hMem

theorem lowerEmitFnCfgOpenSearchSchedulerProgram_preserves_witness
    (p : SrcFnCfgOpenSearchSchedulerProgram) :
    srcOpenSearchSchedulerWitness p →
      rOpenSearchSchedulerWitness
        (emitRFnCfgOpenSearchSchedulerProgram (lowerFnCfgOpenSearchSchedulerProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchSchedulerProgram_preserves_witness _
    (lowerFnCfgOpenSearchSchedulerProgram_preserves_witness _ h)

def stableFnCfgOpenSearchSchedulerProgram : SrcFnCfgOpenSearchSchedulerProgram :=
  { dynProg := stableFnCfgDynamicOpenSearchProgram
  , futureRounds := [[stableClosedLoopSummary]]
  , scheduledRounds := [[stableClosedLoopSummary, []], [stableClosedLoopSummary]]
  }

theorem stableFnCfgOpenSearchSchedulerProgram_meta_preserved :
    (lowerFnCfgOpenSearchSchedulerProgram stableFnCfgOpenSearchSchedulerProgram).dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchSchedulerProgram stableFnCfgOpenSearchSchedulerProgram).futureRounds =
        [[stableClosedLoopSummary]] ∧
      (lowerFnCfgOpenSearchSchedulerProgram stableFnCfgOpenSearchSchedulerProgram).scheduledRounds =
        [[stableClosedLoopSummary, []], [stableClosedLoopSummary]] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchSchedulerProgram_src_witness :
    srcOpenSearchSchedulerWitness stableFnCfgOpenSearchSchedulerProgram := by
  constructor
  · exact stableFnCfgDynamicOpenSearchProgram_src_witness
  constructor
  · rfl
  · exact stableFnCfgDynamicOpenSearchProgram_src_witness.2.2

theorem stableFnCfgOpenSearchSchedulerProgram_eval_preserved :
    evalRFnCfgOpenSearchSchedulerProgram
      (emitRFnCfgOpenSearchSchedulerProgram
        (lowerFnCfgOpenSearchSchedulerProgram stableFnCfgOpenSearchSchedulerProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchSchedulerProgram_preserved :
    rOpenSearchSchedulerWitness
      (emitRFnCfgOpenSearchSchedulerProgram
        (lowerFnCfgOpenSearchSchedulerProgram stableFnCfgOpenSearchSchedulerProgram)) := by
  exact lowerEmitFnCfgOpenSearchSchedulerProgram_preserves_witness _
    stableFnCfgOpenSearchSchedulerProgram_src_witness

end RRProofs
