import RRProofs.PipelineFnCfgOpenSearchFrontierSummaryProtocolSubset

namespace RRProofs

structure SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram where
  summaryProg : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram
  haltSummary : PriorityTrace

structure MirFnCfgOpenSearchFrontierConvergenceProtocolProgram where
  summaryProg : MirFnCfgOpenSearchFrontierSummaryProtocolProgram
  haltSummary : PriorityTrace

structure RFnCfgOpenSearchFrontierConvergenceProtocolProgram where
  summaryProg : RFnCfgOpenSearchFrontierSummaryProtocolProgram
  haltSummary : PriorityTrace

def lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram
    (p : SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    MirFnCfgOpenSearchFrontierConvergenceProtocolProgram :=
  { summaryProg := lowerFnCfgOpenSearchFrontierSummaryProtocolProgram p.summaryProg
  , haltSummary := p.haltSummary
  }

def emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram
    (p : MirFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    RFnCfgOpenSearchFrontierConvergenceProtocolProgram :=
  { summaryProg := emitRFnCfgOpenSearchFrontierSummaryProtocolProgram p.summaryProg
  , haltSummary := p.haltSummary
  }

def evalSrcFnCfgOpenSearchFrontierConvergenceProtocolProgram
    (p : SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram) : PriorityTrace :=
  lastOpenSearchFrontierSummaryTrace (evalSrcFnCfgOpenSearchFrontierSummaryProtocolProgram p.summaryProg)

def evalMirFnCfgOpenSearchFrontierConvergenceProtocolProgram
    (p : MirFnCfgOpenSearchFrontierConvergenceProtocolProgram) : PriorityTrace :=
  lastOpenSearchFrontierSummaryTrace (evalMirFnCfgOpenSearchFrontierSummaryProtocolProgram p.summaryProg)

def evalRFnCfgOpenSearchFrontierConvergenceProtocolProgram
    (p : RFnCfgOpenSearchFrontierConvergenceProtocolProgram) : PriorityTrace :=
  lastOpenSearchFrontierSummaryTrace (evalRFnCfgOpenSearchFrontierSummaryProtocolProgram p.summaryProg)

def srcOpenSearchFrontierConvergenceProtocolWitness
    (p : SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram) : Prop :=
  srcOpenSearchFrontierSummaryProtocolWitness p.summaryProg ∧
    evalSrcFnCfgOpenSearchFrontierConvergenceProtocolProgram p = p.haltSummary ∧
    lastTwoOpenSearchFrontierSummariesAgree (evalSrcFnCfgOpenSearchFrontierSummaryProtocolProgram p.summaryProg)

def mirOpenSearchFrontierConvergenceProtocolWitness
    (p : MirFnCfgOpenSearchFrontierConvergenceProtocolProgram) : Prop :=
  mirOpenSearchFrontierSummaryProtocolWitness p.summaryProg ∧
    evalMirFnCfgOpenSearchFrontierConvergenceProtocolProgram p = p.haltSummary ∧
    lastTwoOpenSearchFrontierSummariesAgree (evalMirFnCfgOpenSearchFrontierSummaryProtocolProgram p.summaryProg)

def rOpenSearchFrontierConvergenceProtocolWitness
    (p : RFnCfgOpenSearchFrontierConvergenceProtocolProgram) : Prop :=
  rOpenSearchFrontierSummaryProtocolWitness p.summaryProg ∧
    evalRFnCfgOpenSearchFrontierConvergenceProtocolProgram p = p.haltSummary ∧
    lastTwoOpenSearchFrontierSummariesAgree (evalRFnCfgOpenSearchFrontierSummaryProtocolProgram p.summaryProg)

theorem lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram p).summaryProg.rounds.length = p.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram p).haltSummary = p.haltSummary := by
  constructor
  · simp [lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram,
      lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_meta]
  · rfl

theorem emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    (emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram p).summaryProg.rounds.length = p.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram p).haltSummary = p.haltSummary := by
  constructor
  · simp [emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram,
      emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_meta]
  · rfl

theorem lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    evalMirFnCfgOpenSearchFrontierConvergenceProtocolProgram (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram p) =
      evalSrcFnCfgOpenSearchFrontierConvergenceProtocolProgram p := by
  simp [evalMirFnCfgOpenSearchFrontierConvergenceProtocolProgram,
    evalSrcFnCfgOpenSearchFrontierConvergenceProtocolProgram,
    lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram,
    lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval]

