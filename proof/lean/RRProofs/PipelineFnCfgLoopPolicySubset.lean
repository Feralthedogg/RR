import RRProofs.PipelineFnCfgLoopPrioritySubset

namespace RRProofs

abbrev PriorityRule := Nat × Nat

def rewritePriority (rules : List PriorityRule) (prio : Nat) : Nat :=
  match rules.find? (fun entry => entry.1 = prio) with
  | some (_, newPrio) => newPrio
  | none => prio

def rewritePriorityTrace (rules : List PriorityRule) (trace : PriorityTrace) : PriorityTrace :=
  trace.map (fun (prio, updates) => (rewritePriority rules prio, updates))

structure SrcFnCfgLoopPolicyProgram where
  priorityProg : SrcFnCfgLoopPriorityProgram
  rules : List PriorityRule

structure MirFnCfgLoopPolicyProgram where
  priorityProg : MirFnCfgLoopPriorityProgram
  rules : List PriorityRule

structure RFnCfgLoopPolicyProgram where
  priorityProg : RFnCfgLoopPriorityProgram
  rules : List PriorityRule

def lowerFnCfgLoopPolicyProgram (p : SrcFnCfgLoopPolicyProgram) : MirFnCfgLoopPolicyProgram :=
  { priorityProg := lowerFnCfgLoopPriorityProgram p.priorityProg
  , rules := p.rules
  }

def emitRFnCfgLoopPolicyProgram (p : MirFnCfgLoopPolicyProgram) : RFnCfgLoopPolicyProgram :=
  { priorityProg := emitRFnCfgLoopPriorityProgram p.priorityProg
  , rules := p.rules
  }

def evalSrcFnCfgLoopPolicyProgram (p : SrcFnCfgLoopPolicyProgram) : PriorityTrace :=
  rewritePriorityTrace p.rules (evalSrcFnCfgLoopPriorityProgram p.priorityProg)

def evalMirFnCfgLoopPolicyProgram (p : MirFnCfgLoopPolicyProgram) : PriorityTrace :=
  rewritePriorityTrace p.rules (evalMirFnCfgLoopPriorityProgram p.priorityProg)

def evalRFnCfgLoopPolicyProgram (p : RFnCfgLoopPolicyProgram) : PriorityTrace :=
  rewritePriorityTrace p.rules (evalRFnCfgLoopPriorityProgram p.priorityProg)

def srcLoopPolicyWitness (p : SrcFnCfgLoopPolicyProgram) : Prop :=
  srcLoopPriorityWitness p.priorityProg ∧
    prioritiesNonincreasing ((evalSrcFnCfgLoopPolicyProgram p).map Prod.fst)

def mirLoopPolicyWitness (p : MirFnCfgLoopPolicyProgram) : Prop :=
  mirLoopPriorityWitness p.priorityProg ∧
    prioritiesNonincreasing ((evalMirFnCfgLoopPolicyProgram p).map Prod.fst)

def rLoopPolicyWitness (p : RFnCfgLoopPolicyProgram) : Prop :=
  rLoopPriorityWitness p.priorityProg ∧
    prioritiesNonincreasing ((evalRFnCfgLoopPolicyProgram p).map Prod.fst)

theorem lowerFnCfgLoopPolicyProgram_preserves_meta
    (p : SrcFnCfgLoopPolicyProgram) :
    (lowerFnCfgLoopPolicyProgram p).priorityProg.pending.length = p.priorityProg.pending.length ∧
      (lowerFnCfgLoopPolicyProgram p).priorityProg.reinserts.length = p.priorityProg.reinserts.length ∧
      (lowerFnCfgLoopPolicyProgram p).rules = p.rules := by
  constructor
  · simp [lowerFnCfgLoopPolicyProgram, lowerFnCfgLoopPriorityProgram_preserves_meta]
  constructor
  · simp [lowerFnCfgLoopPolicyProgram, lowerFnCfgLoopPriorityProgram_preserves_meta]
  · rfl

theorem emitRFnCfgLoopPolicyProgram_preserves_meta
    (p : MirFnCfgLoopPolicyProgram) :
    (emitRFnCfgLoopPolicyProgram p).priorityProg.pending.length = p.priorityProg.pending.length ∧
      (emitRFnCfgLoopPolicyProgram p).priorityProg.reinserts.length = p.priorityProg.reinserts.length ∧
      (emitRFnCfgLoopPolicyProgram p).rules = p.rules := by
  constructor
  · simp [emitRFnCfgLoopPolicyProgram, emitRFnCfgLoopPriorityProgram_preserves_meta]
  constructor
  · simp [emitRFnCfgLoopPolicyProgram, emitRFnCfgLoopPriorityProgram_preserves_meta]
  · rfl

