import RRProofs.PipelineFnCfgOpenSearchFrontierReopenSchedulerSubset

namespace RRProofs

abbrev OpenSearchFrontierReopenPriorityRound := Nat × List PriorityTrace

structure SrcFnCfgOpenSearchFrontierReopenPriorityProgram where
  schedProg : SrcFnCfgOpenSearchFrontierReopenSchedulerProgram
  policyTail : List OpenSearchFrontierReopenPriorityRound
  prioritizedRounds : List OpenSearchFrontierReopenPriorityRound

structure MirFnCfgOpenSearchFrontierReopenPriorityProgram where
  schedProg : MirFnCfgOpenSearchFrontierReopenSchedulerProgram
  policyTail : List OpenSearchFrontierReopenPriorityRound
  prioritizedRounds : List OpenSearchFrontierReopenPriorityRound

structure RFnCfgOpenSearchFrontierReopenPriorityProgram where
  schedProg : RFnCfgOpenSearchFrontierReopenSchedulerProgram
  policyTail : List OpenSearchFrontierReopenPriorityRound
  prioritizedRounds : List OpenSearchFrontierReopenPriorityRound

def lowerFnCfgOpenSearchFrontierReopenPriorityProgram
    (p : SrcFnCfgOpenSearchFrontierReopenPriorityProgram) :
    MirFnCfgOpenSearchFrontierReopenPriorityProgram :=
  { schedProg := lowerFnCfgOpenSearchFrontierReopenSchedulerProgram p.schedProg
  , policyTail := p.policyTail
  , prioritizedRounds := p.prioritizedRounds
  }

def emitRFnCfgOpenSearchFrontierReopenPriorityProgram
    (p : MirFnCfgOpenSearchFrontierReopenPriorityProgram) :
    RFnCfgOpenSearchFrontierReopenPriorityProgram :=
  { schedProg := emitRFnCfgOpenSearchFrontierReopenSchedulerProgram p.schedProg
  , policyTail := p.policyTail
  , prioritizedRounds := p.prioritizedRounds
  }

def evalSrcFnCfgOpenSearchFrontierReopenPriorityProgram
    (p : SrcFnCfgOpenSearchFrontierReopenPriorityProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchFrontierReopenSchedulerProgram p.schedProg

def evalMirFnCfgOpenSearchFrontierReopenPriorityProgram
    (p : MirFnCfgOpenSearchFrontierReopenPriorityProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchFrontierReopenSchedulerProgram p.schedProg

def evalRFnCfgOpenSearchFrontierReopenPriorityProgram
    (p : RFnCfgOpenSearchFrontierReopenPriorityProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchFrontierReopenSchedulerProgram p.schedProg

def srcOpenSearchFrontierReopenPriorityWitness
    (p : SrcFnCfgOpenSearchFrontierReopenPriorityProgram) : Prop :=
  srcOpenSearchFrontierReopenSchedulerWitness p.schedProg ∧
    p.prioritizedRounds = (5, p.schedProg.dynProg.nextFrontier) :: p.policyTail ∧
    p.schedProg.dynProg.reopenProg.haltProg.selected ∈ p.schedProg.dynProg.nextFrontier

def mirOpenSearchFrontierReopenPriorityWitness
    (p : MirFnCfgOpenSearchFrontierReopenPriorityProgram) : Prop :=
  mirOpenSearchFrontierReopenSchedulerWitness p.schedProg ∧
    p.prioritizedRounds = (5, p.schedProg.dynProg.nextFrontier) :: p.policyTail ∧
    p.schedProg.dynProg.reopenProg.haltProg.selected ∈ p.schedProg.dynProg.nextFrontier

def rOpenSearchFrontierReopenPriorityWitness
    (p : RFnCfgOpenSearchFrontierReopenPriorityProgram) : Prop :=
  rOpenSearchFrontierReopenSchedulerWitness p.schedProg ∧
    p.prioritizedRounds = (5, p.schedProg.dynProg.nextFrontier) :: p.policyTail ∧
    p.schedProg.dynProg.reopenProg.haltProg.selected ∈ p.schedProg.dynProg.nextFrontier

theorem lowerFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierReopenPriorityProgram) :
    (lowerFnCfgOpenSearchFrontierReopenPriorityProgram p).schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierReopenPriorityProgram p).policyTail = p.policyTail ∧
      (lowerFnCfgOpenSearchFrontierReopenPriorityProgram p).prioritizedRounds = p.prioritizedRounds := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierReopenPriorityProgram] using
      (lowerFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_meta p.schedProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierReopenPriorityProgram) :
    (emitRFnCfgOpenSearchFrontierReopenPriorityProgram p).schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierReopenPriorityProgram p).policyTail = p.policyTail ∧
      (emitRFnCfgOpenSearchFrontierReopenPriorityProgram p).prioritizedRounds = p.prioritizedRounds := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierReopenPriorityProgram] using
      (emitRFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_meta p.schedProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenPriorityProgram) :
    evalMirFnCfgOpenSearchFrontierReopenPriorityProgram
        (lowerFnCfgOpenSearchFrontierReopenPriorityProgram p) =
      evalSrcFnCfgOpenSearchFrontierReopenPriorityProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierReopenPriorityProgram) :
    evalRFnCfgOpenSearchFrontierReopenPriorityProgram
        (emitRFnCfgOpenSearchFrontierReopenPriorityProgram p) =
      evalMirFnCfgOpenSearchFrontierReopenPriorityProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenPriorityProgram) :
    evalRFnCfgOpenSearchFrontierReopenPriorityProgram
        (emitRFnCfgOpenSearchFrontierReopenPriorityProgram
          (lowerFnCfgOpenSearchFrontierReopenPriorityProgram p)) =
      evalSrcFnCfgOpenSearchFrontierReopenPriorityProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenPriorityProgram) :
    srcOpenSearchFrontierReopenPriorityWitness p →
      mirOpenSearchFrontierReopenPriorityWitness
        (lowerFnCfgOpenSearchFrontierReopenPriorityProgram p) := by
  intro h
  rcases h with ⟨hSched, hPrio, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_witness _ hSched
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierReopenPriorityProgram] using hPrio
  · simpa [lowerFnCfgOpenSearchFrontierReopenPriorityProgram] using hMem

