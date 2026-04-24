import RRProofs.PipelineFnCfgOpenSearchPolicySubset

namespace RRProofs

def recomputeOpenSearchRules : List OpenSearchPriorityRule → List Nat → List OpenSearchPriorityRule
  | [], _ => []
  | _, [] => []
  | (src, _) :: rules, dst :: feedback =>
      (src, dst) :: recomputeOpenSearchRules rules feedback

structure SrcFnCfgOpenSearchAdaptivePolicyProgram where
  policyProg : SrcFnCfgOpenSearchPolicyProgram
  baseRules : List OpenSearchPriorityRule
  feedback : List Nat
  recomputedRules : List OpenSearchPriorityRule

structure MirFnCfgOpenSearchAdaptivePolicyProgram where
  policyProg : MirFnCfgOpenSearchPolicyProgram
  baseRules : List OpenSearchPriorityRule
  feedback : List Nat
  recomputedRules : List OpenSearchPriorityRule

structure RFnCfgOpenSearchAdaptivePolicyProgram where
  policyProg : RFnCfgOpenSearchPolicyProgram
  baseRules : List OpenSearchPriorityRule
  feedback : List Nat
  recomputedRules : List OpenSearchPriorityRule

def lowerFnCfgOpenSearchAdaptivePolicyProgram
    (p : SrcFnCfgOpenSearchAdaptivePolicyProgram) : MirFnCfgOpenSearchAdaptivePolicyProgram :=
  { policyProg := lowerFnCfgOpenSearchPolicyProgram p.policyProg
  , baseRules := p.baseRules
  , feedback := p.feedback
  , recomputedRules := p.recomputedRules
  }

def emitRFnCfgOpenSearchAdaptivePolicyProgram
    (p : MirFnCfgOpenSearchAdaptivePolicyProgram) : RFnCfgOpenSearchAdaptivePolicyProgram :=
  { policyProg := emitRFnCfgOpenSearchPolicyProgram p.policyProg
  , baseRules := p.baseRules
  , feedback := p.feedback
  , recomputedRules := p.recomputedRules
  }

def evalSrcFnCfgOpenSearchAdaptivePolicyProgram
    (p : SrcFnCfgOpenSearchAdaptivePolicyProgram) : PriorityTrace :=
  evalSrcFnCfgOpenSearchPolicyProgram p.policyProg

def evalMirFnCfgOpenSearchAdaptivePolicyProgram
    (p : MirFnCfgOpenSearchAdaptivePolicyProgram) : PriorityTrace :=
  evalMirFnCfgOpenSearchPolicyProgram p.policyProg

def evalRFnCfgOpenSearchAdaptivePolicyProgram
    (p : RFnCfgOpenSearchAdaptivePolicyProgram) : PriorityTrace :=
  evalRFnCfgOpenSearchPolicyProgram p.policyProg

def srcOpenSearchAdaptivePolicyWitness (p : SrcFnCfgOpenSearchAdaptivePolicyProgram) : Prop :=
  srcOpenSearchPolicyWitness p.policyProg ∧
    p.recomputedRules = recomputeOpenSearchRules p.baseRules p.feedback

def mirOpenSearchAdaptivePolicyWitness (p : MirFnCfgOpenSearchAdaptivePolicyProgram) : Prop :=
  mirOpenSearchPolicyWitness p.policyProg ∧
    p.recomputedRules = recomputeOpenSearchRules p.baseRules p.feedback

def rOpenSearchAdaptivePolicyWitness (p : RFnCfgOpenSearchAdaptivePolicyProgram) : Prop :=
  rOpenSearchPolicyWitness p.policyProg ∧
    p.recomputedRules = recomputeOpenSearchRules p.baseRules p.feedback

theorem lowerFnCfgOpenSearchAdaptivePolicyProgram_preserves_meta
    (p : SrcFnCfgOpenSearchAdaptivePolicyProgram) :
    (lowerFnCfgOpenSearchAdaptivePolicyProgram p).policyProg.priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.policyProg.priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (lowerFnCfgOpenSearchAdaptivePolicyProgram p).baseRules = p.baseRules ∧
      (lowerFnCfgOpenSearchAdaptivePolicyProgram p).feedback = p.feedback ∧
      (lowerFnCfgOpenSearchAdaptivePolicyProgram p).recomputedRules = p.recomputedRules := by
  constructor
  · simpa [lowerFnCfgOpenSearchAdaptivePolicyProgram] using
      (lowerFnCfgOpenSearchPolicyProgram_preserves_meta p.policyProg).1
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem emitRFnCfgOpenSearchAdaptivePolicyProgram_preserves_meta
    (p : MirFnCfgOpenSearchAdaptivePolicyProgram) :
    (emitRFnCfgOpenSearchAdaptivePolicyProgram p).policyProg.priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length =
        p.policyProg.priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length ∧
      (emitRFnCfgOpenSearchAdaptivePolicyProgram p).baseRules = p.baseRules ∧
      (emitRFnCfgOpenSearchAdaptivePolicyProgram p).feedback = p.feedback ∧
      (emitRFnCfgOpenSearchAdaptivePolicyProgram p).recomputedRules = p.recomputedRules := by
  constructor
  · simpa [emitRFnCfgOpenSearchAdaptivePolicyProgram] using
      (emitRFnCfgOpenSearchPolicyProgram_preserves_meta p.policyProg).1
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem lowerFnCfgOpenSearchAdaptivePolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchAdaptivePolicyProgram) :
    evalMirFnCfgOpenSearchAdaptivePolicyProgram (lowerFnCfgOpenSearchAdaptivePolicyProgram p) =
      evalSrcFnCfgOpenSearchAdaptivePolicyProgram p := by
  rfl

