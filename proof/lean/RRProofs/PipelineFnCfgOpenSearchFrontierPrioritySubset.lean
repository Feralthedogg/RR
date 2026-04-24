import RRProofs.PipelineFnCfgOpenSearchFrontierSchedulerSubset

namespace RRProofs

abbrev OpenSearchFrontierPriorityRound := Nat × List PriorityTrace

structure SrcFnCfgOpenSearchFrontierPriorityProgram where
  schedProg : SrcFnCfgOpenSearchFrontierSchedulerProgram
  policyTail : List OpenSearchFrontierPriorityRound
  prioritizedRounds : List OpenSearchFrontierPriorityRound

structure MirFnCfgOpenSearchFrontierPriorityProgram where
  schedProg : MirFnCfgOpenSearchFrontierSchedulerProgram
  policyTail : List OpenSearchFrontierPriorityRound
  prioritizedRounds : List OpenSearchFrontierPriorityRound

structure RFnCfgOpenSearchFrontierPriorityProgram where
  schedProg : RFnCfgOpenSearchFrontierSchedulerProgram
  policyTail : List OpenSearchFrontierPriorityRound
  prioritizedRounds : List OpenSearchFrontierPriorityRound

def lowerFnCfgOpenSearchFrontierPriorityProgram
    (p : SrcFnCfgOpenSearchFrontierPriorityProgram) : MirFnCfgOpenSearchFrontierPriorityProgram :=
  { schedProg := lowerFnCfgOpenSearchFrontierSchedulerProgram p.schedProg
  , policyTail := p.policyTail
  , prioritizedRounds := p.prioritizedRounds
  }

def emitRFnCfgOpenSearchFrontierPriorityProgram
    (p : MirFnCfgOpenSearchFrontierPriorityProgram) : RFnCfgOpenSearchFrontierPriorityProgram :=
  { schedProg := emitRFnCfgOpenSearchFrontierSchedulerProgram p.schedProg
  , policyTail := p.policyTail
  , prioritizedRounds := p.prioritizedRounds
  }

def evalSrcFnCfgOpenSearchFrontierPriorityProgram (p : SrcFnCfgOpenSearchFrontierPriorityProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchFrontierSchedulerProgram p.schedProg

def evalMirFnCfgOpenSearchFrontierPriorityProgram (p : MirFnCfgOpenSearchFrontierPriorityProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchFrontierSchedulerProgram p.schedProg

def evalRFnCfgOpenSearchFrontierPriorityProgram (p : RFnCfgOpenSearchFrontierPriorityProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchFrontierSchedulerProgram p.schedProg

def srcOpenSearchFrontierPriorityWitness (p : SrcFnCfgOpenSearchFrontierPriorityProgram) : Prop :=
  srcOpenSearchFrontierSchedulerWitness p.schedProg ∧
    p.prioritizedRounds = (5, p.schedProg.dynProg.nextFrontier) :: p.policyTail ∧
    p.schedProg.dynProg.frontierProg.haltProg.selected ∈ p.schedProg.dynProg.nextFrontier

def mirOpenSearchFrontierPriorityWitness (p : MirFnCfgOpenSearchFrontierPriorityProgram) : Prop :=
  mirOpenSearchFrontierSchedulerWitness p.schedProg ∧
    p.prioritizedRounds = (5, p.schedProg.dynProg.nextFrontier) :: p.policyTail ∧
    p.schedProg.dynProg.frontierProg.haltProg.selected ∈ p.schedProg.dynProg.nextFrontier

def rOpenSearchFrontierPriorityWitness (p : RFnCfgOpenSearchFrontierPriorityProgram) : Prop :=
  rOpenSearchFrontierSchedulerWitness p.schedProg ∧
    p.prioritizedRounds = (5, p.schedProg.dynProg.nextFrontier) :: p.policyTail ∧
    p.schedProg.dynProg.frontierProg.haltProg.selected ∈ p.schedProg.dynProg.nextFrontier

theorem lowerFnCfgOpenSearchFrontierPriorityProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierPriorityProgram) :
    (lowerFnCfgOpenSearchFrontierPriorityProgram p).schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierPriorityProgram p).policyTail = p.policyTail ∧
      (lowerFnCfgOpenSearchFrontierPriorityProgram p).prioritizedRounds = p.prioritizedRounds := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierPriorityProgram] using
      (lowerFnCfgOpenSearchFrontierSchedulerProgram_preserves_meta p.schedProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierPriorityProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierPriorityProgram) :
    (emitRFnCfgOpenSearchFrontierPriorityProgram p).schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierPriorityProgram p).policyTail = p.policyTail ∧
      (emitRFnCfgOpenSearchFrontierPriorityProgram p).prioritizedRounds = p.prioritizedRounds := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierPriorityProgram] using
      (emitRFnCfgOpenSearchFrontierSchedulerProgram_preserves_meta p.schedProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierPriorityProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierPriorityProgram) :
    evalMirFnCfgOpenSearchFrontierPriorityProgram (lowerFnCfgOpenSearchFrontierPriorityProgram p) =
      evalSrcFnCfgOpenSearchFrontierPriorityProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierPriorityProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierPriorityProgram) :
    evalRFnCfgOpenSearchFrontierPriorityProgram (emitRFnCfgOpenSearchFrontierPriorityProgram p) =
      evalMirFnCfgOpenSearchFrontierPriorityProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierPriorityProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierPriorityProgram) :
    evalRFnCfgOpenSearchFrontierPriorityProgram
        (emitRFnCfgOpenSearchFrontierPriorityProgram (lowerFnCfgOpenSearchFrontierPriorityProgram p)) =
      evalSrcFnCfgOpenSearchFrontierPriorityProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierPriorityProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierPriorityProgram) :
    srcOpenSearchFrontierPriorityWitness p →
      mirOpenSearchFrontierPriorityWitness (lowerFnCfgOpenSearchFrontierPriorityProgram p) := by
  intro h
  rcases h with ⟨hSched, hPrio, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierSchedulerProgram_preserves_witness _ hSched
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierPriorityProgram] using hPrio
  · simpa [lowerFnCfgOpenSearchFrontierPriorityProgram] using hMem

