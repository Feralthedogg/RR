import RRProofs.PipelineFnCfgOpenSearchSchedulerSubset

namespace RRProofs

abbrev OpenSearchPriorityRound := Nat × List PriorityTrace

structure SrcFnCfgOpenSearchPriorityProgram where
  schedProg : SrcFnCfgOpenSearchSchedulerProgram
  policyTail : List OpenSearchPriorityRound
  prioritizedRounds : List OpenSearchPriorityRound

structure MirFnCfgOpenSearchPriorityProgram where
  schedProg : MirFnCfgOpenSearchSchedulerProgram
  policyTail : List OpenSearchPriorityRound
  prioritizedRounds : List OpenSearchPriorityRound

structure RFnCfgOpenSearchPriorityProgram where
  schedProg : RFnCfgOpenSearchSchedulerProgram
  policyTail : List OpenSearchPriorityRound
  prioritizedRounds : List OpenSearchPriorityRound

def lowerFnCfgOpenSearchPriorityProgram
    (p : SrcFnCfgOpenSearchPriorityProgram) : MirFnCfgOpenSearchPriorityProgram :=
  { schedProg := lowerFnCfgOpenSearchSchedulerProgram p.schedProg
  , policyTail := p.policyTail
  , prioritizedRounds := p.prioritizedRounds
  }

def emitRFnCfgOpenSearchPriorityProgram
    (p : MirFnCfgOpenSearchPriorityProgram) : RFnCfgOpenSearchPriorityProgram :=
  { schedProg := emitRFnCfgOpenSearchSchedulerProgram p.schedProg
  , policyTail := p.policyTail
  , prioritizedRounds := p.prioritizedRounds
  }

