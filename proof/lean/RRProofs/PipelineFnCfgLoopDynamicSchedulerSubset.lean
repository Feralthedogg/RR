import RRProofs.PipelineFnCfgLoopSchedulerSubset

namespace RRProofs

structure SrcFnCfgLoopDynamicSchedulerProgram where
  baseBatches : List SrcFnCfgLoopQueueProgram
  reinserts : List SrcFnCfgLoopQueueProgram

structure MirFnCfgLoopDynamicSchedulerProgram where
  baseBatches : List MirFnCfgLoopQueueProgram
  reinserts : List MirFnCfgLoopQueueProgram

structure RFnCfgLoopDynamicSchedulerProgram where
  baseBatches : List RFnCfgLoopQueueProgram
  reinserts : List RFnCfgLoopQueueProgram

def toSrcFnCfgLoopSchedulerProgram (p : SrcFnCfgLoopDynamicSchedulerProgram) : SrcFnCfgLoopSchedulerProgram :=
  { batches := p.baseBatches ++ p.reinserts }

def toMirFnCfgLoopSchedulerProgram (p : MirFnCfgLoopDynamicSchedulerProgram) : MirFnCfgLoopSchedulerProgram :=
  { batches := p.baseBatches ++ p.reinserts }

def toRFnCfgLoopSchedulerProgram (p : RFnCfgLoopDynamicSchedulerProgram) : RFnCfgLoopSchedulerProgram :=
  { batches := p.baseBatches ++ p.reinserts }

def lowerFnCfgLoopDynamicSchedulerProgram (p : SrcFnCfgLoopDynamicSchedulerProgram) :
    MirFnCfgLoopDynamicSchedulerProgram :=
  { baseBatches := p.baseBatches.map lowerFnCfgLoopQueueProgram
  , reinserts := p.reinserts.map lowerFnCfgLoopQueueProgram
  }

def emitRFnCfgLoopDynamicSchedulerProgram (p : MirFnCfgLoopDynamicSchedulerProgram) :
    RFnCfgLoopDynamicSchedulerProgram :=
  { baseBatches := p.baseBatches.map emitRFnCfgLoopQueueProgram
  , reinserts := p.reinserts.map emitRFnCfgLoopQueueProgram
  }

def evalSrcFnCfgLoopDynamicSchedulerProgram (p : SrcFnCfgLoopDynamicSchedulerProgram) :
    List (List LoopUpdate) :=
  evalSrcFnCfgLoopSchedulerProgram (toSrcFnCfgLoopSchedulerProgram p)

def evalMirFnCfgLoopDynamicSchedulerProgram (p : MirFnCfgLoopDynamicSchedulerProgram) :
    List (List LoopUpdate) :=
  evalMirFnCfgLoopSchedulerProgram (toMirFnCfgLoopSchedulerProgram p)

def evalRFnCfgLoopDynamicSchedulerProgram (p : RFnCfgLoopDynamicSchedulerProgram) :
    List (List LoopUpdate) :=
  evalRFnCfgLoopSchedulerProgram (toRFnCfgLoopSchedulerProgram p)

def srcLoopDynamicSchedulerWitness (p : SrcFnCfgLoopDynamicSchedulerProgram) : Prop :=
  srcLoopSchedulerWitness (toSrcFnCfgLoopSchedulerProgram p)

def mirLoopDynamicSchedulerWitness (p : MirFnCfgLoopDynamicSchedulerProgram) : Prop :=
  mirLoopSchedulerWitness (toMirFnCfgLoopSchedulerProgram p)

def rLoopDynamicSchedulerWitness (p : RFnCfgLoopDynamicSchedulerProgram) : Prop :=
  rLoopSchedulerWitness (toRFnCfgLoopSchedulerProgram p)

theorem lowerFnCfgLoopDynamicSchedulerProgram_preserves_meta
    (p : SrcFnCfgLoopDynamicSchedulerProgram) :
    (lowerFnCfgLoopDynamicSchedulerProgram p).baseBatches.length = p.baseBatches.length ∧
      (lowerFnCfgLoopDynamicSchedulerProgram p).reinserts.length = p.reinserts.length := by
  constructor
  · simp [lowerFnCfgLoopDynamicSchedulerProgram]
  · simp [lowerFnCfgLoopDynamicSchedulerProgram]

theorem emitRFnCfgLoopDynamicSchedulerProgram_preserves_meta
    (p : MirFnCfgLoopDynamicSchedulerProgram) :
    (emitRFnCfgLoopDynamicSchedulerProgram p).baseBatches.length = p.baseBatches.length ∧
      (emitRFnCfgLoopDynamicSchedulerProgram p).reinserts.length = p.reinserts.length := by
  constructor
  · simp [emitRFnCfgLoopDynamicSchedulerProgram]
  · simp [emitRFnCfgLoopDynamicSchedulerProgram]

theorem lowerFnCfgLoopDynamicSchedulerProgram_preserves_eval
    (p : SrcFnCfgLoopDynamicSchedulerProgram) :
    evalMirFnCfgLoopDynamicSchedulerProgram (lowerFnCfgLoopDynamicSchedulerProgram p) =
      evalSrcFnCfgLoopDynamicSchedulerProgram p := by
  simpa [evalMirFnCfgLoopDynamicSchedulerProgram, evalSrcFnCfgLoopDynamicSchedulerProgram,
    toSrcFnCfgLoopSchedulerProgram, toMirFnCfgLoopSchedulerProgram, lowerFnCfgLoopDynamicSchedulerProgram,
    lowerFnCfgLoopSchedulerProgram, List.map_append] using
      (lowerFnCfgLoopSchedulerProgram_preserves_eval (toSrcFnCfgLoopSchedulerProgram p))

