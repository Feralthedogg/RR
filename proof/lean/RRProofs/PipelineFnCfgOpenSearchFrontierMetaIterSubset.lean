import RRProofs.PipelineFnCfgOpenSearchFrontierClosedLoopSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

def lastOpenSearchFrontierSummary : AdaptiveOpenSearchFrontierTrace → PriorityTrace
  | [] => []
  | [trace] => trace
  | _ :: rest => lastOpenSearchFrontierSummary rest

def lastTwoOpenSearchFrontierSummariesAgree : AdaptiveOpenSearchFrontierTrace → Prop
  | [] => True
  | [_] => True
  | [a, b] => a = b
  | _ :: rest => lastTwoOpenSearchFrontierSummariesAgree rest

structure SrcFnCfgOpenSearchFrontierMetaIterProgram where
  closedLoopProg : SrcFnCfgOpenSearchFrontierClosedLoopProgram
  summary : PriorityTrace

structure MirFnCfgOpenSearchFrontierMetaIterProgram where
  closedLoopProg : MirFnCfgOpenSearchFrontierClosedLoopProgram
  summary : PriorityTrace

structure RFnCfgOpenSearchFrontierMetaIterProgram where
  closedLoopProg : RFnCfgOpenSearchFrontierClosedLoopProgram
  summary : PriorityTrace

def lowerFnCfgOpenSearchFrontierMetaIterProgram
    (p : SrcFnCfgOpenSearchFrontierMetaIterProgram) : MirFnCfgOpenSearchFrontierMetaIterProgram :=
  { closedLoopProg := lowerFnCfgOpenSearchFrontierClosedLoopProgram p.closedLoopProg
  , summary := p.summary
  }

def emitRFnCfgOpenSearchFrontierMetaIterProgram
    (p : MirFnCfgOpenSearchFrontierMetaIterProgram) : RFnCfgOpenSearchFrontierMetaIterProgram :=
  { closedLoopProg := emitRFnCfgOpenSearchFrontierClosedLoopProgram p.closedLoopProg
  , summary := p.summary
  }

def evalSrcFnCfgOpenSearchFrontierMetaIterProgram (p : SrcFnCfgOpenSearchFrontierMetaIterProgram) : PriorityTrace :=
  lastOpenSearchFrontierSummary (evalSrcFnCfgOpenSearchFrontierClosedLoopProgram p.closedLoopProg)

def evalMirFnCfgOpenSearchFrontierMetaIterProgram (p : MirFnCfgOpenSearchFrontierMetaIterProgram) : PriorityTrace :=
  lastOpenSearchFrontierSummary (evalMirFnCfgOpenSearchFrontierClosedLoopProgram p.closedLoopProg)

def evalRFnCfgOpenSearchFrontierMetaIterProgram (p : RFnCfgOpenSearchFrontierMetaIterProgram) : PriorityTrace :=
  lastOpenSearchFrontierSummary (evalRFnCfgOpenSearchFrontierClosedLoopProgram p.closedLoopProg)

def srcOpenSearchFrontierMetaIterWitness (p : SrcFnCfgOpenSearchFrontierMetaIterProgram) : Prop :=
  evalSrcFnCfgOpenSearchFrontierMetaIterProgram p = p.summary ∧
    lastTwoOpenSearchFrontierSummariesAgree (evalSrcFnCfgOpenSearchFrontierClosedLoopProgram p.closedLoopProg)

def mirOpenSearchFrontierMetaIterWitness (p : MirFnCfgOpenSearchFrontierMetaIterProgram) : Prop :=
  evalMirFnCfgOpenSearchFrontierMetaIterProgram p = p.summary ∧
    lastTwoOpenSearchFrontierSummariesAgree (evalMirFnCfgOpenSearchFrontierClosedLoopProgram p.closedLoopProg)

