import RRProofs.PipelineFnCfgLoopQueueSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

structure SrcFnCfgLoopSchedulerProgram where
  batches : List SrcFnCfgLoopQueueProgram

structure MirFnCfgLoopSchedulerProgram where
  batches : List MirFnCfgLoopQueueProgram

structure RFnCfgLoopSchedulerProgram where
  batches : List RFnCfgLoopQueueProgram

def lowerFnCfgLoopSchedulerProgram (p : SrcFnCfgLoopSchedulerProgram) : MirFnCfgLoopSchedulerProgram :=
  { batches := p.batches.map lowerFnCfgLoopQueueProgram }

def emitRFnCfgLoopSchedulerProgram (p : MirFnCfgLoopSchedulerProgram) : RFnCfgLoopSchedulerProgram :=
  { batches := p.batches.map emitRFnCfgLoopQueueProgram }

def evalSrcFnCfgLoopSchedulerProgram (p : SrcFnCfgLoopSchedulerProgram) : List (List LoopUpdate) :=
  p.batches.map evalSrcFnCfgLoopQueueProgram

def evalMirFnCfgLoopSchedulerProgram (p : MirFnCfgLoopSchedulerProgram) : List (List LoopUpdate) :=
  p.batches.map evalMirFnCfgLoopQueueProgram

def evalRFnCfgLoopSchedulerProgram (p : RFnCfgLoopSchedulerProgram) : List (List LoopUpdate) :=
  p.batches.map evalRFnCfgLoopQueueProgram

def srcLoopSchedulerWitness (p : SrcFnCfgLoopSchedulerProgram) : Prop :=
  ∀ batch, batch ∈ p.batches → srcLoopQueueWitness batch

def mirLoopSchedulerWitness (p : MirFnCfgLoopSchedulerProgram) : Prop :=
  ∀ batch, batch ∈ p.batches → mirLoopQueueWitness batch

def rLoopSchedulerWitness (p : RFnCfgLoopSchedulerProgram) : Prop :=
  ∀ batch, batch ∈ p.batches → rLoopQueueWitness batch

theorem lowerLoopSchedulerBatches_preserves_eval
    (batches : List SrcFnCfgLoopQueueProgram) :
    (batches.map (fun batch => evalMirFnCfgLoopQueueProgram (lowerFnCfgLoopQueueProgram batch))) =
      batches.map evalSrcFnCfgLoopQueueProgram := by
  induction batches with
  | nil =>
      rfl
  | cons batch rest ih =>
      simp [lowerFnCfgLoopQueueProgram_preserves_eval, ih]

theorem emitRLoopSchedulerBatches_preserves_eval
    (batches : List MirFnCfgLoopQueueProgram) :
    (batches.map (fun batch => evalRFnCfgLoopQueueProgram (emitRFnCfgLoopQueueProgram batch))) =
      batches.map evalMirFnCfgLoopQueueProgram := by
  induction batches with
  | nil =>
      rfl
  | cons batch rest ih =>
      simp [emitRFnCfgLoopQueueProgram_preserves_eval, ih]

theorem lowerFnCfgLoopSchedulerProgram_preserves_meta
    (p : SrcFnCfgLoopSchedulerProgram) :
    (lowerFnCfgLoopSchedulerProgram p).batches.length = p.batches.length := by
  simp [lowerFnCfgLoopSchedulerProgram]

theorem emitRFnCfgLoopSchedulerProgram_preserves_meta
    (p : MirFnCfgLoopSchedulerProgram) :
    (emitRFnCfgLoopSchedulerProgram p).batches.length = p.batches.length := by
  simp [emitRFnCfgLoopSchedulerProgram]

theorem lowerFnCfgLoopSchedulerProgram_preserves_eval
    (p : SrcFnCfgLoopSchedulerProgram) :
    evalMirFnCfgLoopSchedulerProgram (lowerFnCfgLoopSchedulerProgram p) =
      evalSrcFnCfgLoopSchedulerProgram p := by
  simpa [evalMirFnCfgLoopSchedulerProgram, evalSrcFnCfgLoopSchedulerProgram, lowerFnCfgLoopSchedulerProgram] using
    lowerLoopSchedulerBatches_preserves_eval p.batches

