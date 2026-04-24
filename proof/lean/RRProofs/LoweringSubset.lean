namespace RRProofs

inductive RValue where
  | int : Int -> RValue
  | bool : Bool -> RValue
  | record : List (String × RValue) -> RValue
deriving Repr

inductive SrcExpr where
  | constInt : Int -> SrcExpr
  | constBool : Bool -> SrcExpr
  | neg : SrcExpr -> SrcExpr
  | add : SrcExpr -> SrcExpr -> SrcExpr
  | record : List (String × SrcExpr) -> SrcExpr
  | field : SrcExpr -> String -> SrcExpr
deriving Repr

inductive MirExpr where
  | constInt : Int -> MirExpr
  | constBool : Bool -> MirExpr
  | unaryNeg : MirExpr -> MirExpr
  | binaryAdd : MirExpr -> MirExpr -> MirExpr
  | recordLit : List (String × MirExpr) -> MirExpr
  | fieldGet : MirExpr -> String -> MirExpr
deriving Repr

def lookupField (fields : List (String × RValue)) (name : String) : Option RValue :=
  match fields.find? (fun (entry : String × RValue) => entry.1 = name) with
  | some (_, value) => some value
  | none => none

mutual
  def evalSrc : SrcExpr -> Option RValue
    | .constInt i => some (.int i)
    | .constBool b => some (.bool b)
    | .neg e => do
        let v <- evalSrc e
        match v with
        | .int i => some (.int (-i))
        | _ => none
    | .add lhs rhs => do
        let lv <- evalSrc lhs
        let rv <- evalSrc rhs
        match lv, rv with
        | .int l, .int r => some (.int (l + r))
        | _, _ => none
    | .record fields => do
        let vals <- evalSrcFields fields
        some (.record vals)
    | .field base name => do
        let v <- evalSrc base
        match v with
        | .record fields => lookupField fields name
        | _ => none

  def evalSrcFields : List (String × SrcExpr) -> Option (List (String × RValue))
    | [] => some []
    | (name, expr) :: rest => do
        let v <- evalSrc expr
        let tail <- evalSrcFields rest
        some ((name, v) :: tail)
end

mutual
  def evalMir : MirExpr -> Option RValue
    | .constInt i => some (.int i)
    | .constBool b => some (.bool b)
    | .unaryNeg e => do
        let v <- evalMir e
        match v with
        | .int i => some (.int (-i))
        | _ => none
    | .binaryAdd lhs rhs => do
        let lv <- evalMir lhs
        let rv <- evalMir rhs
        match lv, rv with
        | .int l, .int r => some (.int (l + r))
        | _, _ => none
    | .recordLit fields => do
        let vals <- evalMirFields fields
        some (.record vals)
    | .fieldGet base name => do
        let v <- evalMir base
        match v with
        | .record fields => lookupField fields name
        | _ => none

  def evalMirFields : List (String × MirExpr) -> Option (List (String × RValue))
    | [] => some []
    | (name, expr) :: rest => do
        let v <- evalMir expr
        let tail <- evalMirFields rest
        some ((name, v) :: tail)
end

mutual
  def lower : SrcExpr -> MirExpr
    | .constInt i => .constInt i
    | .constBool b => .constBool b
    | .neg e => .unaryNeg (lower e)
    | .add lhs rhs => .binaryAdd (lower lhs) (lower rhs)
    | .record fields => .recordLit (lowerFields fields)
    | .field base name => .fieldGet (lower base) name

  def lowerFields : List (String × SrcExpr) -> List (String × MirExpr)
    | [] => []
    | (name, expr) :: rest => (name, lower expr) :: lowerFields rest
end

mutual
  theorem lower_preserves_eval
      (expr : SrcExpr) :
      evalMir (lower expr) = evalSrc expr := by
    match expr with
    | .constInt i =>
        simp [lower, evalMir, evalSrc]
    | .constBool b =>
        simp [lower, evalMir, evalSrc]
    | .neg e =>
        simp [lower, evalMir, evalSrc, lower_preserves_eval e]
    | .add lhs rhs =>
        simp [lower, evalMir, evalSrc, lower_preserves_eval lhs, lower_preserves_eval rhs]
    | .record fields =>
        simp [lower, evalMir, evalSrc, lowerFields_preserves_eval fields]
    | .field base name =>
        simp [lower, evalMir, evalSrc, lower_preserves_eval base]

  theorem lowerFields_preserves_eval
      (fields : List (String × SrcExpr)) :
      evalMirFields (lowerFields fields) = evalSrcFields fields := by
    match fields with
    | [] =>
        simp [lowerFields, evalMirFields, evalSrcFields]
    | (name, expr) :: rest =>
        simp [lowerFields, evalMirFields, evalSrcFields,
          lower_preserves_eval expr, lowerFields_preserves_eval rest]
end

def nestedFieldSrc : SrcExpr :=
  .field (.field (.record [("inner", .record [("x", .constInt 7)])]) "inner") "x"

theorem nestedFieldSrc_preserved :
    evalMir (lower nestedFieldSrc) = some (.int 7) := by
  simp [nestedFieldSrc, lower_preserves_eval, evalSrc, evalSrcFields, lookupField]

end RRProofs
