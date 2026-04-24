import RRProofs.PipelineFnCfgOpenSearchFrontierPolicySubset

namespace RRProofs

def recomputeOpenSearchFrontierRules
    : List OpenSearchFrontierPriorityRule → List Nat → List OpenSearchFrontierPriorityRule
  | [], _ => []
  | _, [] => []
  | (src, _) :: rules, dst :: feedback =>
      (src, dst) :: recomputeOpenSearchFrontierRules rules feedback

structure SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram where
  policyProg : SrcFnCfgOpenSearchFrontierPolicyProgram
  baseRules : List OpenSearchFrontierPriorityRule
  feedback : List Nat
  recomputedRules : List OpenSearchFrontierPriorityRule

structure MirFnCfgOpenSearchFrontierAdaptivePolicyProgram where
  policyProg : MirFnCfgOpenSearchFrontierPolicyProgram
  baseRules : List OpenSearchFrontierPriorityRule
  feedback : List Nat
  recomputedRules : List OpenSearchFrontierPriorityRule

structure RFnCfgOpenSearchFrontierAdaptivePolicyProgram where
  policyProg : RFnCfgOpenSearchFrontierPolicyProgram
  baseRules : List OpenSearchFrontierPriorityRule
  feedback : List Nat
  recomputedRules : List OpenSearchFrontierPriorityRule

def lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram
    (p : SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    MirFnCfgOpenSearchFrontierAdaptivePolicyProgram :=
  { policyProg := lowerFnCfgOpenSearchFrontierPolicyProgram p.policyProg
  , baseRules := p.baseRules
  , feedback := p.feedback
  , recomputedRules := p.recomputedRules
  }

def emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram
    (p : MirFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    RFnCfgOpenSearchFrontierAdaptivePolicyProgram :=
  { policyProg := emitRFnCfgOpenSearchFrontierPolicyProgram p.policyProg
  , baseRules := p.baseRules
  , feedback := p.feedback
  , recomputedRules := p.recomputedRules
  }

def evalSrcFnCfgOpenSearchFrontierAdaptivePolicyProgram
    (p : SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchFrontierPolicyProgram p.policyProg

def evalMirFnCfgOpenSearchFrontierAdaptivePolicyProgram
    (p : MirFnCfgOpenSearchFrontierAdaptivePolicyProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchFrontierPolicyProgram p.policyProg

def evalRFnCfgOpenSearchFrontierAdaptivePolicyProgram
    (p : RFnCfgOpenSearchFrontierAdaptivePolicyProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchFrontierPolicyProgram p.policyProg

def srcOpenSearchFrontierAdaptivePolicyWitness
    (p : SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram) : Prop :=
  srcOpenSearchFrontierPolicyWitness p.policyProg ∧
    p.recomputedRules = recomputeOpenSearchFrontierRules p.baseRules p.feedback

def mirOpenSearchFrontierAdaptivePolicyWitness
    (p : MirFnCfgOpenSearchFrontierAdaptivePolicyProgram) : Prop :=
  mirOpenSearchFrontierPolicyWitness p.policyProg ∧
    p.recomputedRules = recomputeOpenSearchFrontierRules p.baseRules p.feedback

def rOpenSearchFrontierAdaptivePolicyWitness
    (p : RFnCfgOpenSearchFrontierAdaptivePolicyProgram) : Prop :=
  rOpenSearchFrontierPolicyWitness p.policyProg ∧
    p.recomputedRules = recomputeOpenSearchFrontierRules p.baseRules p.feedback

theorem lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_meta
    (p : SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram p).policyProg.priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.policyProg.priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram p).baseRules = p.baseRules ∧
      (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram p).feedback = p.feedback ∧
      (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram p).recomputedRules = p.recomputedRules := by
  constructor
  · simpa [lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram] using
      (lowerFnCfgOpenSearchFrontierPolicyProgram_preserves_meta p.policyProg).1
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_meta
    (p : MirFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram p).policyProg.priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.policyProg.priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram p).baseRules = p.baseRules ∧
      (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram p).feedback = p.feedback ∧
      (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram p).recomputedRules = p.recomputedRules := by
  constructor
  · simpa [emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram] using
      (emitRFnCfgOpenSearchFrontierPolicyProgram_preserves_meta p.policyProg).1
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    evalMirFnCfgOpenSearchFrontierAdaptivePolicyProgram (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram p) =
      evalSrcFnCfgOpenSearchFrontierAdaptivePolicyProgram p := by
  rfl

theorem emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_eval
    (p : MirFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    evalRFnCfgOpenSearchFrontierAdaptivePolicyProgram (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram p) =
      evalMirFnCfgOpenSearchFrontierAdaptivePolicyProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    evalRFnCfgOpenSearchFrontierAdaptivePolicyProgram
        (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram
          (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram p)) =
      evalSrcFnCfgOpenSearchFrontierAdaptivePolicyProgram p := by
  rfl

theorem lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    srcOpenSearchFrontierAdaptivePolicyWitness p →
      mirOpenSearchFrontierAdaptivePolicyWitness
        (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram p) := by
  intro h
  rcases h with ⟨hPolicy, hRules⟩
  constructor
  · exact lowerFnCfgOpenSearchFrontierPolicyProgram_preserves_witness _ hPolicy
  · simpa [lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram] using hRules

theorem emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_witness
    (p : MirFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    mirOpenSearchFrontierAdaptivePolicyWitness p →
      rOpenSearchFrontierAdaptivePolicyWitness
        (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram p) := by
  intro h
  rcases h with ⟨hPolicy, hRules⟩
  constructor
  · exact emitRFnCfgOpenSearchFrontierPolicyProgram_preserves_witness _ hPolicy
  · simpa [emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram] using hRules

theorem lowerEmitFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram) :
    srcOpenSearchFrontierAdaptivePolicyWitness p →
      rOpenSearchFrontierAdaptivePolicyWitness
        (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram
          (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_witness _
    (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_witness _ h)

def stableFnCfgOpenSearchFrontierAdaptivePolicyProgram :
    SrcFnCfgOpenSearchFrontierAdaptivePolicyProgram :=
  { policyProg := stableFnCfgOpenSearchFrontierPolicyProgram
  , baseRules := [(5, 9), (3, 9)]
  , feedback := [3, 1]
  , recomputedRules := [(5, 3), (3, 1)]
  }

theorem stableFnCfgOpenSearchFrontierAdaptivePolicyProgram_meta_preserved :
    (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram stableFnCfgOpenSearchFrontierAdaptivePolicyProgram).policyProg.priorityProg.schedProg.dynProg.frontierProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram stableFnCfgOpenSearchFrontierAdaptivePolicyProgram).baseRules =
        [(5, 9), (3, 9)] ∧
      (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram stableFnCfgOpenSearchFrontierAdaptivePolicyProgram).feedback =
        [3, 1] ∧
      (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram stableFnCfgOpenSearchFrontierAdaptivePolicyProgram).recomputedRules =
        [(5, 3), (3, 1)] := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchFrontierAdaptivePolicyProgram_src_witness :
    srcOpenSearchFrontierAdaptivePolicyWitness stableFnCfgOpenSearchFrontierAdaptivePolicyProgram := by
  constructor
  · exact stableFnCfgOpenSearchFrontierPolicyProgram_src_witness
  · rfl

theorem stableFnCfgOpenSearchFrontierAdaptivePolicyProgram_eval_preserved :
    evalRFnCfgOpenSearchFrontierAdaptivePolicyProgram
      (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram
        (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram stableFnCfgOpenSearchFrontierAdaptivePolicyProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserved :
    rOpenSearchFrontierAdaptivePolicyWitness
      (emitRFnCfgOpenSearchFrontierAdaptivePolicyProgram
        (lowerFnCfgOpenSearchFrontierAdaptivePolicyProgram stableFnCfgOpenSearchFrontierAdaptivePolicyProgram)) := by
  exact lowerEmitFnCfgOpenSearchFrontierAdaptivePolicyProgram_preserves_witness _
    stableFnCfgOpenSearchFrontierAdaptivePolicyProgram_src_witness

end RRProofs
