import RRProofs.PipelineFnCfgOpenSearchFrontierReopenDynamicSubset

namespace RRProofs

structure SrcFnCfgOpenSearchFrontierReopenSchedulerProgram where
  dynProg : SrcFnCfgOpenSearchFrontierReopenDynamicProgram
  futureRounds : List (List PriorityTrace)
  scheduledRounds : List (List PriorityTrace)

structure MirFnCfgOpenSearchFrontierReopenSchedulerProgram where
  dynProg : MirFnCfgOpenSearchFrontierReopenDynamicProgram
  futureRounds : List (List PriorityTrace)
  scheduledRounds : List (List PriorityTrace)

structure RFnCfgOpenSearchFrontierReopenSchedulerProgram where
  dynProg : RFnCfgOpenSearchFrontierReopenDynamicProgram
  futureRounds : List (List PriorityTrace)
  scheduledRounds : List (List PriorityTrace)

def lowerFnCfgOpenSearchFrontierReopenSchedulerProgram
    (p : SrcFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    MirFnCfgOpenSearchFrontierReopenSchedulerProgram :=
  { dynProg := lowerFnCfgOpenSearchFrontierReopenDynamicProgram p.dynProg
  , futureRounds := p.futureRounds
  , scheduledRounds := p.scheduledRounds
  }

def emitRFnCfgOpenSearchFrontierReopenSchedulerProgram
    (p : MirFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    RFnCfgOpenSearchFrontierReopenSchedulerProgram :=
  { dynProg := emitRFnCfgOpenSearchFrontierReopenDynamicProgram p.dynProg
  , futureRounds := p.futureRounds
  , scheduledRounds := p.scheduledRounds
  }

def evalSrcFnCfgOpenSearchFrontierReopenSchedulerProgram
    (p : SrcFnCfgOpenSearchFrontierReopenSchedulerProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchFrontierReopenDynamicProgram p.dynProg

def evalMirFnCfgOpenSearchFrontierReopenSchedulerProgram
    (p : MirFnCfgOpenSearchFrontierReopenSchedulerProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchFrontierReopenDynamicProgram p.dynProg

def evalRFnCfgOpenSearchFrontierReopenSchedulerProgram
    (p : RFnCfgOpenSearchFrontierReopenSchedulerProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchFrontierReopenDynamicProgram p.dynProg

def srcOpenSearchFrontierReopenSchedulerWitness
    (p : SrcFnCfgOpenSearchFrontierReopenSchedulerProgram) : Prop :=
  srcOpenSearchFrontierReopenDynamicWitness p.dynProg ∧
    p.scheduledRounds = p.dynProg.nextFrontier :: p.futureRounds ∧
    p.dynProg.reopenProg.haltProg.selected ∈ p.dynProg.nextFrontier

def mirOpenSearchFrontierReopenSchedulerWitness
    (p : MirFnCfgOpenSearchFrontierReopenSchedulerProgram) : Prop :=
  mirOpenSearchFrontierReopenDynamicWitness p.dynProg ∧
    p.scheduledRounds = p.dynProg.nextFrontier :: p.futureRounds ∧
    p.dynProg.reopenProg.haltProg.selected ∈ p.dynProg.nextFrontier

def rOpenSearchFrontierReopenSchedulerWitness
    (p : RFnCfgOpenSearchFrontierReopenSchedulerProgram) : Prop :=
  rOpenSearchFrontierReopenDynamicWitness p.dynProg ∧
    p.scheduledRounds = p.dynProg.nextFrontier :: p.futureRounds ∧
    p.dynProg.reopenProg.haltProg.selected ∈ p.dynProg.nextFrontier

theorem lowerFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram p).dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram p).futureRounds = p.futureRounds ∧
      (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram p).scheduledRounds = p.scheduledRounds := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierReopenSchedulerProgram] using
      (lowerFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_meta p.dynProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram p).dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram p).futureRounds = p.futureRounds ∧
      (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram p).scheduledRounds = p.scheduledRounds := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierReopenSchedulerProgram] using
      (emitRFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_meta p.dynProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    evalMirFnCfgOpenSearchFrontierReopenSchedulerProgram
        (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram p) =
      evalSrcFnCfgOpenSearchFrontierReopenSchedulerProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    evalRFnCfgOpenSearchFrontierReopenSchedulerProgram
        (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram p) =
      evalMirFnCfgOpenSearchFrontierReopenSchedulerProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    evalRFnCfgOpenSearchFrontierReopenSchedulerProgram
        (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram
          (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram p)) =
      evalSrcFnCfgOpenSearchFrontierReopenSchedulerProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    srcOpenSearchFrontierReopenSchedulerWitness p →
      mirOpenSearchFrontierReopenSchedulerWitness
        (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram p) := by
  intro h
  rcases h with ⟨hDyn, hSched, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_witness _ hDyn
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierReopenSchedulerProgram] using hSched
  · simpa [lowerFnCfgOpenSearchFrontierReopenSchedulerProgram] using hMem

theorem emitRFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    mirOpenSearchFrontierReopenSchedulerWitness p →
      rOpenSearchFrontierReopenSchedulerWitness
        (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram p) := by
  intro h
  rcases h with ⟨hDyn, hSched, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierReopenDynamicProgram_preserves_witness _ hDyn
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierReopenSchedulerProgram] using hSched
  · simpa [emitRFnCfgOpenSearchFrontierReopenSchedulerProgram] using hMem

theorem lowerEmitFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenSchedulerProgram) :
    srcOpenSearchFrontierReopenSchedulerWitness p →
      rOpenSearchFrontierReopenSchedulerWitness
        (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram
          (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierReopenSchedulerProgram :
    SrcFnCfgOpenSearchFrontierReopenSchedulerProgram :=
  { dynProg := stableFnCfgOpenSearchFrontierReopenDynamicProgram
  , futureRounds := [[stableClosedLoopSummary]]
  , scheduledRounds := [[stableClosedLoopSummary, []], [stableClosedLoopSummary]]
  }

theorem stableFnCfgOpenSearchFrontierReopenSchedulerProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram
      stableFnCfgOpenSearchFrontierReopenSchedulerProgram).dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram
        stableFnCfgOpenSearchFrontierReopenSchedulerProgram).futureRounds =
        [[stableClosedLoopSummary]] ∧
      (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram
        stableFnCfgOpenSearchFrontierReopenSchedulerProgram).scheduledRounds =
        [[stableClosedLoopSummary, []], [stableClosedLoopSummary]] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierReopenSchedulerProgram_src_witness :
    srcOpenSearchFrontierReopenSchedulerWitness
      stableFnCfgOpenSearchFrontierReopenSchedulerProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierReopenDynamicProgram_src_witness
  constructor
  · rfl
  · exact stableFnCfgOpenSearchFrontierReopenDynamicProgram_src_witness.2.2

theorem stableFnCfgOpenSearchFrontierReopenSchedulerProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierReopenSchedulerProgram
      (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram
        (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram
          stableFnCfgOpenSearchFrontierReopenSchedulerProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierReopenSchedulerProgram_preserved :
    rOpenSearchFrontierReopenSchedulerWitness
      (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram
        (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram
          stableFnCfgOpenSearchFrontierReopenSchedulerProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierReopenSchedulerProgram_src_witness

end RRProofs
