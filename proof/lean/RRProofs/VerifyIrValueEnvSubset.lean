import RRProofs.LoweringSubset

namespace RRProofs.VerifyIrValueEnvSubset

abbrev ValueId := Nat
abbrev BlockId := Nat
abbrev ValueEnv := ValueId -> Option RValue

inductive EnvExpr where
  | const : RValue -> EnvExpr
  | use : ValueId -> EnvExpr
  | add : EnvExpr -> EnvExpr -> EnvExpr
  | field : EnvExpr -> String -> EnvExpr
deriving Repr

def evalEnvExpr (env : ValueEnv) : EnvExpr -> Option RValue
  | .const v => some v
  | .use vid => env vid
  | .add lhs rhs => do
      let lv <- evalEnvExpr env lhs
      let rv <- evalEnvExpr env rhs
      match lv, rv with
      | .int l, .int r => some (.int (l + r))
      | _, _ => none
  | .field base name => do
      let v <- evalEnvExpr env base
      match v with
      | .record fields => lookupField fields name
      | _ => none

def rewritePhiUse (phi arg : ValueId) : EnvExpr -> EnvExpr
  | .const v => .const v
  | .use vid => .use (if vid = phi then arg else vid)
  | .add lhs rhs => .add (rewritePhiUse phi arg lhs) (rewritePhiUse phi arg rhs)
  | .field base name => .field (rewritePhiUse phi arg base) name

def mergedEnv (env : ValueEnv) (phi arg : ValueId) : ValueEnv :=
  fun vid => if vid = phi then env arg else env vid

def phiSelect (edges : List (ValueId × BlockId)) (pred : BlockId) : Option ValueId :=
  match edges.find? (fun entry => entry.2 = pred) with
  | some (vid, _) => some vid
  | none => none

theorem eval_rewritePhiUse
    (env : ValueEnv) (phi arg : ValueId) :
    ∀ expr,
      evalEnvExpr (mergedEnv env phi arg) expr =
        evalEnvExpr env (rewritePhiUse phi arg expr)
  | .const _ => rfl
  | .use vid => by
      by_cases h : vid = phi <;> simp [evalEnvExpr, rewritePhiUse, mergedEnv, h]
  | .add lhs rhs => by
      simp [evalEnvExpr, rewritePhiUse, eval_rewritePhiUse env phi arg lhs,
        eval_rewritePhiUse env phi arg rhs]
  | .field base name => by
      simp [evalEnvExpr, rewritePhiUse, eval_rewritePhiUse env phi arg base]

theorem eval_after_phi_edge
    (env : ValueEnv) (phi arg pred : ValueId) (edges : List (ValueId × BlockId))
    (expr : EnvExpr)
    (_hSel : phiSelect edges pred = some arg) :
    evalEnvExpr (mergedEnv env phi arg) expr =
      evalEnvExpr env (rewritePhiUse phi arg expr) := by
  simpa using eval_rewritePhiUse env phi arg expr

def exampleEnv : ValueEnv
  | 1 => some (.int 4)
  | 3 => some (.int 5)
  | 7 => some (.record [("x", .int 9)])
  | _ => none

def examplePhiArgs : List (ValueId × BlockId) :=
  [(1, 0), (3, 1)]

def exampleConsumer : EnvExpr :=
  .add (.use 9) (.const (.int 3))

def exampleFieldPhiArgs : List (ValueId × BlockId) :=
  [(7, 2)]

def exampleFieldConsumer : EnvExpr :=
  .field (.use 12) "x"

theorem examplePhiSelect_zero :
    phiSelect examplePhiArgs 0 = some 1 := by
  rfl

theorem exampleConsumer_preserved_on_selected_edge :
    evalEnvExpr (mergedEnv exampleEnv 9 1) exampleConsumer =
      evalEnvExpr exampleEnv (rewritePhiUse 9 1 exampleConsumer) := by
  simpa using
    eval_after_phi_edge exampleEnv 9 1 0 examplePhiArgs exampleConsumer examplePhiSelect_zero

theorem exampleConsumer_value :
    evalEnvExpr (mergedEnv exampleEnv 9 1) exampleConsumer = some (.int 7) := by
  rfl

theorem exampleFieldPhiSelect :
    phiSelect exampleFieldPhiArgs 2 = some 7 := by
  rfl

theorem exampleFieldConsumer_preserved_on_selected_edge :
    evalEnvExpr (mergedEnv exampleEnv 12 7) exampleFieldConsumer =
      evalEnvExpr exampleEnv (rewritePhiUse 12 7 exampleFieldConsumer) := by
  simpa using
    eval_after_phi_edge exampleEnv 12 7 2 exampleFieldPhiArgs exampleFieldConsumer
      exampleFieldPhiSelect

theorem exampleFieldConsumer_value :
    evalEnvExpr (mergedEnv exampleEnv 12 7) exampleFieldConsumer = some (.int 9) := by
  rfl

end RRProofs.VerifyIrValueEnvSubset
