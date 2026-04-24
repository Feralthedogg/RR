import RRProofs.MirSemanticsLite
import RRProofs.MirInvariantBundle

namespace RRProofs.DataflowOptSoundness

open RRProofs.MirSemanticsLite
open RRProofs.MirInvariantBundle

def valueFingerprint : MirValue -> Int
  | .int i => i
  | .bool true => 1
  | .bool false => 0
  | .null => -1
  | .array items => 1000 + items.length
  | .record fields => 2000 + fields.length

def exprSize : MirExpr -> Nat
  | .const _ => 1
  | .load _ => 1
  | .add lhs rhs => exprSize lhs + exprSize rhs + 1
  | .mul lhs rhs => exprSize lhs + exprSize rhs + 1
  | .neg arg => exprSize arg + 1
  | .lt lhs rhs => exprSize lhs + exprSize rhs + 1

def exprFingerprint : MirExpr -> Int
  | .const value => 100 + valueFingerprint value
  | .load name => 300 + name.length
  | .add lhs rhs => 500 + 31 * exprFingerprint lhs + 37 * exprFingerprint rhs
  | .mul lhs rhs => 700 + 41 * exprFingerprint lhs + 43 * exprFingerprint rhs
  | .neg arg => 900 + 47 * exprFingerprint arg
  | .lt lhs rhs => 1100 + 53 * exprFingerprint lhs + 59 * exprFingerprint rhs

def exprLt (lhs rhs : MirExpr) : Bool :=
  decide
    (exprFingerprint lhs < exprFingerprint rhs ∨
      (exprFingerprint lhs = exprFingerprint rhs ∧ exprSize lhs < exprSize rhs))

def canonicalizeExpr : MirExpr -> MirExpr
  | .const value => .const value
  | .load name => .load name
  | .add lhs rhs =>
      let lhs' := canonicalizeExpr lhs
      let rhs' := canonicalizeExpr rhs
      if exprLt rhs' lhs' then .add rhs' lhs' else .add lhs' rhs'
  | .mul lhs rhs =>
      let lhs' := canonicalizeExpr lhs
      let rhs' := canonicalizeExpr rhs
      if exprLt rhs' lhs' then .mul rhs' lhs' else .mul lhs' rhs'
  | .neg arg => .neg (canonicalizeExpr arg)
  | .lt lhs rhs => .lt (canonicalizeExpr lhs) (canonicalizeExpr rhs)

theorem eval_add_comm (env : Env) (lhs rhs : MirExpr) :
    evalExpr env (.add lhs rhs) = evalExpr env (.add rhs lhs) := by
  cases hL : evalExpr env lhs with
  | none =>
      cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]
  | some lv =>
      cases lv with
      | int l =>
          cases hR : evalExpr env rhs with
          | none =>
              simp [evalExpr, hL, hR]
          | some rv =>
              cases rv with
              | int r =>
                  simp [evalExpr, hL, hR, Int.add_comm]
              | bool b =>
                  simp [evalExpr, hL, hR]
              | null =>
                  simp [evalExpr, hL, hR]
              | array items =>
                  simp [evalExpr, hL, hR]
              | record fields =>
                  simp [evalExpr, hL, hR]
      | bool b =>
          cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]
      | null =>
          cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]
      | array items =>
          cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]
      | record fields =>
          cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]

theorem eval_mul_comm (env : Env) (lhs rhs : MirExpr) :
    evalExpr env (.mul lhs rhs) = evalExpr env (.mul rhs lhs) := by
  cases hL : evalExpr env lhs with
  | none =>
      cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]
  | some lv =>
      cases lv with
      | int l =>
          cases hR : evalExpr env rhs with
          | none =>
              simp [evalExpr, hL, hR]
          | some rv =>
              cases rv with
              | int r =>
                  simp [evalExpr, hL, hR, Int.mul_comm]
              | bool b =>
                  simp [evalExpr, hL, hR]
              | null =>
                  simp [evalExpr, hL, hR]
              | array items =>
                  simp [evalExpr, hL, hR]
              | record fields =>
                  simp [evalExpr, hL, hR]
      | bool b =>
          cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]
      | null =>
          cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]
      | array items =>
          cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]
      | record fields =>
          cases hR : evalExpr env rhs <;> simp [evalExpr, hL, hR]

