import RRProofs.PipelineFnCfgSummaryProtocolSubset

namespace RRProofs

structure SrcFnCfgConvergenceProtocolProgram where
  summaryProg : SrcFnCfgSummaryProtocolProgram
  haltSummary : PriorityTrace

structure MirFnCfgConvergenceProtocolProgram where
  summaryProg : MirFnCfgSummaryProtocolProgram
  haltSummary : PriorityTrace

structure RFnCfgConvergenceProtocolProgram where
  summaryProg : RFnCfgSummaryProtocolProgram
  haltSummary : PriorityTrace

def lowerFnCfgConvergenceProtocolProgram (p : SrcFnCfgConvergenceProtocolProgram) :
    MirFnCfgConvergenceProtocolProgram :=
  { summaryProg := lowerFnCfgSummaryProtocolProgram p.summaryProg
  , haltSummary := p.haltSummary
  }

def emitRFnCfgConvergenceProtocolProgram (p : MirFnCfgConvergenceProtocolProgram) :
    RFnCfgConvergenceProtocolProgram :=
  { summaryProg := emitRFnCfgSummaryProtocolProgram p.summaryProg
  , haltSummary := p.haltSummary
  }

def evalSrcFnCfgConvergenceProtocolProgram (p : SrcFnCfgConvergenceProtocolProgram) : PriorityTrace :=
  lastSummaryTrace (evalSrcFnCfgSummaryProtocolProgram p.summaryProg)

def evalMirFnCfgConvergenceProtocolProgram (p : MirFnCfgConvergenceProtocolProgram) : PriorityTrace :=
  lastSummaryTrace (evalMirFnCfgSummaryProtocolProgram p.summaryProg)

def evalRFnCfgConvergenceProtocolProgram (p : RFnCfgConvergenceProtocolProgram) : PriorityTrace :=
  lastSummaryTrace (evalRFnCfgSummaryProtocolProgram p.summaryProg)

def srcConvergenceProtocolWitness (p : SrcFnCfgConvergenceProtocolProgram) : Prop :=
  srcSummaryProtocolWitness p.summaryProg ∧
    evalSrcFnCfgConvergenceProtocolProgram p = p.haltSummary ∧
    lastTwoSummariesAgree (evalSrcFnCfgSummaryProtocolProgram p.summaryProg)

def mirConvergenceProtocolWitness (p : MirFnCfgConvergenceProtocolProgram) : Prop :=
  mirSummaryProtocolWitness p.summaryProg ∧
    evalMirFnCfgConvergenceProtocolProgram p = p.haltSummary ∧
    lastTwoSummariesAgree (evalMirFnCfgSummaryProtocolProgram p.summaryProg)

def rConvergenceProtocolWitness (p : RFnCfgConvergenceProtocolProgram) : Prop :=
  rSummaryProtocolWitness p.summaryProg ∧
    evalRFnCfgConvergenceProtocolProgram p = p.haltSummary ∧
    lastTwoSummariesAgree (evalRFnCfgSummaryProtocolProgram p.summaryProg)

theorem lowerFnCfgConvergenceProtocolProgram_preserves_meta
    (p : SrcFnCfgConvergenceProtocolProgram) :
    (lowerFnCfgConvergenceProtocolProgram p).summaryProg.rounds.length = p.summaryProg.rounds.length ∧
      (lowerFnCfgConvergenceProtocolProgram p).haltSummary = p.haltSummary := by
  constructor
  · simp [lowerFnCfgConvergenceProtocolProgram, lowerFnCfgSummaryProtocolProgram_preserves_meta]
  · rfl

theorem emitRFnCfgConvergenceProtocolProgram_preserves_meta
    (p : MirFnCfgConvergenceProtocolProgram) :
    (emitRFnCfgConvergenceProtocolProgram p).summaryProg.rounds.length = p.summaryProg.rounds.length ∧
      (emitRFnCfgConvergenceProtocolProgram p).haltSummary = p.haltSummary := by
  constructor
  · simp [emitRFnCfgConvergenceProtocolProgram, emitRFnCfgSummaryProtocolProgram_preserves_meta]
  · rfl

theorem lowerFnCfgConvergenceProtocolProgram_preserves_eval
    (p : SrcFnCfgConvergenceProtocolProgram) :
    evalMirFnCfgConvergenceProtocolProgram (lowerFnCfgConvergenceProtocolProgram p) =
      evalSrcFnCfgConvergenceProtocolProgram p := by
  simp [evalMirFnCfgConvergenceProtocolProgram, evalSrcFnCfgConvergenceProtocolProgram,
    lowerFnCfgConvergenceProtocolProgram, lowerFnCfgSummaryProtocolProgram_preserves_eval]

theorem emitRFnCfgConvergenceProtocolProgram_preserves_eval
    (p : MirFnCfgConvergenceProtocolProgram) :
    evalRFnCfgConvergenceProtocolProgram (emitRFnCfgConvergenceProtocolProgram p) =
      evalMirFnCfgConvergenceProtocolProgram p := by
  simp [evalRFnCfgConvergenceProtocolProgram, evalMirFnCfgConvergenceProtocolProgram,
    emitRFnCfgConvergenceProtocolProgram, emitRFnCfgSummaryProtocolProgram_preserves_eval]

