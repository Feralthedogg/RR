import RRProofs.VectorizeMirRewriteSubset
import RRProofs.VectorizeValueRewriteSubset

namespace RRProofs

open RRProofs.VectorizeMirRewriteSubset

def graphTargetVar : VarName := "dest"

def graphEnv (site : ReducedRewriteSite) : ValEnv := fun
  | "dest" => runOriginal site
  | _ => 0

def graphReplacement (site : ReducedRewriteSite) : RewriteExpr :=
  .constInt ((runRewrite site).exitValue?.getD 0)

theorem graphReturn_preserved
    (site : ReducedRewriteSite)
    (ret : RewriteExpr)
    (hRun : (runRewrite site).exitValue? = some (runOriginal site)) :
    rewrittenReturn (graphEnv site) graphTargetVar (graphReplacement site) ret =
      originalReturn (graphEnv site) ret := by
  apply rewrittenReturn_preserves_original
  simp [graphEnv, graphReplacement, graphTargetVar, evalRewriteExpr, hRun]

def graphRet : RewriteExpr := .add (.load graphTargetVar) (.constInt 3)

theorem mirFallbackCase_graph_preserved :
    rewrittenReturn (graphEnv mirFallbackCase) graphTargetVar (graphReplacement mirFallbackCase) graphRet = 10 := by
  apply graphReturn_preserved
  exact mirFallbackCase_preserved

theorem mirApplyCase_graph_preserved :
    rewrittenReturn (graphEnv mirApplyCase) graphTargetVar (graphReplacement mirApplyCase) graphRet = 10 := by
  apply graphReturn_preserved
  exact mirApplyCase_preserved

theorem mirCondFallbackCase_graph_preserved :
    rewrittenReturn (graphEnv mirCondFallbackCase) graphTargetVar (graphReplacement mirCondFallbackCase) graphRet = 7 := by
  apply graphReturn_preserved
  exact mirCondFallbackCase_preserved

theorem mirCondApplyCase_graph_preserved :
    rewrittenReturn (graphEnv mirCondApplyCase) graphTargetVar (graphReplacement mirCondApplyCase) graphRet = 7 := by
  apply graphReturn_preserved
  exact mirCondApplyCase_preserved

end RRProofs
