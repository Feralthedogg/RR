namespace RRProofs.SroaRecordReturnSubset

abbrev VarName := String
abbrev FieldName := String

structure SroaEnv where
  scalar : VarName -> Int
  record : VarName -> FieldName -> Int

inductive SroaExpr where
  | constInt : Int -> SroaExpr
  | load : VarName -> SroaExpr
  | field : VarName -> FieldName -> SroaExpr
  | callField : VarName -> FieldName -> SroaExpr
  | add : SroaExpr -> SroaExpr -> SroaExpr
deriving Repr

def evalSroaExpr (ρ : SroaEnv) : SroaExpr -> Int
  | .constInt i => i
  | .load v => ρ.scalar v
  | .field alias field => ρ.record alias field
  | .callField call field => ρ.record call field
  | .add lhs rhs => evalSroaExpr ρ lhs + evalSroaExpr ρ rhs

def rewriteAliasFieldToTemp
    (alias : VarName)
    (field : FieldName)
    (temp : VarName) : SroaExpr -> SroaExpr
  | .constInt i => .constInt i
  | .load v => .load v
  | .field currentAlias currentField =>
      if currentAlias = alias then
        if currentField = field then .load temp else .field currentAlias currentField
      else
        .field currentAlias currentField
  | .callField call field => .callField call field
  | .add lhs rhs =>
      .add
        (rewriteAliasFieldToTemp alias field temp lhs)
        (rewriteAliasFieldToTemp alias field temp rhs)

def rewriteAliasFieldToValue
    (alias : VarName)
    (field : FieldName)
    (replacement : SroaExpr) : SroaExpr -> SroaExpr
  | .constInt i => .constInt i
  | .load v => .load v
  | .field currentAlias currentField =>
      if currentAlias = alias then
        if currentField = field then replacement else .field currentAlias currentField
      else
        .field currentAlias currentField
  | .callField call field => .callField call field
  | .add lhs rhs =>
      .add
        (rewriteAliasFieldToValue alias field replacement lhs)
        (rewriteAliasFieldToValue alias field replacement rhs)

def rewriteDirectCallFieldToValue
    (call : VarName)
    (field : FieldName)
    (replacement : SroaExpr) : SroaExpr -> SroaExpr
  | .constInt i => .constInt i
  | .load v => .load v
  | .field alias field => .field alias field
  | .callField currentCall currentField =>
      if currentCall = call then
        if currentField = field then replacement else .callField currentCall currentField
      else
        .callField currentCall currentField
  | .add lhs rhs =>
      .add
        (rewriteDirectCallFieldToValue call field replacement lhs)
        (rewriteDirectCallFieldToValue call field replacement rhs)

theorem rewriteAliasFieldToTemp_preserves_eval
    (ρ : SroaEnv)
    (alias : VarName)
    (field : FieldName)
    (temp : VarName)
    (expr : SroaExpr)
    (hTemp : ρ.scalar temp = ρ.record alias field) :
    evalSroaExpr ρ (rewriteAliasFieldToTemp alias field temp expr) =
      evalSroaExpr ρ expr := by
  induction expr with
  | constInt i =>
      simp [rewriteAliasFieldToTemp, evalSroaExpr]
  | load v =>
      simp [rewriteAliasFieldToTemp, evalSroaExpr]
  | field currentAlias currentField =>
      by_cases hAlias : currentAlias = alias
      · subst currentAlias
        by_cases hField : currentField = field
        · subst currentField
          simp [rewriteAliasFieldToTemp, evalSroaExpr, hTemp]
        · simp [rewriteAliasFieldToTemp, evalSroaExpr, hField]
      · simp [rewriteAliasFieldToTemp, evalSroaExpr, hAlias]
  | callField call field =>
      simp [rewriteAliasFieldToTemp, evalSroaExpr]
  | add lhs rhs ihL ihR =>
      simp [rewriteAliasFieldToTemp, evalSroaExpr, ihL, ihR]

