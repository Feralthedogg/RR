import RRProofs.PipelineFnCfgLoopMetaIterSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

abbrev SummaryTrace := List PriorityTrace

structure SrcFnCfgSummaryProtocolProgram where
  rounds : List SrcFnCfgLoopMetaIterProgram
  stableSummary : PriorityTrace

structure MirFnCfgSummaryProtocolProgram where
  rounds : List MirFnCfgLoopMetaIterProgram
  stableSummary : PriorityTrace

structure RFnCfgSummaryProtocolProgram where
  rounds : List RFnCfgLoopMetaIterProgram
  stableSummary : PriorityTrace

def lowerFnCfgSummaryProtocolProgram (p : SrcFnCfgSummaryProtocolProgram) :
    MirFnCfgSummaryProtocolProgram :=
  { rounds := p.rounds.map lowerFnCfgLoopMetaIterProgram
  , stableSummary := p.stableSummary
  }

def emitRFnCfgSummaryProtocolProgram (p : MirFnCfgSummaryProtocolProgram) :
    RFnCfgSummaryProtocolProgram :=
  { rounds := p.rounds.map emitRFnCfgLoopMetaIterProgram
  , stableSummary := p.stableSummary
  }

def evalSrcFnCfgSummaryProtocolProgram (p : SrcFnCfgSummaryProtocolProgram) : SummaryTrace :=
  p.rounds.map evalSrcFnCfgLoopMetaIterProgram

def evalMirFnCfgSummaryProtocolProgram (p : MirFnCfgSummaryProtocolProgram) : SummaryTrace :=
  p.rounds.map evalMirFnCfgLoopMetaIterProgram

def evalRFnCfgSummaryProtocolProgram (p : RFnCfgSummaryProtocolProgram) : SummaryTrace :=
  p.rounds.map evalRFnCfgLoopMetaIterProgram

def lastSummaryTrace : SummaryTrace → PriorityTrace
  | [] => []
  | [summary] => summary
  | _ :: rest => lastSummaryTrace rest

def srcSummaryProtocolWitness (p : SrcFnCfgSummaryProtocolProgram) : Prop :=
  (∀ round, round ∈ p.rounds → srcLoopMetaIterWitness round) ∧
    lastSummaryTrace (evalSrcFnCfgSummaryProtocolProgram p) = p.stableSummary

def mirSummaryProtocolWitness (p : MirFnCfgSummaryProtocolProgram) : Prop :=
  (∀ round, round ∈ p.rounds → mirLoopMetaIterWitness round) ∧
    lastSummaryTrace (evalMirFnCfgSummaryProtocolProgram p) = p.stableSummary

def rSummaryProtocolWitness (p : RFnCfgSummaryProtocolProgram) : Prop :=
  (∀ round, round ∈ p.rounds → rLoopMetaIterWitness round) ∧
    lastSummaryTrace (evalRFnCfgSummaryProtocolProgram p) = p.stableSummary

theorem lowerSummaryProtocolRounds_preserves_eval
    (rounds : List SrcFnCfgLoopMetaIterProgram) :
    rounds.map (fun round => evalMirFnCfgLoopMetaIterProgram (lowerFnCfgLoopMetaIterProgram round)) =
      rounds.map evalSrcFnCfgLoopMetaIterProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [lowerFnCfgLoopMetaIterProgram_preserves_eval, ih]

theorem emitRSummaryProtocolRounds_preserves_eval
    (rounds : List MirFnCfgLoopMetaIterProgram) :
    rounds.map (fun round => evalRFnCfgLoopMetaIterProgram (emitRFnCfgLoopMetaIterProgram round)) =
      rounds.map evalMirFnCfgLoopMetaIterProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [emitRFnCfgLoopMetaIterProgram_preserves_eval, ih]

theorem lowerFnCfgSummaryProtocolProgram_preserves_meta
    (p : SrcFnCfgSummaryProtocolProgram) :
    (lowerFnCfgSummaryProtocolProgram p).rounds.length = p.rounds.length ∧
      (lowerFnCfgSummaryProtocolProgram p).stableSummary = p.stableSummary := by
  constructor
  · simp [lowerFnCfgSummaryProtocolProgram]
  · rfl

theorem emitRFnCfgSummaryProtocolProgram_preserves_meta
    (p : MirFnCfgSummaryProtocolProgram) :
    (emitRFnCfgSummaryProtocolProgram p).rounds.length = p.rounds.length ∧
      (emitRFnCfgSummaryProtocolProgram p).stableSummary = p.stableSummary := by
  constructor
  · simp [emitRFnCfgSummaryProtocolProgram]
  · rfl

theorem lowerFnCfgSummaryProtocolProgram_preserves_eval
    (p : SrcFnCfgSummaryProtocolProgram) :
    evalMirFnCfgSummaryProtocolProgram (lowerFnCfgSummaryProtocolProgram p) =
      evalSrcFnCfgSummaryProtocolProgram p := by
  simpa [evalMirFnCfgSummaryProtocolProgram, evalSrcFnCfgSummaryProtocolProgram,
    lowerFnCfgSummaryProtocolProgram] using
    lowerSummaryProtocolRounds_preserves_eval p.rounds

