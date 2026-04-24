import RRProofs.VectorizeApplySubset

namespace RRProofs

structure ReducedRewriteSite where
  plan : ReducedVectorPlan
  applyTaken : Bool
deriving Repr

def originalExitValue (site : ReducedRewriteSite) : Int :=
  scalarResult site.plan

def rewrittenExitPhiValue (site : ReducedRewriteSite) : Int :=
  if site.applyTaken then vectorResult site.plan else scalarResult site.plan

theorem rewrittenExitPhiValue_fallback_eq_original (site : ReducedRewriteSite)
    (h : site.applyTaken = false) :
    rewrittenExitPhiValue site = originalExitValue site := by
  simp [rewrittenExitPhiValue, originalExitValue, h]

theorem rewrittenExitPhiValue_apply_eq_original
    (site : ReducedRewriteSite)
    (hApply : site.applyTaken = true)
    (hPres : resultPreserving site.plan) :
    rewrittenExitPhiValue site = originalExitValue site := by
  simp [rewrittenExitPhiValue, originalExitValue, hApply, resultPreserving] at *
  simp [hPres]

theorem rewrittenExitPhiValue_preserved_if_resultPreserving
    (site : ReducedRewriteSite)
    (hPres : resultPreserving site.plan) :
    rewrittenExitPhiValue site = originalExitValue site := by
  cases hApply : site.applyTaken with
  | false =>
      exact rewrittenExitPhiValue_fallback_eq_original site hApply
  | true =>
      exact rewrittenExitPhiValue_apply_eq_original site hApply hPres

def fallbackRewriteCase : ReducedRewriteSite :=
  { plan := rejectExprMapCase, applyTaken := false }

def applyRewriteCase : ReducedRewriteSite :=
  { plan := pureExprMapCase, applyTaken := true }

def condFallbackRewriteCase : ReducedRewriteSite :=
  { plan := rejectCondBranchCase, applyTaken := false }

def condApplyRewriteCase : ReducedRewriteSite :=
  { plan := storeOnlyCondCase, applyTaken := true }

theorem fallbackRewriteCase_preserved :
    rewrittenExitPhiValue fallbackRewriteCase = originalExitValue fallbackRewriteCase := by
  exact rewrittenExitPhiValue_fallback_eq_original _ rfl

theorem applyRewriteCase_preserved :
    rewrittenExitPhiValue applyRewriteCase = originalExitValue applyRewriteCase := by
  apply rewrittenExitPhiValue_apply_eq_original
  · rfl
  · rfl

theorem condFallbackRewriteCase_preserved :
    rewrittenExitPhiValue condFallbackRewriteCase = originalExitValue condFallbackRewriteCase := by
  exact rewrittenExitPhiValue_fallback_eq_original _ rfl

theorem condApplyRewriteCase_preserved :
    rewrittenExitPhiValue condApplyRewriteCase = originalExitValue condApplyRewriteCase := by
  apply rewrittenExitPhiValue_apply_eq_original
  · rfl
  · rfl

end RRProofs
