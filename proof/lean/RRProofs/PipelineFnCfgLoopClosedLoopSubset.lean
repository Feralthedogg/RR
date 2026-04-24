import RRProofs.PipelineFnCfgLoopAdaptivePolicySubset

set_option linter.unusedSimpArgs false

namespace RRProofs

abbrev AdaptivePriorityTrace := List PriorityTrace

structure SrcFnCfgLoopClosedLoopProgram where
  rounds : List SrcFnCfgLoopAdaptivePolicyProgram

structure MirFnCfgLoopClosedLoopProgram where
  rounds : List MirFnCfgLoopAdaptivePolicyProgram

structure RFnCfgLoopClosedLoopProgram where
  rounds : List RFnCfgLoopAdaptivePolicyProgram

def lowerFnCfgLoopClosedLoopProgram (p : SrcFnCfgLoopClosedLoopProgram) :
    MirFnCfgLoopClosedLoopProgram :=
  { rounds := p.rounds.map lowerFnCfgLoopAdaptivePolicyProgram }

def emitRFnCfgLoopClosedLoopProgram (p : MirFnCfgLoopClosedLoopProgram) :
    RFnCfgLoopClosedLoopProgram :=
  { rounds := p.rounds.map emitRFnCfgLoopAdaptivePolicyProgram }

def evalSrcFnCfgLoopClosedLoopProgram (p : SrcFnCfgLoopClosedLoopProgram) : AdaptivePriorityTrace :=
  p.rounds.map evalSrcFnCfgLoopAdaptivePolicyProgram

def evalMirFnCfgLoopClosedLoopProgram (p : MirFnCfgLoopClosedLoopProgram) : AdaptivePriorityTrace :=
  p.rounds.map evalMirFnCfgLoopAdaptivePolicyProgram

def evalRFnCfgLoopClosedLoopProgram (p : RFnCfgLoopClosedLoopProgram) : AdaptivePriorityTrace :=
  p.rounds.map evalRFnCfgLoopAdaptivePolicyProgram

def srcLoopClosedLoopWitness (p : SrcFnCfgLoopClosedLoopProgram) : Prop :=
  ∀ round, round ∈ p.rounds → srcLoopAdaptivePolicyWitness round

def mirLoopClosedLoopWitness (p : MirFnCfgLoopClosedLoopProgram) : Prop :=
  ∀ round, round ∈ p.rounds → mirLoopAdaptivePolicyWitness round

def rLoopClosedLoopWitness (p : RFnCfgLoopClosedLoopProgram) : Prop :=
  ∀ round, round ∈ p.rounds → rLoopAdaptivePolicyWitness round

theorem lowerClosedLoopRounds_preserves_eval
    (rounds : List SrcFnCfgLoopAdaptivePolicyProgram) :
    rounds.map (fun round => evalMirFnCfgLoopAdaptivePolicyProgram (lowerFnCfgLoopAdaptivePolicyProgram round)) =
      rounds.map evalSrcFnCfgLoopAdaptivePolicyProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [lowerFnCfgLoopAdaptivePolicyProgram_preserves_eval, ih]

theorem emitRClosedLoopRounds_preserves_eval
    (rounds : List MirFnCfgLoopAdaptivePolicyProgram) :
    rounds.map (fun round => evalRFnCfgLoopAdaptivePolicyProgram (emitRFnCfgLoopAdaptivePolicyProgram round)) =
      rounds.map evalMirFnCfgLoopAdaptivePolicyProgram := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [emitRFnCfgLoopAdaptivePolicyProgram_preserves_eval, ih]

theorem lowerFnCfgLoopClosedLoopProgram_preserves_meta
    (p : SrcFnCfgLoopClosedLoopProgram) :
    (lowerFnCfgLoopClosedLoopProgram p).rounds.length = p.rounds.length := by
  simp [lowerFnCfgLoopClosedLoopProgram]

theorem emitRFnCfgLoopClosedLoopProgram_preserves_meta
    (p : MirFnCfgLoopClosedLoopProgram) :
    (emitRFnCfgLoopClosedLoopProgram p).rounds.length = p.rounds.length := by
  simp [emitRFnCfgLoopClosedLoopProgram]

theorem lowerFnCfgLoopClosedLoopProgram_preserves_eval
    (p : SrcFnCfgLoopClosedLoopProgram) :
    evalMirFnCfgLoopClosedLoopProgram (lowerFnCfgLoopClosedLoopProgram p) =
      evalSrcFnCfgLoopClosedLoopProgram p := by
  simpa [evalMirFnCfgLoopClosedLoopProgram, evalSrcFnCfgLoopClosedLoopProgram,
    lowerFnCfgLoopClosedLoopProgram] using
    lowerClosedLoopRounds_preserves_eval p.rounds

