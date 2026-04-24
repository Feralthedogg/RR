import RRProofs.PipelineFnCfgOpenSearchPrioritySubset

namespace RRProofs

abbrev OpenSearchPriorityRule := Nat × Nat

def rewriteOpenSearchPriority (rules : List OpenSearchPriorityRule) (prio : Nat) : Nat :=
  match rules.find? (fun entry => entry.1 = prio) with
  | some (_, newPrio) => newPrio
  | none => prio

def rewriteOpenSearchRounds
    (rules : List OpenSearchPriorityRule)
    (rounds : List OpenSearchPriorityRound) : List OpenSearchPriorityRound :=
  rounds.map (fun (prio, updates) => (rewriteOpenSearchPriority rules prio, updates))

structure SrcFnCfgOpenSearchPolicyProgram where
  priorityProg : SrcFnCfgOpenSearchPriorityProgram
  rules : List OpenSearchPriorityRule
  normalizedRounds : List OpenSearchPriorityRound

structure MirFnCfgOpenSearchPolicyProgram where
  priorityProg : MirFnCfgOpenSearchPriorityProgram
  rules : List OpenSearchPriorityRule
  normalizedRounds : List OpenSearchPriorityRound

structure RFnCfgOpenSearchPolicyProgram where
  priorityProg : RFnCfgOpenSearchPriorityProgram
  rules : List OpenSearchPriorityRule
  normalizedRounds : List OpenSearchPriorityRound

def lowerFnCfgOpenSearchPolicyProgram
    (p : SrcFnCfgOpenSearchPolicyProgram) : MirFnCfgOpenSearchPolicyProgram :=
  { priorityProg := lowerFnCfgOpenSearchPriorityProgram p.priorityProg
  , rules := p.rules
  , normalizedRounds := p.normalizedRounds
  }

def emitRFnCfgOpenSearchPolicyProgram
    (p : MirFnCfgOpenSearchPolicyProgram) : RFnCfgOpenSearchPolicyProgram :=
  { priorityProg := emitRFnCfgOpenSearchPriorityProgram p.priorityProg
  , rules := p.rules
  , normalizedRounds := p.normalizedRounds
  }

def evalSrcFnCfgOpenSearchPolicyProgram (p : SrcFnCfgOpenSearchPolicyProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchPriorityProgram p.priorityProg

def evalMirFnCfgOpenSearchPolicyProgram (p : MirFnCfgOpenSearchPolicyProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchPriorityProgram p.priorityProg

def evalRFnCfgOpenSearchPolicyProgram (p : RFnCfgOpenSearchPolicyProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchPriorityProgram p.priorityProg

def srcOpenSearchPolicyWitness (p : SrcFnCfgOpenSearchPolicyProgram) : Prop :=
  srcOpenSearchPriorityWitness p.priorityProg ∧
    p.normalizedRounds = rewriteOpenSearchRounds p.rules p.priorityProg.prioritizedRounds

def mirOpenSearchPolicyWitness (p : MirFnCfgOpenSearchPolicyProgram) : Prop :=
  mirOpenSearchPriorityWitness p.priorityProg ∧
    p.normalizedRounds = rewriteOpenSearchRounds p.rules p.priorityProg.prioritizedRounds

def rOpenSearchPolicyWitness (p : RFnCfgOpenSearchPolicyProgram) : Prop :=
  rOpenSearchPriorityWitness p.priorityProg ∧
    p.normalizedRounds = rewriteOpenSearchRounds p.rules p.priorityProg.prioritizedRounds

theorem lowerFnCfgOpenSearchPolicyProgram_preserves_meta
    (p : SrcFnCfgOpenSearchPolicyProgram) :
    (lowerFnCfgOpenSearchPolicyProgram p).priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchPolicyProgram p).rules = p.rules ∧
      (lowerFnCfgOpenSearchPolicyProgram p).normalizedRounds = p.normalizedRounds := by
  constructor
  · simpa [lowerFnCfgOpenSearchPolicyProgram] using
      (lowerFnCfgOpenSearchPriorityProgram_preserves_meta p.priorityProg).1
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchPolicyProgram_preserves_meta
    (p : MirFnCfgOpenSearchPolicyProgram) :
    (emitRFnCfgOpenSearchPolicyProgram p).priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchPolicyProgram p).rules = p.rules ∧
      (emitRFnCfgOpenSearchPolicyProgram p).normalizedRounds = p.normalizedRounds := by
  constructor
  · simpa [emitRFnCfgOpenSearchPolicyProgram] using
      (emitRFnCfgOpenSearchPriorityProgram_preserves_meta p.priorityProg).1
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchPolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchPolicyProgram) :
    evalMirFnCfgOpenSearchPolicyProgram (lowerFnCfgOpenSearchPolicyProgram p) =
      evalSrcFnCfgOpenSearchPolicyProgram p := by
  rfl

