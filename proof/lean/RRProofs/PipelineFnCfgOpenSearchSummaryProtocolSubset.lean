import RRProofs.PipelineFnCfgOpenSearchMetaIterSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

abbrev OpenSearchSummaryTrace := List PriorityTrace

structure SrcFnCfgOpenSearchSummaryProtocolProgram where
  rounds : List SrcFnCfgOpenSearchMetaIterProgram
  stableSummary : PriorityTrace

structure MirFnCfgOpenSearchSummaryProtocolProgram where
  rounds : List MirFnCfgOpenSearchMetaIterProgram
  stableSummary : PriorityTrace

structure RFnCfgOpenSearchSummaryProtocolProgram where
  rounds : List RFnCfgOpenSearchMetaIterProgram
  stableSummary : PriorityTrace

def lowerFnCfgOpenSearchSummaryProtocolProgram
    (p : SrcFnCfgOpenSearchSummaryProtocolProgram) :
    MirFnCfgOpenSearchSummaryProtocolProgram :=
  { rounds := p.rounds.map lowerFnCfgOpenSearchMetaIterProgram
  , stableSummary := p.stableSummary
  }

def emitRFnCfgOpenSearchSummaryProtocolProgram
    (p : MirFnCfgOpenSearchSummaryProtocolProgram) :
    RFnCfgOpenSearchSummaryProtocolProgram :=
  { rounds := p.rounds.map emitRFnCfgOpenSearchMetaIterProgram
  , stableSummary := p.stableSummary
  }

def evalSrcFnCfgOpenSearchSummaryProtocolProgram
    (p : SrcFnCfgOpenSearchSummaryProtocolProgram) : OpenSearchSummaryTrace :=
  p.rounds.map evalSrcFnCfgOpenSearchMetaIterProgram

def evalMirFnCfgOpenSearchSummaryProtocolProgram
    (p : MirFnCfgOpenSearchSummaryProtocolProgram) : OpenSearchSummaryTrace :=
  p.rounds.map evalMirFnCfgOpenSearchMetaIterProgram

def evalRFnCfgOpenSearchSummaryProtocolProgram
    (p : RFnCfgOpenSearchSummaryProtocolProgram) : OpenSearchSummaryTrace :=
  p.rounds.map evalRFnCfgOpenSearchMetaIterProgram

def lastOpenSearchSummaryTrace : OpenSearchSummaryTrace → PriorityTrace
  | [] => []
  | [summary] => summary
  | _ :: rest => lastOpenSearchSummaryTrace rest

def srcOpenSearchSummaryProtocolWitness (p : SrcFnCfgOpenSearchSummaryProtocolProgram) : Prop :=
  (∀ round, round ∈ p.rounds → srcOpenSearchMetaIterWitness round) ∧
    lastOpenSearchSummaryTrace (evalSrcFnCfgOpenSearchSummaryProtocolProgram p) = p.stableSummary

def mirOpenSearchSummaryProtocolWitness (p : MirFnCfgOpenSearchSummaryProtocolProgram) : Prop :=
  (∀ round, round ∈ p.rounds → mirOpenSearchMetaIterWitness round) ∧
    lastOpenSearchSummaryTrace (evalMirFnCfgOpenSearchSummaryProtocolProgram p) = p.stableSummary

def rOpenSearchSummaryProtocolWitness (p : RFnCfgOpenSearchSummaryProtocolProgram) : Prop :=
  (∀ round, round ∈ p.rounds → rOpenSearchMetaIterWitness round) ∧
    lastOpenSearchSummaryTrace (evalRFnCfgOpenSearchSummaryProtocolProgram p) = p.stableSummary

theorem lowerOpenSearchSummaryProtocolRounds_preserves_eval
    (rounds : List SrcFnCfgOpenSearchMetaIterProgram) :
    rounds.map (fun round => evalMirFnCfgOpenSearchMetaIterProgram
      (lowerFnCfgOpenSearchMetaIterProgram round)) =
      rounds.map evalSrcFnCfgOpenSearchMetaIterProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [lowerFnCfgOpenSearchMetaIterProgram_preserves_eval, ih]

