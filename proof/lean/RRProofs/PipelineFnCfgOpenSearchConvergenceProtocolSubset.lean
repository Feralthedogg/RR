import RRProofs.PipelineFnCfgOpenSearchSummaryProtocolSubset

namespace RRProofs

structure SrcFnCfgOpenSearchConvergenceProtocolProgram where
  summaryProg : SrcFnCfgOpenSearchSummaryProtocolProgram
  haltSummary : PriorityTrace

structure MirFnCfgOpenSearchConvergenceProtocolProgram where
  summaryProg : MirFnCfgOpenSearchSummaryProtocolProgram
  haltSummary : PriorityTrace

structure RFnCfgOpenSearchConvergenceProtocolProgram where
  summaryProg : RFnCfgOpenSearchSummaryProtocolProgram
  haltSummary : PriorityTrace

def lowerFnCfgOpenSearchConvergenceProtocolProgram
    (p : SrcFnCfgOpenSearchConvergenceProtocolProgram) :
    MirFnCfgOpenSearchConvergenceProtocolProgram :=
  { summaryProg := lowerFnCfgOpenSearchSummaryProtocolProgram p.summaryProg
  , haltSummary := p.haltSummary
  }

def emitRFnCfgOpenSearchConvergenceProtocolProgram
    (p : MirFnCfgOpenSearchConvergenceProtocolProgram) :
    RFnCfgOpenSearchConvergenceProtocolProgram :=
  { summaryProg := emitRFnCfgOpenSearchSummaryProtocolProgram p.summaryProg
  , haltSummary := p.haltSummary
  }

def evalSrcFnCfgOpenSearchConvergenceProtocolProgram
    (p : SrcFnCfgOpenSearchConvergenceProtocolProgram) : PriorityTrace :=
  lastOpenSearchSummaryTrace (evalSrcFnCfgOpenSearchSummaryProtocolProgram p.summaryProg)

def evalMirFnCfgOpenSearchConvergenceProtocolProgram
    (p : MirFnCfgOpenSearchConvergenceProtocolProgram) : PriorityTrace :=
  lastOpenSearchSummaryTrace (evalMirFnCfgOpenSearchSummaryProtocolProgram p.summaryProg)

def evalRFnCfgOpenSearchConvergenceProtocolProgram
    (p : RFnCfgOpenSearchConvergenceProtocolProgram) : PriorityTrace :=
  lastOpenSearchSummaryTrace (evalRFnCfgOpenSearchSummaryProtocolProgram p.summaryProg)

def srcOpenSearchConvergenceProtocolWitness (p : SrcFnCfgOpenSearchConvergenceProtocolProgram) : Prop :=
  srcOpenSearchSummaryProtocolWitness p.summaryProg ∧
    evalSrcFnCfgOpenSearchConvergenceProtocolProgram p = p.haltSummary ∧
    lastTwoOpenSearchSummariesAgree (evalSrcFnCfgOpenSearchSummaryProtocolProgram p.summaryProg)

def mirOpenSearchConvergenceProtocolWitness (p : MirFnCfgOpenSearchConvergenceProtocolProgram) : Prop :=
  mirOpenSearchSummaryProtocolWitness p.summaryProg ∧
    evalMirFnCfgOpenSearchConvergenceProtocolProgram p = p.haltSummary ∧
    lastTwoOpenSearchSummariesAgree (evalMirFnCfgOpenSearchSummaryProtocolProgram p.summaryProg)

def rOpenSearchConvergenceProtocolWitness (p : RFnCfgOpenSearchConvergenceProtocolProgram) : Prop :=
  rOpenSearchSummaryProtocolWitness p.summaryProg ∧
    evalRFnCfgOpenSearchConvergenceProtocolProgram p = p.haltSummary ∧
    lastTwoOpenSearchSummariesAgree (evalRFnCfgOpenSearchSummaryProtocolProgram p.summaryProg)

