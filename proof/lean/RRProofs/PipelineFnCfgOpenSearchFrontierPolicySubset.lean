import RRProofs.PipelineFnCfgOpenSearchFrontierPrioritySubset

namespace RRProofs

abbrev OpenSearchFrontierPriorityRule := Nat × Nat

def rewriteOpenSearchFrontierPriority
    (rules : List OpenSearchFrontierPriorityRule) (prio : Nat) : Nat :=
  match rules.find? (fun entry => entry.1 = prio) with
  | some (_, newPrio) => newPrio
  | none => prio

def rewriteOpenSearchFrontierRounds
    (rules : List OpenSearchFrontierPriorityRule)
    (rounds : List OpenSearchFrontierPriorityRound) : List OpenSearchFrontierPriorityRound :=
  rounds.map (fun (prio, updates) => (rewriteOpenSearchFrontierPriority rules prio, updates))

structure SrcFnCfgOpenSearchFrontierPolicyProgram where
  priorityProg : SrcFnCfgOpenSearchFrontierPriorityProgram
  rules : List OpenSearchFrontierPriorityRule
  normalizedRounds : List OpenSearchFrontierPriorityRound

structure MirFnCfgOpenSearchFrontierPolicyProgram where
  priorityProg : MirFnCfgOpenSearchFrontierPriorityProgram
  rules : List OpenSearchFrontierPriorityRule
  normalizedRounds : List OpenSearchFrontierPriorityRound

structure RFnCfgOpenSearchFrontierPolicyProgram where
  priorityProg : RFnCfgOpenSearchFrontierPriorityProgram
  rules : List OpenSearchFrontierPriorityRule
  normalizedRounds : List OpenSearchFrontierPriorityRound

def lowerFnCfgOpenSearchFrontierPolicyProgram
    (p : SrcFnCfgOpenSearchFrontierPolicyProgram) : MirFnCfgOpenSearchFrontierPolicyProgram :=
  { priorityProg := lowerFnCfgOpenSearchFrontierPriorityProgram p.priorityProg
  , rules := p.rules
  , normalizedRounds := p.normalizedRounds
  }

def emitRFnCfgOpenSearchFrontierPolicyProgram
    (p : MirFnCfgOpenSearchFrontierPolicyProgram) : RFnCfgOpenSearchFrontierPolicyProgram :=
  { priorityProg := emitRFnCfgOpenSearchFrontierPriorityProgram p.priorityProg
  , rules := p.rules
  , normalizedRounds := p.normalizedRounds
  }

