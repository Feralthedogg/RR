import RRProofs.MirSemanticsLite

namespace RRProofs.FreshAliasRewriteSoundness

open RRProofs.MirSemanticsLite

def renameAliasExpr (alias source : String) : MirExpr -> MirExpr
  | .const v => .const v
  | .load name => if name = alias then .load source else .load name
  | .add lhs rhs => .add (renameAliasExpr alias source lhs) (renameAliasExpr alias source rhs)
  | .mul lhs rhs => .mul (renameAliasExpr alias source lhs) (renameAliasExpr alias source rhs)
  | .neg arg => .neg (renameAliasExpr alias source arg)
  | .lt lhs rhs => .lt (renameAliasExpr alias source lhs) (renameAliasExpr alias source rhs)

def envAliasAgrees (env : Env) (alias source : String) : Prop :=
  lookupEnv env alias = lookupEnv env source

theorem rename_alias_expr_preserves_eval
    (env : Env) (alias source : String) (expr : MirExpr)
    (hAgree : envAliasAgrees env alias source) :
    evalExpr env (renameAliasExpr alias source expr) = evalExpr env expr := by
  induction expr with
  | const v =>
      simp [renameAliasExpr, evalExpr]
  | load name =>
      by_cases hName : name = alias
      · subst hName
        simp [renameAliasExpr, evalExpr]
        exact hAgree.symm
      · simp [renameAliasExpr, evalExpr, hName]
  | add lhs rhs ihL ihR =>
      simp [renameAliasExpr, evalExpr, ihL, ihR]
  | mul lhs rhs ihL ihR =>
      simp [renameAliasExpr, evalExpr, ihL, ihR]
  | neg arg ih =>
      simp [renameAliasExpr, evalExpr, ih]
  | lt lhs rhs ihL ihR =>
      simp [renameAliasExpr, evalExpr, ihL, ihR]

end RRProofs.FreshAliasRewriteSoundness