def rOpenSearchFrontierMetaIterWitness (p : RFnCfgOpenSearchFrontierMetaIterProgram) : Prop :=
  evalRFnCfgOpenSearchFrontierMetaIterProgram p = p.summary ∧
    lastTwoOpenSearchFrontierSummariesAgree (evalRFnCfgOpenSearchFrontierClosedLoopProgram p.closedLoopProg)

theorem lowerFnCfgOpenSearchFrontierMetaIterProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierMetaIterProgram) :
    (lowerFnCfgOpenSearchFrontierMetaIterProgram p).closedLoopProg.rounds.length = p.closedLoopProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierMetaIterProgram p).summary = p.summary := by
  constructor
  · simp [lowerFnCfgOpenSearchFrontierMetaIterProgram,
      lowerFnCfgOpenSearchFrontierClosedLoopProgram_preserves_meta]
  · rfl

theorem emitRFnCfgOpenSearchFrontierMetaIterProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierMetaIterProgram) :
    (emitRFnCfgOpenSearchFrontierMetaIterProgram p).closedLoopProg.rounds.length = p.closedLoopProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierMetaIterProgram p).summary = p.summary := by
  constructor
  · simp [emitRFnCfgOpenSearchFrontierMetaIterProgram,
      emitRFnCfgOpenSearchFrontierClosedLoopProgram_preserves_meta]
  · rfl

theorem lowerFnCfgOpenSearchFrontierMetaIterProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierMetaIterProgram) :
    evalMirFnCfgOpenSearchFrontierMetaIterProgram (lowerFnCfgOpenSearchFrontierMetaIterProgram p) =
      evalSrcFnCfgOpenSearchFrontierMetaIterProgram p := by
  simp [evalMirFnCfgOpenSearchFrontierMetaIterProgram,
    evalSrcFnCfgOpenSearchFrontierMetaIterProgram,
    lowerFnCfgOpenSearchFrontierMetaIterProgram,
    lowerFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval]

theorem emitRFnCfgOpenSearchFrontierMetaIterProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierMetaIterProgram) :
    evalRFnCfgOpenSearchFrontierMetaIterProgram (emitRFnCfgOpenSearchFrontierMetaIterProgram p) =
      evalMirFnCfgOpenSearchFrontierMetaIterProgram p := by
  simp [evalRFnCfgOpenSearchFrontierMetaIterProgram,
    evalMirFnCfgOpenSearchFrontierMetaIterProgram,
    emitRFnCfgOpenSearchFrontierMetaIterProgram,
    emitRFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval]

theorem lowerEmitFnCfgOpenSearchFrontierMetaIterProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierMetaIterProgram) :
    evalRFnCfgOpenSearchFrontierMetaIterProgram
        (emitRFnCfgOpenSearchFrontierMetaIterProgram (lowerFnCfgOpenSearchFrontierMetaIterProgram p)) =
      evalSrcFnCfgOpenSearchFrontierMetaIterProgram p := by
  rw [emitRFnCfgOpenSearchFrontierMetaIterProgram_preserves_eval,
    lowerFnCfgOpenSearchFrontierMetaIterProgram_preserves_eval]

theorem lowerFnCfgOpenSearchFrontierMetaIterProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierMetaIterProgram) :
    srcOpenSearchFrontierMetaIterWitness p →
      mirOpenSearchFrontierMetaIterWitness (lowerFnCfgOpenSearchFrontierMetaIterProgram p) := by
  intro h
  rcases h with ⟨hSummary, hStable⟩
  constructor
  · simpa [evalMirFnCfgOpenSearchFrontierMetaIterProgram,
      evalSrcFnCfgOpenSearchFrontierMetaIterProgram,
      lowerFnCfgOpenSearchFrontierMetaIterProgram,
      lowerFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval] using hSummary
  · simpa [lowerFnCfgOpenSearchFrontierMetaIterProgram,
      lowerFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval] using hStable