def evalSrcFnCfgOpenSearchFrontierPolicyProgram (p : SrcFnCfgOpenSearchFrontierPolicyProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchFrontierPriorityProgram p.priorityProg

def evalMirFnCfgOpenSearchFrontierPolicyProgram (p : MirFnCfgOpenSearchFrontierPolicyProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchFrontierPriorityProgram p.priorityProg

def evalRFnCfgOpenSearchFrontierPolicyProgram (p : RFnCfgOpenSearchFrontierPolicyProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchFrontierPriorityProgram p.priorityProg

def srcOpenSearchFrontierPolicyWitness (p : SrcFnCfgOpenSearchFrontierPolicyProgram) : Prop :=
  srcOpenSearchFrontierPriorityWitness p.priorityProg ∧
    p.normalizedRounds = rewriteOpenSearchFrontierRounds p.rules p.priorityProg.prioritizedRounds

def mirOpenSearchFrontierPolicyWitness (p : MirFnCfgOpenSearchFrontierPolicyProgram) : Prop :=
  mirOpenSearchFrontierPriorityWitness p.priorityProg ∧
    p.normalizedRounds = rewriteOpenSearchFrontierRounds p.rules p.priorityProg.prioritizedRounds

def rOpenSearchFrontierPolicyWitness (p : RFnCfgOpenSearchFrontierPolicyProgram) : Prop :=
  rOpenSearchFrontierPriorityWitness p.priorityProg ∧
    p.normalizedRounds = rewriteOpenSearchFrontierRounds p.rules p.priorityProg.prioritizedRounds

theorem lowerFnCfgOpenSearchFrontierPolicyProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierPolicyProgram) :
    (lowerFnCfgOpenSearchFrontierPolicyProgram p).priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierPolicyProgram p).rules = p.rules ∧
      (lowerFnCfgOpenSearchFrontierPolicyProgram p).normalizedRounds = p.normalizedRounds := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierPolicyProgram] using
      (lowerFnCfgOpenSearchFrontierPriorityProgram_preserves_meta p.priorityProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierPolicyProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierPolicyProgram) :
    (emitRFnCfgOpenSearchFrontierPolicyProgram p).priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierPolicyProgram p).rules = p.rules ∧
      (emitRFnCfgOpenSearchFrontierPolicyProgram p).normalizedRounds = p.normalizedRounds := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierPolicyProgram] using
      (emitRFnCfgOpenSearchFrontierPriorityProgram_preserves_meta p.priorityProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierPolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierPolicyProgram) :
    evalMirFnCfgOpenSearchFrontierPolicyProgram (lowerFnCfgOpenSearchFrontierPolicyProgram p) =
      evalSrcFnCfgOpenSearchFrontierPolicyProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierPolicyProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierPolicyProgram) :
    evalRFnCfgOpenSearchFrontierPolicyProgram (emitRFnCfgOpenSearchFrontierPolicyProgram p) =
      evalMirFnCfgOpenSearchFrontierPolicyProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierPolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierPolicyProgram) :
    evalRFnCfgOpenSearchFrontierPolicyProgram
        (emitRFnCfgOpenSearchFrontierPolicyProgram (lowerFnCfgOpenSearchFrontierPolicyProgram p)) =
      evalSrcFnCfgOpenSearchFrontierPolicyProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierPolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierPolicyProgram) :
    srcOpenSearchFrontierPolicyWitness p →
      mirOpenSearchFrontierPolicyWitness (lowerFnCfgOpenSearchFrontierPolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierPriorityProgram_preserves_witness _ hPrio
  · simpa [lowerFnCfgOpenSearchFrontierPolicyProgram] using hNorm

theorem emitRFnCfgOpenSearchFrontierPolicyProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierPolicyProgram) :
    mirOpenSearchFrontierPolicyWitness p →
      rOpenSearchFrontierPolicyWitness (emitRFnCfgOpenSearchFrontierPolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierPriorityProgram_preserves_witness _ hPrio
  · simpa [emitRFnCfgOpenSearchFrontierPolicyProgram] using hNorm

theorem lowerEmitFnCfgOpenSearchFrontierPolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierPolicyProgram) :
    srcOpenSearchFrontierPolicyWitness p →
      rOpenSearchFrontierPolicyWitness
        (emitRFnCfgOpenSearchFrontierPolicyProgram (lowerFnCfgOpenSearchFrontierPolicyProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierPolicyProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierPolicyProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierPolicyProgram : SrcFnCfgOpenSearchFrontierPolicyProgram :=
  { priorityProg := stableFnCfgOpenSearchFrontierPriorityProgram
  , rules := [(5, 3), (3, 1)]
  , normalizedRounds := [(3, [stableClosedLoopSummary, []]), (1, [stableClosedLoopSummary])]
  }

theorem stableFnCfgOpenSearchFrontierPolicyProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierPolicyProgram stableFnCfgOpenSearchFrontierPolicyProgram).priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierPolicyProgram stableFnCfgOpenSearchFrontierPolicyProgram).rules =
        [(5, 3), (3, 1)] ∧
      (lowerFnCfgOpenSearchFrontierPolicyProgram stableFnCfgOpenSearchFrontierPolicyProgram).normalizedRounds =
        [(3, [stableClosedLoopSummary, []]), (1, [stableClosedLoopSummary])] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierPolicyProgram_src_witness :
    srcOpenSearchFrontierPolicyWitness stableFnCfgOpenSearchFrontierPolicyProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierPriorityProgram_src_witness
  · rfl

theorem stableFnCfgOpenSearchFrontierPolicyProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierPolicyProgram
      (emitRFnCfgOpenSearchFrontierPolicyProgram
        (lowerFnCfgOpenSearchFrontierPolicyProgram stableFnCfgOpenSearchFrontierPolicyProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierPolicyProgram_preserved :
    rOpenSearchFrontierPolicyWitness
      (emitRFnCfgOpenSearchFrontierPolicyProgram
        (lowerFnCfgOpenSearchFrontierPolicyProgram stableFnCfgOpenSearchFrontierPolicyProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierPolicyProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierPolicyProgram_src_witness

end RRProofs
