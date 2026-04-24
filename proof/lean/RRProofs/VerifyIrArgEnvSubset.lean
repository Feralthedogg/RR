import RRProofs.VerifyIrArgListTraversalSubset
import RRProofs.LoweringSubset

namespace RRProofs

abbrev ArgValueId := Nat
abbrev ArgBlockId := Nat
abbrev ArgValueEnv := ArgValueId -> Option RValue

inductive ArgEnvExpr where
  | const : RValue -> ArgEnvExpr
  | use : ArgValueId -> ArgEnvExpr
  | add : ArgEnvExpr -> ArgEnvExpr -> ArgEnvExpr
  | field : ArgEnvExpr -> String -> ArgEnvExpr
deriving Repr

abbrev ArgEnvField := String × ArgEnvExpr

def evalArgEnvExpr (env : ArgValueEnv) : ArgEnvExpr -> Option RValue
  | .const v => some v
  | .use vid => env vid
  | .add lhs rhs =>
      match evalArgEnvExpr env lhs, evalArgEnvExpr env rhs with
      | some (.int l), some (.int r) => some (.int (l + r))
      | _, _ => none
  | .field base name =>
      match evalArgEnvExpr env base with
      | some (.record fields) => lookupField fields name
      | _ => none

def rewriteArgPhiUse (phi arg : ArgValueId) : ArgEnvExpr -> ArgEnvExpr
  | .const v => .const v
  | .use vid => .use (if vid = phi then arg else vid)
  | .add lhs rhs => .add (rewriteArgPhiUse phi arg lhs) (rewriteArgPhiUse phi arg rhs)
  | .field base name => .field (rewriteArgPhiUse phi arg base) name

def mergedArgEnv (env : ArgValueEnv) (phi arg : ArgValueId) : ArgValueEnv :=
  fun vid => if vid = phi then env arg else env vid

def argPhiSelect (edges : List (ArgValueId × ArgBlockId)) (pred : ArgBlockId) : Option ArgValueId :=
  match edges.find? (fun entry => entry.2 = pred) with
  | some (vid, _) => some vid
  | none => none

def evalArgEnvExprList (env : ArgValueEnv) : List ArgEnvExpr -> Option (List RValue)
  | [] => some []
  | e :: rest =>
      match evalArgEnvExpr env e, evalArgEnvExprList env rest with
      | some v, some vs => some (v :: vs)
      | _, _ => none

def rewriteArgPhiUseList (phi arg : ArgValueId) (es : List ArgEnvExpr) : List ArgEnvExpr :=
  es.map (rewriteArgPhiUse phi arg)

def evalArgEnvFields (env : ArgValueEnv) : List ArgEnvField -> Option (List (String × RValue))
  | [] => some []
  | (name, e) :: rest =>
      match evalArgEnvExpr env e, evalArgEnvFields env rest with
      | some v, some vs => some ((name, v) :: vs)
      | _, _ => none

def rewriteArgPhiUseFields (phi arg : ArgValueId) (fs : List ArgEnvField) : List ArgEnvField :=
  fs.map (fun p => (p.1, rewriteArgPhiUse phi arg p.2))

theorem evalArgEnvExpr_rewriteArgPhiUse
    (env : ArgValueEnv) (phi arg : ArgValueId) :
    ∀ e,
      evalArgEnvExpr (mergedArgEnv env phi arg) e =
        evalArgEnvExpr env (rewriteArgPhiUse phi arg e)
  | .const _ => rfl
  | .use vid => by
      by_cases h : vid = phi <;> simp [evalArgEnvExpr, rewriteArgPhiUse, mergedArgEnv, h]
  | .add lhs rhs => by
      simp [evalArgEnvExpr, rewriteArgPhiUse,
        evalArgEnvExpr_rewriteArgPhiUse env phi arg lhs,
        evalArgEnvExpr_rewriteArgPhiUse env phi arg rhs]
  | .field base name => by
      simp [evalArgEnvExpr, rewriteArgPhiUse,
        evalArgEnvExpr_rewriteArgPhiUse env phi arg base]

theorem evalArgEnvExprList_rewriteArgPhiUse
    (env : ArgValueEnv) (phi arg : ArgValueId) :
    ∀ es,
      evalArgEnvExprList (mergedArgEnv env phi arg) es =
        evalArgEnvExprList env (rewriteArgPhiUseList phi arg es)
  | [] => rfl
  | e :: rest => by
      simp [evalArgEnvExprList, rewriteArgPhiUseList,
        evalArgEnvExpr_rewriteArgPhiUse env phi arg e,
        evalArgEnvExprList_rewriteArgPhiUse env phi arg rest]

