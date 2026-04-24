import RRProofs.PipelineFnCfgLoopWorklistSubset

namespace RRProofs

abbrev LoopUpdate := List RValue × List RValue

structure SrcFnCfgLoopQueueProgram where
  rounds : List SrcFnCfgLoopWorklistProgram

structure MirFnCfgLoopQueueProgram where
  rounds : List MirFnCfgLoopWorklistProgram

structure RFnCfgLoopQueueProgram where
  rounds : List RFnCfgLoopWorklistProgram

def lowerFnCfgLoopQueueProgram (p : SrcFnCfgLoopQueueProgram) : MirFnCfgLoopQueueProgram :=
  { rounds := p.rounds.map lowerFnCfgLoopWorklistProgram }

def emitRFnCfgLoopQueueProgram (p : MirFnCfgLoopQueueProgram) : RFnCfgLoopQueueProgram :=
  { rounds := p.rounds.map emitRFnCfgLoopWorklistProgram }

def drainSrcLoopQueue : List SrcFnCfgLoopWorklistProgram → List LoopUpdate
  | [] => []
  | round :: rest => srcLoopWorklistUpdate round :: drainSrcLoopQueue rest

def drainMirLoopQueue : List MirFnCfgLoopWorklistProgram → List LoopUpdate
  | [] => []
  | round :: rest => mirLoopWorklistUpdate round :: drainMirLoopQueue rest

def drainRLoopQueue : List RFnCfgLoopWorklistProgram → List LoopUpdate
  | [] => []
  | round :: rest => rLoopWorklistUpdate round :: drainRLoopQueue rest

def evalSrcFnCfgLoopQueueProgram (p : SrcFnCfgLoopQueueProgram) : List LoopUpdate :=
  drainSrcLoopQueue p.rounds

def evalMirFnCfgLoopQueueProgram (p : MirFnCfgLoopQueueProgram) : List LoopUpdate :=
  drainMirLoopQueue p.rounds

def evalRFnCfgLoopQueueProgram (p : RFnCfgLoopQueueProgram) : List LoopUpdate :=
  drainRLoopQueue p.rounds

def srcLoopQueueWitness (p : SrcFnCfgLoopQueueProgram) : Prop :=
  ∀ round, round ∈ p.rounds → srcLoopWorklistWitness round

def mirLoopQueueWitness (p : MirFnCfgLoopQueueProgram) : Prop :=
  ∀ round, round ∈ p.rounds → mirLoopWorklistWitness round

def rLoopQueueWitness (p : RFnCfgLoopQueueProgram) : Prop :=
  ∀ round, round ∈ p.rounds → rLoopWorklistWitness round

theorem lowerLoopQueueRounds_preserves_eval
    (rounds : List SrcFnCfgLoopWorklistProgram) :
    drainMirLoopQueue (rounds.map lowerFnCfgLoopWorklistProgram) =
      drainSrcLoopQueue rounds := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [drainMirLoopQueue, drainSrcLoopQueue, lowerFnCfgLoopWorklistProgram_preserves_update, ih]

theorem emitRLoopQueueRounds_preserves_eval
    (rounds : List MirFnCfgLoopWorklistProgram) :
    drainRLoopQueue (rounds.map emitRFnCfgLoopWorklistProgram) =
      drainMirLoopQueue rounds := by
  induction rounds with
  | nil =>
      rfl
  | cons round rest ih =>
      simp [drainRLoopQueue, drainMirLoopQueue, emitRFnCfgLoopWorklistProgram_preserves_update, ih]

theorem lowerFnCfgLoopQueueProgram_preserves_meta
    (p : SrcFnCfgLoopQueueProgram) :
    (lowerFnCfgLoopQueueProgram p).rounds.length = p.rounds.length := by
  simp [lowerFnCfgLoopQueueProgram]

theorem emitRFnCfgLoopQueueProgram_preserves_meta
    (p : MirFnCfgLoopQueueProgram) :
    (emitRFnCfgLoopQueueProgram p).rounds.length = p.rounds.length := by
  simp [emitRFnCfgLoopQueueProgram]