theorem emitRFnCfgOpenSearchAdaptivePolicyProgram_preserves_eval
    (p : MirFnCfgOpenSearchAdaptivePolicyProgram) :
    evalRFnCfgOpenSearchAdaptivePolicyProgram (emitRFnCfgOpenSearchAdaptivePolicyProgram p) =
      evalMirFnCfgOpenSearchAdaptivePolicyProgram p := by
  rfl

theorem lowerEmitFnCfgOpenSearchAdaptivePolicyProgram_preserves_eval
    (p : SrcFnCfgOpenSearchAdaptivePolicyProgram) :
    evalRFnCfgOpenSearchAdaptivePolicyProgram
        (emitRFnCfgOpenSearchAdaptivePolicyProgram (lowerFnCfgOpenSearchAdaptivePolicyProgram p)) =
      evalSrcFnCfgOpenSearchAdaptivePolicyProgram p := by
  rfl

theorem lowerFnCfgOpenSearchAdaptivePolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchAdaptivePolicyProgram) :
    srcOpenSearchAdaptivePolicyWitness p →
      mirOpenSearchAdaptivePolicyWitness (lowerFnCfgOpenSearchAdaptivePolicyProgram p) := by
  intro h
  rcases h with ⟨hPolicy, hRules⟩
  constructor
  · exact lowerFnCfgOpenSearchPolicyProgram_preserves_witness _ hPolicy
  · simpa [lowerFnCfgOpenSearchAdaptivePolicyProgram] using hRules

theorem emitRFnCfgOpenSearchAdaptivePolicyProgram_preserves_witness
    (p : MirFnCfgOpenSearchAdaptivePolicyProgram) :
    mirOpenSearchAdaptivePolicyWitness p →
      rOpenSearchAdaptivePolicyWitness (emitRFnCfgOpenSearchAdaptivePolicyProgram p) := by
  intro h
  rcases h with ⟨hPolicy, hRules⟩
  constructor
  · exact emitRFnCfgOpenSearchPolicyProgram_preserves_witness _ hPolicy
  · simpa [emitRFnCfgOpenSearchAdaptivePolicyProgram] using hRules

theorem lowerEmitFnCfgOpenSearchAdaptivePolicyProgram_preserves_witness
    (p : SrcFnCfgOpenSearchAdaptivePolicyProgram) :
    srcOpenSearchAdaptivePolicyWitness p →
      rOpenSearchAdaptivePolicyWitness
        (emitRFnCfgOpenSearchAdaptivePolicyProgram (lowerFnCfgOpenSearchAdaptivePolicyProgram p)) := by
  intro h
  exact emitRFnCfgOpenSearchAdaptivePolicyProgram_preserves_witness _
    (lowerFnCfgOpenSearchAdaptivePolicyProgram_preserves_witness _ h)

def stableFnCfgOpenSearchAdaptivePolicyProgram : SrcFnCfgOpenSearchAdaptivePolicyProgram :=
  { policyProg := stableFnCfgOpenSearchPolicyProgram
  , baseRules := [(5, 9), (3, 9)]
  , feedback := [3, 1]
  , recomputedRules := [(5, 3), (3, 1)]
  }

theorem stableFnCfgOpenSearchAdaptivePolicyProgram_meta_preserved :
    (lowerFnCfgOpenSearchAdaptivePolicyProgram stableFnCfgOpenSearchAdaptivePolicyProgram).policyProg.priorityProg.schedProg.dynProg.openProg.haltProg.protocolProg.summaryProg.rounds.length = 2 ∧
      (lowerFnCfgOpenSearchAdaptivePolicyProgram stableFnCfgOpenSearchAdaptivePolicyProgram).baseRules = [(5, 9), (3, 9)] ∧
      (lowerFnCfgOpenSearchAdaptivePolicyProgram stableFnCfgOpenSearchAdaptivePolicyProgram).feedback = [3, 1] ∧
      (lowerFnCfgOpenSearchAdaptivePolicyProgram stableFnCfgOpenSearchAdaptivePolicyProgram).recomputedRules = [(5, 3), (3, 1)] := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem stableFnCfgOpenSearchAdaptivePolicyProgram_src_witness :
    srcOpenSearchAdaptivePolicyWitness stableFnCfgOpenSearchAdaptivePolicyProgram := by
  constructor
  · exact stableFnCfgOpenSearchPolicyProgram_src_witness
  · rfl

theorem stableFnCfgOpenSearchAdaptivePolicyProgram_eval_preserved :
    evalRFnCfgOpenSearchAdaptivePolicyProgram
      (emitRFnCfgOpenSearchAdaptivePolicyProgram
        (lowerFnCfgOpenSearchAdaptivePolicyProgram stableFnCfgOpenSearchAdaptivePolicyProgram)) =
      stableClosedLoopSummary := by
  rfl

theorem stableFnCfgOpenSearchAdaptivePolicyProgram_preserved :
    rOpenSearchAdaptivePolicyWitness
      (emitRFnCfgOpenSearchAdaptivePolicyProgram
        (lowerFnCfgOpenSearchAdaptivePolicyProgram stableFnCfgOpenSearchAdaptivePolicyProgram)) := by
  exact lowerEmitFnCfgOpenSearchAdaptivePolicyProgram_preserves_witness _
    stableFnCfgOpenSearchAdaptivePolicyProgram_src_witness

end RRProofs