theorem canonicalizeExpr_preserves_eval (env : Env) (expr : MirExpr) :
    evalExpr env (canonicalizeExpr expr) = evalExpr env expr := by
  induction expr with
  | const value =>
      simp [canonicalizeExpr, evalExpr]
  | load name =>
      simp [canonicalizeExpr, evalExpr]
  | add lhs rhs ihL ihR =>
      cases hswap : exprLt (canonicalizeExpr rhs) (canonicalizeExpr lhs) with
      | false =>
          simp [canonicalizeExpr, hswap, evalExpr, ihL, ihR]
      | true =>
          calc
            evalExpr env (canonicalizeExpr (.add lhs rhs))
                = evalExpr env (.add (canonicalizeExpr rhs) (canonicalizeExpr lhs)) := by
                    simp [canonicalizeExpr, hswap]
            _ = evalExpr env (.add (canonicalizeExpr lhs) (canonicalizeExpr rhs)) := by
                    symm
                    exact eval_add_comm env (canonicalizeExpr lhs) (canonicalizeExpr rhs)
            _ = evalExpr env (.add lhs rhs) := by simp [evalExpr, ihL, ihR]
  | mul lhs rhs ihL ihR =>
      cases hswap : exprLt (canonicalizeExpr rhs) (canonicalizeExpr lhs) with
      | false =>
          simp [canonicalizeExpr, hswap, evalExpr, ihL, ihR]
      | true =>
          calc
            evalExpr env (canonicalizeExpr (.mul lhs rhs))
                = evalExpr env (.mul (canonicalizeExpr rhs) (canonicalizeExpr lhs)) := by
                    simp [canonicalizeExpr, hswap]
            _ = evalExpr env (.mul (canonicalizeExpr lhs) (canonicalizeExpr rhs)) := by
                    symm
                    exact eval_mul_comm env (canonicalizeExpr lhs) (canonicalizeExpr rhs)
            _ = evalExpr env (.mul lhs rhs) := by simp [evalExpr, ihL, ihR]
  | neg arg ih =>
      simp [canonicalizeExpr, evalExpr, ih]
  | lt lhs rhs ihL ihR =>
      simp [canonicalizeExpr, evalExpr, ihL, ihR]

abbrev ConstEnv := Env

def envAgreesOnConsts (env : Env) (consts : ConstEnv) : Prop :=
  ∀ name value, lookupEnv consts name = some value -> lookupEnv env name = some value

def constPropExpr (consts : ConstEnv) : MirExpr -> MirExpr
  | .const value => .const value
  | .load name =>
      match lookupEnv consts name with
      | some value => .const value
      | none => .load name
  | .add lhs rhs => .add (constPropExpr consts lhs) (constPropExpr consts rhs)
  | .mul lhs rhs => .mul (constPropExpr consts lhs) (constPropExpr consts rhs)
  | .neg arg => .neg (constPropExpr consts arg)
  | .lt lhs rhs => .lt (constPropExpr consts lhs) (constPropExpr consts rhs)

theorem constPropExpr_preserves_eval
    (env : Env) (consts : ConstEnv) (expr : MirExpr)
    (hAgree : envAgreesOnConsts env consts) :
    evalExpr env (constPropExpr consts expr) = evalExpr env expr := by
  induction expr with
  | const value =>
      simp [constPropExpr, evalExpr]
  | load name =>
      unfold constPropExpr
      cases hConst : lookupEnv consts name with
      | none =>
          simp [evalExpr]
      | some value =>
          have hEnv : lookupEnv env name = some value := hAgree name value hConst
          simp [evalExpr, hEnv]
  | add lhs rhs ihL ihR =>
      simp [constPropExpr, evalExpr, ihL, ihR]
  | mul lhs rhs ihL ihR =>
      simp [constPropExpr, evalExpr, ihL, ihR]
  | neg arg ih =>
      simp [constPropExpr, evalExpr, ih]
  | lt lhs rhs ihL ihR =>
      simp [constPropExpr, evalExpr, ihL, ihR]

def rewriteExpr (consts : ConstEnv) (expr : MirExpr) : MirExpr :=
  canonicalizeExpr (constPropExpr consts expr)

theorem rewriteExpr_preserves_eval
    (env : Env) (consts : ConstEnv) (expr : MirExpr)
    (hAgree : envAgreesOnConsts env consts) :
    evalExpr env (rewriteExpr consts expr) = evalExpr env expr := by
  unfold rewriteExpr
  calc
    evalExpr env (canonicalizeExpr (constPropExpr consts expr))
        = evalExpr env (constPropExpr consts expr) :=
          canonicalizeExpr_preserves_eval env (constPropExpr consts expr)
    _ = evalExpr env expr := constPropExpr_preserves_eval env consts expr hAgree

def exprDependsOn (target : String) : MirExpr -> Prop
  | .const _ => False
  | .load name => name = target
  | .add lhs rhs => exprDependsOn target lhs ∨ exprDependsOn target rhs
  | .mul lhs rhs => exprDependsOn target lhs ∨ exprDependsOn target rhs
  | .neg arg => exprDependsOn target arg
  | .lt lhs rhs => exprDependsOn target lhs ∨ exprDependsOn target rhs

