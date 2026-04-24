set_option linter.unusedVariables false

namespace RRProofs

inductive DceExpr where
  | pureConst : Int -> DceExpr
  | impureCall : String -> DceExpr
  | add : DceExpr -> DceExpr -> DceExpr
  | intrinsic : String -> DceExpr -> DceExpr
  | record1 : String -> DceExpr -> DceExpr
  | fieldGet : DceExpr -> String -> DceExpr
  | fieldSet : DceExpr -> String -> DceExpr -> DceExpr
  | len : DceExpr -> DceExpr
  | range : DceExpr -> DceExpr -> DceExpr
  | indices : DceExpr -> DceExpr
  | index1D : DceExpr -> DceExpr -> DceExpr
  | index2D : DceExpr -> DceExpr -> DceExpr -> DceExpr
  | index3D : DceExpr -> DceExpr -> DceExpr -> DceExpr -> DceExpr
  | phi : DceExpr -> DceExpr -> DceExpr
deriving Repr

def effectCount : DceExpr -> Nat
  | .pureConst _ => 0
  | .impureCall _ => 1
  | .add lhs rhs => effectCount lhs + effectCount rhs
  | .intrinsic _ arg => effectCount arg
  | .record1 _ value => effectCount value
  | .fieldGet base _ => effectCount base
  | .fieldSet base _ value => effectCount base + effectCount value
  | .len base => effectCount base
  | .range start stop => effectCount start + effectCount stop
  | .indices base => effectCount base
  | .index1D base idx => effectCount base + effectCount idx
  | .index2D base r c => effectCount base + effectCount r + effectCount c
  | .index3D base i j k => effectCount base + effectCount i + effectCount j + effectCount k
  | .phi lhs rhs => effectCount lhs + effectCount rhs

inductive DceInstr where
  | eval : DceExpr -> DceInstr
deriving Repr

def effectInstr : DceInstr -> Nat
  | .eval expr => effectCount expr

def effectInstrs : List DceInstr -> Nat
  | [] => 0
  | instr :: rest => effectInstr instr + effectInstrs rest

def dceDeadAssign (expr : DceExpr) : List DceInstr :=
  if h : effectCount expr = 0 then [] else [.eval expr]

theorem dceDeadAssign_preserves_effects (expr : DceExpr) :
    effectInstrs (dceDeadAssign expr) = effectCount expr := by
  unfold dceDeadAssign
  by_cases h : effectCount expr = 0
  · simp [h, effectInstrs]
  · simp [h, effectInstrs, effectInstr]

theorem dceDeadAssign_pure_erases (expr : DceExpr) (h : effectCount expr = 0) :
    dceDeadAssign expr = [] := by
  simp [dceDeadAssign, h]

theorem dceDeadAssign_impure_demotes_to_eval (expr : DceExpr) (h : effectCount expr ≠ 0) :
    dceDeadAssign expr = [.eval expr] := by
  simp [dceDeadAssign, h]

def nestedFieldSetExpr : DceExpr :=
  .fieldSet (.record1 "x" (.pureConst 1)) "x" (.impureCall "f")

def nestedIndex3DExpr : DceExpr :=
  .index3D (.pureConst 1) (.impureCall "f") (.pureConst 1) (.pureConst 1)

def nestedPhiExpr : DceExpr :=
  .phi (.impureCall "f") (.pureConst 1)

def nestedRangeExpr : DceExpr :=
  .range (.impureCall "f") (.pureConst 1)

def nestedIndicesExpr : DceExpr :=
  .indices (.impureCall "f")

theorem nestedFieldSet_preserved :
    effectInstrs (dceDeadAssign nestedFieldSetExpr) = 1 := by
  simpa [nestedFieldSetExpr, effectCount] using dceDeadAssign_preserves_effects nestedFieldSetExpr

theorem nestedIndex3D_preserved :
    effectInstrs (dceDeadAssign nestedIndex3DExpr) = 1 := by
  simpa [nestedIndex3DExpr, effectCount] using dceDeadAssign_preserves_effects nestedIndex3DExpr

theorem nestedPhi_preserved :
    effectInstrs (dceDeadAssign nestedPhiExpr) = 1 := by
  simpa [nestedPhiExpr, effectCount] using dceDeadAssign_preserves_effects nestedPhiExpr

theorem nestedRange_preserved :
    effectInstrs (dceDeadAssign nestedRangeExpr) = 1 := by
  simpa [nestedRangeExpr, effectCount] using dceDeadAssign_preserves_effects nestedRangeExpr

theorem nestedIndices_preserved :
    effectInstrs (dceDeadAssign nestedIndicesExpr) = 1 := by
  simpa [nestedIndicesExpr, effectCount] using dceDeadAssign_preserves_effects nestedIndicesExpr

end RRProofs
