namespace RRProofs.GvnSubset

inductive GvnValue where
  | int : Int -> GvnValue
  | record : List (String × GvnValue) -> GvnValue
deriving Repr

inductive GvnExpr where
  | constInt : Int -> GvnExpr
  | add : GvnExpr -> GvnExpr -> GvnExpr
  | intrinsicAbs : GvnExpr -> GvnExpr
  | record : List (String × GvnExpr) -> GvnExpr
  | field : GvnExpr -> String -> GvnExpr
  | fieldSet : GvnExpr -> String -> GvnExpr -> GvnExpr
deriving Repr

def lookupField (fields : List (String × GvnValue)) (name : String) : Option GvnValue :=
  match fields.find? (fun (entry : String × GvnValue) => entry.1 = name) with
  | some (_, value) => some value
  | none => none

def setField (fields : List (String × GvnValue)) (name : String) (value : GvnValue) :
    List (String × GvnValue) :=
  match fields with
  | [] => [(name, value)]
  | (field, current) :: rest =>
      if field == name then
        (field, value) :: rest
      else
        (field, current) :: setField rest name value

mutual
  def eval : GvnExpr -> Option GvnValue
    | .constInt i => some (.int i)
    | .add lhs rhs => do
        let lv <- eval lhs
        let rv <- eval rhs
        match lv, rv with
        | .int l, .int r => some (.int (l + r))
        | _, _ => none
    | .intrinsicAbs arg => do
        let v <- eval arg
        match v with
        | .int i => some (.int (if i < 0 then -i else i))
        | _ => none
    | .record fields => do
        let vals <- evalFields fields
        some (.record vals)
    | .field base name => do
        let v <- eval base
        match v with
        | .record fields => lookupField fields name
        | _ => none
    | .fieldSet base name value => do
        let b <- eval base
        let v <- eval value
        match b with
        | .record fields => some (.record (setField fields name v))
        | _ => none

  def evalFields : List (String × GvnExpr) -> Option (List (String × GvnValue))
    | [] => some []
    | (name, expr) :: rest => do
        let v <- eval expr
        let tail <- evalFields rest
        some ((name, v) :: tail)
end

mutual
  def exprSize : GvnExpr -> Nat
    | .constInt _ => 1
    | .add lhs rhs => exprSize lhs + exprSize rhs + 1
    | .intrinsicAbs arg => exprSize arg + 1
    | .record fields => fieldSize fields + 1
    | .field base _ => exprSize base + 1
    | .fieldSet base _ value => exprSize base + exprSize value + 1

  def fieldSize : List (String × GvnExpr) -> Nat
    | [] => 0
    | (_, expr) :: rest => exprSize expr + fieldSize rest + 1
end

mutual
  def exprFingerprint : GvnExpr -> Int
    | .constInt i => i
    | .add lhs rhs => 3000 + 37 * exprFingerprint lhs + 41 * exprFingerprint rhs
    | .intrinsicAbs arg => 3500 + 43 * exprFingerprint arg
    | .record fields => 4000 + fieldFingerprint fields
    | .field base name => 5000 + 47 * exprFingerprint base + Int.ofNat name.length
    | .fieldSet base name value =>
        6000 + 53 * exprFingerprint base + Int.ofNat name.length + 59 * exprFingerprint value

  def fieldFingerprint : List (String × GvnExpr) -> Int
    | [] => 0
    | (name, expr) :: rest =>
        61 * exprFingerprint expr + Int.ofNat name.length + 67 * fieldFingerprint rest
end

def exprLt (lhs rhs : GvnExpr) : Bool :=
  decide
    (exprFingerprint lhs < exprFingerprint rhs ∨
      (exprFingerprint lhs = exprFingerprint rhs ∧ exprSize lhs < exprSize rhs))

mutual
  def canonicalize : GvnExpr -> GvnExpr
    | .constInt i => .constInt i
    | .add lhs rhs =>
        let lhs' := canonicalize lhs
        let rhs' := canonicalize rhs
        if exprLt rhs' lhs' then .add rhs' lhs' else .add lhs' rhs'
    | .intrinsicAbs arg => .intrinsicAbs (canonicalize arg)
    | .record fields => .record (canonicalizeFields fields)
    | .field base name => .field (canonicalize base) name
    | .fieldSet base name value => .fieldSet (canonicalize base) name (canonicalize value)

  def canonicalizeFields : List (String × GvnExpr) -> List (String × GvnExpr)
    | [] => []
    | (name, expr) :: rest => (name, canonicalize expr) :: canonicalizeFields rest
end

theorem eval_add_comm (lhs rhs : GvnExpr) :
    eval (.add lhs rhs) = eval (.add rhs lhs) := by
  cases hL : eval lhs with
  | none =>
      cases hR : eval rhs <;> simp [eval, hL, hR]
  | some lv =>
      cases lv with
      | int l =>
          cases hR : eval rhs with
          | none =>
              simp [eval, hL, hR]
          | some rv =>
              cases rv with
              | int r =>
                  simp [eval, hL, hR, Int.add_comm]
              | record fields =>
                  simp [eval, hL, hR]
      | record fields =>
          cases hR : eval rhs <;> simp [eval, hL, hR]

