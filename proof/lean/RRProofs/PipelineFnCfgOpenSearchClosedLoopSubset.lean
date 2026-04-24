import RRProofs.PipelineFnCfgOpenSearchAdaptivePolicySubset

set_option linter.unusedSimpArgs false

namespace RRProofs

abbrev AdaptiveOpenSearchTrace := List PriorityTrace

structure SrcFnCfgOpenSearchClosedLoopProgram where
  rounds : List SrcFnCfgOpenSearchAdaptivePolicyProgram

structure MirFnCfgOpenSearchClosedLoopProgram where
  rounds : List MirFnCfgOpenSearchAdaptivePolicyProgram

structure RFnCfgOpenSearchClosedLoopProgram where
  rounds : List RFnCfgOpenSearchAdaptivePolicyProgram

def lowerFnCfgOpenSearchClosedLoopProgram
    (p : SrcFnCfgOpenSearchClosedLoopProgram) : MirFnCfgOpenSearchClosedLoopProgram :=
  { rounds := p.rounds.map lowerFnCfgOpenSearchAdaptivePolicyProgram }

def emitRFnCfgOpenSearchClosedLoopProgram
    (p : MirFnCfgOpenSearchClosedLoopProgram) : RFnCfgOpenSearchClosedLoopProgram :=
  { rounds := p.rounds.map emitRFnCfgOpenSearchAdaptivePolicyProgram }

def evalSrcFnCfgOpenSearchClosedLoopProgram
    (p : SrcFnCfgOpenSearchClosedLoopProgram) : AdaptiveOpenSearchTrace :=
  p.rounds.map evalSrcFnCfgOpenSearchAdaptivePolicyProgram

def evalMirFnCfgOpenSearchClosedLoopProgram
    (p : MirFnCfgOpenSearchClosedLoopProgram) : AdaptiveOpenSearchTrace :=
  p.rounds.map evalMirFnCfgOpenSearchAdaptivePolicyProgram

def evalRFnCfgOpenSearchClosedLoopProgram
    (p : RFnCfgOpenSearchClosedLoopProgram) : AdaptiveOpenSearchTrace :=
  p.rounds.map evalRFnCfgOpenSearchAdaptivePolicyProgram

def srcOpenSearchClosedLoopWitness (p : SrcFnCfgOpenSearchClosedLoopProgram) : Prop :=
  ∀ round, round ∈ p.rounds → srcOpenSearchAdaptivePolicyWitness round

def mirOpenSearchClosedLoopWitness (p : MirFnCfgOpenSearchClosedLoopProgram) : Prop :=
  ∀ round, round ∈ p.rounds → mirOpenSearchAdaptivePolicyWitness round

def rOpenSearchClosedLoopWitness (p : RFnCfgOpenSearchClosedLoopProgram) : Prop :=
  ∀ round, round ∈ p.rounds → rOpenSearchAdaptivePolicyWitness round

theorem lowerOpenSearchClosedLoopRounds_preserves_eval
    (rounds : List SrcFnCfgOpenSearchAdaptivePolicyProgram) :
    rounds.map (fun round => evalMirFnCfgOpenSearchAdaptivePolicyProgram
      (lowerFnCfgOpenSearchAdaptivePolicyProgram round)) =
      rounds.map evalSrcFnCfgOpenSearchAdaptivePolicyProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [lowerFnCfgOpenSearchAdaptivePolicyProgram_preserves_eval, ih]

theorem emitROpenSearchClosedLoopRounds_preserves_eval
    (rounds : List MirFnCfgOpenSearchAdaptivePolicyProgram) :
    rounds.map (fun round => evalRFnCfgOpenSearchAdaptivePolicyProgram
      (emitRFnCfgOpenSearchAdaptivePolicyProgram round)) =
      rounds.map evalMirFnCfgOpenSearchAdaptivePolicyProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [emitRFnCfgOpenSearchAdaptivePolicyProgram_preserves_eval, ih]

theorem lowerFnCfgOpenSearchClosedLoopProgram_preserves_meta
    (p : SrcFnCfgOpenSearchClosedLoopProgram) :
    (lowerFnCfgOpenSearchClosedLoopProgram p).rounds.length = p.rounds.length := by
  simp [lowerFnCfgOpenSearchClosedLoopProgram]

theorem emitRFnCfgOpenSearchClosedLoopProgram_preserves_meta
    (p : MirFnCfgOpenSearchClosedLoopProgram) :
    (emitRFnCfgOpenSearchClosedLoopProgram p).rounds.length = p.rounds.length := by
  simp [emitRFnCfgOpenSearchClosedLoopProgram]

theorem lowerFnCfgOpenSearchClosedLoopProgram_preserves_eval
    (p : SrcFnCfgOpenSearchClosedLoopProgram) :
    evalMirFnCfgOpenSearchClosedLoopProgram (lowerFnCfgOpenSearchClosedLoopProgram p) =
      evalSrcFnCfgOpenSearchClosedLoopProgram p := by
  simpa [evalMirFnCfgOpenSearchClosedLoopProgram, evalSrcFnCfgOpenSearchClosedLoopProgram,
    lowerFnCfgOpenSearchClosedLoopProgram] using
    lowerOpenSearchClosedLoopRounds_preserves_eval p.rounds

