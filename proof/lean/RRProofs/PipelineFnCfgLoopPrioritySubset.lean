import RRProofs.PipelineFnCfgLoopDynamicSchedulerSubset

namespace RRProofs

abbrev PriorityBatch α := Nat × α
abbrev PriorityTrace := List (Nat × List LoopUpdate)

structure SrcFnCfgLoopPriorityProgram where
  pending : List (PriorityBatch SrcFnCfgLoopQueueProgram)
  reinserts : List (PriorityBatch SrcFnCfgLoopQueueProgram)

structure MirFnCfgLoopPriorityProgram where
  pending : List (PriorityBatch MirFnCfgLoopQueueProgram)
  reinserts : List (PriorityBatch MirFnCfgLoopQueueProgram)

structure RFnCfgLoopPriorityProgram where
  pending : List (PriorityBatch RFnCfgLoopQueueProgram)
  reinserts : List (PriorityBatch RFnCfgLoopQueueProgram)

def prioritiesNonincreasing : List Nat → Prop
  | [] => True
  | [_] => True
  | x :: y :: rest => x >= y ∧ prioritiesNonincreasing (y :: rest)

def toSrcFnCfgLoopDynamicSchedulerProgram (p : SrcFnCfgLoopPriorityProgram) :
    SrcFnCfgLoopDynamicSchedulerProgram :=
  { baseBatches := p.pending.map Prod.snd
  , reinserts := p.reinserts.map Prod.snd
  }

def toMirFnCfgLoopDynamicSchedulerProgram (p : MirFnCfgLoopPriorityProgram) :
    MirFnCfgLoopDynamicSchedulerProgram :=
  { baseBatches := p.pending.map Prod.snd
  , reinserts := p.reinserts.map Prod.snd
  }

def toRFnCfgLoopDynamicSchedulerProgram (p : RFnCfgLoopPriorityProgram) :
    RFnCfgLoopDynamicSchedulerProgram :=
  { baseBatches := p.pending.map Prod.snd
  , reinserts := p.reinserts.map Prod.snd
  }

def executionPriorities (p : SrcFnCfgLoopPriorityProgram) : List Nat :=
  (p.pending ++ p.reinserts).map Prod.fst

def lowerFnCfgLoopPriorityProgram (p : SrcFnCfgLoopPriorityProgram) : MirFnCfgLoopPriorityProgram :=
  { pending := p.pending.map (fun (prio, batch) => (prio, lowerFnCfgLoopQueueProgram batch))
  , reinserts := p.reinserts.map (fun (prio, batch) => (prio, lowerFnCfgLoopQueueProgram batch))
  }

def emitRFnCfgLoopPriorityProgram (p : MirFnCfgLoopPriorityProgram) : RFnCfgLoopPriorityProgram :=
  { pending := p.pending.map (fun (prio, batch) => (prio, emitRFnCfgLoopQueueProgram batch))
  , reinserts := p.reinserts.map (fun (prio, batch) => (prio, emitRFnCfgLoopQueueProgram batch))
  }

def evalSrcPriorityBatches : List (PriorityBatch SrcFnCfgLoopQueueProgram) → PriorityTrace
  | [] => []
  | (prio, batch) :: rest => (prio, evalSrcFnCfgLoopQueueProgram batch) :: evalSrcPriorityBatches rest

def evalMirPriorityBatches : List (PriorityBatch MirFnCfgLoopQueueProgram) → PriorityTrace
  | [] => []
  | (prio, batch) :: rest => (prio, evalMirFnCfgLoopQueueProgram batch) :: evalMirPriorityBatches rest

def evalRPriorityBatches : List (PriorityBatch RFnCfgLoopQueueProgram) → PriorityTrace
  | [] => []
  | (prio, batch) :: rest => (prio, evalRFnCfgLoopQueueProgram batch) :: evalRPriorityBatches rest