theorem rewriteAliasFieldToValue_preserves_eval
    (ρ : SroaEnv)
    (alias : VarName)
    (field : FieldName)
    (replacement expr : SroaExpr)
    (hReplacement : evalSroaExpr ρ replacement = ρ.record alias field) :
    evalSroaExpr ρ (rewriteAliasFieldToValue alias field replacement expr) =
      evalSroaExpr ρ expr := by
  induction expr with
  | constInt i =>
      simp [rewriteAliasFieldToValue, evalSroaExpr]
  | load v =>
      simp [rewriteAliasFieldToValue, evalSroaExpr]
  | field currentAlias currentField =>
      by_cases hAlias : currentAlias = alias
      · subst currentAlias
        by_cases hField : currentField = field
        · subst currentField
          simp [rewriteAliasFieldToValue, evalSroaExpr, hReplacement]
        · simp [rewriteAliasFieldToValue, evalSroaExpr, hField]
      · simp [rewriteAliasFieldToValue, evalSroaExpr, hAlias]
  | callField call field =>
      simp [rewriteAliasFieldToValue, evalSroaExpr]
  | add lhs rhs ihL ihR =>
      simp [rewriteAliasFieldToValue, evalSroaExpr, ihL, ihR]

theorem rewriteDirectCallFieldToValue_preserves_eval
    (ρ : SroaEnv)
    (call : VarName)
    (field : FieldName)
    (replacement expr : SroaExpr)
    (hReplacement : evalSroaExpr ρ replacement = ρ.record call field) :
    evalSroaExpr ρ (rewriteDirectCallFieldToValue call field replacement expr) =
      evalSroaExpr ρ expr := by
  induction expr with
  | constInt i =>
      simp [rewriteDirectCallFieldToValue, evalSroaExpr]
  | load v =>
      simp [rewriteDirectCallFieldToValue, evalSroaExpr]
  | field alias field =>
      simp [rewriteDirectCallFieldToValue, evalSroaExpr]
  | callField currentCall currentField =>
      by_cases hCall : currentCall = call
      · subst currentCall
        by_cases hField : currentField = field
        · subst currentField
          simp [rewriteDirectCallFieldToValue, evalSroaExpr, hReplacement]
        · simp [rewriteDirectCallFieldToValue, evalSroaExpr, hField]
      · simp [rewriteDirectCallFieldToValue, evalSroaExpr, hCall]
  | add lhs rhs ihL ihR =>
      simp [rewriteDirectCallFieldToValue, evalSroaExpr, ihL, ihR]

def repeatedProjectionExpr : SroaExpr :=
  .add (.field "p" "x") (.field "p" "x")

def repeatedProjectionEnv : SroaEnv :=
  { scalar := fun
      | "p__rr_sroa_ret_x" => 7
      | _ => 0
    record := fun
      | "p", "x" => 7
      | _, _ => 0 }

theorem repeatedProjection_sharedTemp_preserved :
    evalSroaExpr repeatedProjectionEnv
        (rewriteAliasFieldToTemp "p" "x" "p__rr_sroa_ret_x" repeatedProjectionExpr) =
      evalSroaExpr repeatedProjectionEnv repeatedProjectionExpr := by
  apply rewriteAliasFieldToTemp_preserves_eval
  simp [repeatedProjectionEnv]

def localRecordProjectionExpr : SroaExpr :=
  .add (.field "moved" "x") (.constInt 3)

def localRecordProjectionEnv : SroaEnv :=
  { scalar := fun
      | "entity_x" => 10
      | "velocity_x" => 2
      | _ => 0
    record := fun
      | "moved", "x" => 12
      | _, _ => 0 }

theorem localRecordProjection_scalarValue_preserved :
    evalSroaExpr localRecordProjectionEnv
        (rewriteAliasFieldToValue
          "moved"
          "x"
          (.add (.load "entity_x") (.load "velocity_x"))
          localRecordProjectionExpr) =
      evalSroaExpr localRecordProjectionEnv localRecordProjectionExpr := by
  apply rewriteAliasFieldToValue_preserves_eval
  simp [localRecordProjectionEnv, evalSroaExpr]

