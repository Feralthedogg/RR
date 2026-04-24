namespace RRProofs

abbrev VarName := String
abbrev ValEnv := VarName -> Int

inductive RewriteExpr where
  | constInt : Int -> RewriteExpr
  | load : VarName -> RewriteExpr
  | add : RewriteExpr -> RewriteExpr -> RewriteExpr
deriving Repr

def evalRewriteExpr (ρ : ValEnv) : RewriteExpr -> Int
  | .constInt i => i
  | .load v => ρ v
  | .add lhs rhs => evalRewriteExpr ρ lhs + evalRewriteExpr ρ rhs

def rewriteLoadsForVar (target : VarName) (replacement : RewriteExpr) : RewriteExpr -> RewriteExpr
  | .constInt i => .constInt i
  | .load v => if v = target then replacement else .load v
  | .add lhs rhs =>
      .add (rewriteLoadsForVar target replacement lhs) (rewriteLoadsForVar target replacement rhs)

theorem rewriteLoadsForVar_preserves_eval
    (ρ : ValEnv)
    (target : VarName)
    (replacement expr : RewriteExpr)
    (hPres : evalRewriteExpr ρ replacement = evalRewriteExpr ρ (.load target)) :
    evalRewriteExpr ρ (rewriteLoadsForVar target replacement expr) = evalRewriteExpr ρ expr := by
  induction expr with
  | constInt i =>
      simp [rewriteLoadsForVar, evalRewriteExpr]
  | load v =>
      by_cases hEq : v = target
      · simp [rewriteLoadsForVar, evalRewriteExpr, hEq]
        simpa [evalRewriteExpr] using hPres
      · simp [rewriteLoadsForVar, evalRewriteExpr, hEq]
  | add lhs rhs ihL ihR =>
      simp [rewriteLoadsForVar, evalRewriteExpr, ihL, ihR]

def originalReturn (ρ : ValEnv) (ret : RewriteExpr) : Int :=
  evalRewriteExpr ρ ret

def rewrittenReturn (ρ : ValEnv) (target : VarName) (replacement ret : RewriteExpr) : Int :=
  evalRewriteExpr ρ (rewriteLoadsForVar target replacement ret)

theorem rewrittenReturn_preserves_original
    (ρ : ValEnv)
    (target : VarName)
    (replacement ret : RewriteExpr)
    (hPres : evalRewriteExpr ρ replacement = evalRewriteExpr ρ (.load target)) :
    rewrittenReturn ρ target replacement ret = originalReturn ρ ret := by
  unfold rewrittenReturn originalReturn
  exact rewriteLoadsForVar_preserves_eval ρ target replacement ret hPres

def sampleRet : RewriteExpr := .add (.load "x") (.constInt 3)
def sampleReplacement : RewriteExpr := .constInt 7
def sampleEnv : ValEnv := fun
  | "x" => 7
  | _ => 0

theorem sampleRet_preserved :
    rewrittenReturn sampleEnv "x" sampleReplacement sampleRet = 10 := by
  apply rewrittenReturn_preserves_original
  simp [sampleEnv, sampleReplacement, evalRewriteExpr]

end RRProofs