theorem emitRFnCfgLoopClosedLoopProgram_preserves_eval
    (p : MirFnCfgLoopClosedLoopProgram) :
    evalRFnCfgLoopClosedLoopProgram (emitRFnCfgLoopClosedLoopProgram p) =
      evalMirFnCfgLoopClosedLoopProgram p := by
  simpa [evalRFnCfgLoopClosedLoopProgram, evalMirFnCfgLoopClosedLoopProgram,
    emitRFnCfgLoopClosedLoopProgram] using
    emitRClosedLoopRounds_preserves_eval p.rounds

theorem lowerEmitFnCfgLoopClosedLoopProgram_preserves_eval
    (p : SrcFnCfgLoopClosedLoopProgram) :
    evalRFnCfgLoopClosedLoopProgram (emitRFnCfgLoopClosedLoopProgram (lowerFnCfgLoopClosedLoopProgram p)) =
      evalSrcFnCfgLoopClosedLoopProgram p := by
  rw [emitRFnCfgLoopClosedLoopProgram_preserves_eval, lowerFnCfgLoopClosedLoopProgram_preserves_eval]

theorem lowerFnCfgLoopClosedLoopProgram_preserves_witness
    (p : SrcFnCfgLoopClosedLoopProgram) :
    srcLoopClosedLoopWitness p →
      mirLoopClosedLoopWitness (lowerFnCfgLoopClosedLoopProgram p) := by
  intro h round hmem
  simp [lowerFnCfgLoopClosedLoopProgram] at hmem
  rcases hmem with ⟨srcRound, hsrc, rfl⟩
  exact lowerFnCfgLoopAdaptivePolicyProgram_preserves_witness srcRound (h srcRound hsrc)

theorem emitRFnCfgLoopClosedLoopProgram_preserves_witness
    (p : MirFnCfgLoopClosedLoopProgram) :
    mirLoopClosedLoopWitness p →
      rLoopClosedLoopWitness (emitRFnCfgLoopClosedLoopProgram p) := by
  intro h round hmem
  simp [emitRFnCfgLoopClosedLoopProgram] at hmem
  rcases hmem with ⟨mirRound, hmir, rfl⟩
  exact emitRFnCfgLoopAdaptivePolicyProgram_preserves_witness mirRound (h mirRound hmir)

theorem lowerEmitFnCfgLoopClosedLoopProgram_preserves_witness
    (p : SrcFnCfgLoopClosedLoopProgram) :
    srcLoopClosedLoopWitness p →
      rLoopClosedLoopWitness (emitRFnCfgLoopClosedLoopProgram (lowerFnCfgLoopClosedLoopProgram p)) := by
  intro h
  exact emitRFnCfgLoopClosedLoopProgram_preserves_witness _
    (lowerFnCfgLoopClosedLoopProgram_preserves_witness _ h)

def stableFnCfgLoopClosedLoopProgram : SrcFnCfgLoopClosedLoopProgram :=
  { rounds := [stableFnCfgLoopAdaptivePolicyProgram, stableFnCfgLoopAdaptivePolicyProgram] }

theorem stableFnCfgLoopClosedLoopProgram_meta_preserved :
    (lowerFnCfgLoopClosedLoopProgram stableFnCfgLoopClosedLoopProgram).rounds.length = 2 := by
  rfl

theorem stableFnCfgLoopClosedLoopProgram_src_witness :
    srcLoopClosedLoopWitness stableFnCfgLoopClosedLoopProgram := by
  intro round hmem
  simp [stableFnCfgLoopClosedLoopProgram] at hmem
  rcases hmem with rfl | rfl
  · exact stableFnCfgLoopAdaptivePolicyProgram_src_witness

theorem stableFnCfgLoopClosedLoopProgram_eval_preserved :
    evalRFnCfgLoopClosedLoopProgram
      (emitRFnCfgLoopClosedLoopProgram (lowerFnCfgLoopClosedLoopProgram stableFnCfgLoopClosedLoopProgram)) =
      [[ (3, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
       , (2, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
       , (1, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
       ],
       [ (3, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
       , (2, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
       , (1, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
       ]] := by
  rw [lowerEmitFnCfgLoopClosedLoopProgram_preserves_eval]
  rfl

theorem stableFnCfgLoopClosedLoopProgram_preserved :
    rLoopClosedLoopWitness
      (emitRFnCfgLoopClosedLoopProgram (lowerFnCfgLoopClosedLoopProgram stableFnCfgLoopClosedLoopProgram)) := by
  exact lowerEmitFnCfgLoopClosedLoopProgram_preserves_witness _
    stableFnCfgLoopClosedLoopProgram_src_witness

end RRProofs
