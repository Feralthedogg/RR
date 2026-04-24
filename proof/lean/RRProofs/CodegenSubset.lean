import RRProofs.LoweringSubset

namespace RRProofs

inductive RExpr where
  | constInt : Int -> RExpr
  | constBool : Bool -> RExpr
  | unaryNeg : RExpr -> RExpr
  | binaryAdd : RExpr -> RExpr -> RExpr
  | listLit : List (String × RExpr) -> RExpr
  | fieldGet : RExpr -> String -> RExpr
deriving Repr

mutual
  def evalRExpr : RExpr -> Option RValue
    | .constInt i => some (.int i)
    | .constBool b => some (.bool b)
    | .unaryNeg e => do
        let v <- evalRExpr e
        match v with
        | .int i => some (.int (-i))
        | _ => none
    | .binaryAdd lhs rhs => do
        let lv <- evalRExpr lhs
        let rv <- evalRExpr rhs
        match lv, rv with
        | .int l, .int r => some (.int (l + r))
        | _, _ => none
    | .listLit fields => do
        let vals <- evalRFields fields
        some (.record vals)
    | .fieldGet base name => do
        let v <- evalRExpr base
        match v with
        | .record fields => lookupField fields name
        | _ => none

  def evalRFields : List (String × RExpr) -> Option (List (String × RValue))
    | [] => some []
    | (name, expr) :: rest => do
        let v <- evalRExpr expr
        let tail <- evalRFields rest
        some ((name, v) :: tail)
end

mutual
  def emitR : MirExpr -> RExpr
    | .constInt i => .constInt i
    | .constBool b => .constBool b
    | .unaryNeg e => .unaryNeg (emitR e)
    | .binaryAdd lhs rhs => .binaryAdd (emitR lhs) (emitR rhs)
    | .recordLit fields => .listLit (emitRFields fields)
    | .fieldGet base name => .fieldGet (emitR base) name

  def emitRFields : List (String × MirExpr) -> List (String × RExpr)
    | [] => []
    | (name, expr) :: rest => (name, emitR expr) :: emitRFields rest
end

mutual
  theorem emitR_preserves_eval
      (expr : MirExpr) :
      evalRExpr (emitR expr) = evalMir expr := by
    match expr with
    | .constInt i =>
        simp [emitR, evalRExpr, evalMir]
    | .constBool b =>
        simp [emitR, evalRExpr, evalMir]
    | .unaryNeg e =>
        simp [emitR, evalRExpr, evalMir, emitR_preserves_eval e]
        rfl
    | .binaryAdd lhs rhs =>
        simp [emitR, evalRExpr, evalMir, emitR_preserves_eval lhs, emitR_preserves_eval rhs]
        rfl
    | .recordLit fields =>
        simp [emitR, evalRExpr, evalMir, emitRFields_preserves_eval fields]
    | .fieldGet base name =>
        simp [emitR, evalRExpr, evalMir, emitR_preserves_eval base]
        rfl

  theorem emitRFields_preserves_eval
      (fields : List (String × MirExpr)) :
      evalRFields (emitRFields fields) = evalMirFields fields := by
    match fields with
    | [] =>
        simp [emitRFields, evalRFields, evalMirFields]
    | (name, expr) :: rest =>
        simp [emitRFields, evalRFields, evalMirFields,
          emitR_preserves_eval expr, emitRFields_preserves_eval rest]
end

def nestedFieldMirExpr : MirExpr :=
  .fieldGet (.fieldGet (.recordLit [("inner", .recordLit [("x", .constInt 7)])]) "inner") "x"

theorem nestedFieldMirExpr_codegen_preserved :
    evalRExpr (emitR nestedFieldMirExpr) = some (.int 7) := by
  rw [emitR_preserves_eval]
  simp [nestedFieldMirExpr, evalMir, evalMirFields, lookupField]

end RRProofs
