import RRProofs.PipelineFnCfgOpenSearchFrontierMetaIterSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

abbrev OpenSearchFrontierSummaryTrace := List PriorityTrace

structure SrcFnCfgOpenSearchFrontierSummaryProtocolProgram where
  rounds : List SrcFnCfgOpenSearchFrontierMetaIterProgram
  stableSummary : PriorityTrace

structure MirFnCfgOpenSearchFrontierSummaryProtocolProgram where
  rounds : List MirFnCfgOpenSearchFrontierMetaIterProgram
  stableSummary : PriorityTrace

structure RFnCfgOpenSearchFrontierSummaryProtocolProgram where
  rounds : List RFnCfgOpenSearchFrontierMetaIterProgram
  stableSummary : PriorityTrace

def lowerFnCfgOpenSearchFrontierSummaryProtocolProgram
    (p : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    MirFnCfgOpenSearchFrontierSummaryProtocolProgram :=
  { rounds := p.rounds.map lowerFnCfgOpenSearchFrontierMetaIterProgram
  , stableSummary := p.stableSummary
  }

def emitRFnCfgOpenSearchFrontierSummaryProtocolProgram
    (p : MirFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    RFnCfgOpenSearchFrontierSummaryProtocolProgram :=
  { rounds := p.rounds.map emitRFnCfgOpenSearchFrontierMetaIterProgram
  , stableSummary := p.stableSummary
  }

def evalSrcFnCfgOpenSearchFrontierSummaryProtocolProgram
    (p : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram) : OpenSearchFrontierSummaryTrace :=
  p.rounds.map evalSrcFnCfgOpenSearchFrontierMetaIterProgram

def evalMirFnCfgOpenSearchFrontierSummaryProtocolProgram
    (p : MirFnCfgOpenSearchFrontierSummaryProtocolProgram) : OpenSearchFrontierSummaryTrace :=
  p.rounds.map evalMirFnCfgOpenSearchFrontierMetaIterProgram

def evalRFnCfgOpenSearchFrontierSummaryProtocolProgram
    (p : RFnCfgOpenSearchFrontierSummaryProtocolProgram) : OpenSearchFrontierSummaryTrace :=
  p.rounds.map evalRFnCfgOpenSearchFrontierMetaIterProgram

def lastOpenSearchFrontierSummaryTrace : OpenSearchFrontierSummaryTrace → PriorityTrace
  | [] => []
  | [summary] => summary
  | _ :: rest => lastOpenSearchFrontierSummaryTrace rest

def srcOpenSearchFrontierSummaryProtocolWitness
    (p : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram) : Prop :=
  (∀ round, round ∈ p.rounds → srcOpenSearchFrontierMetaIterWitness round) ∧
    lastOpenSearchFrontierSummaryTrace (evalSrcFnCfgOpenSearchFrontierSummaryProtocolProgram p) =
      p.stableSummary

def mirOpenSearchFrontierSummaryProtocolWitness
    (p : MirFnCfgOpenSearchFrontierSummaryProtocolProgram) : Prop :=
  (∀ round, round ∈ p.rounds → mirOpenSearchFrontierMetaIterWitness round) ∧
    lastOpenSearchFrontierSummaryTrace (evalMirFnCfgOpenSearchFrontierSummaryProtocolProgram p) =
      p.stableSummary

def rOpenSearchFrontierSummaryProtocolWitness
    (p : RFnCfgOpenSearchFrontierSummaryProtocolProgram) : Prop :=
  (∀ round, round ∈ p.rounds → rOpenSearchFrontierMetaIterWitness round) ∧
    lastOpenSearchFrontierSummaryTrace (evalRFnCfgOpenSearchFrontierSummaryProtocolProgram p) =
      p.stableSummary

theorem lowerOpenSearchFrontierSummaryProtocolRounds_preserves_eval
    (rounds : List SrcFnCfgOpenSearchFrontierMetaIterProgram) :
    rounds.map (fun round => evalMirFnCfgOpenSearchFrontierMetaIterProgram
      (lowerFnCfgOpenSearchFrontierMetaIterProgram round)) =
      rounds.map evalSrcFnCfgOpenSearchFrontierMetaIterProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [lowerFnCfgOpenSearchFrontierMetaIterProgram_preserves_eval, ih]

theorem emitROpenSearchFrontierSummaryProtocolRounds_preserves_eval
    (rounds : List MirFnCfgOpenSearchFrontierMetaIterProgram) :
    rounds.map (fun round => evalRFnCfgOpenSearchFrontierMetaIterProgram
      (emitRFnCfgOpenSearchFrontierMetaIterProgram round)) =
      rounds.map evalMirFnCfgOpenSearchFrontierMetaIterProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [emitRFnCfgOpenSearchFrontierMetaIterProgram_preserves_eval, ih]

theorem lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram p).rounds.length = p.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram p).stableSummary = p.stableSummary := by
  constructor
  · simp [lowerFnCfgOpenSearchFrontierSummaryProtocolProgram]
  · rfl

theorem emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram p).rounds.length = p.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram p).stableSummary = p.stableSummary := by
  constructor
  · simp [emitRFnCfgOpenSearchFrontierSummaryProtocolProgram]
  · rfl

