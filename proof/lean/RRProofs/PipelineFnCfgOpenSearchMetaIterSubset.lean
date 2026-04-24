import RRProofs.PipelineFnCfgOpenSearchClosedLoopSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

def lastOpenSearchSummary : AdaptiveOpenSearchTrace → PriorityTrace
  | [] => []
  | [trace] => trace
  | _ :: rest => lastOpenSearchSummary rest

def lastTwoOpenSearchSummariesAgree : AdaptiveOpenSearchTrace → Prop
  | [] => True
  | [_] => True
  | [a, b] => a = b
  | _ :: rest => lastTwoOpenSearchSummariesAgree rest

structure SrcFnCfgOpenSearchMetaIterProgram where
  closedLoopProg : SrcFnCfgOpenSearchClosedLoopProgram
  summary : PriorityTrace

structure MirFnCfgOpenSearchMetaIterProgram where
  closedLoopProg : MirFnCfgOpenSearchClosedLoopProgram
  summary : PriorityTrace

structure RFnCfgOpenSearchMetaIterProgram where
  closedLoopProg : RFnCfgOpenSearchClosedLoopProgram
  summary : PriorityTrace

def lowerFnCfgOpenSearchMetaIterProgram
    (p : SrcFnCfgOpenSearchMetaIterProgram) : MirFnCfgOpenSearchMetaIterProgram :=
  { closedLoopProg := lowerFnCfgOpenSearchClosedLoopProgram p.closedLoopProg
  , summary := p.summary
  }

def emitRFnCfgOpenSearchMetaIterProgram
    (p : MirFnCfgOpenSearchMetaIterProgram) : RFnCfgOpenSearchMetaIterProgram :=
  { closedLoopProg := emitRFnCfgOpenSearchClosedLoopProgram p.closedLoopProg
  , summary := p.summary
  }

def evalSrcFnCfgOpenSearchMetaIterProgram (p : SrcFnCfgOpenSearchMetaIterProgram) : PriorityTrace :=
  lastOpenSearchSummary (evalSrcFnCfgOpenSearchClosedLoopProgram p.closedLoopProg)

def evalMirFnCfgOpenSearchMetaIterProgram (p : MirFnCfgOpenSearchMetaIterProgram) : PriorityTrace :=
  lastOpenSearchSummary (evalMirFnCfgOpenSearchClosedLoopProgram p.closedLoopProg)

def evalRFnCfgOpenSearchMetaIterProgram (p : RFnCfgOpenSearchMetaIterProgram) : PriorityTrace :=
  lastOpenSearchSummary (evalRFnCfgOpenSearchClosedLoopProgram p.closedLoopProg)

def srcOpenSearchMetaIterWitness (p : SrcFnCfgOpenSearchMetaIterProgram) : Prop :=
  evalSrcFnCfgOpenSearchMetaIterProgram p = p.summary ∧
    lastTwoOpenSearchSummariesAgree (evalSrcFnCfgOpenSearchClosedLoopProgram p.closedLoopProg)

def mirOpenSearchMetaIterWitness (p : MirFnCfgOpenSearchMetaIterProgram) : Prop :=
  evalMirFnCfgOpenSearchMetaIterProgram p = p.summary ∧
    lastTwoOpenSearchSummariesAgree (evalMirFnCfgOpenSearchClosedLoopProgram p.closedLoopProg)

def rOpenSearchMetaIterWitness (p : RFnCfgOpenSearchMetaIterProgram) : Prop :=
  evalRFnCfgOpenSearchMetaIterProgram p = p.summary ∧
    lastTwoOpenSearchSummariesAgree (evalRFnCfgOpenSearchClosedLoopProgram p.closedLoopProg)

