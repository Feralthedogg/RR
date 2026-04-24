namespace RRProofs

inductive InlineValue where
  | int : Int -> InlineValue
  | record : List (String × InlineValue) -> InlineValue
deriving Repr

inductive InlineExpr where
  | constInt : Int -> InlineExpr
  | add : InlineExpr -> InlineExpr -> InlineExpr
  | record : List (String × InlineExpr) -> InlineExpr
  | field : InlineExpr -> String -> InlineExpr
deriving Repr

inductive HelperShape where
  | arg
  | addConst : Int -> HelperShape
  | field : String -> HelperShape
  | fieldAddConst : String -> Int -> HelperShape
deriving Repr

def lookupInlineField (fields : List (String × InlineValue)) (name : String) : Option InlineValue :=
  match fields.find? (fun (entry : String × InlineValue) => entry.1 = name) with
  | some (_, value) => some value
  | none => none

mutual
  def evalInlineExpr : InlineExpr -> Option InlineValue
    | .constInt i => some (.int i)
    | .add lhs rhs => do
        let lv <- evalInlineExpr lhs
        let rv <- evalInlineExpr rhs
        match lv, rv with
        | .int l, .int r => some (.int (l + r))
        | _, _ => none
    | .record fields => do
        let vals <- evalInlineFields fields
        some (.record vals)
    | .field base name => do
        let v <- evalInlineExpr base
        match v with
        | .record fields => lookupInlineField fields name
        | _ => none

  def evalInlineFields : List (String × InlineExpr) -> Option (List (String × InlineValue))
    | [] => some []
    | (name, expr) :: rest => do
        let v <- evalInlineExpr expr
        let tail <- evalInlineFields rest
        some ((name, v) :: tail)
end

def evalHelperShape (helper : HelperShape) (arg : InlineValue) : Option InlineValue :=
  match helper with
  | .arg => some arg
  | .addConst k =>
      match arg with
      | .int i => some (.int (i + k))
      | _ => none
  | .field name =>
      match arg with
      | .record fields => lookupInlineField fields name
      | _ => none
  | .fieldAddConst name k =>
      match arg with
      | .record fields => do
          let v <- lookupInlineField fields name
          match v with
          | .int i => some (.int (i + k))
          | _ => none
      | _ => none

def inlineCall (helper : HelperShape) (arg : InlineExpr) : InlineExpr :=
  match helper with
  | .arg => arg
  | .addConst k => .add arg (.constInt k)
  | .field name => .field arg name
  | .fieldAddConst name k => .add (.field arg name) (.constInt k)

def evalInlineCall (helper : HelperShape) (arg : InlineExpr) : Option InlineValue := do
  let v <- evalInlineExpr arg
  evalHelperShape helper v

theorem inlineCall_preserves_eval (helper : HelperShape) (arg : InlineExpr) :
    evalInlineExpr (inlineCall helper arg) = evalInlineCall helper arg := by
  cases helper with
  | arg =>
      cases hArg : evalInlineExpr arg <;> simp [inlineCall, evalInlineCall, evalHelperShape, hArg]
  | addConst k =>
      cases hArg : evalInlineExpr arg with
      | none =>
          simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg]
      | some v =>
          cases v with
          | int i =>
              simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg, Int.add_comm]
          | record fields =>
              simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg]
  | field name =>
      cases hArg : evalInlineExpr arg with
      | none =>
          simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg]
      | some v =>
          cases v with
          | int i =>
              simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg]
          | record fields =>
              simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg]
  | fieldAddConst name k =>
      cases hArg : evalInlineExpr arg with
      | none =>
          simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg]
      | some v =>
          cases v with
          | int i =>
              simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg]
          | record fields =>
              cases hField : lookupInlineField fields name with
              | none =>
                  simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg, hField]
              | some fv =>
                  cases fv with
                  | int i =>
                      simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg, hField, Int.add_comm]
                  | record inner =>
                      simp [inlineCall, evalInlineCall, evalHelperShape, evalInlineExpr, hArg, hField]

def inlineAddArg : InlineExpr := .constInt 6
def inlineFieldArg : InlineExpr := .record [("x", .constInt 9)]
def inlineFieldAddArg : InlineExpr := .record [("x", .constInt 9)]

theorem addConst_helper_preserved :
    evalInlineExpr (inlineCall (.addConst 3) inlineAddArg) = some (.int 9) := by
  rw [inlineCall_preserves_eval]
  simp [evalInlineCall, evalInlineExpr, evalHelperShape, inlineAddArg]

theorem field_helper_preserved :
    evalInlineExpr (inlineCall (.field "x") inlineFieldArg) = some (.int 9) := by
  rw [inlineCall_preserves_eval]
  simp [evalInlineCall, evalInlineExpr, evalInlineFields, evalHelperShape, inlineFieldArg,
    lookupInlineField]

theorem field_add_helper_preserved :
    evalInlineExpr (inlineCall (.fieldAddConst "x" 3) inlineFieldAddArg) = some (.int 12) := by
  rw [inlineCall_preserves_eval]
  simp [evalInlineCall, evalInlineExpr, evalInlineFields, evalHelperShape, inlineFieldAddArg,
    lookupInlineField]

end RRProofs