theorem emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    evalRFnCfgOpenSearchFrontierConvergenceProtocolProgram (emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram p) =
      evalMirFnCfgOpenSearchFrontierConvergenceProtocolProgram p := by
  simp [evalRFnCfgOpenSearchFrontierConvergenceProtocolProgram,
    evalMirFnCfgOpenSearchFrontierConvergenceProtocolProgram,
    emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram,
    emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval]

theorem lowerEmitFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    evalRFnCfgOpenSearchFrontierConvergenceProtocolProgram
        (emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram
          (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram p)) =
      evalSrcFnCfgOpenSearchFrontierConvergenceProtocolProgram p := by
  rw [emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_eval,
    lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_eval]

theorem lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    srcOpenSearchFrontierConvergenceProtocolWitness p →
      mirOpenSearchFrontierConvergenceProtocolWitness (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram p) := by
  intro h
  rcases h with ⟨hSummary, hEval, hStable⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_witness _ hSummary
  constructor
  · exact (congrArg lastOpenSearchFrontierSummaryTrace
      (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval p.summaryProg)).trans hEval
  · simpa [lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram,
      lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval] using hStable

theorem emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    mirOpenSearchFrontierConvergenceProtocolWitness p →
      rOpenSearchFrontierConvergenceProtocolWitness (emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram p) := by
  intro h
  rcases h with ⟨hSummary, hEval, hStable⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_witness _ hSummary
  constructor
  · exact (congrArg lastOpenSearchFrontierSummaryTrace
      (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval p.summaryProg)).trans hEval
  · simpa [emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram,
      emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval] using hStable

theorem lowerEmitFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram) :
    srcOpenSearchFrontierConvergenceProtocolWitness p →
      rOpenSearchFrontierConvergenceProtocolWitness
        (emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram
          (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierConvergenceProtocolProgram :
    SrcFnCfgOpenSearchFrontierConvergenceProtocolProgram :=
  { summaryProg := stableFnCfgOpenSearchFrontierSummaryProtocolProgram
  , haltSummary := stableClosedLoopSummary
  }

theorem stableFnCfgOpenSearchFrontierConvergenceProtocolProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram stableFnCfgOpenSearchFrontierConvergenceProtocolProgram).summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram stableFnCfgOpenSearchFrontierConvergenceProtocolProgram).haltSummary =
        stableClosedLoopSummary := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierConvergenceProtocolProgram_src_witness :
    srcOpenSearchFrontierConvergenceProtocolWitness stableFnCfgOpenSearchFrontierConvergenceProtocolProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierSummaryProtocolProgram_src_witness
  constructor
  · simp [stableFnCfgOpenSearchFrontierConvergenceProtocolProgram,
      evalSrcFnCfgOpenSearchFrontierConvergenceProtocolProgram,
      stableFnCfgOpenSearchFrontierSummaryProtocolProgram,
      evalSrcFnCfgOpenSearchFrontierSummaryProtocolProgram,
      lastOpenSearchFrontierSummaryTrace]
    rfl
  · simpa [stableFnCfgOpenSearchFrontierConvergenceProtocolProgram] using
      stableFnCfgOpenSearchFrontierSummaryProtocolProgram_src_witness.2

theorem stableFnCfgOpenSearchFrontierConvergenceProtocolProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierConvergenceProtocolProgram
      (emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram
        (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram stableFnCfgOpenSearchFrontierConvergenceProtocolProgram)) =
      stableClosedLoopSummary := by
  rw [lowerEmitFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_eval]
  exact stableFnCfgOpenSearchFrontierConvergenceProtocolProgram_src_witness.2.1

theorem stableFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserved :
    rOpenSearchFrontierConvergenceProtocolWitness
      (emitRFnCfgOpenSearchFrontierConvergenceProtocolProgram
        (lowerFnCfgOpenSearchFrontierConvergenceProtocolProgram stableFnCfgOpenSearchFrontierConvergenceProtocolProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierConvergenceProtocolProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierConvergenceProtocolProgram_src_witness

end RRProofs