def evalSrcFnCfgLoopPriorityProgram (p : SrcFnCfgLoopPriorityProgram) : PriorityTrace :=
  evalSrcPriorityBatches (p.pending ++ p.reinserts)

def evalMirFnCfgLoopPriorityProgram (p : MirFnCfgLoopPriorityProgram) : PriorityTrace :=
  evalMirPriorityBatches (p.pending ++ p.reinserts)

def evalRFnCfgLoopPriorityProgram (p : RFnCfgLoopPriorityProgram) : PriorityTrace :=
  evalRPriorityBatches (p.pending ++ p.reinserts)

def srcLoopPriorityWitness (p : SrcFnCfgLoopPriorityProgram) : Prop :=
  (∀ pair, pair ∈ p.pending ++ p.reinserts → srcLoopQueueWitness pair.2) ∧
    prioritiesNonincreasing (executionPriorities p)

def mirLoopPriorityWitness (p : MirFnCfgLoopPriorityProgram) : Prop :=
  (∀ pair, pair ∈ p.pending ++ p.reinserts → mirLoopQueueWitness pair.2) ∧
    prioritiesNonincreasing ((p.pending ++ p.reinserts).map Prod.fst)

def rLoopPriorityWitness (p : RFnCfgLoopPriorityProgram) : Prop :=
  (∀ pair, pair ∈ p.pending ++ p.reinserts → rLoopQueueWitness pair.2) ∧
    prioritiesNonincreasing ((p.pending ++ p.reinserts).map Prod.fst)

theorem evalSrcPriorityBatches_append
    (xs ys : List (PriorityBatch SrcFnCfgLoopQueueProgram)) :
    evalSrcPriorityBatches (xs ++ ys) =
      evalSrcPriorityBatches xs ++ evalSrcPriorityBatches ys := by
  induction xs with
  | nil =>
      rfl
  | cons head rest ih =>
      cases head with
      | mk prio batch =>
          simp [evalSrcPriorityBatches, ih]

theorem evalMirPriorityBatches_append
    (xs ys : List (PriorityBatch MirFnCfgLoopQueueProgram)) :
    evalMirPriorityBatches (xs ++ ys) =
      evalMirPriorityBatches xs ++ evalMirPriorityBatches ys := by
  induction xs with
  | nil =>
      rfl
  | cons head rest ih =>
      cases head with
      | mk prio batch =>
          simp [evalMirPriorityBatches, ih]

theorem evalRPriorityBatches_append
    (xs ys : List (PriorityBatch RFnCfgLoopQueueProgram)) :
    evalRPriorityBatches (xs ++ ys) =
      evalRPriorityBatches xs ++ evalRPriorityBatches ys := by
  induction xs with
  | nil =>
      rfl
  | cons head rest ih =>
      cases head with
      | mk prio batch =>
          simp [evalRPriorityBatches, ih]

theorem lowerPriorityBatches_preserves_eval
    (batches : List (PriorityBatch SrcFnCfgLoopQueueProgram)) :
    evalMirPriorityBatches
        (batches.map (fun (prio, batch) => (prio, lowerFnCfgLoopQueueProgram batch))) =
      evalSrcPriorityBatches batches := by
  induction batches with
  | nil =>
      rfl
  | cons head rest ih =>
      cases head with
      | mk prio batch =>
          simp [evalMirPriorityBatches, evalSrcPriorityBatches,
            lowerFnCfgLoopQueueProgram_preserves_eval, ih]

theorem emitRPriorityBatches_preserves_eval
    (batches : List (PriorityBatch MirFnCfgLoopQueueProgram)) :
    evalRPriorityBatches
        (batches.map (fun (prio, batch) => (prio, emitRFnCfgLoopQueueProgram batch))) =
      evalMirPriorityBatches batches := by
  induction batches with
  | nil =>
      rfl
  | cons head rest ih =>
      cases head with
      | mk prio batch =>
          simp [evalRPriorityBatches, evalMirPriorityBatches,
            emitRFnCfgLoopQueueProgram_preserves_eval, ih]

