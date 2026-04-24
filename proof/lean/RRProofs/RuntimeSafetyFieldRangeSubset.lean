namespace RRProofs

inductive RsValue where
  | int : Int -> RsValue
  | record : List (String × RsValue) -> RsValue
deriving Repr

inductive RsExpr where
  | constInt : Int -> RsExpr
  | record : List (String × RsExpr) -> RsExpr
  | field : RsExpr -> String -> RsExpr
  | fieldSet : RsExpr -> String -> RsExpr -> RsExpr
deriving Repr

def lookupRsField (fields : List (String × RsValue)) (name : String) : Option RsValue :=
  match fields.find? (fun (entry : String × RsValue) => entry.1 = name) with
  | some (_, value) => some value
  | none => none

def setRsField (fields : List (String × RsValue)) (name : String) (value : RsValue) :
    List (String × RsValue) :=
  match fields with
  | [] => [(name, value)]
  | (field, current) :: rest =>
      if field == name then
        (field, value) :: rest
      else
        (field, current) :: setRsField rest name value

mutual
  def evalRs : RsExpr -> Option RsValue
    | .constInt i => some (.int i)
    | .record fields => do
        let vals <- evalRsFields fields
        some (.record vals)
    | .field base name => do
        let v <- evalRs base
        match v with
        | .record fields => lookupRsField fields name
        | _ => none
    | .fieldSet base name value => do
        let b <- evalRs base
        let v <- evalRs value
        match b with
        | .record fields => some (.record (setRsField fields name v))
        | _ => none

  def evalRsFields : List (String × RsExpr) -> Option (List (String × RsValue))
    | [] => some []
    | (name, expr) :: rest => do
        let v <- evalRs expr
        let tail <- evalRsFields rest
        some ((name, v) :: tail)
end

def exactIntervalOf (expr : RsExpr) : Option (Int × Int) := do
  let v <- evalRs expr
  match v with
  | .int i => some (i, i)
  | _ => none

def intervalBelowOne (bounds : Int × Int) : Bool :=
  bounds.2 < 1

def intervalNegative (bounds : Int × Int) : Bool :=
  bounds.2 < 0

theorem evalRs_int_implies_exactIntervalOf {expr : RsExpr} {i : Int}
    (h : evalRs expr = some (.int i)) :
    exactIntervalOf expr = some (i, i) := by
  simp [exactIntervalOf, h]

theorem exact_negative_interval_implies_below_one {i : Int} (h : i < 0) :
    intervalBelowOne (i, i) = true := by
  have h1 : i < 1 := Int.lt_trans h (by decide : (0 : Int) < 1)
  simp [intervalBelowOne, h1]

def exampleRecordFieldNegative : RsExpr :=
  .field (.record [("i", .constInt (-1)), ("j", .constInt 2)]) "i"

theorem exampleRecordFieldNegative_interval :
    exactIntervalOf exampleRecordFieldNegative = some (-1, -1) := by
  apply evalRs_int_implies_exactIntervalOf
  simp [exampleRecordFieldNegative, evalRs, evalRsFields, lookupRsField]

theorem exampleRecordFieldNegative_below_one :
    intervalBelowOne (-1, -1) = true := by
  exact exact_negative_interval_implies_below_one (by decide)

theorem exampleRecordFieldNegative_is_negative :
    intervalNegative (-1, -1) = true := by
  simp [intervalNegative]

def exampleFieldSetNegative : RsExpr :=
  .field (.fieldSet (.record [("i", .constInt 5)]) "i" (.constInt (-2))) "i"

theorem exampleFieldSetNegative_interval :
    exactIntervalOf exampleFieldSetNegative = some (-2, -2) := by
  apply evalRs_int_implies_exactIntervalOf
  simp [exampleFieldSetNegative, evalRs, evalRsFields, lookupRsField, setRsField]

theorem exampleFieldSetNegative_below_one :
    intervalBelowOne (-2, -2) = true := by
  exact exact_negative_interval_implies_below_one (by decide)

def exampleFieldSetOverridePositive : RsExpr :=
  .field (.fieldSet (.record [("i", .constInt (-1))]) "i" (.constInt 5)) "i"

theorem exampleFieldSetOverridePositive_interval :
    exactIntervalOf exampleFieldSetOverridePositive = some (5, 5) := by
  apply evalRs_int_implies_exactIntervalOf
  simp [exampleFieldSetOverridePositive, evalRs, evalRsFields, lookupRsField, setRsField]

theorem exampleFieldSetOverridePositive_not_below_one :
    intervalBelowOne (5, 5) = false := by
  simp [intervalBelowOne]

def exampleNestedRecordFieldNegative : RsExpr :=
  .field (.field (.record [("inner", .record [("i", .constInt (-1))])]) "inner") "i"

theorem exampleNestedRecordFieldNegative_interval :
    exactIntervalOf exampleNestedRecordFieldNegative = some (-1, -1) := by
  apply evalRs_int_implies_exactIntervalOf
  simp [exampleNestedRecordFieldNegative, evalRs, evalRsFields, lookupRsField]

theorem exampleNestedRecordFieldNegative_below_one :
    intervalBelowOne (-1, -1) = true := by
  exact exampleRecordFieldNegative_below_one

end RRProofs