theorem lowerEmitFnCfgConvergenceProtocolProgram_preserves_eval
    (p : SrcFnCfgConvergenceProtocolProgram) :
    evalRFnCfgConvergenceProtocolProgram
        (emitRFnCfgConvergenceProtocolProgram (lowerFnCfgConvergenceProtocolProgram p)) =
      evalSrcFnCfgConvergenceProtocolProgram p := by
  rw [emitRFnCfgConvergenceProtocolProgram_preserves_eval,
    lowerFnCfgConvergenceProtocolProgram_preserves_eval]

theorem lowerFnCfgConvergenceProtocolProgram_preserves_witness
    (p : SrcFnCfgConvergenceProtocolProgram) :
    srcConvergenceProtocolWitness p →
      mirConvergenceProtocolWitness (lowerFnCfgConvergenceProtocolProgram p) := by
  intro h
  rcases h with ⟨hSummary, hEval, hStable⟩
  constructor
  · exact lowerFnCfgSummaryProtocolProgram_preserves_witness _ hSummary
  constructor
  · exact (congrArg lastSummaryTrace (lowerFnCfgSummaryProtocolProgram_preserves_eval p.summaryProg)).trans hEval
  · simpa [lowerFnCfgConvergenceProtocolProgram, lowerFnCfgSummaryProtocolProgram_preserves_eval] using hStable

theorem emitRFnCfgConvergenceProtocolProgram_preserves_witness
    (p : MirFnCfgConvergenceProtocolProgram) :
    mirConvergenceProtocolWitness p →
      rConvergenceProtocolWitness (emitRFnCfgConvergenceProtocolProgram p) := by
  intro h
  rcases h with ⟨hSummary, hEval, hStable⟩
  constructor
  · exact emitRFnCfgSummaryProtocolProgram_preserves_witness _ hSummary
  constructor
  · exact (congrArg lastSummaryTrace (emitRFnCfgSummaryProtocolProgram_preserves_eval p.summaryProg)).trans hEval
  · simpa [emitRFnCfgConvergenceProtocolProgram, emitRFnCfgSummaryProtocolProgram_preserves_eval] using hStable

theorem lowerEmitFnCfgConvergenceProtocolProgram_preserves_witness
    (p : SrcFnCfgConvergenceProtocolProgram) :
    srcConvergenceProtocolWitness p →
      rConvergenceProtocolWitness
        (emitRFnCfgConvergenceProtocolProgram (lowerFnCfgConvergenceProtocolProgram p)) := by
  intro h
  exact emitRFnCfgConvergenceProtocolProgram_preserves_witness _
    (lowerFnCfgConvergenceProtocolProgram_preserves_witness _ h)

def stableFnCfgConvergenceProtocolProgram : SrcFnCfgConvergenceProtocolProgram :=
  { summaryProg := stableFnCfgSummaryProtocolProgram
  , haltSummary := stableClosedLoopSummary
  }

theorem stableFnCfgConvergenceProtocolProgram_meta_preserved :
    (lowerFnCfgConvergenceProtocolProgram stableFnCfgConvergenceProtocolProgram).summaryProg.rounds.length = 2 ∧
      (lowerFnCfgConvergenceProtocolProgram stableFnCfgConvergenceProtocolProgram).haltSummary = stableClosedLoopSummary := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgConvergenceProtocolProgram_src_witness :
    srcConvergenceProtocolWitness stableFnCfgConvergenceProtocolProgram := by
  constructor
  · exact stableFnCfgSummaryProtocolProgram_src_witness
  constructor
  · simp [stableFnCfgConvergenceProtocolProgram, evalSrcFnCfgConvergenceProtocolProgram,
      stableFnCfgSummaryProtocolProgram, evalSrcFnCfgSummaryProtocolProgram, lastSummaryTrace]
    rfl
  · simpa [stableFnCfgConvergenceProtocolProgram] using
      stableFnCfgSummaryProtocolProgram_src_witness.2

theorem stableFnCfgConvergenceProtocolProgram_eval_preserved :
    evalRFnCfgConvergenceProtocolProgram
      (emitRFnCfgConvergenceProtocolProgram (lowerFnCfgConvergenceProtocolProgram stableFnCfgConvergenceProtocolProgram)) =
      stableClosedLoopSummary := by
  rw [lowerEmitFnCfgConvergenceProtocolProgram_preserves_eval]
  exact stableFnCfgConvergenceProtocolProgram_src_witness.2.1

theorem stableFnCfgConvergenceProtocolProgram_preserved :
    rConvergenceProtocolWitness
      (emitRFnCfgConvergenceProtocolProgram (lowerFnCfgConvergenceProtocolProgram stableFnCfgConvergenceProtocolProgram)) := by
  exact lowerEmitFnCfgConvergenceProtocolProgram_preserves_witness _
    stableFnCfgConvergenceProtocolProgram_src_witness

end RRProofs
