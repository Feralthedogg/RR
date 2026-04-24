import RRProofs.PipelineFnCfgOpenSearchFrontierAdaptivePolicySubset

set_option linter.unusedSimpArgs false

namespace RRProofs

abbrev AdaptiveOpenSearchFrontierTrace := List PriorityTrace

structure SrcFnCfgOpenSearchFrontierClosedLoopProgram where
  rounds : List SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram

structure MirFnCfgOpenSearchFrontierClosedLoopProgram where
  rounds : List MirFnCfgOpenSearchFrontierAdaptivePolicyProgram

structure RFnCfgOpenSearchFrontierClosedLoopProgram where
  rounds : List RFnCfgOpenSearchFrontierAdaptivePolicyProgram

def lowerFnCfgOpenSearchFrontierClosedLoopProgram
    (p : SrcFnCfgOpenSearchFrontierClosedLoopProgram) : MirFnCfgOpenSearchFrontierClosedLoopProgram :=
  { rounds := p.rounds.map lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram }

def emitRFnCfgOpenSearchFrontierClosedLoopProgram
    (p : MirFnCfgOpenSearchFrontierClosedLoopProgram) : RFnCfgOpenSearchFrontierClosedLoopProgram :=
  { rounds := p.rounds.map emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram }

def evalSrcFnCfgOpenSearchFrontierClosedLoopProgram
    (p : SrcFnCfgOpenSearchFrontierClosedLoopProgram) : AdaptiveOpenSearchFrontierTrace :=
  p.rounds.map evalSrcFnCfgOpenSearchFrontierAdaptivePolicyProgram

def evalMirFnCfgOpenSearchFrontierClosedLoopProgram
    (p : MirFnCfgOpenSearchFrontierClosedLoopProgram) : AdaptiveOpenSearchFrontierTrace :=
  p.rounds.map evalMirFnCfgOpenSearchFrontierAdaptivePolicyProgram

def evalRFnCfgOpenSearchFrontierClosedLoopProgram
    (p : RFnCfgOpenSearchFrontierClosedLoopProgram) : AdaptiveOpenSearchFrontierTrace :=
  p.rounds.map evalRFnCfgOpenSearchFrontierAdaptivePolicyProgram

def srcOpenSearchFrontierClosedLoopWitness (p : SrcFnCfgOpenSearchFrontierClosedLoopProgram) : Prop :=
  ∀ round, round ∈ p.rounds → srcOpenSearchFrontierAdaptivePolicyWitness round

def mirOpenSearchFrontierClosedLoopWitness (p : MirFnCfgOpenSearchFrontierClosedLoopProgram) : Prop :=
  ∀ round, round ∈ p.rounds → mirOpenSearchFrontierAdaptivePolicyWitness round

def rOpenSearchFrontierClosedLoopWitness (p : RFnCfgOpenSearchFrontierClosedLoopProgram) : Prop :=
  ∀ round, round ∈ p.rounds → rOpenSearchFrontierAdaptivePolicyWitness round

theorem lowerOpenSearchFrontierClosedLoopRounds_preserves_eval
    (rounds : List SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    rounds.map (fun round => evalMirFnCfgOpenSearchFrontierAdaptivePolicyProgram
      (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram round)) =
      rounds.map evalSrcFnCfgOpenSearchFrontierAdaptivePolicyProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_eval, ih]

theorem emitROpenSearchFrontierClosedLoopRounds_preserves_eval
    (rounds : List MirFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    rounds.map (fun round => evalRFnCfgOpenSearchFrontierAdaptivePolicyProgram
      (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram round)) =
      rounds.map evalMirFnCfgOpenSearchFrontierAdaptivePolicyProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_eval, ih]

theorem lowerFnCfgOpenSearchFrontierClosedLoopProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierClosedLoopProgram) :
    (lowerFnCfgOpenSearchFrontierClosedLoopProgram p).rounds.length = p.rounds.length := by
  simp [lowerFnCfgOpenSearchFrontierClosedLoopProgram]

theorem emitRFnCfgOpenSearchFrontierClosedLoopProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierClosedLoopProgram) :
    (emitRFnCfgOpenSearchFrontierClosedLoopProgram p).rounds.length = p.rounds.length := by
  simp [emitRFnCfgOpenSearchFrontierClosedLoopProgram]

theorem lowerFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierClosedLoopProgram) :
    evalMirFnCfgOpenSearchFrontierClosedLoopProgram (lowerFnCfgOpenSearchFrontierClosedLoopProgram p) =
      evalSrcFnCfgOpenSearchFrontierClosedLoopProgram p := by
  simpa [evalMirFnCfgOpenSearchFrontierClosedLoopProgram, evalSrcFnCfgOpenSearchFrontierClosedLoopProgram,
    lowerFnCfgOpenSearchFrontierClosedLoopProgram] using
    lowerOpenSearchFrontierClosedLoopRounds_preserves_eval p.rounds