theorem lowerFnCfgOpenSearchMetaIterProgram_preserves_meta
    (p : SrcFnCfgOpenSearchMetaIterProgram) :
    (lowerFnCfgOpenSearchMetaIterProgram p).closedLoopProg.rounds.length = p.closedLoopProg.rounds.length ∧
      (lowerFnCfgOpenSearchMetaIterProgram p).summary = p.summary := by
  constructor
  · simp [lowerFnCfgOpenSearchMetaIterProgram, lowerFnCfgOpenSearchClosedLoopProgram_preserves_meta]
  · rfl

theorem emitRFnCfgOpenSearchMetaIterProgram_preserves_meta
    (p : MirFnCfgOpenSearchMetaIterProgram) :
    (emitRFnCfgOpenSearchMetaIterProgram p).closedLoopProg.rounds.length = p.closedLoopProg.rounds.length ∧
      (emitRFnCfgOpenSearchMetaIterProgram p).summary = p.summary := by
  constructor
  · simp [emitRFnCfgOpenSearchMetaIterProgram, emitRFnCfgOpenSearchClosedLoopProgram_preserves_meta]
  · rfl

theorem lowerFnCfgOpenSearchMetaIterProgram_preserves_eval
    (p : SrcFnCfgOpenSearchMetaIterProgram) :
    evalMirFnCfgOpenSearchMetaIterProgram (lowerFnCfgOpenSearchMetaIterProgram p) =
      evalSrcFnCfgOpenSearchMetaIterProgram p := by
  simp [evalMirFnCfgOpenSearchMetaIterProgram, evalSrcFnCfgOpenSearchMetaIterProgram,
    lowerFnCfgOpenSearchMetaIterProgram, lowerFnCfgOpenSearchClosedLoopProgram_preserves_eval]

theorem emitRFnCfgOpenSearchMetaIterProgram_preserves_eval
    (p : MirFnCfgOpenSearchMetaIterProgram) :
    evalRFnCfgOpenSearchMetaIterProgram (emitRFnCfgOpenSearchMetaIterProgram p) =
      evalMirFnCfgOpenSearchMetaIterProgram p := by
  simp [evalRFnCfgOpenSearchMetaIterProgram, evalMirFnCfgOpenSearchMetaIterProgram,
    emitRFnCfgOpenSearchMetaIterProgram, emitRFnCfgOpenSearchClosedLoopProgram_preserves_eval]

theorem lowerEmitFnCfgOpenSearchMetaIterProgram_preserves_eval
    (p : SrcFnCfgOpenSearchMetaIterProgram) :
    evalRFnCfgOpenSearchMetaIterProgram
        (emitRFnCfgOpenSearchMetaIterProgram (lowerFnCfgOpenSearchMetaIterProgram p)) =
      evalSrcFnCfgOpenSearchMetaIterProgram p := by
  rw [emitRFnCfgOpenSearchMetaIterProgram_preserves_eval,
    lowerFnCfgOpenSearchMetaIterProgram_preserves_eval]

theorem lowerFnCfgOpenSearchMetaIterProgram_preserves_witness
    (p : SrcFnCfgOpenSearchMetaIterProgram) :
    srcOpenSearchMetaIterWitness p →
      mirOpenSearchMetaIterWitness (lowerFnCfgOpenSearchMetaIterProgram p) := by
  intro h
  rcases h with ⟨hSummary, hStable⟩
  constructor
  · simpa [evalMirFnCfgOpenSearchMetaIterProgram, evalSrcFnCfgOpenSearchMetaIterProgram,
      lowerFnCfgOpenSearchMetaIterProgram, lowerFnCfgOpenSearchClosedLoopProgram_preserves_eval] using hSummary
  · simpa [lowerFnCfgOpenSearchMetaIterProgram, lowerFnCfgOpenSearchClosedLoopProgram_preserves_eval] using hStable