theorem lowerFnCfgLoopQueueProgram_preserves_eval
    (p : SrcFnCfgLoopQueueProgram) :
    evalMirFnCfgLoopQueueProgram (lowerFnCfgLoopQueueProgram p) =
      evalSrcFnCfgLoopQueueProgram p := by
  simpa [evalMirFnCfgLoopQueueProgram, evalSrcFnCfgLoopQueueProgram, lowerFnCfgLoopQueueProgram] using
    lowerLoopQueueRounds_preserves_eval p.rounds

theorem emitRFnCfgLoopQueueProgram_preserves_eval
    (p : MirFnCfgLoopQueueProgram) :
    evalRFnCfgLoopQueueProgram (emitRFnCfgLoopQueueProgram p) =
      evalMirFnCfgLoopQueueProgram p := by
  simpa [evalRFnCfgLoopQueueProgram, evalMirFnCfgLoopQueueProgram, emitRFnCfgLoopQueueProgram] using
    emitRLoopQueueRounds_preserves_eval p.rounds

theorem lowerEmitFnCfgLoopQueueProgram_preserves_eval
    (p : SrcFnCfgLoopQueueProgram) :
    evalRFnCfgLoopQueueProgram (emitRFnCfgLoopQueueProgram (lowerFnCfgLoopQueueProgram p)) =
      evalSrcFnCfgLoopQueueProgram p := by
  rw [emitRFnCfgLoopQueueProgram_preserves_eval, lowerFnCfgLoopQueueProgram_preserves_eval]

theorem lowerFnCfgLoopQueueProgram_preserves_witness
    (p : SrcFnCfgLoopQueueProgram) :
    srcLoopQueueWitness p →
      mirLoopQueueWitness (lowerFnCfgLoopQueueProgram p) := by
  intro h round hmem
  simp [lowerFnCfgLoopQueueProgram] at hmem
  rcases hmem with ⟨srcRound, hsrc, rfl⟩
  exact lowerFnCfgLoopWorklistProgram_preserves_witness srcRound (h srcRound hsrc)

theorem emitRFnCfgLoopQueueProgram_preserves_witness
    (p : MirFnCfgLoopQueueProgram) :
    mirLoopQueueWitness p →
      rLoopQueueWitness (emitRFnCfgLoopQueueProgram p) := by
  intro h round hmem
  simp [emitRFnCfgLoopQueueProgram] at hmem
  rcases hmem with ⟨mirRound, hmir, rfl⟩
  exact emitRFnCfgLoopWorklistProgram_preserves_witness mirRound (h mirRound hmir)

theorem lowerEmitFnCfgLoopQueueProgram_preserves_witness
    (p : SrcFnCfgLoopQueueProgram) :
    srcLoopQueueWitness p →
      rLoopQueueWitness (emitRFnCfgLoopQueueProgram (lowerFnCfgLoopQueueProgram p)) := by
  intro h
  exact emitRFnCfgLoopQueueProgram_preserves_witness _ (lowerFnCfgLoopQueueProgram_preserves_witness _ h)

def stableFnCfgLoopQueueProgram : SrcFnCfgLoopQueueProgram :=
  { rounds := [stableFnCfgLoopWorklistProgram, stableFnCfgLoopWorklistProgram] }

theorem stableFnCfgLoopQueueProgram_meta_preserved :
    (lowerFnCfgLoopQueueProgram stableFnCfgLoopQueueProgram).rounds.length = 2 := by
  rfl

theorem stableFnCfgLoopQueueProgram_src_witness :
    srcLoopQueueWitness stableFnCfgLoopQueueProgram := by
  intro round hmem
  simp [stableFnCfgLoopQueueProgram] at hmem
  rcases hmem with rfl | rfl
  · exact stableFnCfgLoopWorklistProgram_src_witness

theorem stableFnCfgLoopQueueProgram_eval_preserved :
    evalRFnCfgLoopQueueProgram
      (emitRFnCfgLoopQueueProgram (lowerFnCfgLoopQueueProgram stableFnCfgLoopQueueProgram)) =
      [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])] := by
  rw [lowerEmitFnCfgLoopQueueProgram_preserves_eval]
  rfl

theorem stableFnCfgLoopQueueProgram_preserved :
    rLoopQueueWitness
      (emitRFnCfgLoopQueueProgram (lowerFnCfgLoopQueueProgram stableFnCfgLoopQueueProgram)) := by
  exact lowerEmitFnCfgLoopQueueProgram_preserves_witness _ stableFnCfgLoopQueueProgram_src_witness

end RRProofs
