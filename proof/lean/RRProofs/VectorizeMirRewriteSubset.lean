import RRProofs.VectorizeRewriteSubset

namespace RRProofs.VectorizeMirRewriteSubset

inductive TinyPc where
  | preheader
  | apply
  | fallback
  | exit
  | done
deriving Repr

structure TinyRewriteState where
  site : ReducedRewriteSite
  scalarSlot : Int
  vectorSlot : Int
  incomingApply? : Option Bool
  exitValue? : Option Int
  pc : TinyPc
deriving Repr

def initialRewriteState (site : ReducedRewriteSite) : TinyRewriteState :=
  { site := site
  , scalarSlot := originalExitValue site
  , vectorSlot := vectorResult site.plan
  , incomingApply? := none
  , exitValue? := none
  , pc := .preheader
  }

def stepRewrite (s : TinyRewriteState) : TinyRewriteState :=
  match s.pc with
  | .preheader =>
      if s.site.applyTaken then { s with pc := .apply } else { s with pc := .fallback }
  | .apply =>
      { s with incomingApply? := some true, pc := .exit }
  | .fallback =>
      { s with incomingApply? := some false, pc := .exit }
  | .exit =>
      let merged :=
        match s.incomingApply? with
        | some true => s.vectorSlot
        | _ => s.scalarSlot
      { s with exitValue? := some merged, pc := .done }
  | .done => s

def runRewrite (site : ReducedRewriteSite) : TinyRewriteState :=
  stepRewrite (stepRewrite (stepRewrite (initialRewriteState site)))

def runOriginal (site : ReducedRewriteSite) : Int :=
  originalExitValue site

theorem runRewrite_fallback_preserves_original
    (site : ReducedRewriteSite)
    (h : site.applyTaken = false) :
    (runRewrite site).exitValue? = some (runOriginal site) := by
  simp [runRewrite, initialRewriteState, stepRewrite, runOriginal, originalExitValue, h]

theorem runRewrite_apply_preserves_original
    (site : ReducedRewriteSite)
    (hApply : site.applyTaken = true)
    (hPres : resultPreserving site.plan) :
    (runRewrite site).exitValue? = some (runOriginal site) := by
  simp [runRewrite, initialRewriteState, stepRewrite, runOriginal, originalExitValue, hApply]
  simpa [resultPreserving] using hPres

theorem runRewrite_preserves_original_if_resultPreserving
    (site : ReducedRewriteSite)
    (h : site.applyTaken = false ∨ resultPreserving site.plan) :
    (runRewrite site).exitValue? = some (runOriginal site) := by
  cases h with
  | inl hFallback =>
      exact runRewrite_fallback_preserves_original site hFallback
  | inr hPres =>
      cases hApply : site.applyTaken with
      | false =>
          exact runRewrite_fallback_preserves_original site hApply
      | true =>
          exact runRewrite_apply_preserves_original site hApply hPres

def mirFallbackCase : ReducedRewriteSite := fallbackRewriteCase
def mirApplyCase : ReducedRewriteSite := applyRewriteCase
def mirCondFallbackCase : ReducedRewriteSite := condFallbackRewriteCase
def mirCondApplyCase : ReducedRewriteSite := condApplyRewriteCase

theorem mirFallbackCase_preserved :
    (runRewrite mirFallbackCase).exitValue? = some 7 := by
  exact runRewrite_fallback_preserves_original _ rfl

theorem mirApplyCase_preserved :
    (runRewrite mirApplyCase).exitValue? = some 7 := by
  exact runRewrite_apply_preserves_original _ rfl rfl

theorem mirCondFallbackCase_preserved :
    (runRewrite mirCondFallbackCase).exitValue? = some 4 := by
  exact runRewrite_fallback_preserves_original _ rfl

theorem mirCondApplyCase_preserved :
    (runRewrite mirCondApplyCase).exitValue? = some 4 := by
  exact runRewrite_apply_preserves_original _ rfl rfl

end RRProofs.VectorizeMirRewriteSubset