theorem emitRFnCfgLoopDynamicSchedulerProgram_preserves_eval
    (p : MirFnCfgLoopDynamicSchedulerProgram) :
    evalRFnCfgLoopDynamicSchedulerProgram (emitRFnCfgLoopDynamicSchedulerProgram p) =
      evalMirFnCfgLoopDynamicSchedulerProgram p := by
  simpa [evalRFnCfgLoopDynamicSchedulerProgram, evalMirFnCfgLoopDynamicSchedulerProgram,
    toMirFnCfgLoopSchedulerProgram, toRFnCfgLoopSchedulerProgram, emitRFnCfgLoopDynamicSchedulerProgram,
    emitRFnCfgLoopSchedulerProgram, List.map_append] using
      (emitRFnCfgLoopSchedulerProgram_preserves_eval (toMirFnCfgLoopSchedulerProgram p))

theorem lowerEmitFnCfgLoopDynamicSchedulerProgram_preserves_eval
    (p : SrcFnCfgLoopDynamicSchedulerProgram) :
    evalRFnCfgLoopDynamicSchedulerProgram
        (emitRFnCfgLoopDynamicSchedulerProgram (lowerFnCfgLoopDynamicSchedulerProgram p)) =
      evalSrcFnCfgLoopDynamicSchedulerProgram p := by
  rw [emitRFnCfgLoopDynamicSchedulerProgram_preserves_eval,
    lowerFnCfgLoopDynamicSchedulerProgram_preserves_eval]

theorem lowerFnCfgLoopDynamicSchedulerProgram_preserves_witness
    (p : SrcFnCfgLoopDynamicSchedulerProgram) :
    srcLoopDynamicSchedulerWitness p →
      mirLoopDynamicSchedulerWitness (lowerFnCfgLoopDynamicSchedulerProgram p) := by
  simpa [srcLoopDynamicSchedulerWitness, mirLoopDynamicSchedulerWitness,
    toSrcFnCfgLoopSchedulerProgram, toMirFnCfgLoopSchedulerProgram,
    lowerFnCfgLoopDynamicSchedulerProgram, lowerFnCfgLoopSchedulerProgram, List.map_append] using
      (lowerFnCfgLoopSchedulerProgram_preserves_witness (toSrcFnCfgLoopSchedulerProgram p))

theorem emitRFnCfgLoopDynamicSchedulerProgram_preserves_witness
    (p : MirFnCfgLoopDynamicSchedulerProgram) :
    mirLoopDynamicSchedulerWitness p →
      rLoopDynamicSchedulerWitness (emitRFnCfgLoopDynamicSchedulerProgram p) := by
  simpa [mirLoopDynamicSchedulerWitness, rLoopDynamicSchedulerWitness,
    toMirFnCfgLoopSchedulerProgram, toRFnCfgLoopSchedulerProgram,
    emitRFnCfgLoopDynamicSchedulerProgram, emitRFnCfgLoopSchedulerProgram, List.map_append] using
      (emitRFnCfgLoopSchedulerProgram_preserves_witness (toMirFnCfgLoopSchedulerProgram p))

theorem lowerEmitFnCfgLoopDynamicSchedulerProgram_preserves_witness
    (p : SrcFnCfgLoopDynamicSchedulerProgram) :
    srcLoopDynamicSchedulerWitness p →
      rLoopDynamicSchedulerWitness
        (emitRFnCfgLoopDynamicSchedulerProgram (lowerFnCfgLoopDynamicSchedulerProgram p)) := by
  intro h
  exact emitRFnCfgLoopDynamicSchedulerProgram_preserves_witness _
    (lowerFnCfgLoopDynamicSchedulerProgram_preserves_witness _ h)

def stableFnCfgLoopDynamicSchedulerProgram : SrcFnCfgLoopDynamicSchedulerProgram :=
  { baseBatches := [stableFnCfgLoopQueueProgram]
  , reinserts := [stableFnCfgLoopQueueProgram]
  }

theorem stableFnCfgLoopDynamicSchedulerProgram_meta_preserved :
    (lowerFnCfgLoopDynamicSchedulerProgram stableFnCfgLoopDynamicSchedulerProgram).baseBatches.length = 1 ∧
      (lowerFnCfgLoopDynamicSchedulerProgram stableFnCfgLoopDynamicSchedulerProgram).reinserts.length = 1 := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgLoopDynamicSchedulerProgram_src_witness :
    srcLoopDynamicSchedulerWitness stableFnCfgLoopDynamicSchedulerProgram := by
  simpa [srcLoopDynamicSchedulerWitness, stableFnCfgLoopDynamicSchedulerProgram,
    toSrcFnCfgLoopSchedulerProgram, stableFnCfgLoopSchedulerProgram] using
      stableFnCfgLoopSchedulerProgram_src_witness

theorem stableFnCfgLoopDynamicSchedulerProgram_eval_preserved :
    evalRFnCfgLoopDynamicSchedulerProgram
      (emitRFnCfgLoopDynamicSchedulerProgram
        (lowerFnCfgLoopDynamicSchedulerProgram stableFnCfgLoopDynamicSchedulerProgram)) =
      [[([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])],
       [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])]] := by
  rw [lowerEmitFnCfgLoopDynamicSchedulerProgram_preserves_eval]
  rfl

theorem stableFnCfgLoopDynamicSchedulerProgram_preserved :
    rLoopDynamicSchedulerWitness
      (emitRFnCfgLoopDynamicSchedulerProgram
        (lowerFnCfgLoopDynamicSchedulerProgram stableFnCfgLoopDynamicSchedulerProgram)) := by
  exact lowerEmitFnCfgLoopDynamicSchedulerProgram_preserves_witness _
    stableFnCfgLoopDynamicSchedulerProgram_src_witness

end RRProofs