theorem emitRFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierReopenPriorityProgram) :
    mirOpenSearchFrontierReopenPriorityWitness p →
      rOpenSearchFrontierReopenPriorityWitness
        (emitRFnCfgOpenSearchFrontierReopenPriorityProgram p) := by
  intro h
  rcases h with ⟨hSched, hPrio, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierReopenSchedulerProgram_preserves_witness _ hSched
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierReopenPriorityProgram] using hPrio
  · simpa [emitRFnCfgOpenSearchFrontierReopenPriorityProgram] using hMem

theorem lowerEmitFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenPriorityProgram) :
    srcOpenSearchFrontierReopenPriorityWitness p →
      rOpenSearchFrontierReopenPriorityWitness
        (emitRFnCfgOpenSearchFrontierReopenPriorityProgram
          (lowerFnCfgOpenSearchFrontierReopenPriorityProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierReopenPriorityProgram :
    SrcFnCfgOpenSearchFrontierReopenPriorityProgram :=
  { schedProg := stableFnCfgOpenSearchFrontierReopenSchedulerProgram
  , policyTail := [(3, [stableClosedLoopSummary])]
  , prioritizedRounds := [(5, [stableClosedLoopSummary, []]), (3, [stableClosedLoopSummary])]
  }

theorem stableFnCfgOpenSearchFrontierReopenPriorityProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierReopenPriorityProgram
      stableFnCfgOpenSearchFrontierReopenPriorityProgram).schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierReopenPriorityProgram
        stableFnCfgOpenSearchFrontierReopenPriorityProgram).policyTail =
        [(3, [stableClosedLoopSummary])] ∧
      (lowerFnCfgOpenSearchFrontierReopenPriorityProgram
        stableFnCfgOpenSearchFrontierReopenPriorityProgram).prioritizedRounds =
        [(5, [stableClosedLoopSummary, []]), (3, [stableClosedLoopSummary])] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierReopenPriorityProgram_src_witness :
    srcOpenSearchFrontierReopenPriorityWitness
      stableFnCfgOpenSearchFrontierReopenPriorityProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierReopenSchedulerProgram_src_witness
  constructor
  · rfl
  · exact stableFnCfgOpenSearchFrontierReopenSchedulerProgram_src_witness.2.2

theorem stableFnCfgOpenSearchFrontierReopenPriorityProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierReopenPriorityProgram
      (emitRFnCfgOpenSearchFrontierReopenPriorityProgram
        (lowerFnCfgOpenSearchFrontierReopenPriorityProgram
          stableFnCfgOpenSearchFrontierReopenPriorityProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierReopenPriorityProgram_preserved :
    rOpenSearchFrontierReopenPriorityWitness
      (emitRFnCfgOpenSearchFrontierReopenPriorityProgram
        (lowerFnCfgOpenSearchFrontierReopenPriorityProgram
          stableFnCfgOpenSearchFrontierReopenPriorityProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierReopenPriorityProgram_src_witness

end RRProofs
