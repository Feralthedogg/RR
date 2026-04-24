import RRProofs.PipelineFnCfgOpenSearchFrontierReopenPrioritySubset

namespace RRProofs

abbrev OpenSearchFrontierReopenPriorityRule := Nat × Nat

def rewriteOpenSearchFrontierReopenPriority
    (rules : List OpenSearchFrontierReopenPriorityRule) (prio : Nat) : Nat :=
  match rules.find? (fun entry => entry.1 = prio) with
  | some (_, newPrio) => newPrio
  | none => prio

def rewriteOpenSearchFrontierReopenRounds
    (rules : List OpenSearchFrontierReopenPriorityRule)
    (rounds : List OpenSearchFrontierReopenPriorityRound) :
    List OpenSearchFrontierReopenPriorityRound :=
  rounds.map (fun (prio, updates) =>
    (rewriteOpenSearchFrontierReopenPriority rules prio, updates))

structure SrcFnCfgOpenSearchFrontierReopenPolicyProgram where
  priorityProg : SrcFnCfgOpenSearchFrontierReopenPriorityProgram
  rules : List OpenSearchFrontierReopenPriorityRule
  normalizedRounds : List OpenSearchFrontierReopenPriorityRound

structure MirFnCfgOpenSearchFrontierReopenPolicyProgram where
  priorityProg : MirFnCfgOpenSearchFrontierReopenPriorityProgram
  rules : List OpenSearchFrontierReopenPriorityRule
  normalizedRounds : List OpenSearchFrontierReopenPriorityRound

structure RFnCfgOpenSearchFrontierReopenPolicyProgram where
  priorityProg : RFnCfgOpenSearchFrontierReopenPriorityProgram
  rules : List OpenSearchFrontierReopenPriorityRule
  normalizedRounds : List OpenSearchFrontierReopenPriorityRound

def lowerFnCfgOpenSearchFrontierReopenPolicyProgram
    (p : SrcFnCfgOpenSearchFrontierReopenPolicyProgram) :
    MirFnCfgOpenSearchFrontierReopenPolicyProgram :=
  { priorityProg := lowerFnCfgOpenSearchFrontierReopenPriorityProgram p.priorityProg
  , rules := p.rules
  , normalizedRounds := p.normalizedRounds
  }

def emitRFnCfgOpenSearchFrontierReopenPolicyProgram
    (p : MirFnCfgOpenSearchFrontierReopenPolicyProgram) :
    RFnCfgOpenSearchFrontierReopenPolicyProgram :=
  { priorityProg := emitRFnCfgOpenSearchFrontierReopenPriorityProgram p.priorityProg
  , rules := p.rules
  , normalizedRounds := p.normalizedRounds
  }