def snapshotRecordProjectionExpr : SroaExpr :=
  .add (.field "point" "x") (.field "point" "y")

def snapshotRecordProjectionEnv : SroaEnv :=
  { scalar := fun
      | "point__rr_sroa_snap_x" => 4
      | "point__rr_sroa_snap_y" => 9
      | _ => 0
    record := fun
      | "point", "x" => 4
      | "point", "y" => 9
      | _, _ => 0 }

theorem snapshotRecordProjection_temps_preserved :
    evalSroaExpr snapshotRecordProjectionEnv
        (rewriteAliasFieldToTemp
          "point"
          "y"
          "point__rr_sroa_snap_y"
          (rewriteAliasFieldToTemp
            "point"
            "x"
            "point__rr_sroa_snap_x"
            snapshotRecordProjectionExpr)) =
      evalSroaExpr snapshotRecordProjectionEnv snapshotRecordProjectionExpr := by
  calc
    evalSroaExpr snapshotRecordProjectionEnv
        (rewriteAliasFieldToTemp
          "point"
          "y"
          "point__rr_sroa_snap_y"
          (rewriteAliasFieldToTemp
            "point"
            "x"
            "point__rr_sroa_snap_x"
            snapshotRecordProjectionExpr))
        =
        evalSroaExpr snapshotRecordProjectionEnv
          (rewriteAliasFieldToTemp
            "point"
            "x"
            "point__rr_sroa_snap_x"
            snapshotRecordProjectionExpr) := by
          apply rewriteAliasFieldToTemp_preserves_eval
          simp [snapshotRecordProjectionEnv]
    _ = evalSroaExpr snapshotRecordProjectionEnv snapshotRecordProjectionExpr := by
          apply rewriteAliasFieldToTemp_preserves_eval
          simp [snapshotRecordProjectionEnv]

def directProjectionExpr : SroaExpr :=
  .add (.callField "make_xy()" "x") (.constInt 5)

def directProjectionEnv : SroaEnv :=
  { scalar := fun _ => 0
    record := fun
      | "make_xy()", "x" => 11
      | _, _ => 0 }

theorem directProjection_inlineValue_preserved :
    evalSroaExpr directProjectionEnv
        (rewriteDirectCallFieldToValue
          "make_xy()" "x" (.constInt 11) directProjectionExpr) =
      evalSroaExpr directProjectionEnv directProjectionExpr := by
  apply rewriteDirectCallFieldToValue_preserves_eval
  simp [directProjectionEnv, evalSroaExpr]

def lookupPositionalArg :
    List VarName -> List VarName -> VarName -> Option VarName
  | param :: params, arg :: args, target =>
      if param = target then some arg else lookupPositionalArg params args target
  | _, _, _ => none

def lookupNamedOrPositionalArg :
    List VarName -> List (Option VarName) -> List VarName -> VarName -> Option VarName
  | param :: params, name :: names, arg :: args, target =>
      match name with
      | some explicit =>
          if explicit = target then
            some arg
          else
            lookupNamedOrPositionalArg params names args target
      | none =>
          if param = target then
            some arg
          else
            lookupNamedOrPositionalArg params names args target
  | _, _, _, _ => none

theorem paramOrderNamedArgs_erased_forRecordArgSroa :
    lookupNamedOrPositionalArg ["p"] [some "p"] ["point"] "p" =
      lookupPositionalArg ["p"] ["point"] "p" := by
  simp [lookupNamedOrPositionalArg, lookupPositionalArg]

theorem paramOrderNamedArgs_erased_forRecordReturnSroa :
    lookupNamedOrPositionalArg
        ["x", "y"]
        [some "x", some "y"]
        ["arg_x", "arg_y"]
        "y" =
      lookupPositionalArg ["x", "y"] ["arg_x", "arg_y"] "y" := by
  simp [lookupNamedOrPositionalArg, lookupPositionalArg]

end RRProofs.SroaRecordReturnSubset
