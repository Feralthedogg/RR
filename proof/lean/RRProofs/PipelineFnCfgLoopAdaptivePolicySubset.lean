import RRProofs.PipelineFnCfgLoopPolicySubset

namespace RRProofs

def recomputePriorityRules : List PriorityRule → List Nat → List PriorityRule
  | [], _ => []
  | _, [] => []
  | (src, _) :: rules, dst :: feedback =>
      (src, dst) :: recomputePriorityRules rules feedback

structure SrcFnCfgLoopAdaptivePolicyProgram where
  priorityProg : SrcFnCfgLoopPriorityProgram
  baseRules : List PriorityRule
  feedback : List Nat

structure MirFnCfgLoopAdaptivePolicyProgram where
  priorityProg : MirFnCfgLoopPriorityProgram
  baseRules : List PriorityRule
  feedback : List Nat

structure RFnCfgLoopAdaptivePolicyProgram where
  priorityProg : RFnCfgLoopPriorityProgram
  baseRules : List PriorityRule
  feedback : List Nat

def lowerFnCfgLoopAdaptivePolicyProgram (p : SrcFnCfgLoopAdaptivePolicyProgram) :
    MirFnCfgLoopAdaptivePolicyProgram :=
  { priorityProg := lowerFnCfgLoopPriorityProgram p.priorityProg
  , baseRules := p.baseRules
  , feedback := p.feedback
  }

def emitRFnCfgLoopAdaptivePolicyProgram (p : MirFnCfgLoopAdaptivePolicyProgram) :
    RFnCfgLoopAdaptivePolicyProgram :=
  { priorityProg := emitRFnCfgLoopPriorityProgram p.priorityProg
  , baseRules := p.baseRules
  , feedback := p.feedback
  }

def evalSrcFnCfgLoopAdaptivePolicyProgram (p : SrcFnCfgLoopAdaptivePolicyProgram) : PriorityTrace :=
  rewritePriorityTrace (recomputePriorityRules p.baseRules p.feedback)
    (evalSrcFnCfgLoopPriorityProgram p.priorityProg)

def evalMirFnCfgLoopAdaptivePolicyProgram (p : MirFnCfgLoopAdaptivePolicyProgram) : PriorityTrace :=
  rewritePriorityTrace (recomputePriorityRules p.baseRules p.feedback)
    (evalMirFnCfgLoopPriorityProgram p.priorityProg)

def evalRFnCfgLoopAdaptivePolicyProgram (p : RFnCfgLoopAdaptivePolicyProgram) : PriorityTrace :=
  rewritePriorityTrace (recomputePriorityRules p.baseRules p.feedback)
    (evalRFnCfgLoopPriorityProgram p.priorityProg)

def srcLoopAdaptivePolicyWitness (p : SrcFnCfgLoopAdaptivePolicyProgram) : Prop :=
  srcLoopPriorityWitness p.priorityProg ∧
    prioritiesNonincreasing ((evalSrcFnCfgLoopAdaptivePolicyProgram p).map Prod.fst)

def mirLoopAdaptivePolicyWitness (p : MirFnCfgLoopAdaptivePolicyProgram) : Prop :=
  mirLoopPriorityWitness p.priorityProg ∧
    prioritiesNonincreasing ((evalMirFnCfgLoopAdaptivePolicyProgram p).map Prod.fst)

def rLoopAdaptivePolicyWitness (p : RFnCfgLoopAdaptivePolicyProgram) : Prop :=
  rLoopPriorityWitness p.priorityProg ∧
    prioritiesNonincreasing ((evalRFnCfgLoopAdaptivePolicyProgram p).map Prod.fst)