theorem emitROpenSearchSummaryProtocolRounds_preserves_eval
    (rounds : List MirFnCfgOpenSearchMetaIterProgram) :
    rounds.map (fun round => evalRFnCfgOpenSearchMetaIterProgram
      (emitRFnCfgOpenSearchMetaIterProgram round)) =
      rounds.map evalMirFnCfgOpenSearchMetaIterProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [emitRFnCfgOpenSearchMetaIterProgram_preserves_eval, ih]

theorem lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_meta
    (p : SrcFnCfgOpenSearchSummaryProtocolProgram) :
    (lowerFnCfgOpenSearchSummaryProtocolProgram p).rounds.length = p.rounds.length ∧
      (lowerFnCfgOpenSearchSummaryProtocolProgram p).stableSummary = p.stableSummary := by
  constructor
  · simp [lowerFnCfgOpenSearchSummaryProtocolProgram]
  · rfl

theorem emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_meta
    (p : MirFnCfgOpenSearchSummaryProtocolProgram) :
    (emitRFnCfgOpenSearchSummaryProtocolProgram p).rounds.length = p.rounds.length ∧
      (emitRFnCfgOpenSearchSummaryProtocolProgram p).stableSummary = p.stableSummary := by
  constructor
  · simp [emitRFnCfgOpenSearchSummaryProtocolProgram]
  · rfl

theorem lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_eval
    (p : SrcFnCfgOpenSearchSummaryProtocolProgram) :
    evalMirFnCfgOpenSearchSummaryProtocolProgram (lowerFnCfgOpenSearchSummaryProtocolProgram p) =
      evalSrcFnCfgOpenSearchSummaryProtocolProgram p := by
  simpa [evalMirFnCfgOpenSearchSummaryProtocolProgram, evalSrcFnCfgOpenSearchSummaryProtocolProgram,
    lowerFnCfgOpenSearchSummaryProtocolProgram] using
    lowerOpenSearchSummaryProtocolRounds_preserves_eval p.rounds

theorem emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_eval
    (p : MirFnCfgOpenSearchSummaryProtocolProgram) :
    evalRFnCfgOpenSearchSummaryProtocolProgram (emitRFnCfgOpenSearchSummaryProtocolProgram p) =
      evalMirFnCfgOpenSearchSummaryProtocolProgram p := by
  simpa [evalRFnCfgOpenSearchSummaryProtocolProgram, evalMirFnCfgOpenSearchSummaryProtocolProgram,
    emitRFnCfgOpenSearchSummaryProtocolProgram] using
    emitROpenSearchSummaryProtocolRounds_preserves_eval p.rounds

theorem lowerEmitFnCfgOpenSearchSummaryProtocolProgram_preserves_eval
    (p : SrcFnCfgOpenSearchSummaryProtocolProgram) :
    evalRFnCfgOpenSearchSummaryProtocolProgram
        (emitRFnCfgOpenSearchSummaryProtocolProgram
          (lowerFnCfgOpenSearchSummaryProtocolProgram p)) =
      evalSrcFnCfgOpenSearchSummaryProtocolProgram p := by
  rw [emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_eval,
    lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_eval]

theorem lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_witness
    (p : SrcFnCfgOpenSearchSummaryProtocolProgram) :
    srcOpenSearchSummaryProtocolWitness p →
      mirOpenSearchSummaryProtocolWitness (lowerFnCfgOpenSearchSummaryProtocolProgram p) := by
  intro h
  rcases h with ⟨hRounds, hStable⟩
  constructor
  · intro round hmem
    simp [lowerFnCfgOpenSearchSummaryProtocolProgram] at hmem
    rcases hmem with ⟨srcRound, hsrc, rfl⟩
    exact lowerFnCfgOpenSearchMetaIterProgram_preserves_witness srcRound (hRounds srcRound hsrc)
  · exact (congrArg lastOpenSearchSummaryTrace
      (lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_eval p)).trans hStable