theorem emitRFnCfgOpenSearchPolicyProgram_preserves_eval
    (p : MirFnCfgOpenSearchPolicyProgram) :
    evalRFnCfgOpenSearchPolicyProgram (emitRFnCfgOpenSearchPolicyProgram p) =
      evalMirFnCfgOpenSearchPolicyProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchPolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchPolicyProgram) :
    evalRFnCfgOpenSearchPolicyProgram
        (emitRFnCfgOpenSearchPolicyProgram (lowerFnCfgOpenSearchPolicyProgram p)) =
      evalSrcFnCfgOpenSearchPolicyProgram p := by
  rfl

theorem lowerFnCfgOpenSearchPolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchPolicyProgram) :
    srcOpenSearchPolicyWitness p →
      mirOpenSearchPolicyWitness (lowerFnCfgOpenSearchPolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact lowerFnCfgOpenSearchPriorityProgram_preserves_witness _ hPrio
  · simpa [lowerFnCfgOpenSearchPolicyProgram] using hNorm

theorem emitRFnCfgOpenSearchPolicyProgram_preserves_witness
    (p : MirFnCfgOpenSearchPolicyProgram) :
    mirOpenSearchPolicyWitness p →
      rOpenSearchPolicyWitness (emitRFnCfgOpenSearchPolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact emitRFnCfgOpenSearchPriorityProgram_preserves_witness _ hPrio
  · simpa [emitRFnCfgOpenSearchPolicyProgram] using hNorm

theorem lowerEmitFnCfgOpenSearchPolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchPolicyProgram) :
    srcOpenSearchPolicyWitness p →
      rOpenSearchPolicyWitness
        (emitRFnCfgOpenSearchPolicyProgram (lowerFnCfgOpenSearchPolicyProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchPolicyProgram_preserves_witness _
    (lowerFnCfgOpenSearchPolicyProgram_preserves_witness _ h)

def stableFnCfgOpenSearchPolicyProgram : SrcFnCfgOpenSearchPolicyProgram :=
  { priorityProg := stableFnCfgOpenSearchPriorityProgram
  , rules := [(5, 3), (3, 1)]
  , normalizedRounds := [(3, [stableClosedLoopSummary, []]), (1, [stableClosedLoopSummary])]
  }

theorem stableFnCfgOpenSearchPolicyProgram_meta_preserved :
    (lowerFnCfgOpenSearchPolicyProgram stableFnCfgOpenSearchPolicyProgram).priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchPolicyProgram stableFnCfgOpenSearchPolicyProgram).rules = [(5, 3), (3, 1)] ∧
      (lowerFnCfgOpenSearchPolicyProgram stableFnCfgOpenSearchPolicyProgram).normalizedRounds =
        [(3, [stableClosedLoopSummary, []]), (1, [stableClosedLoopSummary])] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchPolicyProgram_src_witness :
    srcOpenSearchPolicyWitness stableFnCfgOpenSearchPolicyProgram := by
  constructor
  · exact stableFnCfgOpenSearchPriorityProgram_src_witness
  · rfl

theorem stableFnCfgOpenSearchPolicyProgram_eval_preserved :
    evalRFnCfgOpenSearchPolicyProgram
      (emitRFnCfgOpenSearchPolicyProgram
        (lowerFnCfgOpenSearchPolicyProgram stableFnCfgOpenSearchPolicyProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchPolicyProgram_preserved :
    rOpenSearchPolicyWitness
      (emitRFnCfgOpenSearchPolicyProgram
        (lowerFnCfgOpenSearchPolicyProgram stableFnCfgOpenSearchPolicyProgram)) := by
  exact lowerEmitFnCfgOpenSearchPolicyProgram_preserves_witness _
    stableFnCfgOpenSearchPolicyProgram_src_witness

end RRProofs