theorem lowerFnCfgLoopAdaptivePolicyProgram_preserves_meta
    (p : SrcFnCfgLoopAdaptivePolicyProgram) :
    (lowerFnCfgLoopAdaptivePolicyProgram p).priorityProg.pending.length = p.priorityProg.pending.length ∧
      (lowerFnCfgLoopAdaptivePolicyProgram p).priorityProg.reinserts.length = p.priorityProg.reinserts.length ∧
      (lowerFnCfgLoopAdaptivePolicyProgram p).baseRules = p.baseRules ∧
      (lowerFnCfgLoopAdaptivePolicyProgram p).feedback = p.feedback := by
  constructor
  · simp [lowerFnCfgLoopAdaptivePolicyProgram, lowerFnCfgLoopPriorityProgram_preserves_meta]
  constructor
  · simp [lowerFnCfgLoopAdaptivePolicyProgram, lowerFnCfgLoopPriorityProgram_preserves_meta]
  constructor
  · rfl
  · rfl

theorem emitRFnCfgLoopAdaptivePolicyProgram_preserves_meta
    (p : MirFnCfgLoopAdaptivePolicyProgram) :
    (emitRFnCfgLoopAdaptivePolicyProgram p).priorityProg.pending.length = p.priorityProg.pending.length ∧
      (emitRFnCfgLoopAdaptivePolicyProgram p).priorityProg.reinserts.length = p.priorityProg.reinserts.length ∧
      (emitRFnCfgLoopAdaptivePolicyProgram p).baseRules = p.baseRules ∧
      (emitRFnCfgLoopAdaptivePolicyProgram p).feedback = p.feedback := by
  constructor
  · simp [emitRFnCfgLoopAdaptivePolicyProgram, emitRFnCfgLoopPriorityProgram_preserves_meta]
  constructor
  · simp [emitRFnCfgLoopAdaptivePolicyProgram, emitRFnCfgLoopPriorityProgram_preserves_meta]
  constructor
  · rfl
  · rfl

theorem lowerFnCfgLoopAdaptivePolicyProgram_preserves_eval
    (p : SrcFnCfgLoopAdaptivePolicyProgram) :
    evalMirFnCfgLoopAdaptivePolicyProgram (lowerFnCfgLoopAdaptivePolicyProgram p) =
      evalSrcFnCfgLoopAdaptivePolicyProgram p := by
  simp [evalMirFnCfgLoopAdaptivePolicyProgram, evalSrcFnCfgLoopAdaptivePolicyProgram,
    lowerFnCfgLoopAdaptivePolicyProgram, lowerFnCfgLoopPriorityProgram_preserves_eval]

theorem emitRFnCfgLoopAdaptivePolicyProgram_preserves_eval
    (p : MirFnCfgLoopAdaptivePolicyProgram) :
    evalRFnCfgLoopAdaptivePolicyProgram (emitRFnCfgLoopAdaptivePolicyProgram p) =
      evalMirFnCfgLoopAdaptivePolicyProgram p := by
  simp [evalRFnCfgLoopAdaptivePolicyProgram, evalMirFnCfgLoopAdaptivePolicyProgram,
    emitRFnCfgLoopAdaptivePolicyProgram, emitRFnCfgLoopPriorityProgram_preserves_eval]

theorem lowerEmitFnCfgLoopAdaptivePolicyProgram_preserves_eval
    (p : SrcFnCfgLoopAdaptivePolicyProgram) :
    evalRFnCfgLoopAdaptivePolicyProgram
        (emitRFnCfgLoopAdaptivePolicyProgram (lowerFnCfgLoopAdaptivePolicyProgram p)) =
      evalSrcFnCfgLoopAdaptivePolicyProgram p := by
  rw [emitRFnCfgLoopAdaptivePolicyProgram_preserves_eval,
    lowerFnCfgLoopAdaptivePolicyProgram_preserves_eval]