theorem emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_witness
    (p : MirFnCfgOpenSearchSummaryProtocolProgram) :
    mirOpenSearchSummaryProtocolWitness p →
      rOpenSearchSummaryProtocolWitness (emitRFnCfgOpenSearchSummaryProtocolProgram p) := by
  intro h
  rcases h with ⟨hRounds, hStable⟩
  constructor
  · intro round hmem
    simp [emitRFnCfgOpenSearchSummaryProtocolProgram] at hmem
    rcases hmem with ⟨mirRound, hmir, rfl⟩
    exact emitRFnCfgOpenSearchMetaIterProgram_preserves_witness mirRound (hRounds mirRound hmir)
  · exact (congrArg lastOpenSearchSummaryTrace
      (emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_eval p)).trans hStable

theorem lowerEmitFnCfgOpenSearchSummaryProtocolProgram_preserves_witness
    (p : SrcFnCfgOpenSearchSummaryProtocolProgram) :
    srcOpenSearchSummaryProtocolWitness p →
      rOpenSearchSummaryProtocolWitness
        (emitRFnCfgOpenSearchSummaryProtocolProgram
          (lowerFnCfgOpenSearchSummaryProtocolProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchSummaryProtocolProgram_preserves_witness _
    (lowerFnCfgOpenSearchSummaryProtocolProgram_preserves_witness _ h)

def stableFnCfgOpenSearchSummaryProtocolProgram : SrcFnCfgOpenSearchSummaryProtocolProgram :=
  { rounds := [stableFnCfgOpenSearchMetaIterProgram, stableFnCfgOpenSearchMetaIterProgram]
  , stableSummary := stableClosedLoopSummary
  }

theorem stableFnCfgOpenSearchSummaryProtocolProgram_meta_preserved :
    (lowerFnCfgOpenSearchSummaryProtocolProgram stableFnCfgOpenSearchSummaryProtocolProgram).rounds.length = 2 ∧
      (lowerFnCfgOpenSearchSummaryProtocolProgram stableFnCfgOpenSearchSummaryProtocolProgram).stableSummary =
        stableClosedLoopSummary := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchSummaryProtocolProgram_src_witness :
    srcOpenSearchSummaryProtocolWitness stableFnCfgOpenSearchSummaryProtocolProgram := by
  constructor
  · intro round hmem
    simp [stableFnCfgOpenSearchSummaryProtocolProgram] at hmem
    rcases hmem with rfl | rfl
    · exact stableFnCfgOpenSearchMetaIterProgram_src_witness
  · simp [stableFnCfgOpenSearchSummaryProtocolProgram,
      evalSrcFnCfgOpenSearchSummaryProtocolProgram, lastOpenSearchSummaryTrace]
    rfl

theorem stableFnCfgOpenSearchSummaryProtocolProgram_eval_preserved :
    evalRFnCfgOpenSearchSummaryProtocolProgram
      (emitRFnCfgOpenSearchSummaryProtocolProgram
        (lowerFnCfgOpenSearchSummaryProtocolProgram stableFnCfgOpenSearchSummaryProtocolProgram)) =
      [stableClosedLoopSummary, stableClosedLoopSummary] := by
  rw [lowerEmitFnCfgOpenSearchSummaryProtocolProgram_preserves_eval]
  rfl

theorem stableFnCfgOpenSearchSummaryProtocolProgram_preserved :
    rOpenSearchSummaryProtocolWitness
      (emitRFnCfgOpenSearchSummaryProtocolProgram
        (lowerFnCfgOpenSearchSummaryProtocolProgram stableFnCfgOpenSearchSummaryProtocolProgram)) := by
  exact lowerEmitFnCfgOpenSearchSummaryProtocolProgram_preserves_witness _
    stableFnCfgOpenSearchSummaryProtocolProgram_src_witness

end RRProofs