def evalSrcFnCfgOpenSearchPriorityProgram (p : SrcFnCfgOpenSearchPriorityProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchSchedulerProgram p.schedProg

def evalMirFnCfgOpenSearchPriorityProgram (p : MirFnCfgOpenSearchPriorityProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchSchedulerProgram p.schedProg

def evalRFnCfgOpenSearchPriorityProgram (p : RFnCfgOpenSearchPriorityProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchSchedulerProgram p.schedProg

def srcOpenSearchPriorityWitness (p : SrcFnCfgOpenSearchPriorityProgram) : Prop :=
  srcOpenSearchSchedulerWitness p.schedProg ∧
    p.prioritizedRounds = (5, p.schedProg.dynProg.nextFrontier) :: p.policyTail ∧
    p.schedProg.dynProg.openProg.haltProg.selected ∈ p.schedProg.dynProg.nextFrontier

def mirOpenSearchPriorityWitness (p : MirFnCfgOpenSearchPriorityProgram) : Prop :=
  mirOpenSearchSchedulerWitness p.schedProg ∧
    p.prioritizedRounds = (5, p.schedProg.dynProg.nextFrontier) :: p.policyTail ∧
    p.schedProg.dynProg.openProg.haltProg.selected ∈ p.schedProg.dynProg.nextFrontier

def rOpenSearchPriorityWitness (p : RFnCfgOpenSearchPriorityProgram) : Prop :=
  rOpenSearchSchedulerWitness p.schedProg ∧
    p.prioritizedRounds = (5, p.schedProg.dynProg.nextFrontier) :: p.policyTail ∧
    p.schedProg.dynProg.openProg.haltProg.selected ∈ p.schedProg.dynProg.nextFrontier

theorem lowerFnCfgOpenSearchPriorityProgram_preserves_meta
    (p : SrcFnCfgOpenSearchPriorityProgram) :
    (lowerFnCfgOpenSearchPriorityProgram p).schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchPriorityProgram p).policyTail = p.policyTail ∧
      (lowerFnCfgOpenSearchPriorityProgram p).prioritizedRounds = p.prioritizedRounds := by
  constructor
  · simpa [lowerFnCfgOpenSearchPriorityProgram] using
      (lowerFnCfgOpenSearchSchedulerProgram_preserves_meta p.schedProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchPriorityProgram_preserves_meta
    (p : MirFnCfgOpenSearchPriorityProgram) :
    (emitRFnCfgOpenSearchPriorityProgram p).schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchPriorityProgram p).policyTail = p.policyTail ∧
      (emitRFnCfgOpenSearchPriorityProgram p).prioritizedRounds = p.prioritizedRounds := by
  constructor
  · simpa [emitRFnCfgOpenSearchPriorityProgram] using
      (emitRFnCfgOpenSearchSchedulerProgram_preserves_meta p.schedProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchPriorityProgram_preserves_eval
    (p : SrcFnCfgOpenSearchPriorityProgram) :
    evalMirFnCfgOpenSearchPriorityProgram (lowerFnCfgOpenSearchPriorityProgram p) =
      evalSrcFnCfgOpenSearchPriorityProgram p := by
  rfl

theorem emitRFnCfgOpenSearchPriorityProgram_preserves_eval
    (p : MirFnCfgOpenSearchPriorityProgram) :
    evalRFnCfgOpenSearchPriorityProgram (emitRFnCfgOpenSearchPriorityProgram p) =
      evalMirFnCfgOpenSearchPriorityProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchPriorityProgram_preserves_eval
    (p : SrcFnCfgOpenSearchPriorityProgram) :
    evalRFnCfgOpenSearchPriorityProgram
        (emitRFnCfgOpenSearchPriorityProgram (lowerFnCfgOpenSearchPriorityProgram p)) =
      evalSrcFnCfgOpenSearchPriorityProgram p := by
  rfl

theorem lowerFnCfgOpenSearchPriorityProgram_preserves_witness
    (p : SrcFnCfgOpenSearchPriorityProgram) :
    srcOpenSearchPriorityWitness p →
      mirOpenSearchPriorityWitness (lowerFnCfgOpenSearchPriorityProgram p) := by
  intro h
  rcases h with ⟨hSched, hPrio, hMem⟩
  constructor
  · exact lowerFnCfgOpenSearchSchedulerProgram_preserves_witness _ hSched
  constructor
  · simpa [lowerFnCfgOpenSearchPriorityProgram] using hPrio
  · simpa [lowerFnCfgOpenSearchPriorityProgram] using hMem

theorem emitRFnCfgOpenSearchPriorityProgram_preserves_witness
    (p : MirFnCfgOpenSearchPriorityProgram) :
    mirOpenSearchPriorityWitness p →
      rOpenSearchPriorityWitness (emitRFnCfgOpenSearchPriorityProgram p) := by
  intro h
  rcases h with ⟨hSched, hPrio, hMem⟩
  constructor
  · exact emitRFnCfgOpenSearchSchedulerProgram_preserves_witness _ hSched
  constructor
  · simpa [emitRFnCfgOpenSearchPriorityProgram] using hPrio
  · simpa [emitRFnCfgOpenSearchPriorityProgram] using hMem

theorem lowerEmitFnCfgOpenSearchPriorityProgram_preserves_witness
    (p : SrcFnCfgOpenSearchPriorityProgram) :
    srcOpenSearchPriorityWitness p →
      rOpenSearchPriorityWitness
        (emitRFnCfgOpenSearchPriorityProgram (lowerFnCfgOpenSearchPriorityProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchPriorityProgram_preserves_witness _
    (lowerFnCfgOpenSearchPriorityProgram_preserves_witness _ h)

def stableFnCfgOpenSearchPriorityProgram : SrcFnCfgOpenSearchPriorityProgram :=
  { schedProg := stableFnCfgOpenSearchSchedulerProgram
  , policyTail := [(3, [stableClosedLoopSummary])]
  , prioritizedRounds := [(5, [stableClosedLoopSummary, []]), (3, [stableClosedLoopSummary])]
  }

theorem stableFnCfgOpenSearchPriorityProgram_meta_preserved :
    (lowerFnCfgOpenSearchPriorityProgram stableFnCfgOpenSearchPriorityProgram).schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchPriorityProgram stableFnCfgOpenSearchPriorityProgram).policyTail =
        [(3, [stableClosedLoopSummary])] ∧
      (lowerFnCfgOpenSearchPriorityProgram stableFnCfgOpenSearchPriorityProgram).prioritizedRounds =
        [(5, [stableClosedLoopSummary, []]), (3, [stableClosedLoopSummary])] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchPriorityProgram_src_witness :
    srcOpenSearchPriorityWitness stableFnCfgOpenSearchPriorityProgram := by
  constructor
  · exact stableFnCfgOpenSearchSchedulerProgram_src_witness
  constructor
  · rfl
  · exact stableFnCfgOpenSearchSchedulerProgram_src_witness.2.2

theorem stableFnCfgOpenSearchPriorityProgram_eval_preserved :
    evalRFnCfgOpenSearchPriorityProgram
      (emitRFnCfgOpenSearchPriorityProgram
        (lowerFnCfgOpenSearchPriorityProgram stableFnCfgOpenSearchPriorityProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchPriorityProgram_preserved :
    rOpenSearchPriorityWitness
      (emitRFnCfgOpenSearchPriorityProgram
        (lowerFnCfgOpenSearchPriorityProgram stableFnCfgOpenSearchPriorityProgram)) := by
  exact lowerEmitFnCfgOpenSearchPriorityProgram_preserves_witness _
    stableFnCfgOpenSearchPriorityProgram_src_witness

end RRProofs