mutual
  theorem canonicalize_preserves_eval (expr : GvnExpr) :
      eval (canonicalize expr) = eval expr := by
    match expr with
    | .constInt i =>
        simp [canonicalize, eval]
    | .add lhs rhs =>
        have ihL := canonicalize_preserves_eval lhs
        have ihR := canonicalize_preserves_eval rhs
        cases hswap : exprLt (canonicalize rhs) (canonicalize lhs) with
        | false =>
            calc
              eval (canonicalize (.add lhs rhs))
                  = eval (.add (canonicalize lhs) (canonicalize rhs)) := by
                      simp [canonicalize, hswap]
              _ = eval (.add lhs rhs) := by
                      simp [eval, ihL, ihR]
        | true =>
            calc
              eval (canonicalize (.add lhs rhs))
                  = eval (.add (canonicalize rhs) (canonicalize lhs)) := by
                      simp [canonicalize, hswap]
              _ = eval (.add (canonicalize lhs) (canonicalize rhs)) := eval_add_comm _ _
              _ = eval (.add lhs rhs) := by simp [eval, ihL, ihR]
    | .intrinsicAbs arg =>
        simp [canonicalize, eval, canonicalize_preserves_eval arg]
    | .record fields =>
        simp [canonicalize, eval, canonicalizeFields_preserves_eval fields]
    | .field base name =>
        simp [canonicalize, eval, canonicalize_preserves_eval base]
    | .fieldSet base name value =>
        simp [canonicalize, eval, canonicalize_preserves_eval base, canonicalize_preserves_eval value]

  theorem canonicalizeFields_preserves_eval (fields : List (String × GvnExpr)) :
      evalFields (canonicalizeFields fields) = evalFields fields := by
    match fields with
    | [] =>
        simp [canonicalizeFields, evalFields]
    | (name, expr) :: rest =>
        simp [canonicalizeFields, evalFields,
          canonicalize_preserves_eval expr, canonicalizeFields_preserves_eval rest]
end

theorem canonicalFormEqual_implies_same_eval
    {lhs rhs : GvnExpr}
    (hCanon : canonicalize lhs = canonicalize rhs) :
    eval lhs = eval rhs := by
  calc
    eval lhs = eval (canonicalize lhs) := by
      symm
      exact canonicalize_preserves_eval lhs
    _ = eval (canonicalize rhs) := by simp [hCanon]
    _ = eval rhs := canonicalize_preserves_eval rhs

def swappedAddA : GvnExpr := .add (.constInt 2) (.constInt 5)
def swappedAddB : GvnExpr := .add (.constInt 5) (.constInt 2)

theorem swappedAdd_cse_preserved :
    eval swappedAddA = eval swappedAddB := by
  apply canonicalFormEqual_implies_same_eval
  simp [swappedAddA, swappedAddB, canonicalize, exprLt, exprFingerprint, exprSize]

def duplicateIntrinsicA : GvnExpr :=
  .intrinsicAbs (.add (.constInt (-5)) (.constInt 2))

def duplicateIntrinsicB : GvnExpr :=
  .intrinsicAbs (.add (.constInt 2) (.constInt (-5)))

theorem duplicateIntrinsic_cse_preserved :
    eval duplicateIntrinsicA = eval duplicateIntrinsicB := by
  apply canonicalFormEqual_implies_same_eval
  simp [duplicateIntrinsicA, duplicateIntrinsicB, canonicalize, exprLt, exprFingerprint, exprSize]

def duplicateFieldGetA : GvnExpr :=
  .field (.record [("x", .add (.constInt 2) (.constInt 5))]) "x"

def duplicateFieldGetB : GvnExpr :=
  .field (.record [("x", .add (.constInt 5) (.constInt 2))]) "x"

theorem duplicateFieldGet_cse_preserved :
    eval duplicateFieldGetA = eval duplicateFieldGetB := by
  apply canonicalFormEqual_implies_same_eval
  simp [duplicateFieldGetA, duplicateFieldGetB, canonicalize, canonicalizeFields,
    exprLt, exprFingerprint, exprSize]

def duplicateFieldSetGetA : GvnExpr :=
  .field (.fieldSet (.record [("x", .constInt 1)]) "x" (.add (.constInt 2) (.constInt 5))) "x"

def duplicateFieldSetGetB : GvnExpr :=
  .field (.fieldSet (.record [("x", .constInt 1)]) "x" (.add (.constInt 5) (.constInt 2))) "x"

theorem duplicateFieldSetGet_cse_preserved :
    eval duplicateFieldSetGetA = eval duplicateFieldSetGetB := by
  apply canonicalFormEqual_implies_same_eval
  simp [duplicateFieldSetGetA, duplicateFieldSetGetB, canonicalize, canonicalizeFields,
    exprLt, exprFingerprint, exprSize]

end RRProofs.GvnSubset