theorem lowerFnCfgLoopPolicyProgram_preserves_eval
    (p : SrcFnCfgLoopPolicyProgram) :
    evalMirFnCfgLoopPolicyProgram (lowerFnCfgLoopPolicyProgram p) =
      evalSrcFnCfgLoopPolicyProgram p := by
  simp [evalMirFnCfgLoopPolicyProgram, evalSrcFnCfgLoopPolicyProgram, lowerFnCfgLoopPolicyProgram,
    lowerFnCfgLoopPriorityProgram_preserves_eval]

theorem emitRFnCfgLoopPolicyProgram_preserves_eval
    (p : MirFnCfgLoopPolicyProgram) :
    evalRFnCfgLoopPolicyProgram (emitRFnCfgLoopPolicyProgram p) =
      evalMirFnCfgLoopPolicyProgram p := by
  simp [evalRFnCfgLoopPolicyProgram, evalMirFnCfgLoopPolicyProgram, emitRFnCfgLoopPolicyProgram,
    emitRFnCfgLoopPriorityProgram_preserves_eval]

theorem lowerEmitFnCfgLoopPolicyProgram_preserves_eval
    (p : SrcFnCfgLoopPolicyProgram) :
    evalRFnCfgLoopPolicyProgram (emitRFnCfgLoopPolicyProgram (lowerFnCfgLoopPolicyProgram p)) =
      evalSrcFnCfgLoopPolicyProgram p := by
  rw [emitRFnCfgLoopPolicyProgram_preserves_eval, lowerFnCfgLoopPolicyProgram_preserves_eval]

theorem lowerFnCfgLoopPolicyProgram_preserves_witness
    (p : SrcFnCfgLoopPolicyProgram) :
    srcLoopPolicyWitness p →
      mirLoopPolicyWitness (lowerFnCfgLoopPolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact lowerFnCfgLoopPriorityProgram_preserves_witness _ hPrio
  · simpa [evalMirFnCfgLoopPolicyProgram, evalSrcFnCfgLoopPolicyProgram, lowerFnCfgLoopPolicyProgram,
      lowerFnCfgLoopPriorityProgram_preserves_eval] using hNorm

theorem emitRFnCfgLoopPolicyProgram_preserves_witness
    (p : MirFnCfgLoopPolicyProgram) :
    mirLoopPolicyWitness p →
      rLoopPolicyWitness (emitRFnCfgLoopPolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact emitRFnCfgLoopPriorityProgram_preserves_witness _ hPrio
  · simpa [evalRFnCfgLoopPolicyProgram, evalMirFnCfgLoopPolicyProgram, emitRFnCfgLoopPolicyProgram,
      emitRFnCfgLoopPriorityProgram_preserves_eval] using hNorm

theorem lowerEmitFnCfgLoopPolicyProgram_preserves_witness
    (p : SrcFnCfgLoopPolicyProgram) :
    srcLoopPolicyWitness p →
      rLoopPolicyWitness (emitRFnCfgLoopPolicyProgram (lowerFnCfgLoopPolicyProgram p)) := by
  intro h
  exact emitRFnCfgLoopPolicyProgram_preserves_witness _ (lowerFnCfgLoopPolicyProgram_preserves_witness _ h)

def stableFnCfgLoopPolicyProgram : SrcFnCfgLoopPolicyProgram :=
  { priorityProg := stableFnCfgLoopPriorityProgram
  , rules := [(5, 3), (4, 2), (3, 1)]
  }

theorem stableFnCfgLoopPolicyProgram_meta_preserved :
    (lowerFnCfgLoopPolicyProgram stableFnCfgLoopPolicyProgram).priorityProg.pending.length = 2 ∧
      (lowerFnCfgLoopPolicyProgram stableFnCfgLoopPolicyProgram).priorityProg.reinserts.length = 1 ∧
      (lowerFnCfgLoopPolicyProgram stableFnCfgLoopPolicyProgram).rules = [(5, 3), (4, 2), (3, 1)] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgLoopPolicyProgram_src_witness :
    srcLoopPolicyWitness stableFnCfgLoopPolicyProgram := by
  constructor
  · simpa [stableFnCfgLoopPolicyProgram] using stableFnCfgLoopPriorityProgram_src_witness
  · change prioritiesNonincreasing [3, 2, 1]
    constructor
    · decide
    · constructor
      · decide
      · trivial

theorem stableFnCfgLoopPolicyProgram_eval_preserved :
    evalRFnCfgLoopPolicyProgram
      (emitRFnCfgLoopPolicyProgram (lowerFnCfgLoopPolicyProgram stableFnCfgLoopPolicyProgram)) =
      [ (3, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
      , (2, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
      , (1, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
      ] := by
  rw [lowerEmitFnCfgLoopPolicyProgram_preserves_eval]
  rfl

theorem stableFnCfgLoopPolicyProgram_preserved :
    rLoopPolicyWitness
      (emitRFnCfgLoopPolicyProgram (lowerFnCfgLoopPolicyProgram stableFnCfgLoopPolicyProgram)) := by
  exact lowerEmitFnCfgLoopPolicyProgram_preserves_witness _ stableFnCfgLoopPolicyProgram_src_witness

end RRProofs