theorem emitRFnCfgSummaryProtocolProgram_preserves_eval
    (p : MirFnCfgSummaryProtocolProgram) :
    evalRFnCfgSummaryProtocolProgram (emitRFnCfgSummaryProtocolProgram p) =
      evalMirFnCfgSummaryProtocolProgram p := by
  simpa [evalRFnCfgSummaryProtocolProgram, evalMirFnCfgSummaryProtocolProgram,
    emitRFnCfgSummaryProtocolProgram] using
    emitRSummaryProtocolRounds_preserves_eval p.rounds

theorem lowerEmitFnCfgSummaryProtocolProgram_preserves_eval
    (p : SrcFnCfgSummaryProtocolProgram) :
    evalRFnCfgSummaryProtocolProgram (emitRFnCfgSummaryProtocolProgram (lowerFnCfgSummaryProtocolProgram p)) =
      evalSrcFnCfgSummaryProtocolProgram p := by
  rw [emitRFnCfgSummaryProtocolProgram_preserves_eval, lowerFnCfgSummaryProtocolProgram_preserves_eval]

theorem lowerFnCfgSummaryProtocolProgram_preserves_witness
    (p : SrcFnCfgSummaryProtocolProgram) :
    srcSummaryProtocolWitness p →
      mirSummaryProtocolWitness (lowerFnCfgSummaryProtocolProgram p) := by
  intro h
  rcases h with ⟨hRounds, hStable⟩
  constructor
  · intro round hmem
    simp [lowerFnCfgSummaryProtocolProgram] at hmem
    rcases hmem with ⟨srcRound, hsrc, rfl⟩
    exact lowerFnCfgLoopMetaIterProgram_preserves_witness srcRound (hRounds srcRound hsrc)
  · exact (congrArg lastSummaryTrace (lowerFnCfgSummaryProtocolProgram_preserves_eval p)).trans hStable

theorem emitRFnCfgSummaryProtocolProgram_preserves_witness
    (p : MirFnCfgSummaryProtocolProgram) :
    mirSummaryProtocolWitness p →
      rSummaryProtocolWitness (emitRFnCfgSummaryProtocolProgram p) := by
  intro h
  rcases h with ⟨hRounds, hStable⟩
  constructor
  · intro round hmem
    simp [emitRFnCfgSummaryProtocolProgram] at hmem
    rcases hmem with ⟨mirRound, hmir, rfl⟩
    exact emitRFnCfgLoopMetaIterProgram_preserves_witness mirRound (hRounds mirRound hmir)
  · exact (congrArg lastSummaryTrace (emitRFnCfgSummaryProtocolProgram_preserves_eval p)).trans hStable

theorem lowerEmitFnCfgSummaryProtocolProgram_preserves_witness
    (p : SrcFnCfgSummaryProtocolProgram) :
    srcSummaryProtocolWitness p →
      rSummaryProtocolWitness (emitRFnCfgSummaryProtocolProgram (lowerFnCfgSummaryProtocolProgram p)) := by
  intro h
  exact emitRFnCfgSummaryProtocolProgram_preserves_witness _
    (lowerFnCfgSummaryProtocolProgram_preserves_witness _ h)

def stableFnCfgSummaryProtocolProgram : SrcFnCfgSummaryProtocolProgram :=
  { rounds := [stableFnCfgLoopMetaIterProgram, stableFnCfgLoopMetaIterProgram]
  , stableSummary := stableClosedLoopSummary
  }

theorem stableFnCfgSummaryProtocolProgram_meta_preserved :
    (lowerFnCfgSummaryProtocolProgram stableFnCfgSummaryProtocolProgram).rounds.length = 2 ∧
      (lowerFnCfgSummaryProtocolProgram stableFnCfgSummaryProtocolProgram).stableSummary = stableClosedLoopSummary := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgSummaryProtocolProgram_src_witness :
    srcSummaryProtocolWitness stableFnCfgSummaryProtocolProgram := by
  constructor
  · intro round hmem
    simp [stableFnCfgSummaryProtocolProgram] at hmem
    rcases hmem with rfl | rfl
    · exact stableFnCfgLoopMetaIterProgram_src_witness
  · simp [stableFnCfgSummaryProtocolProgram, evalSrcFnCfgSummaryProtocolProgram, lastSummaryTrace]
    rfl

theorem stableFnCfgSummaryProtocolProgram_eval_preserved :
    evalRFnCfgSummaryProtocolProgram
      (emitRFnCfgSummaryProtocolProgram (lowerFnCfgSummaryProtocolProgram stableFnCfgSummaryProtocolProgram)) =
      [stableClosedLoopSummary, stableClosedLoopSummary] := by
  rw [lowerEmitFnCfgSummaryProtocolProgram_preserves_eval]
  rfl

theorem stableFnCfgSummaryProtocolProgram_preserved :
    rSummaryProtocolWitness
      (emitRFnCfgSummaryProtocolProgram (lowerFnCfgSummaryProtocolProgram stableFnCfgSummaryProtocolProgram)) := by
  exact lowerEmitFnCfgSummaryProtocolProgram_preserves_witness _
    stableFnCfgSummaryProtocolProgram_src_witness

end RRProofs
