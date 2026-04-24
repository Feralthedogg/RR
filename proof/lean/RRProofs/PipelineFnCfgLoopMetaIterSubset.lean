import RRProofs.PipelineFnCfgLoopClosedLoopSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

def lastPrioritySummary : AdaptivePriorityTrace → PriorityTrace
  | [] => []
  | [trace] => trace
  | _ :: rest => lastPrioritySummary rest

def lastTwoSummariesAgree : AdaptivePriorityTrace → Prop
  | [] => True
  | [_] => True
  | [a, b] => a = b
  | _ :: rest => lastTwoSummariesAgree rest

structure SrcFnCfgLoopMetaIterProgram where
  closedLoopProg : SrcFnCfgLoopClosedLoopProgram
  summary : PriorityTrace

structure MirFnCfgLoopMetaIterProgram where
  closedLoopProg : MirFnCfgLoopClosedLoopProgram
  summary : PriorityTrace

structure RFnCfgLoopMetaIterProgram where
  closedLoopProg : RFnCfgLoopClosedLoopProgram
  summary : PriorityTrace

def lowerFnCfgLoopMetaIterProgram (p : SrcFnCfgLoopMetaIterProgram) : MirFnCfgLoopMetaIterProgram :=
  { closedLoopProg := lowerFnCfgLoopClosedLoopProgram p.closedLoopProg
  , summary := p.summary
  }

def emitRFnCfgLoopMetaIterProgram (p : MirFnCfgLoopMetaIterProgram) : RFnCfgLoopMetaIterProgram :=
  { closedLoopProg := emitRFnCfgLoopClosedLoopProgram p.closedLoopProg
  , summary := p.summary
  }

def evalSrcFnCfgLoopMetaIterProgram (p : SrcFnCfgLoopMetaIterProgram) : PriorityTrace :=
  lastPrioritySummary (evalSrcFnCfgLoopClosedLoopProgram p.closedLoopProg)

def evalMirFnCfgLoopMetaIterProgram (p : MirFnCfgLoopMetaIterProgram) : PriorityTrace :=
  lastPrioritySummary (evalMirFnCfgLoopClosedLoopProgram p.closedLoopProg)

def evalRFnCfgLoopMetaIterProgram (p : RFnCfgLoopMetaIterProgram) : PriorityTrace :=
  lastPrioritySummary (evalRFnCfgLoopClosedLoopProgram p.closedLoopProg)

def srcLoopMetaIterWitness (p : SrcFnCfgLoopMetaIterProgram) : Prop :=
  evalSrcFnCfgLoopMetaIterProgram p = p.summary ∧
    lastTwoSummariesAgree (evalSrcFnCfgLoopClosedLoopProgram p.closedLoopProg)

def mirLoopMetaIterWitness (p : MirFnCfgLoopMetaIterProgram) : Prop :=
  evalMirFnCfgLoopMetaIterProgram p = p.summary ∧
    lastTwoSummariesAgree (evalMirFnCfgLoopClosedLoopProgram p.closedLoopProg)

def rLoopMetaIterWitness (p : RFnCfgLoopMetaIterProgram) : Prop :=
  evalRFnCfgLoopMetaIterProgram p = p.summary ∧
    lastTwoSummariesAgree (evalRFnCfgLoopClosedLoopProgram p.closedLoopProg)

theorem lowerFnCfgLoopMetaIterProgram_preserves_meta
    (p : SrcFnCfgLoopMetaIterProgram) :
    (lowerFnCfgLoopMetaIterProgram p).closedLoopProg.rounds.length = p.closedLoopProg.rounds.length ∧
      (lowerFnCfgLoopMetaIterProgram p).summary = p.summary := by
  constructor
  · simp [lowerFnCfgLoopMetaIterProgram, lowerFnCfgLoopClosedLoopProgram_preserves_meta]
  · rfl

theorem emitRFnCfgLoopMetaIterProgram_preserves_meta
    (p : MirFnCfgLoopMetaIterProgram) :
    (emitRFnCfgLoopMetaIterProgram p).closedLoopProg.rounds.length = p.closedLoopProg.rounds.length ∧
      (emitRFnCfgLoopMetaIterProgram p).summary = p.summary := by
  constructor
  · simp [emitRFnCfgLoopMetaIterProgram, emitRFnCfgLoopClosedLoopProgram_preserves_meta]
  · rfl

theorem lowerFnCfgLoopMetaIterProgram_preserves_eval
    (p : SrcFnCfgLoopMetaIterProgram) :
    evalMirFnCfgLoopMetaIterProgram (lowerFnCfgLoopMetaIterProgram p) =
      evalSrcFnCfgLoopMetaIterProgram p := by
  simp [evalMirFnCfgLoopMetaIterProgram, evalSrcFnCfgLoopMetaIterProgram,
    lowerFnCfgLoopMetaIterProgram, lowerFnCfgLoopClosedLoopProgram_preserves_eval]

theorem emitRFnCfgLoopMetaIterProgram_preserves_eval
    (p : MirFnCfgLoopMetaIterProgram) :
    evalRFnCfgLoopMetaIterProgram (emitRFnCfgLoopMetaIterProgram p) =
      evalMirFnCfgLoopMetaIterProgram p := by
  simp [evalRFnCfgLoopMetaIterProgram, evalMirFnCfgLoopMetaIterProgram,
    emitRFnCfgLoopMetaIterProgram, emitRFnCfgLoopClosedLoopProgram_preserves_eval]