theorem emitRFnCfgOpenSearchClosedLoopProgram_preserves_eval
    (p : MirFnCfgOpenSearchClosedLoopProgram) :
    evalRFnCfgOpenSearchClosedLoopProgram (emitRFnCfgOpenSearchClosedLoopProgram p) =
      evalMirFnCfgOpenSearchClosedLoopProgram p := by
  simpa [evalRFnCfgOpenSearchClosedLoopProgram, evalMirFnCfgOpenSearchClosedLoopProgram,
    emitRFnCfgOpenSearchClosedLoopProgram] using
    emitROpenSearchClosedLoopRounds_preserves_eval p.rounds

theorem lowerEmitFnCfgOpenSearchClosedLoopProgram_preserves_eval
    (p : SrcFnCfgOpenSearchClosedLoopProgram) :
    evalRFnCfgOpenSearchClosedLoopProgram
        (emitRFnCfgOpenSearchClosedLoopProgram (lowerFnCfgOpenSearchClosedLoopProgram p)) =
      evalSrcFnCfgOpenSearchClosedLoopProgram p := by
  rw [emitRFnCfgOpenSearchClosedLoopProgram_preserves_eval,
    lowerFnCfgOpenSearchClosedLoopProgram_preserves_eval]

theorem lowerFnCfgOpenSearchClosedLoopProgram_preserves_witness
    (p : SrcFnCfgOpenSearchClosedLoopProgram) :
    srcOpenSearchClosedLoopWitness p →
      mirOpenSearchClosedLoopWitness (lowerFnCfgOpenSearchClosedLoopProgram p) := by
  intro h round hmem
  simp [lowerFnCfgOpenSearchClosedLoopProgram] at hmem
  rcases hmem with ⟨srcRound, hsrc, rfl⟩
  exact lowerFnCfgOpenSearchAdaptivePolicyProgram_preserves_witness srcRound (h srcRound hsrc)

theorem emitRFnCfgOpenSearchClosedLoopProgram_preserves_witness
    (p : MirFnCfgOpenSearchClosedLoopProgram) :
    mirOpenSearchClosedLoopWitness p →
      rOpenSearchClosedLoopWitness (emitRFnCfgOpenSearchClosedLoopProgram p) := by
  intro h round hmem
  simp [emitRFnCfgOpenSearchClosedLoopProgram] at hmem
  rcases hmem with ⟨mirRound, hmir, rfl⟩
  exact emitRFnCfgOpenSearchAdaptivePolicyProgram_preserves_witness mirRound (h mirRound hmir)

theorem lowerEmitFnCfgOpenSearchClosedLoopProgram_preserves_witness
    (p : SrcFnCfgOpenSearchClosedLoopProgram) :
    srcOpenSearchClosedLoopWitness p →
      rOpenSearchClosedLoopWitness
        (emitRFnCfgOpenSearchClosedLoopProgram (lowerFnCfgOpenSearchClosedLoopProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchClosedLoopProgram_preserves_witness _
    (lowerFnCfgOpenSearchClosedLoopProgram_preserves_witness _ h)

def stableFnCfgOpenSearchClosedLoopProgram : SrcFnCfgOpenSearchClosedLoopProgram :=
  { rounds := [stableFnCfgOpenSearchAdaptivePolicyProgram, stableFnCfgOpenSearchAdaptivePolicyProgram] }

theorem stableFnCfgOpenSearchClosedLoopProgram_meta_preserved :
    (lowerFnCfgOpenSearchClosedLoopProgram stableFnCfgOpenSearchClosedLoopProgram).rounds.length = 2 := by
  rfl

theorem stableFnCfgOpenSearchClosedLoopProgram_src_witness :
    srcOpenSearchClosedLoopWitness stableFnCfgOpenSearchClosedLoopProgram := by
  intro round hmem
  simp [stableFnCfgOpenSearchClosedLoopProgram] at hmem
  rcases hmem with rfl | rfl
  · exact stableFnCfgOpenSearchAdaptivePolicyProgram_src_witness

theorem stableFnCfgOpenSearchClosedLoopProgram_eval_preserved :
    evalRFnCfgOpenSearchClosedLoopProgram
      (emitRFnCfgOpenSearchClosedLoopProgram
        (lowerFnCfgOpenSearchClosedLoopProgram stableFnCfgOpenSearchClosedLoopProgram)) =
      [stableClosedLoopSummary, stableClosedLoopSummary] := by
  rw [lowerEmitFnCfgOpenSearchClosedLoopProgram_preserves_eval]
  rfl

theorem stableFnCfgOpenSearchClosedLoopProgram_preserved :
    rOpenSearchClosedLoopWitness
      (emitRFnCfgOpenSearchClosedLoopProgram
        (lowerFnCfgOpenSearchClosedLoopProgram stableFnCfgOpenSearchClosedLoopProgram)) := by
  exact lowerEmitFnCfgOpenSearchClosedLoopProgram_preserves_witness _
    stableFnCfgOpenSearchClosedLoopProgram_src_witness

end RRProofs