theorem emitRFnCfgOpenSearchFrontierPriorityProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierPriorityProgram) :
    mirOpenSearchFrontierPriorityWitness p →
      rOpenSearchFrontierPriorityWitness (emitRFnCfgOpenSearchFrontierPriorityProgram p) := by
  intro h
  rcases h with ⟨hSched, hPrio, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierSchedulerProgram_preserves_witness _ hSched
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierPriorityProgram] using hPrio
  · simpa [emitRFnCfgOpenSearchFrontierPriorityProgram] using hMem

theorem lowerEmitFnCfgOpenSearchFrontierPriorityProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierPriorityProgram) :
    srcOpenSearchFrontierPriorityWitness p →
      rOpenSearchFrontierPriorityWitness
        (emitRFnCfgOpenSearchFrontierPriorityProgram (lowerFnCfgOpenSearchFrontierPriorityProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierPriorityProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierPriorityProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierPriorityProgram : SrcFnCfgOpenSearchFrontierPriorityProgram :=
  { schedProg := stableFnCfgOpenSearchFrontierSchedulerProgram
  , policyTail := [(3, [stableClosedLoopSummary])]
  , prioritizedRounds := [(5, [stableClosedLoopSummary, []]), (3, [stableClosedLoopSummary])]
  }

theorem stableFnCfgOpenSearchFrontierPriorityProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierPriorityProgram stableFnCfgOpenSearchFrontierPriorityProgram).schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierPriorityProgram stableFnCfgOpenSearchFrontierPriorityProgram).policyTail =
        [(3, [stableClosedLoopSummary])] ∧
      (lowerFnCfgOpenSearchFrontierPriorityProgram stableFnCfgOpenSearchFrontierPriorityProgram).prioritizedRounds =
        [(5, [stableClosedLoopSummary, []]), (3, [stableClosedLoopSummary])] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierPriorityProgram_src_witness :
    srcOpenSearchFrontierPriorityWitness stableFnCfgOpenSearchFrontierPriorityProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierSchedulerProgram_src_witness
  constructor
  · rfl
  · exact stableFnCfgOpenSearchFrontierSchedulerProgram_src_witness.2.2

theorem stableFnCfgOpenSearchFrontierPriorityProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierPriorityProgram
      (emitRFnCfgOpenSearchFrontierPriorityProgram
        (lowerFnCfgOpenSearchFrontierPriorityProgram stableFnCfgOpenSearchFrontierPriorityProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierPriorityProgram_preserved :
    rOpenSearchFrontierPriorityWitness
      (emitRFnCfgOpenSearchFrontierPriorityProgram
        (lowerFnCfgOpenSearchFrontierPriorityProgram stableFnCfgOpenSearchFrontierPriorityProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierPriorityProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierPriorityProgram_src_witness

end RRProofs