theorem lowerEmitFnCfgLoopMetaIterProgram_preserves_eval
    (p : SrcFnCfgLoopMetaIterProgram) :
    evalRFnCfgLoopMetaIterProgram (emitRFnCfgLoopMetaIterProgram (lowerFnCfgLoopMetaIterProgram p)) =
      evalSrcFnCfgLoopMetaIterProgram p := by
  rw [emitRFnCfgLoopMetaIterProgram_preserves_eval, lowerFnCfgLoopMetaIterProgram_preserves_eval]

theorem lowerFnCfgLoopMetaIterProgram_preserves_witness
    (p : SrcFnCfgLoopMetaIterProgram) :
    srcLoopMetaIterWitness p →
      mirLoopMetaIterWitness (lowerFnCfgLoopMetaIterProgram p) := by
  intro h
  rcases h with ⟨hSummary, hStable⟩
  constructor
  · simpa [evalMirFnCfgLoopMetaIterProgram, evalSrcFnCfgLoopMetaIterProgram,
      lowerFnCfgLoopMetaIterProgram, lowerFnCfgLoopClosedLoopProgram_preserves_eval] using hSummary
  · simpa [lowerFnCfgLoopMetaIterProgram, lowerFnCfgLoopClosedLoopProgram_preserves_eval] using hStable

theorem emitRFnCfgLoopMetaIterProgram_preserves_witness
    (p : MirFnCfgLoopMetaIterProgram) :
    mirLoopMetaIterWitness p →
      rLoopMetaIterWitness (emitRFnCfgLoopMetaIterProgram p) := by
  intro h
  rcases h with ⟨hSummary, hStable⟩
  constructor
  · simpa [evalRFnCfgLoopMetaIterProgram, evalMirFnCfgLoopMetaIterProgram,
      emitRFnCfgLoopMetaIterProgram, emitRFnCfgLoopClosedLoopProgram_preserves_eval] using hSummary
  · simpa [emitRFnCfgLoopMetaIterProgram, emitRFnCfgLoopClosedLoopProgram_preserves_eval] using hStable

theorem lowerEmitFnCfgLoopMetaIterProgram_preserves_witness
    (p : SrcFnCfgLoopMetaIterProgram) :
    srcLoopMetaIterWitness p →
      rLoopMetaIterWitness (emitRFnCfgLoopMetaIterProgram (lowerFnCfgLoopMetaIterProgram p)) := by
  intro h
  exact emitRFnCfgLoopMetaIterProgram_preserves_witness _
    (lowerFnCfgLoopMetaIterProgram_preserves_witness _ h)

def stableClosedLoopSummary : PriorityTrace :=
  [ (3, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
  , (2, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
  , (1, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
  ]

def stableFnCfgLoopMetaIterProgram : SrcFnCfgLoopMetaIterProgram :=
  { closedLoopProg := stableFnCfgLoopClosedLoopProgram
  , summary := stableClosedLoopSummary
  }

theorem stableFnCfgLoopMetaIterProgram_meta_preserved :
    (lowerFnCfgLoopMetaIterProgram stableFnCfgLoopMetaIterProgram).closedLoopProg.rounds.length = 2 ∧
      (lowerFnCfgLoopMetaIterProgram stableFnCfgLoopMetaIterProgram).summary = stableClosedLoopSummary := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgLoopClosedLoopProgram_src_eval :
    evalSrcFnCfgLoopClosedLoopProgram stableFnCfgLoopClosedLoopProgram =
      [stableClosedLoopSummary, stableClosedLoopSummary] := by
  have h := stableFnCfgLoopClosedLoopProgram_eval_preserved
  rw [lowerEmitFnCfgLoopClosedLoopProgram_preserves_eval] at h
  exact h

theorem stableFnCfgLoopMetaIterProgram_src_witness :
    srcLoopMetaIterWitness stableFnCfgLoopMetaIterProgram := by
  constructor
  · simp [srcLoopMetaIterWitness, evalSrcFnCfgLoopMetaIterProgram, stableFnCfgLoopMetaIterProgram,
      lastPrioritySummary]
    rw [stableFnCfgLoopClosedLoopProgram_src_eval]
    rfl
  · simp [srcLoopMetaIterWitness, stableFnCfgLoopMetaIterProgram, lastTwoSummariesAgree]
    rw [stableFnCfgLoopClosedLoopProgram_src_eval]
    rfl

theorem stableFnCfgLoopMetaIterProgram_eval_preserved :
    evalRFnCfgLoopMetaIterProgram
      (emitRFnCfgLoopMetaIterProgram (lowerFnCfgLoopMetaIterProgram stableFnCfgLoopMetaIterProgram)) =
      stableClosedLoopSummary := by
  rw [lowerEmitFnCfgLoopMetaIterProgram_preserves_eval]
  rw [show evalSrcFnCfgLoopMetaIterProgram stableFnCfgLoopMetaIterProgram = stableClosedLoopSummary by
    exact stableFnCfgLoopMetaIterProgram_src_witness.1]

theorem stableFnCfgLoopMetaIterProgram_preserved :
    rLoopMetaIterWitness
      (emitRFnCfgLoopMetaIterProgram (lowerFnCfgLoopMetaIterProgram stableFnCfgLoopMetaIterProgram)) := by
  exact lowerEmitFnCfgLoopMetaIterProgram_preserves_witness _
    stableFnCfgLoopMetaIterProgram_src_witness

end RRProofs