theorem lowerFnCfgLoopPriorityProgram_preserves_meta
    (p : SrcFnCfgLoopPriorityProgram) :
    (lowerFnCfgLoopPriorityProgram p).pending.length = p.pending.length ∧
      (lowerFnCfgLoopPriorityProgram p).reinserts.length = p.reinserts.length := by
  constructor
  · simp [lowerFnCfgLoopPriorityProgram]
  · simp [lowerFnCfgLoopPriorityProgram]

theorem emitRFnCfgLoopPriorityProgram_preserves_meta
    (p : MirFnCfgLoopPriorityProgram) :
    (emitRFnCfgLoopPriorityProgram p).pending.length = p.pending.length ∧
      (emitRFnCfgLoopPriorityProgram p).reinserts.length = p.reinserts.length := by
  constructor
  · simp [emitRFnCfgLoopPriorityProgram]
  · simp [emitRFnCfgLoopPriorityProgram]

theorem lowerFnCfgLoopPriorityProgram_preserves_eval
    (p : SrcFnCfgLoopPriorityProgram) :
    evalMirFnCfgLoopPriorityProgram (lowerFnCfgLoopPriorityProgram p) =
      evalSrcFnCfgLoopPriorityProgram p := by
  unfold evalMirFnCfgLoopPriorityProgram evalSrcFnCfgLoopPriorityProgram lowerFnCfgLoopPriorityProgram
  rw [evalMirPriorityBatches_append, evalSrcPriorityBatches_append]
  simp [lowerPriorityBatches_preserves_eval]

theorem emitRFnCfgLoopPriorityProgram_preserves_eval
    (p : MirFnCfgLoopPriorityProgram) :
    evalRFnCfgLoopPriorityProgram (emitRFnCfgLoopPriorityProgram p) =
      evalMirFnCfgLoopPriorityProgram p := by
  unfold evalRFnCfgLoopPriorityProgram evalMirFnCfgLoopPriorityProgram emitRFnCfgLoopPriorityProgram
  rw [evalRPriorityBatches_append, evalMirPriorityBatches_append]
  simp [emitRPriorityBatches_preserves_eval]

theorem lowerEmitFnCfgLoopPriorityProgram_preserves_eval
    (p : SrcFnCfgLoopPriorityProgram) :
    evalRFnCfgLoopPriorityProgram (emitRFnCfgLoopPriorityProgram (lowerFnCfgLoopPriorityProgram p)) =
      evalSrcFnCfgLoopPriorityProgram p := by
  rw [emitRFnCfgLoopPriorityProgram_preserves_eval, lowerFnCfgLoopPriorityProgram_preserves_eval]

theorem lowerFnCfgLoopPriorityProgram_preserves_witness
    (p : SrcFnCfgLoopPriorityProgram) :
    srcLoopPriorityWitness p →
      mirLoopPriorityWitness (lowerFnCfgLoopPriorityProgram p) := by
  intro h
  rcases h with ⟨hDyn, hPrio⟩
  constructor
  · intro pair hmem
    simp [lowerFnCfgLoopPriorityProgram] at hmem
    rcases hmem with ⟨prio, srcBatch, hsrc, hEq⟩ | ⟨prio, srcBatch, hsrc, hEq⟩
    · cases hEq
      exact lowerFnCfgLoopQueueProgram_preserves_witness srcBatch
        (hDyn (prio, srcBatch) (List.mem_append.mpr (Or.inl hsrc)))
    · cases hEq
      exact lowerFnCfgLoopQueueProgram_preserves_witness srcBatch
        (hDyn (prio, srcBatch) (List.mem_append.mpr (Or.inr hsrc)))
  · simpa [executionPriorities, lowerFnCfgLoopPriorityProgram, List.map_append] using hPrio