theorem emitRFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierClosedLoopProgram) :
    evalRFnCfgOpenSearchFrontierClosedLoopProgram (emitRFnCfgOpenSearchFrontierClosedLoopProgram p) =
      evalMirFnCfgOpenSearchFrontierClosedLoopProgram p := by
  simpa [evalRFnCfgOpenSearchFrontierClosedLoopProgram, evalMirFnCfgOpenSearchFrontierClosedLoopProgram,
    emitRFnCfgOpenSearchFrontierClosedLoopProgram] using
    emitROpenSearchFrontierClosedLoopRounds_preserves_eval p.rounds

theorem lowerEmitFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierClosedLoopProgram) :
    evalRFnCfgOpenSearchFrontierClosedLoopProgram
        (emitRFnCfgOpenSearchFrontierClosedLoopProgram (lowerFnCfgOpenSearchFrontierClosedLoopProgram p)) =
      evalSrcFnCfgOpenSearchFrontierClosedLoopProgram p := by
  rw [emitRFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval,
    lowerFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval]

theorem lowerFnCfgOpenSearchFrontierClosedLoopProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierClosedLoopProgram) :
    srcOpenSearchFrontierClosedLoopWitness p →
      mirOpenSearchFrontierClosedLoopWitness (lowerFnCfgOpenSearchFrontierClosedLoopProgram p) := by
  intro h round hmem
  simp [lowerFnCfgOpenSearchFrontierClosedLoopProgram] at hmem
  rcases hmem with ⟨srcRound, hsrc, rfl⟩
  exact lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_witness srcRound (h srcRound hsrc)

theorem emitRFnCfgOpenSearchFrontierClosedLoopProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierClosedLoopProgram) :
    mirOpenSearchFrontierClosedLoopWitness p →
      rOpenSearchFrontierClosedLoopWitness (emitRFnCfgOpenSearchFrontierClosedLoopProgram p) := by
  intro h round hmem
  simp [emitRFnCfgOpenSearchFrontierClosedLoopProgram] at hmem
  rcases hmem with ⟨mirRound, hmir, rfl⟩
  exact emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_witness mirRound (h mirRound hmir)

theorem lowerEmitFnCfgOpenSearchFrontierClosedLoopProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierClosedLoopProgram) :
    srcOpenSearchFrontierClosedLoopWitness p →
      rOpenSearchFrontierClosedLoopWitness
        (emitRFnCfgOpenSearchFrontierClosedLoopProgram (lowerFnCfgOpenSearchFrontierClosedLoopProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierClosedLoopProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierClosedLoopProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierClosedLoopProgram : SrcFnCfgOpenSearchFrontierClosedLoopProgram :=
  { rounds := [stableFnCfgOpenSearchFrontierAdaptivePolicyProgram, stableFnCfgOpenSearchFrontierAdaptivePolicyProgram] }

theorem stableFnCfgOpenSearchFrontierClosedLoopProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierClosedLoopProgram stableFnCfgOpenSearchFrontierClosedLoopProgram).rounds.length = 2 := by
  rfl

theorem stableFnCfgOpenSearchFrontierClosedLoopProgram_src_witness :
    srcOpenSearchFrontierClosedLoopWitness stableFnCfgOpenSearchFrontierClosedLoopProgram := by
  intro round hmem
  simp [stableFnCfgOpenSearchFrontierClosedLoopProgram] at hmem
  rcases hmem with rfl | rfl
  · exact stableFnCfgOpenSearchFrontierAdaptivePolicyProgram_src_witness

theorem stableFnCfgOpenSearchFrontierClosedLoopProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierClosedLoopProgram
      (emitRFnCfgOpenSearchFrontierClosedLoopProgram
        (lowerFnCfgOpenSearchFrontierClosedLoopProgram stableFnCfgOpenSearchFrontierClosedLoopProgram)) =
      [stableClosedLoopSummary, stableClosedLoopSummary] := by
  rw [lowerEmitFnCfgOpenSearchFrontierClosedLoopProgram_preserves_eval]
  rfl

theorem stableFnCfgOpenSearchFrontierClosedLoopProgram_preserved :
    rOpenSearchFrontierClosedLoopWitness
      (emitRFnCfgOpenSearchFrontierClosedLoopProgram
        (lowerFnCfgOpenSearchFrontierClosedLoopProgram stableFnCfgOpenSearchFrontierClosedLoopProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierClosedLoopProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierClosedLoopProgram_src_witness

end RRProofs