theorem lowerFnCfgOpenSearchConvergenceProtocolProgram_preserves_meta
    (p : SrcFnCfgOpenSearchConvergenceProtocolProgram) :
    (lowerFnCfgOpenSearchConvergenceProtocolProgram p).summaryProg.rounds.length = p.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchConvergenceProtocolProgram p).haltSummary = p.haltSummary := by
  constructor
  · simp [lowerFnCfgOpenSearchConvergenceProtocolProgram,
      lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_meta]
  · rfl

theorem emitRFnCfgOpenSearchConvergenceProtocolProgram_preserves_meta
    (p : MirFnCfgOpenSearchConvergenceProtocolProgram) :
    (emitRFnCfgOpenSearchConvergenceProtocolProgram p).summaryProg.rounds.length = p.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchConvergenceProtocolProgram p).haltSummary = p.haltSummary := by
  constructor
  · simp [emitRFnCfgOpenSearchConvergenceProtocolProgram,
      emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_meta]
  · rfl

theorem lowerFnCfgOpenSearchConvergenceProtocolProgram_preserves_eval
    (p : SrcFnCfgOpenSearchConvergenceProtocolProgram) :
    evalMirFnCfgOpenSearchConvergenceProtocolProgram (lowerFnCfgOpenSearchConvergenceProtocolProgram p) =
      evalSrcFnCfgOpenSearchConvergenceProtocolProgram p := by
  simp [evalMirFnCfgOpenSearchConvergenceProtocolProgram,
    evalSrcFnCfgOpenSearchConvergenceProtocolProgram,
    lowerFnCfgOpenSearchConvergenceProtocolProgram,
    lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_eval]

theorem emitRFnCfgOpenSearchConvergenceProtocolProgram_preserves_eval
    (p : MirFnCfgOpenSearchConvergenceProtocolProgram) :
    evalRFnCfgOpenSearchConvergenceProtocolProgram (emitRFnCfgOpenSearchConvergenceProtocolProgram p) =
      evalMirFnCfgOpenSearchConvergenceProtocolProgram p := by
  simp [evalRFnCfgOpenSearchConvergenceProtocolProgram,
    evalMirFnCfgOpenSearchConvergenceProtocolProgram,
    emitRFnCfgOpenSearchConvergenceProtocolProgram,
    emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_eval]

theorem lowerEmitFnCfgOpenSearchConvergenceProtocolProgram_preserves_eval
    (p : SrcFnCfgOpenSearchConvergenceProtocolProgram) :
    evalRFnCfgOpenSearchConvergenceProtocolProgram
        (emitRFnCfgOpenSearchConvergenceProtocolProgram
          (lowerFnCfgOpenSearchConvergenceProtocolProgram p)) =
      evalSrcFnCfgOpenSearchConvergenceProtocolProgram p := by
  rw [emitRFnCfgOpenSearchConvergenceProtocolProgram_preserves_eval,
    lowerFnCfgOpenSearchConvergenceProtocolProgram_preserves_eval]

theorem lowerFnCfgOpenSearchConvergenceProtocolProgram_preserves_witness
    (p : SrcFnCfgOpenSearchConvergenceProtocolProgram) :
    srcOpenSearchConvergenceProtocolWitness p →
      mirOpenSearchConvergenceProtocolWitness (lowerFnCfgOpenSearchConvergenceProtocolProgram p) := by
  intro h
  rcases h with ⟨hSummary, hEval, hStable⟩
  constructor
  · exact lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_witness _ hSummary
  constructor
  · exact (congrArg lastOpenSearchSummaryTrace
      (lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_eval p.summaryProg)).trans hEval
  · simpa [lowerFnCfgOpenSearchConvergenceProtocolProgram,
      lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_eval] using hStable

