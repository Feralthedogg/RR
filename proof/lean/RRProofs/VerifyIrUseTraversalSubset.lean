import RRProofs.VerifyIrMustDefConvergenceSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

inductive UseExpr where
  | const : UseExpr
  | load : Var -> UseExpr
  | wrap : UseExpr -> UseExpr
  | pair : UseExpr -> UseExpr -> UseExpr
  | many : List UseExpr -> UseExpr
  | phi : List UseExpr -> UseExpr
deriving Repr

mutual
  def firstUndefinedLoad (defined : DefSet) (followPhi : Bool) : UseExpr -> Option Var
    | .const => none
    | .load v => if v ∈ defined then none else some v
    | .wrap e => firstUndefinedLoad defined followPhi e
    | .pair lhs rhs =>
        match firstUndefinedLoad defined followPhi lhs with
        | some v => some v
        | none => firstUndefinedLoad defined followPhi rhs
    | .many es => firstUndefinedLoadList defined followPhi es
    | .phi es =>
        if followPhi then firstUndefinedLoadList defined followPhi es else none

  def firstUndefinedLoadList (defined : DefSet) (followPhi : Bool) : List UseExpr -> Option Var
    | [] => none
    | e :: rest =>
        match firstUndefinedLoad defined followPhi e with
        | some v => some v
        | none => firstUndefinedLoadList defined followPhi rest
end

mutual
  def loadsDefined (defined : DefSet) (followPhi : Bool) : UseExpr -> Prop
    | .const => True
    | .load v => v ∈ defined
    | .wrap e => loadsDefined defined followPhi e
    | .pair lhs rhs => loadsDefined defined followPhi lhs ∧ loadsDefined defined followPhi rhs
    | .many es => loadsDefinedList defined followPhi es
    | .phi es => if followPhi then loadsDefinedList defined followPhi es else True

  def loadsDefinedList (defined : DefSet) (followPhi : Bool) : List UseExpr -> Prop
    | [] => True
    | e :: rest => loadsDefined defined followPhi e ∧ loadsDefinedList defined followPhi rest
end

mutual
  theorem firstUndefinedLoad_none_of_loadsDefined
      (defined : DefSet) (followPhi : Bool) :
      ∀ e, loadsDefined defined followPhi e -> firstUndefinedLoad defined followPhi e = none
    | .const, _ => rfl
    | .load v, h => by
        have hv : v ∈ defined := by
          simpa [loadsDefined] using h
        simp [firstUndefinedLoad, hv]
    | .wrap e, h => firstUndefinedLoad_none_of_loadsDefined defined followPhi e h
    | .pair lhs rhs, h => by
        rcases h with ⟨hL, hR⟩
        simp [firstUndefinedLoad,
          firstUndefinedLoad_none_of_loadsDefined defined followPhi lhs hL,
          firstUndefinedLoad_none_of_loadsDefined defined followPhi rhs hR]
    | .many es, h => firstUndefinedLoadList_none_of_loadsDefined defined followPhi es h
    | .phi es, h => by
        by_cases hPhi : followPhi
        · have hEs : loadsDefinedList defined true es := by
            simpa [loadsDefined, hPhi] using h
          simpa [firstUndefinedLoad, hPhi] using
            firstUndefinedLoadList_none_of_loadsDefined defined true es hEs
        · simp [firstUndefinedLoad, loadsDefined, hPhi]

  theorem firstUndefinedLoadList_none_of_loadsDefined
      (defined : DefSet) (followPhi : Bool) :
      ∀ es, loadsDefinedList defined followPhi es -> firstUndefinedLoadList defined followPhi es = none
    | [], _ => rfl
    | e :: rest, h => by
        rcases h with ⟨hHead, hRest⟩
        simp [firstUndefinedLoadList, firstUndefinedLoad_none_of_loadsDefined defined followPhi e hHead,
          firstUndefinedLoadList_none_of_loadsDefined defined followPhi rest hRest]
end

def exampleTraversalExpr : UseExpr :=
  .pair (.wrap (.load "x")) (.many [.load "tmp", .const])

def examplePhiTraversalExpr : UseExpr :=
  .phi [.load "missing"]

theorem exampleStableTraversal_loadsDefined :
    loadsDefined
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      true
      exampleTraversalExpr := by
  rw [exampleStableSeed_iterate_five_block3]
  simp [exampleTraversalExpr, loadsDefined, loadsDefinedList]

theorem exampleStableTraversal_scan_clean :
    firstUndefinedLoad
      (iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
        5 exampleStableSeed 3)
      true
      exampleTraversalExpr = none := by
  exact firstUndefinedLoad_none_of_loadsDefined _ _ _ exampleStableTraversal_loadsDefined

theorem examplePhiTraversal_ignored_when_not_following :
    firstUndefinedLoad [] false examplePhiTraversalExpr = none := by
  simp [examplePhiTraversalExpr, firstUndefinedLoad]

end RRProofs