theorem lookupEnv_updateEnv_ne
    (env : Env) (target name : String) (value : MirValue)
    (hNe : name ≠ target) :
    lookupEnv (updateEnv env target value) name = lookupEnv env name := by
  induction env with
  | nil =>
      by_cases hEq : target == name
      · have : target = name := by exact eq_of_beq hEq
        exact False.elim (hNe this.symm)
      · simp [lookupEnv, updateEnv, hEq]
  | cons entry rest ih =>
      cases entry with
      | mk field current =>
          by_cases hField : field == target
          · have hFieldEq : field = target := by exact eq_of_beq hField
            have hFieldNe : field ≠ name := by
              intro hEqName
              apply hNe
              calc
                name = field := by simpa using hEqName.symm
                _ = target := hFieldEq
            by_cases hName : field == name
            · have : field = name := by exact eq_of_beq hName
              exact False.elim (hFieldNe this)
            · simp [lookupEnv, updateEnv, hField, hName]
          · by_cases hName : field == name
            · simp [lookupEnv, updateEnv, hField, hName]
            · simpa [lookupEnv, updateEnv, hField, hName] using ih

theorem evalExpr_update_irrelevant
    (env : Env) (target : String) (value : MirValue) (expr : MirExpr)
    (hNoRead : ¬ exprDependsOn target expr) :
    evalExpr (updateEnv env target value) expr = evalExpr env expr := by
  induction expr with
  | const val =>
      simp [evalExpr]
  | load name =>
      simp [exprDependsOn] at hNoRead
      simp [evalExpr, lookupEnv_updateEnv_ne env target name value hNoRead]
  | add lhs rhs ihL ihR =>
      have hL : ¬ exprDependsOn target lhs := by
        intro h
        exact hNoRead (Or.inl h)
      have hR : ¬ exprDependsOn target rhs := by
        intro h
        exact hNoRead (Or.inr h)
      simp [evalExpr, ihL hL, ihR hR]
  | mul lhs rhs ihL ihR =>
      have hL : ¬ exprDependsOn target lhs := by
        intro h
        exact hNoRead (Or.inl h)
      have hR : ¬ exprDependsOn target rhs := by
        intro h
        exact hNoRead (Or.inr h)
      simp [evalExpr, ihL hL, ihR hR]
  | neg arg ih =>
      simp [exprDependsOn] at hNoRead
      simp [evalExpr, ih hNoRead]
  | lt lhs rhs ihL ihR =>
      have hL : ¬ exprDependsOn target lhs := by
        intro h
        exact hNoRead (Or.inl h)
      have hR : ¬ exprDependsOn target rhs := by
        intro h
        exact hNoRead (Or.inr h)
      simp [evalExpr, ihL hL, ihR hR]

structure StraightLineBlock where
  instrs : List (String × MirExpr)
  ret : MirExpr
deriving Repr

def execAssigns (env : Env) : List (String × MirExpr) -> Option Env
  | [] => some env
  | (dst, rhs) :: rest => do
      let value <- evalExpr env rhs
      execAssigns (updateEnv env dst value) rest

def execStraightLineBlock (env : Env) (blk : StraightLineBlock) : Option MirValue := do
  let env' <- execAssigns env blk.instrs
  evalExpr env' blk.ret

theorem execAssigns_append
    (env : Env) (instrPrefix suffix : List (String × MirExpr)) :
    execAssigns env (instrPrefix ++ suffix) =
      match execAssigns env instrPrefix with
      | some env' => execAssigns env' suffix
      | none => none := by
  induction instrPrefix generalizing env with
  | nil =>
      simp [execAssigns]
  | cons head rest ih =>
      cases head with
      | mk dst rhs =>
          cases h : evalExpr env rhs <;> simp [execAssigns, h, ih]

theorem drop_last_dead_assign_preserves_block
    (env env' : Env)
    (instrPrefix : List (String × MirExpr))
    (dst : String) (rhs ret : MirExpr) (value : MirValue)
    (hPrefix : execAssigns env instrPrefix = some env')
    (hRhs : evalExpr env' rhs = some value)
    (hRet : ¬ exprDependsOn dst ret) :
    execStraightLineBlock env { instrs := instrPrefix ++ [(dst, rhs)], ret := ret } =
      execStraightLineBlock env { instrs := instrPrefix, ret := ret } := by
  simp [execStraightLineBlock, execAssigns_append, hPrefix]
  simpa [execAssigns, hRhs] using evalExpr_update_irrelevant env' dst value ret hRet

theorem identity_dataflow_layer_preserves_verify_ir_bundle
    {fn : MirFnLite} (h : OptimizerEligible fn) :
    OptimizerEligible (identityPass fn) := by
  exact identity_pass_preserves_verify_ir_bundle h

end RRProofs.DataflowOptSoundness
