import RRProofs.VectorizeValueRewriteSubset

namespace RRProofs

abbrev UseId := Nat

structure ReachableUse where
  id : UseId
  expr : RewriteExpr
deriving Repr

def evalReachableUse (ρ : ValEnv) (u : ReachableUse) : Int :=
  evalRewriteExpr ρ u.expr

def rewriteReachableUse (target : VarName) (replacement : RewriteExpr) (u : ReachableUse) :
    ReachableUse :=
  { u with expr := rewriteLoadsForVar target replacement u.expr }

def evalReachableUses (ρ : ValEnv) (uses : List ReachableUse) : List Int :=
  uses.map (evalReachableUse ρ)

def rewriteReachableUses (target : VarName) (replacement : RewriteExpr)
    (uses : List ReachableUse) : List ReachableUse :=
  uses.map (rewriteReachableUse target replacement)

theorem rewriteReachableUse_preserves_eval
    (ρ : ValEnv)
    (target : VarName)
    (replacement : RewriteExpr)
    (u : ReachableUse)
    (hPres : evalRewriteExpr ρ replacement = evalRewriteExpr ρ (.load target)) :
    evalReachableUse ρ (rewriteReachableUse target replacement u) = evalReachableUse ρ u := by
  simp [evalReachableUse, rewriteReachableUse]
  exact rewriteLoadsForVar_preserves_eval ρ target replacement u.expr hPres

theorem rewriteReachableUses_preserves_eval
    (ρ : ValEnv)
    (target : VarName)
    (replacement : RewriteExpr)
    (uses : List ReachableUse)
    (hPres : evalRewriteExpr ρ replacement = evalRewriteExpr ρ (.load target)) :
    evalReachableUses ρ (rewriteReachableUses target replacement uses) =
      evalReachableUses ρ uses := by
  unfold evalReachableUses rewriteReachableUses
  induction uses with
  | nil =>
      rfl
  | cons u rest ih =>
      simp [rewriteReachableUse_preserves_eval ρ target replacement u hPres, ih]

def sampleUseEnv : ValEnv := fun
  | "dest" => 7
  | _ => 0

def sampleReplacementUse : RewriteExpr := .constInt 7

def sampleUses : List ReachableUse :=
  [ { id := 0, expr := .load "dest" }
  , { id := 1, expr := .add (.load "dest") (.constInt 3) }
  ]

theorem sampleUses_preserved :
    evalReachableUses sampleUseEnv
      (rewriteReachableUses "dest" sampleReplacementUse sampleUses)
    = [7, 10] := by
  apply rewriteReachableUses_preserves_eval
  simp [sampleUseEnv, sampleReplacementUse, evalRewriteExpr]

end RRProofs