theorem emitRFnCfgOpenSearchFrontierMetaIterProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierMetaIterProgram) :
    mirOpenSearchFrontierMetaIterWitness p →
      rOpenSearchFrontierMetaIterWitness (emitRFnCfgOpenSearchFrontierMetaIterProgram p) := by
  intro h
  rcases h with ⟨hSummary, hStable⟩
  constructor
  · simpa [evalRFnCfgOpenSearchFrontierMetaIterProgram,
      evalMirFnCfgOpenSearchFrontierMetaIterProgram,
      emitRFnCfgOpenSearchFrontierMetaIterProgram,
      emitRFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval] using hSummary
  · simpa [emitRFnCfgOpenSearchFrontierMetaIterProgram,
      emitRFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval] using hStable

theorem lowerEmitFnCfgOpenSearchFrontierMetaIterProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierMetaIterProgram) :
    srcOpenSearchFrontierMetaIterWitness p →
      rOpenSearchFrontierMetaIterWitness
        (emitRFnCfgOpenSearchFrontierMetaIterProgram (lowerFnCfgOpenSearchFrontierMetaIterProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierMetaIterProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierMetaIterProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierMetaIterProgram : SrcFnCfgOpenSearchFrontierMetaIterProgram :=
  { closedLoopProg := stableFnCfgOpenSearchFrontierClosedLoopProgram
  , summary := stableClosedLoopSummary
  }

theorem stableFnCfgOpenSearchFrontierMetaIterProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierMetaIterProgram stableFnCfgOpenSearchFrontierMetaIterProgram).closedLoopProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierMetaIterProgram stableFnCfgOpenSearchFrontierMetaIterProgram).summary = stableClosedLoopSummary := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierClosedLoopProgram_src_eval :
    evalSrcFnCfgOpenSearchFrontierClosedLoopProgram stableFnCfgOpenSearchFrontierClosedLoopProgram =
      [stableClosedLoopSummary, stableClosedLoopSummary] := by
  have h := stableFnCfgOpenSearchFrontierClosedLoopProgram_eval_preserved
  rw [lowerEmitFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval] at h
  exact h

theorem stableFnCfgOpenSearchFrontierMetaIterProgram_src_witness :
    srcOpenSearchFrontierMetaIterWitness stableFnCfgOpenSearchFrontierMetaIterProgram := by
  constructor
  · simp [srcOpenSearchFrontierMetaIterWitness, evalSrcFnCfgOpenSearchFrontierMetaIterProgram,
      stableFnCfgOpenSearchFrontierMetaIterProgram, lastOpenSearchFrontierSummary]
    rw [stableFnCfgOpenSearchFrontierClosedLoopProgram_src_eval]
    rfl
  · simp [srcOpenSearchFrontierMetaIterWitness, stableFnCfgOpenSearchFrontierMetaIterProgram,
      lastTwoOpenSearchFrontierSummariesAgree]
    rw [stableFnCfgOpenSearchFrontierClosedLoopProgram_src_eval]
    rfl

theorem stableFnCfgOpenSearchFrontierMetaIterProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierMetaIterProgram
      (emitRFnCfgOpenSearchFrontierMetaIterProgram
        (lowerFnCfgOpenSearchFrontierMetaIterProgram stableFnCfgOpenSearchFrontierMetaIterProgram)) =
      stableClosedLoopSummary := by
  rw [lowerEmitFnCfgOpenSearchFrontierMetaIterProgram_preserves_eval]
  exact stableFnCfgOpenSearchFrontierMetaIterProgram_src_witness.1

theorem stableFnCfgOpenSearchFrontierMetaIterProgram_preserved :
    rOpenSearchFrontierMetaIterWitness
      (emitRFnCfgOpenSearchFrontierMetaIterProgram
        (lowerFnCfgOpenSearchFrontierMetaIterProgram stableFnCfgOpenSearchFrontierMetaIterProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierMetaIterProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierMetaIterProgram_src_witness

end RRProofs