theorem lowerFnCfgLoopAdaptivePolicyProgram_preserves_witness
    (p : SrcFnCfgLoopAdaptivePolicyProgram) :
    srcLoopAdaptivePolicyWitness p →
      mirLoopAdaptivePolicyWitness (lowerFnCfgLoopAdaptivePolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact lowerFnCfgLoopPriorityProgram_preserves_witness _ hPrio
  · simpa [evalMirFnCfgLoopAdaptivePolicyProgram, evalSrcFnCfgLoopAdaptivePolicyProgram,
      lowerFnCfgLoopAdaptivePolicyProgram, lowerFnCfgLoopPriorityProgram_preserves_eval] using hNorm

theorem emitRFnCfgLoopAdaptivePolicyProgram_preserves_witness
    (p : MirFnCfgLoopAdaptivePolicyProgram) :
    mirLoopAdaptivePolicyWitness p →
      rLoopAdaptivePolicyWitness (emitRFnCfgLoopAdaptivePolicyProgram p) := by
  intro h
  rcases h with ⟨hPrio, hNorm⟩
  constructor
  · exact emitRFnCfgLoopPriorityProgram_preserves_witness _ hPrio
  · simpa [evalRFnCfgLoopAdaptivePolicyProgram, evalMirFnCfgLoopAdaptivePolicyProgram,
      emitRFnCfgLoopAdaptivePolicyProgram, emitRFnCfgLoopPriorityProgram_preserves_eval] using hNorm

theorem lowerEmitFnCfgLoopAdaptivePolicyProgram_preserves_witness
    (p : SrcFnCfgLoopAdaptivePolicyProgram) :
    srcLoopAdaptivePolicyWitness p →
      rLoopAdaptivePolicyWitness
        (emitRFnCfgLoopAdaptivePolicyProgram (lowerFnCfgLoopAdaptivePolicyProgram p)) := by
  intro h
  exact emitRFnCfgLoopAdaptivePolicyProgram_preserves_witness _
    (lowerFnCfgLoopAdaptivePolicyProgram_preserves_witness _ h)

def stableFnCfgLoopAdaptivePolicyProgram : SrcFnCfgLoopAdaptivePolicyProgram :=
  { priorityProg := stableFnCfgLoopPriorityProgram
  , baseRules := [(5, 9), (4, 9), (3, 9)]
  , feedback := [3, 2, 1]
  }

theorem stableFnCfgLoopAdaptivePolicyProgram_meta_preserved :
    (lowerFnCfgLoopAdaptivePolicyProgram stableFnCfgLoopAdaptivePolicyProgram).priorityProg.pending.length = 2 ∧
      (lowerFnCfgLoopAdaptivePolicyProgram stableFnCfgLoopAdaptivePolicyProgram).priorityProg.reinserts.length = 1 ∧
      (lowerFnCfgLoopAdaptivePolicyProgram stableFnCfgLoopAdaptivePolicyProgram).baseRules = [(5, 9), (4, 9), (3, 9)] ∧
      (lowerFnCfgLoopAdaptivePolicyProgram stableFnCfgLoopAdaptivePolicyProgram).feedback = [3, 2, 1] := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgLoopAdaptivePolicyProgram_src_witness :
    srcLoopAdaptivePolicyWitness stableFnCfgLoopAdaptivePolicyProgram := by
  constructor
  · simpa [stableFnCfgLoopAdaptivePolicyProgram] using stableFnCfgLoopPriorityProgram_src_witness
  · change prioritiesNonincreasing [3, 2, 1]
    constructor
    · decide
    · constructor
      · decide
      · trivial

theorem stableFnCfgLoopAdaptivePolicyProgram_eval_preserved :
    evalRFnCfgLoopAdaptivePolicyProgram
      (emitRFnCfgLoopAdaptivePolicyProgram (lowerFnCfgLoopAdaptivePolicyProgram stableFnCfgLoopAdaptivePolicyProgram)) =
      [ (3, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
      , (2, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
      , (1, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
      ] := by
  rw [lowerEmitFnCfgLoopAdaptivePolicyProgram_preserves_eval]
  rfl

theorem stableFnCfgLoopAdaptivePolicyProgram_preserved :
    rLoopAdaptivePolicyWitness
      (emitRFnCfgLoopAdaptivePolicyProgram (lowerFnCfgLoopAdaptivePolicyProgram stableFnCfgLoopAdaptivePolicyProgram)) := by
  exact lowerEmitFnCfgLoopAdaptivePolicyProgram_preserves_witness _
    stableFnCfgLoopAdaptivePolicyProgram_src_witness

end RRProofs