theorem emitRFnCfgOpenSearchMetaIterProgram_preserves_witness
    (p : MirFnCfgOpenSearchMetaIterProgram) :
    mirOpenSearchMetaIterWitness p →
      rOpenSearchMetaIterWitness (emitRFnCfgOpenSearchMetaIterProgram p) := by
  intro h
  rcases h with ⟨hSummary, hStable⟩
  constructor
  · simpa [evalRFnCfgOpenSearchMetaIterProgram, evalMirFnCfgOpenSearchMetaIterProgram,
      emitRFnCfgOpenSearchMetaIterProgram, emitRFnCfgOpenSearchClosedLoopProgram_preserves_eval] using hSummary
  · simpa [emitRFnCfgOpenSearchMetaIterProgram, emitRFnCfgOpenSearchClosedLoopProgram_preserves_eval] using hStable

theorem lowerEmitFnCfgOpenSearchMetaIterProgram_preserves_witness
    (p : SrcFnCfgOpenSearchMetaIterProgram) :
    srcOpenSearchMetaIterWitness p →
      rOpenSearchMetaIterWitness
        (emitRFnCfgOpenSearchMetaIterProgram (lowerFnCfgOpenSearchMetaIterProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchMetaIterProgram_preserves_witness _
    (lowerFnCfgOpenSearchMetaIterProgram_preserves_witness _ h)

def stableFnCfgOpenSearchMetaIterProgram : SrcFnCfgOpenSearchMetaIterProgram :=
  { closedLoopProg := stableFnCfgOpenSearchClosedLoopProgram
  , summary := stableClosedLoopSummary
  }

theorem stableFnCfgOpenSearchMetaIterProgram_meta_preserved :
    (lowerFnCfgOpenSearchMetaIterProgram stableFnCfgOpenSearchMetaIterProgram).closedLoopProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchMetaIterProgram stableFnCfgOpenSearchMetaIterProgram).summary = stableClosedLoopSummary := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchClosedLoopProgram_src_eval :
    evalSrcFnCfgOpenSearchClosedLoopProgram stableFnCfgOpenSearchClosedLoopProgram =
      [stableClosedLoopSummary, stableClosedLoopSummary] := by
  have h := stableFnCfgOpenSearchClosedLoopProgram_eval_preserved
  rw [lowerEmitFnCfgOpenSearchClosedLoopProgram_preserves_eval] at h
  exact h

theorem stableFnCfgOpenSearchMetaIterProgram_src_witness :
    srcOpenSearchMetaIterWitness stableFnCfgOpenSearchMetaIterProgram := by
  constructor
  · simp [srcOpenSearchMetaIterWitness, evalSrcFnCfgOpenSearchMetaIterProgram,
      stableFnCfgOpenSearchMetaIterProgram, lastOpenSearchSummary]
    rw [stableFnCfgOpenSearchClosedLoopProgram_src_eval]
    rfl
  · simp [srcOpenSearchMetaIterWitness, stableFnCfgOpenSearchMetaIterProgram,
      lastTwoOpenSearchSummariesAgree]
    rw [stableFnCfgOpenSearchClosedLoopProgram_src_eval]
    rfl

theorem stableFnCfgOpenSearchMetaIterProgram_eval_preserved :
    evalRFnCfgOpenSearchMetaIterProgram
      (emitRFnCfgOpenSearchMetaIterProgram
        (lowerFnCfgOpenSearchMetaIterProgram stableFnCfgOpenSearchMetaIterProgram)) =
      stableClosedLoopSummary := by
  rw [lowerEmitFnCfgOpenSearchMetaIterProgram_preserves_eval]
  exact stableFnCfgOpenSearchMetaIterProgram_src_witness.1

theorem stableFnCfgOpenSearchMetaIterProgram_preserved :
    rOpenSearchMetaIterWitness
      (emitRFnCfgOpenSearchMetaIterProgram
        (lowerFnCfgOpenSearchMetaIterProgram stableFnCfgOpenSearchMetaIterProgram)) := by
  exact lowerEmitFnCfgOpenSearchMetaIterProgram_preserves_witness _
    stableFnCfgOpenSearchMetaIterProgram_src_witness

end RRProofs