theorem emitRFnCfgLoopSchedulerProgram_preserves_eval
    (p : MirFnCfgLoopSchedulerProgram) :
    evalRFnCfgLoopSchedulerProgram (emitRFnCfgLoopSchedulerProgram p) =
      evalMirFnCfgLoopSchedulerProgram p := by
  simpa [evalRFnCfgLoopSchedulerProgram, evalMirFnCfgLoopSchedulerProgram, emitRFnCfgLoopSchedulerProgram] using
    emitRLoopSchedulerBatches_preserves_eval p.batches

theorem lowerEmitFnCfgLoopSchedulerProgram_preserves_eval
    (p : SrcFnCfgLoopSchedulerProgram) :
    evalRFnCfgLoopSchedulerProgram (emitRFnCfgLoopSchedulerProgram (lowerFnCfgLoopSchedulerProgram p)) =
      evalSrcFnCfgLoopSchedulerProgram p := by
  rw [emitRFnCfgLoopSchedulerProgram_preserves_eval, lowerFnCfgLoopSchedulerProgram_preserves_eval]

theorem lowerFnCfgLoopSchedulerProgram_preserves_witness
    (p : SrcFnCfgLoopSchedulerProgram) :
    srcLoopSchedulerWitness p →
      mirLoopSchedulerWitness (lowerFnCfgLoopSchedulerProgram p) := by
  intro h batch hmem
  simp [lowerFnCfgLoopSchedulerProgram] at hmem
  rcases hmem with ⟨srcBatch, hsrc, rfl⟩
  exact lowerFnCfgLoopQueueProgram_preserves_witness srcBatch (h srcBatch hsrc)

theorem emitRFnCfgLoopSchedulerProgram_preserves_witness
    (p : MirFnCfgLoopSchedulerProgram) :
    mirLoopSchedulerWitness p →
      rLoopSchedulerWitness (emitRFnCfgLoopSchedulerProgram p) := by
  intro h batch hmem
  simp [emitRFnCfgLoopSchedulerProgram] at hmem
  rcases hmem with ⟨mirBatch, hmir, rfl⟩
  exact emitRFnCfgLoopQueueProgram_preserves_witness mirBatch (h mirBatch hmir)

theorem lowerEmitFnCfgLoopSchedulerProgram_preserves_witness
    (p : SrcFnCfgLoopSchedulerProgram) :
    srcLoopSchedulerWitness p →
      rLoopSchedulerWitness (emitRFnCfgLoopSchedulerProgram (lowerFnCfgLoopSchedulerProgram p)) := by
  intro h
  exact emitRFnCfgLoopSchedulerProgram_preserves_witness _ (lowerFnCfgLoopSchedulerProgram_preserves_witness _ h)

def stableFnCfgLoopSchedulerProgram : SrcFnCfgLoopSchedulerProgram :=
  { batches := [stableFnCfgLoopQueueProgram, stableFnCfgLoopQueueProgram] }

theorem stableFnCfgLoopSchedulerProgram_meta_preserved :
    (lowerFnCfgLoopSchedulerProgram stableFnCfgLoopSchedulerProgram).batches.length = 2 := by
  rfl

theorem stableFnCfgLoopSchedulerProgram_src_witness :
    srcLoopSchedulerWitness stableFnCfgLoopSchedulerProgram := by
  intro batch hmem
  simp [stableFnCfgLoopSchedulerProgram] at hmem
  rcases hmem with rfl | rfl
  · exact stableFnCfgLoopQueueProgram_src_witness

theorem stableFnCfgLoopSchedulerProgram_eval_preserved :
    evalRFnCfgLoopSchedulerProgram
      (emitRFnCfgLoopSchedulerProgram (lowerFnCfgLoopSchedulerProgram stableFnCfgLoopSchedulerProgram)) =
      [[([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])],
       [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])]] := by
  rw [lowerEmitFnCfgLoopSchedulerProgram_preserves_eval]
  rfl

theorem stableFnCfgLoopSchedulerProgram_preserved :
    rLoopSchedulerWitness
      (emitRFnCfgLoopSchedulerProgram (lowerFnCfgLoopSchedulerProgram stableFnCfgLoopSchedulerProgram)) := by
  exact lowerEmitFnCfgLoopSchedulerProgram_preserves_witness _ stableFnCfgLoopSchedulerProgram_src_witness

end RRProofs