theorem lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    evalMirFnCfgOpenSearchFrontierSummaryProtocolProgram
        (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram p) =
      evalSrcFnCfgOpenSearchFrontierSummaryProtocolProgram p := by
  simpa [evalMirFnCfgOpenSearchFrontierSummaryProtocolProgram,
    evalSrcFnCfgOpenSearchFrontierSummaryProtocolProgram,
    lowerFnCfgOpenSearchFrontierSummaryProtocolProgram] using
    lowerOpenSearchFrontierSummaryProtocolRounds_preserves_eval p.rounds

theorem emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    evalRFnCfgOpenSearchFrontierSummaryProtocolProgram
        (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram p) =
      evalMirFnCfgOpenSearchFrontierSummaryProtocolProgram p := by
  simpa [evalRFnCfgOpenSearchFrontierSummaryProtocolProgram,
    evalMirFnCfgOpenSearchFrontierSummaryProtocolProgram,
    emitRFnCfgOpenSearchFrontierSummaryProtocolProgram] using
    emitROpenSearchFrontierSummaryProtocolRounds_preserves_eval p.rounds

theorem lowerEmitFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    evalRFnCfgOpenSearchFrontierSummaryProtocolProgram
        (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram
          (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram p)) =
      evalSrcFnCfgOpenSearchFrontierSummaryProtocolProgram p := by
  rw [emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval,
    lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval]

theorem lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    srcOpenSearchFrontierSummaryProtocolWitness p →
      mirOpenSearchFrontierSummaryProtocolWitness
        (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram p) := by
  intro h
  rcases h with ⟨hRounds, hStable⟩
  constructor
  · intro round hmem
    simp [lowerFnCfgOpenSearchFrontierSummaryProtocolProgram] at hmem
    rcases hmem with ⟨srcRound, hsrc, rfl⟩
    exact lowerFnCfgOpenSearchFrontierMetaIterProgram_preserves_witness srcRound (hRounds srcRound hsrc)
  · exact (congrArg lastOpenSearchFrontierSummaryTrace
      (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval p)).trans hStable

theorem emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    mirOpenSearchFrontierSummaryProtocolWitness p →
      rOpenSearchFrontierSummaryProtocolWitness
        (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram p) := by
  intro h
  rcases h with ⟨hRounds, hStable⟩
  constructor
  · intro round hmem
    simp [emitRFnCfgOpenSearchFrontierSummaryProtocolProgram] at hmem
    rcases hmem with ⟨mirRound, hmir, rfl⟩
    exact emitRFnCfgOpenSearchFrontierMetaIterProgram_preserves_witness mirRound (hRounds mirRound hmir)
  · exact (congrArg lastOpenSearchFrontierSummaryTrace
      (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval p)).trans hStable

theorem lowerEmitFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram) :
    srcOpenSearchFrontierSummaryProtocolWitness p →
      rOpenSearchFrontierSummaryProtocolWitness
        (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram
          (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierSummaryProtocolProgram : SrcFnCfgOpenSearchFrontierSummaryProtocolProgram :=
  { rounds := [stableFnCfgOpenSearchFrontierMetaIterProgram, stableFnCfgOpenSearchFrontierMetaIterProgram]
  , stableSummary := stableClosedLoopSummary
  }

theorem stableFnCfgOpenSearchFrontierSummaryProtocolProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram stableFnCfgOpenSearchFrontierSummaryProtocolProgram).rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram stableFnCfgOpenSearchFrontierSummaryProtocolProgram).stableSummary =
        stableClosedLoopSummary := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierSummaryProtocolProgram_src_witness :
    srcOpenSearchFrontierSummaryProtocolWitness stableFnCfgOpenSearchFrontierSummaryProtocolProgram := by
  constructor
  · intro round hmem
    simp [stableFnCfgOpenSearchFrontierSummaryProtocolProgram] at hmem
    rcases hmem with rfl | rfl
    · exact stableFnCfgOpenSearchFrontierMetaIterProgram_src_witness
  · simp [stableFnCfgOpenSearchFrontierSummaryProtocolProgram,
      evalSrcFnCfgOpenSearchFrontierSummaryProtocolProgram,
      lastOpenSearchFrontierSummaryTrace]
    rfl

theorem stableFnCfgOpenSearchFrontierSummaryProtocolProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierSummaryProtocolProgram
      (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram
        (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram stableFnCfgOpenSearchFrontierSummaryProtocolProgram)) =
      [stableClosedLoopSummary, stableClosedLoopSummary] := by
  rw [lowerEmitFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_eval]
  rfl

theorem stableFnCfgOpenSearchFrontierSummaryProtocolProgram_preserved :
    rOpenSearchFrontierSummaryProtocolWitness
      (emitRFnCfgOpenSearchFrontierSummaryProtocolProgram
        (lowerFnCfgOpenSearchFrontierSummaryProtocolProgram stableFnCfgOpenSearchFrontierSummaryProtocolProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierSummaryProtocolProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierSummaryProtocolProgram_src_witness

end RRProofs