theorem emitRFnCfgLoopPriorityProgram_preserves_witness
    (p : MirFnCfgLoopPriorityProgram) :
    mirLoopPriorityWitness p →
      rLoopPriorityWitness (emitRFnCfgLoopPriorityProgram p) := by
  intro h
  rcases h with ⟨hDyn, hPrio⟩
  constructor
  · intro pair hmem
    simp [emitRFnCfgLoopPriorityProgram] at hmem
    rcases hmem with ⟨prio, mirBatch, hmir, hEq⟩ | ⟨prio, mirBatch, hmir, hEq⟩
    · cases hEq
      exact emitRFnCfgLoopQueueProgram_preserves_witness mirBatch
        (hDyn (prio, mirBatch) (List.mem_append.mpr (Or.inl hmir)))
    · cases hEq
      exact emitRFnCfgLoopQueueProgram_preserves_witness mirBatch
        (hDyn (prio, mirBatch) (List.mem_append.mpr (Or.inr hmir)))
  · simpa [emitRFnCfgLoopPriorityProgram, List.map_append] using hPrio

theorem lowerEmitFnCfgLoopPriorityProgram_preserves_witness
    (p : SrcFnCfgLoopPriorityProgram) :
    srcLoopPriorityWitness p →
      rLoopPriorityWitness (emitRFnCfgLoopPriorityProgram (lowerFnCfgLoopPriorityProgram p)) := by
  intro h
  exact emitRFnCfgLoopPriorityProgram_preserves_witness _
    (lowerFnCfgLoopPriorityProgram_preserves_witness _ h)

def stableFnCfgLoopPriorityProgram : SrcFnCfgLoopPriorityProgram :=
  { pending := [(5, stableFnCfgLoopQueueProgram), (4, stableFnCfgLoopQueueProgram)]
  , reinserts := [(3, stableFnCfgLoopQueueProgram)]
  }

theorem stableFnCfgLoopPriorityProgram_meta_preserved :
    (lowerFnCfgLoopPriorityProgram stableFnCfgLoopPriorityProgram).pending.length = 2 ∧
      (lowerFnCfgLoopPriorityProgram stableFnCfgLoopPriorityProgram).reinserts.length = 1 := by
  constructor
  · rfl
  · rfl

theorem stableFnCfgLoopPriorityProgram_src_witness :
    srcLoopPriorityWitness stableFnCfgLoopPriorityProgram := by
  constructor
  · intro pair hmem
    simp [stableFnCfgLoopPriorityProgram] at hmem
    rcases hmem with rfl | rfl | rfl
    · exact stableFnCfgLoopQueueProgram_src_witness
    · exact stableFnCfgLoopQueueProgram_src_witness
    · exact stableFnCfgLoopQueueProgram_src_witness
  · change 5 >= 4 ∧ (4 >= 3 ∧ True)
    constructor
    · decide
    · constructor
      · decide
      · trivial

theorem stableFnCfgLoopPriorityProgram_eval_preserved :
    evalRFnCfgLoopPriorityProgram
      (emitRFnCfgLoopPriorityProgram (lowerFnCfgLoopPriorityProgram stableFnCfgLoopPriorityProgram)) =
      [ (5, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
      , (4, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
      , (3, [([.int 10, .int 5], [.int 12]), ([.int 10, .int 5], [.int 12])])
      ] := by
  rw [lowerEmitFnCfgLoopPriorityProgram_preserves_eval]
  rfl

theorem stableFnCfgLoopPriorityProgram_preserved :
    rLoopPriorityWitness
      (emitRFnCfgLoopPriorityProgram (lowerFnCfgLoopPriorityProgram stableFnCfgLoopPriorityProgram)) := by
  exact lowerEmitFnCfgLoopPriorityProgram_preserves_witness _
    stableFnCfgLoopPriorityProgram_src_witness

end RRProofs
