import RRProofs.LoweringSubset

namespace RRProofs

abbrev LetEnv := List (String × RValue)

inductive SrcLetExpr where
  | pure : SrcExpr -> SrcLetExpr
  | var : String -> SrcLetExpr
  | add : SrcLetExpr -> SrcLetExpr -> SrcLetExpr
  | field : SrcLetExpr -> String -> SrcLetExpr
  | let1 : String -> SrcExpr -> SrcLetExpr -> SrcLetExpr
deriving Repr

inductive MirLetExpr where
  | pure : MirExpr -> MirLetExpr
  | var : String -> MirLetExpr
  | add : MirLetExpr -> MirLetExpr -> MirLetExpr
  | field : MirLetExpr -> String -> MirLetExpr
  | let1 : String -> MirExpr -> MirLetExpr -> MirLetExpr
deriving Repr

def evalSrcLet : LetEnv -> SrcLetExpr -> Option RValue
  | _, .pure e => evalSrc e
  | env, .var name => lookupField env name
  | env, .add lhs rhs => do
      let lv <- evalSrcLet env lhs
      let rv <- evalSrcLet env rhs
      match lv, rv with
      | .int l, .int r => some (.int (l + r))
      | _, _ => none
  | env, .field base name => do
      let v <- evalSrcLet env base
      match v with
      | .record fields => lookupField fields name
      | _ => none
  | env, .let1 name bind body => do
      let v <- evalSrc bind
      evalSrcLet ((name, v) :: env) body

def evalMirLet : LetEnv -> MirLetExpr -> Option RValue
  | _, .pure e => evalMir e
  | env, .var name => lookupField env name
  | env, .add lhs rhs => do
      let lv <- evalMirLet env lhs
      let rv <- evalMirLet env rhs
      match lv, rv with
      | .int l, .int r => some (.int (l + r))
      | _, _ => none
  | env, .field base name => do
      let v <- evalMirLet env base
      match v with
      | .record fields => lookupField fields name
      | _ => none
  | env, .let1 name bind body => do
      let v <- evalMir bind
      evalMirLet ((name, v) :: env) body

def lowerLet : SrcLetExpr -> MirLetExpr
  | .pure e => .pure (lower e)
  | .var name => .var name
  | .add lhs rhs => .add (lowerLet lhs) (lowerLet rhs)
  | .field base name => .field (lowerLet base) name
  | .let1 name bind body => .let1 name (lower bind) (lowerLet body)

theorem lowerLet_preserves_eval
    (env : LetEnv) (expr : SrcLetExpr) :
    evalMirLet env (lowerLet expr) = evalSrcLet env expr := by
  induction expr generalizing env with
  | pure e =>
      simp [lowerLet, evalMirLet, evalSrcLet, lower_preserves_eval]
  | var name =>
      simp [lowerLet, evalMirLet, evalSrcLet]
  | add lhs rhs ihL ihR =>
      simp [lowerLet, evalMirLet, evalSrcLet, ihL, ihR]
  | field base name ih =>
      simp [lowerLet, evalMirLet, evalSrcLet, ih]
  | let1 name bind body ih =>
      simp [lowerLet, evalMirLet, evalSrcLet, lower_preserves_eval]
      cases h : evalSrc bind <;> simp [ih]

def simpleLetAddSrc : SrcLetExpr :=
  .let1 "x" (.constInt 4) (.add (.var "x") (.pure (.constInt 3)))

theorem simpleLetAddSrc_preserved :
    evalMirLet [] (lowerLet simpleLetAddSrc) = some (.int 7) := by
  rw [lowerLet_preserves_eval]
  simp [simpleLetAddSrc, evalSrcLet, evalSrc, lookupField]

def letRecordFieldSrc : SrcLetExpr :=
  .let1 "rec" (.record [("x", .constInt 4)])
    (.add (.field (.var "rec") "x") (.pure (.constInt 3)))

theorem letRecordFieldSrc_preserved :
    evalMirLet [] (lowerLet letRecordFieldSrc) = some (.int 7) := by
  rw [lowerLet_preserves_eval]
  simp [letRecordFieldSrc, evalSrcLet, evalSrc, evalSrcFields, lookupField]

def letNestedRecordFieldSrc : SrcLetExpr :=
  .let1 "rec" (.record [("inner", .record [("x", .constInt 4)])])
    (.add (.field (.field (.var "rec") "inner") "x") (.pure (.constInt 3)))

theorem letNestedRecordFieldSrc_preserved :
    evalMirLet [] (lowerLet letNestedRecordFieldSrc) = some (.int 7) := by
  rw [lowerLet_preserves_eval]
  simp [letNestedRecordFieldSrc, evalSrcLet, evalSrc, evalSrcFields, lookupField]

end RRProofs