def evalSrcFnCfgOpenSearchFrontierReopenPolicyProgram
    (p : SrcFnCfgOpenSearchFrontierReopenPolicyProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchFrontierReopenPriorityProgram p.priorityProg

def evalMirFnCfgOpenSearchFrontierReopenPolicyProgram
    (p : MirFnCfgOpenSearchFrontierReopenPolicyProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchFrontierReopenPriorityProgram p.priorityProg

def evalRFnCfgOpenSearchFrontierReopenPolicyProgram
    (p : RFnCfgOpenSearchFrontierReopenPolicyProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchFrontierReopenPriorityProgram p.priorityProg

def srcOpenSearchFrontierReopenPolicyWitness
    (p : SrcFnCfgOpenSearchFrontierReopenPolicyProgram) : Prop :=
  srcOpenSearchFrontierReopenPriorityWitness p.priorityProg ∧
    p.normalizedRounds =
      rewriteOpenSearchFrontierReopenRounds p.rules p.priorityProg.prioritizedRounds

def mirOpenSearchFrontierReopenPolicyWitness
    (p : MirFnCfgOpenSearchFrontierReopenPolicyProgram) : Prop :=
  mirOpenSearchFrontierReopenPriorityWitness p.priorityProg ∧
    p.normalizedRounds =
      rewriteOpenSearchFrontierReopenRounds p.rules p.priorityProg.prioritizedRounds

def rOpenSearchFrontierReopenPolicyWitness
    (p : RFnCfgOpenSearchFrontierReopenPolicyProgram) : Prop :=
  rOpenSearchFrontierReopenPriorityWitness p.priorityProg ∧
    p.normalizedRounds =
      rewriteOpenSearchFrontierReopenRounds p.rules p.priorityProg.prioritizedRounds

theorem lowerFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierReopenPolicyProgram) :
    (lowerFnCfgOpenSearchFrontierReopenPolicyProgram p).priorityProg.schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.priorityProg.schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierReopenPolicyProgram p).rules = p.rules ∧
      (lowerFnCfgOpenSearchFrontierReopenPolicyProgram p).normalizedRounds =
        p.normalizedRounds := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierReopenPolicyProgram] using
      (lowerFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_meta
        p.priorityProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierReopenPolicyProgram) :
    (emitRFnCfgOpenSearchFrontierReopenPolicyProgram p).priorityProg.schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.priorityProg.schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierReopenPolicyProgram p).rules = p.rules ∧
      (emitRFnCfgOpenSearchFrontierReopenPolicyProgram p).normalizedRounds =
        p.normalizedRounds := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierReopenPolicyProgram] using
      (emitRFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_meta
        p.priorityProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenPolicyProgram) :
    evalMirFnCfgOpenSearchFrontierReopenPolicyProgram
        (lowerFnCfgOpenSearchFrontierReopenPolicyProgram p) =
      evalSrcFnCfgOpenSearchFrontierReopenPolicyProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierReopenPolicyProgram) :
    evalRFnCfgOpenSearchFrontierReopenPolicyProgram
        (emitRFnCfgOpenSearchFrontierReopenPolicyProgram p) =
      evalMirFnCfgOpenSearchFrontierReopenPolicyProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierReopenPolicyProgram) :
    evalRFnCfgOpenSearchFrontierReopenPolicyProgram
        (emitRFnCfgOpenSearchFrontierReopenPolicyProgram
          (lowerFnCfgOpenSearchFrontierReopenPolicyProgram p)) =
      evalSrcFnCfgOpenSearchFrontierReopenPolicyProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenPolicyProgram) :
    srcOpenSearchFrontierReopenPolicyWitness p →
      mirOpenSearchFrontierReopenPolicyWitness
        (lowerFnCfgOpenSearchFrontierReopenPolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_witness _
      hPrio
  · simpa [lowerFnCfgOpenSearchFrontierReopenPolicyProgram] using hNorm

theorem emitRFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierReopenPolicyProgram) :
    mirOpenSearchFrontierReopenPolicyWitness p →
      rOpenSearchFrontierReopenPolicyWitness
        (emitRFnCfgOpenSearchFrontierReopenPolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierReopenPriorityProgram_preserves_witness _
      hPrio
  · simpa [emitRFnCfgOpenSearchFrontierReopenPolicyProgram] using hNorm

theorem lowerEmitFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierReopenPolicyProgram) :
    srcOpenSearchFrontierReopenPolicyWitness p →
      rOpenSearchFrontierReopenPolicyWitness
        (emitRFnCfgOpenSearchFrontierReopenPolicyProgram
          (lowerFnCfgOpenSearchFrontierReopenPolicyProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierReopenPolicyProgram :
    SrcFnCfgOpenSearchFrontierReopenPolicyProgram :=
  { priorityProg := stableFnCfgOpenSearchFrontierReopenPriorityProgram
  , rules := [(5, 3), (3, 1)]
  , normalizedRounds :=
      [(3, [stableClosedLoopSummary, []]), (1, [stableClosedLoopSummary])]
  }

theorem stableFnCfgOpenSearchFrontierReopenPolicyProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierReopenPolicyProgram
      stableFnCfgOpenSearchFrontierReopenPolicyProgram).priorityProg.schedProg.dynProg.reopenProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierReopenPolicyProgram
        stableFnCfgOpenSearchFrontierReopenPolicyProgram).rules =
        [(5, 3), (3, 1)] ∧
      (lowerFnCfgOpenSearchFrontierReopenPolicyProgram
        stableFnCfgOpenSearchFrontierReopenPolicyProgram).normalizedRounds =
        [(3, [stableClosedLoopSummary, []]), (1, [stableClosedLoopSummary])] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierReopenPolicyProgram_src_witness :
    srcOpenSearchFrontierReopenPolicyWitness
      stableFnCfgOpenSearchFrontierReopenPolicyProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierReopenPriorityProgram_src_witness
  · rfl

theorem stableFnCfgOpenSearchFrontierReopenPolicyProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierReopenPolicyProgram
      (emitRFnCfgOpenSearchFrontierReopenPolicyProgram
        (lowerFnCfgOpenSearchFrontierReopenPolicyProgram
          stableFnCfgOpenSearchFrontierReopenPolicyProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierReopenPolicyProgram_preserved :
    rOpenSearchFrontierReopenPolicyWitness
      (emitRFnCfgOpenSearchFrontierReopenPolicyProgram
        (lowerFnCfgOpenSearchFrontierReopenPolicyProgram
          stableFnCfgOpenSearchFrontierReopenPolicyProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierReopenPolicyProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierReopenPolicyProgram_src_witness

end RRProofs