theorem evalArgEnvFields_rewriteArgPhiUse
    (env : ArgValueEnv) (phi arg : ArgValueId) :
    ∀ fs,
      evalArgEnvFields (mergedArgEnv env phi arg) fs =
        evalArgEnvFields env (rewriteArgPhiUseFields phi arg fs)
  | [] => rfl
  | (name, e) :: rest => by
      simp [evalArgEnvFields, rewriteArgPhiUseFields,
        evalArgEnvExpr_rewriteArgPhiUse env phi arg e,
        evalArgEnvFields_rewriteArgPhiUse env phi arg rest]

theorem evalArgEnvExprList_after_phi_edge
    (env : ArgValueEnv) (phi arg pred : ArgValueId) (edges : List (ArgValueId × ArgBlockId))
    (es : List ArgEnvExpr)
    (_hSel : argPhiSelect edges pred = some arg) :
    evalArgEnvExprList (mergedArgEnv env phi arg) es =
      evalArgEnvExprList env (rewriteArgPhiUseList phi arg es) := by
  simpa using evalArgEnvExprList_rewriteArgPhiUse env phi arg es

theorem evalArgEnvFields_after_phi_edge
    (env : ArgValueEnv) (phi arg pred : ArgValueId) (edges : List (ArgValueId × ArgBlockId))
    (fs : List ArgEnvField)
    (_hSel : argPhiSelect edges pred = some arg) :
    evalArgEnvFields (mergedArgEnv env phi arg) fs =
      evalArgEnvFields env (rewriteArgPhiUseFields phi arg fs) := by
  simpa using evalArgEnvFields_rewriteArgPhiUse env phi arg fs

def exampleArgEnv : ArgValueEnv
  | 1 => some (.int 4)
  | 3 => some (.int 5)
  | 7 => some (.record [("x", .int 9)])
  | _ => none

def exampleArgPhiArgs : List (ArgValueId × ArgBlockId) :=
  [(1, 0), (3, 1)]

def exampleArgFieldPhiArgs : List (ArgValueId × ArgBlockId) :=
  [(7, 2)]

def exampleCallArgEnvExprs : List ArgEnvExpr :=
  [ .use 9
  , .add (.use 9) (.const (.int 3))
  ]

def exampleRecordArgEnvFields : List ArgEnvField :=
  [ ("a", .use 12)
  , ("b", .field (.use 12) "x")
  ]

theorem exampleArgPhiSelect_zero :
    argPhiSelect exampleArgPhiArgs 0 = some 1 := by
  rfl

theorem exampleArgFieldPhiSelect :
    argPhiSelect exampleArgFieldPhiArgs 2 = some 7 := by
  rfl

theorem exampleCallArgEnvExprs_preserved_on_selected_edge :
    evalArgEnvExprList (mergedArgEnv exampleArgEnv 9 1) exampleCallArgEnvExprs =
      evalArgEnvExprList exampleArgEnv (rewriteArgPhiUseList 9 1 exampleCallArgEnvExprs) := by
  simpa using
    evalArgEnvExprList_after_phi_edge exampleArgEnv 9 1 0 exampleArgPhiArgs
      exampleCallArgEnvExprs exampleArgPhiSelect_zero

theorem exampleCallArgEnvExprs_value :
    evalArgEnvExprList (mergedArgEnv exampleArgEnv 9 1) exampleCallArgEnvExprs =
      some [.int 4, .int 7] := by
  rfl

theorem exampleRecordArgEnvFields_preserved_on_selected_edge :
    evalArgEnvFields (mergedArgEnv exampleArgEnv 12 7) exampleRecordArgEnvFields =
      evalArgEnvFields exampleArgEnv (rewriteArgPhiUseFields 12 7 exampleRecordArgEnvFields) := by
  simpa using
    evalArgEnvFields_after_phi_edge exampleArgEnv 12 7 2 exampleArgFieldPhiArgs
      exampleRecordArgEnvFields exampleArgFieldPhiSelect

theorem exampleRecordArgEnvFields_value :
    evalArgEnvFields (mergedArgEnv exampleArgEnv 12 7) exampleRecordArgEnvFields =
      some [("a", .record [("x", .int 9)]), ("b", .int 9)] := by
  simp [evalArgEnvFields, evalArgEnvExpr, mergedArgEnv, exampleArgEnv,
    exampleRecordArgEnvFields, lookupField]

end RRProofs
