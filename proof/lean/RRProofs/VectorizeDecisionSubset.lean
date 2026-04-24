import RRProofs.VectorizeUseRewriteSubset
import RRProofs.VectorizeOriginMemoSubset

namespace RRProofs

structure RewriteDecisionState where
  node : TinyNode
  use : ReachableUse
  memo : TinyMemo
  nextId : TinyValueId
  changed : Bool

def decisionBaseId (target : String) (replacementId : TinyValueId)
    (s : RewriteDecisionState) : TinyValueId :=
  boundaryRewrite target replacementId s.node

def decisionChosenId (target : String) (replacementId : TinyValueId)
    (s : RewriteDecisionState) : TinyValueId :=
  memoizedResult s.memo
    (decisionBaseId target replacementId s)
    (allocateRewriteId s.nextId s.node.id s.changed)

def rewriteUseWithDecision
    (target : VarName)
    (replacementExpr : RewriteExpr)
    (replacementId : TinyValueId)
    (s : RewriteDecisionState) : TinyValueId × ReachableUse :=
  (decisionChosenId target replacementId s, rewriteReachableUse target replacementExpr s.use)

theorem decisionChosenId_memo_hit
    (target : String)
    (replacementId mapped : TinyValueId)
    (s : RewriteDecisionState)
    (h : s.memo (decisionBaseId target replacementId s) = some mapped) :
    decisionChosenId target replacementId s = mapped := by
  unfold decisionChosenId
  exact memoizedResult_hit_reuses s.memo _ _ _ h

theorem decisionChosenId_unchanged_root
    (target : String)
    (replacementId : TinyValueId)
    (s : RewriteDecisionState)
    (hBoundary : decisionBaseId target replacementId s = s.node.id)
    (hMiss : s.memo s.node.id = none)
    (hChanged : s.changed = false) :
    decisionChosenId target replacementId s = s.node.id := by
  unfold decisionChosenId
  rw [hBoundary]
  rw [memoizedResult_miss_uses_computed _ _ _ hMiss]
  simpa [hChanged] using allocateRewriteId_unchanged_reuses_root s.nextId s.node.id

theorem rewriteUseWithDecision_preserves_eval
    (ρ : ValEnv)
    (target : VarName)
    (replacementExpr : RewriteExpr)
    (replacementId : TinyValueId)
    (s : RewriteDecisionState)
    (hPres : evalRewriteExpr ρ replacementExpr = evalRewriteExpr ρ (.load target)) :
    evalReachableUse ρ (rewriteUseWithDecision target replacementExpr replacementId s).2 =
      evalReachableUse ρ s.use := by
  simp [rewriteUseWithDecision]
  exact rewriteReachableUse_preserves_eval ρ target replacementExpr s.use hPres

def sampleDecisionState : RewriteDecisionState :=
  { node := { id := 4, originVar? := some "dest", kind := .other }
  , use := { id := 0, expr := .add (.load "dest") (.constInt 3) }
  , memo := fun _ => none
  , nextId := 9
  , changed := true
  }

theorem sampleDecision_chosen_fresh :
    decisionChosenId "dest" 7 sampleDecisionState = 9 := by
  simp [decisionChosenId, decisionBaseId, sampleDecisionState, boundaryRewrite,
    memoizedResult, allocateRewriteId]

theorem sampleDecision_use_preserved :
    evalReachableUse sampleUseEnv
      (rewriteUseWithDecision "dest" sampleReplacementUse 7 sampleDecisionState).2 = 10 := by
  apply rewriteUseWithDecision_preserves_eval
  simp [sampleUseEnv, sampleReplacementUse, evalRewriteExpr]

end RRProofs