theorem emitRFnCfgOpenSearchConvergenceProtocolProgram_preserves_witness
    (p : MirFnCfgOpenSearchConvergenceProtocolProgram) :
    mirOpenSearchConvergenceProtocolWitness p →
      rOpenSearchConvergenceProtocolWitness (emitRFnCfgOpenSearchConvergenceProtocolProgram p) := by
  intro h
  rcases h with ⟨hSummary, hEval, hStable⟩
  constructor
  · exact emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_witness _ hSummary
  constructor
  · exact (congrArg lastOpenSearchSummaryTrace
      (emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_eval p.summaryProg)).trans hEval
  · simpa [emitRFnCfgOpenSearchConvergenceProtocolProgram,
      emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_eval] using hStable

theorem lowerEmitFnCfgOpenSearchConvergenceProtocolProgram_preserves_witness
    (p : SrcFnCfgOpenSearchConvergenceProtocolProgram) :
    srcOpenSearchConvergenceProtocolWitness p →
      rOpenSearchConvergenceProtocolWitness
        (emitRFnCfgOpenSearchConvergenceProtocolProgram
          (lowerFnCfgOpenSearchConvergenceProtocolProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchConvergenceProtocolProgram_preserves_witness _
    (lowerFnCfgOpenSearchConvergenceProtocolProgram_preserves_witness _ h)

def stableFnCfgOpenSearchConvergenceProtocolProgram : SrcFnCfgOpenSearchConvergenceProtocolProgram :=
  { summaryProg := stableFnCfgOpenSearchSummaryProtocolProgram
  , haltSummary := stableClosedLoopSummary
  }

theorem stableFnCfgOpenSearchConvergenceProtocolProgram_meta_preserved :
    (lowerFnCfgOpenSearchConvergenceProtocolProgram stableFnCfgOpenSearchConvergenceProtocolProgram).summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchConvergenceProtocolProgram stableFnCfgOpenSearchConvergenceProtocolProgram).haltSummary =
        stableClosedLoopSummary := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchConvergenceProtocolProgram_src_witness :
    srcOpenSearchConvergenceProtocolWitness stableFnCfgOpenSearchConvergenceProtocolProgram := by
  constructor
  · exact stableFnCfgOpenSearchSummaryProtocolProgram_src_witness
  constructor
  · simp [stableFnCfgOpenSearchConvergenceProtocolProgram,
      evalSrcFnCfgOpenSearchConvergenceProtocolProgram,
      stableFnCfgOpenSearchSummaryProtocolProgram,
      evalSrcFnCfgOpenSearchSummaryProtocolProgram,
      lastOpenSearchSummaryTrace]
    rfl
  · simpa [stableFnCfgOpenSearchConvergenceProtocolProgram] using
      stableFnCfgOpenSearchSummaryProtocolProgram_src_witness.2

theorem stableFnCfgOpenSearchConvergenceProtocolProgram_eval_preserved :
    evalRFnCfgOpenSearchConvergenceProtocolProgram
      (emitRFnCfgOpenSearchConvergenceProtocolProgram
        (lowerFnCfgOpenSearchConvergenceProtocolProgram stableFnCfgOpenSearchConvergenceProtocolProgram)) =
      stableClosedLoopSummary := by
  rw [lowerEmitFnCfgOpenSearchConvergenceProtocolProgram_preserves_eval]
  exact stableFnCfgOpenSearchConvergenceProtocolProgram_src_witness.2.1

theorem stableFnCfgOpenSearchConvergenceProtocolProgram_preserved :
    rOpenSearchConvergenceProtocolWitness
      (emitRFnCfgOpenSearchConvergenceProtocolProgram
        (lowerFnCfgOpenSearchConvergenceProtocolProgram stableFnCfgOpenSearchConvergenceProtocolProgram)) := by
  exact lowerEmitFnCfgOpenSearchConvergenceProtocolProgram_preserves_witness _
    stableFnCfgOpenSearchConvergenceProtocolProgram_src_witness

end RRProofs
