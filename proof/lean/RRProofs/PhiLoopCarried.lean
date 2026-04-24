set_option linter.unusedVariables false

namespace RRProofs.PhiLoopCarried

abbrev Var := String
abbrev State := Var -> Int

inductive LoopExpr where
  | const : Int -> LoopExpr
  | preVar : Var -> LoopExpr
  | loopVar : Var -> LoopExpr
  | phi : Var -> Var -> LoopExpr
  | add : LoopExpr -> LoopExpr -> LoopExpr
deriving DecidableEq, Repr

def LoopExpr.eval (iter : Nat) (entry : State) (loop : State) : LoopExpr -> Int
  | .const n => n
  | .preVar x => entry x
  | .loopVar x => loop x
  | .phi seed carried => if iter = 0 then entry seed else loop carried
  | .add lhs rhs => lhs.eval iter entry loop + rhs.eval iter entry loop

def carriedDeps : LoopExpr -> List Var
  | .const _ => []
  | .preVar _ => []
  | .loopVar x => [x]
  | .phi _ carried => [carried]
  | .add lhs rhs => carriedDeps lhs ++ carriedDeps rhs

def safeToHoist (e : LoopExpr) : Prop :=
  carriedDeps e = []

theorem phi_depends_on_carried_after_entry
    (entry : State)
    (loop₁ loop₂ : State)
    (seed carried : Var)
    (h : loop₁ carried ≠ loop₂ carried) :
    (LoopExpr.phi seed carried).eval (Nat.succ 0) entry loop₁ ≠
      (LoopExpr.phi seed carried).eval (Nat.succ 0) entry loop₂ := by
  simp [LoopExpr.eval]
  exact h

theorem phi_plus_const_depends_on_carried_after_entry
    (entry : State)
    (loop₁ loop₂ : State)
    (seed carried : Var)
    (k : Int)
    (h : loop₁ carried ≠ loop₂ carried) :
    (LoopExpr.add (.phi seed carried) (.const k)).eval (Nat.succ 0) entry loop₁ ≠
      (LoopExpr.add (.phi seed carried) (.const k)).eval (Nat.succ 0) entry loop₂ := by
  simp [LoopExpr.eval]
  intro hEq
  have hEq' := congrArg (fun n => n - k) hEq
  have hEq'' : loop₁ carried = loop₂ carried := by
    simpa using hEq'
  exact h hEq''

theorem phi_not_safe_to_hoist (seed carried : Var) :
    ¬ safeToHoist (.phi seed carried) := by
  simp [safeToHoist, carriedDeps]

theorem phi_plus_const_not_safe_to_hoist (seed carried : Var) (k : Int) :
    ¬ safeToHoist (.add (.phi seed carried) (.const k)) := by
  simp [safeToHoist, carriedDeps]

theorem hoisting_loop_phi_as_constant_is_unsound
    (entry : State)
    (loop₁ loop₂ : State)
    (seed carried _tmp : Var)
    (hCarried : loop₁ carried ≠ loop₂ carried) :
    let original := LoopExpr.phi seed carried
    let hoisted₁ := LoopExpr.const (original.eval (Nat.succ 0) entry loop₁)
    let hoisted₂ := LoopExpr.const (original.eval (Nat.succ 0) entry loop₂)
    hoisted₁.eval (Nat.succ 1) entry loop₁ ≠ hoisted₂.eval (Nat.succ 1) entry loop₂ := by
  simp [LoopExpr.eval]
  exact hCarried

end RRProofs.PhiLoopCarried
