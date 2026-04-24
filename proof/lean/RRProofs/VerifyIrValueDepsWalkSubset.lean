import RRProofs.VerifyIrChildDepsSubset

set_option linter.unusedVariables false

namespace RRProofs

inductive ValueDepsKind where
  | constLike
  | paramLike
  | loadLike
  | rSymbolLike
  | len (base : ConsumerNodeId)
  | indices (base : ConsumerNodeId)
  | unary (base : ConsumerNodeId)
  | fieldGet (base : ConsumerNodeId)
  | range (start finish : ConsumerNodeId)
  | binary (lhs rhs : ConsumerNodeId)
  | phi (args : List ConsumerNodeId)
  | call (args : List ConsumerNodeId)
  | intrinsic (args : List ConsumerNodeId)
  | recordLit (fields : List (String × ConsumerNodeId))
  | fieldSet (base value : ConsumerNodeId)
  | index1d (base idx : ConsumerNodeId)
  | index2d (base r c : ConsumerNodeId)
  | index3d (base i j k : ConsumerNodeId)
deriving Repr, DecidableEq

def valueDeps : ValueDepsKind -> List ConsumerNodeId
  | .constLike | .paramLike | .loadLike | .rSymbolLike => []
  | .len base | .indices base | .unary base | .fieldGet base => [base]
  | .range start finish | .binary start finish | .fieldSet start finish
  | .index1d start finish => [start, finish]
  | .phi args | .call args | .intrinsic args => args
  | .recordLit fields => fields.map Prod.snd
  | .index2d base r c => [base, r, c]
  | .index3d base i j k => [base, i, j, k]

def toChildDepsKind? : ValueDepsKind -> Option ChildDepsKind
  | .constLike => some .constLike
  | .paramLike => some .paramLike
  | .loadLike => some .loadLike
  | .rSymbolLike => some .rSymbolLike
  | .len base => some (.len base)
  | .indices base => some (.indices base)
  | .unary base => some (.unary base)
  | .fieldGet base => some (.fieldGet base)
  | .range start finish => some (.range start finish)
  | .binary lhs rhs => some (.binary lhs rhs)
  | .phi _ => none
  | .call args => some (.call args)
  | .intrinsic args => some (.intrinsic args)
  | .recordLit fields => some (.recordLit fields)
  | .fieldSet base value => some (.fieldSet base value)
  | .index1d base idx => some (.index1d base idx)
  | .index2d base r c => some (.index2d base r c)
  | .index3d base i j k => some (.index3d base i j k)

theorem valueDeps_eq_nonPhiDeps_of_toChildDeps
    (kind : ValueDepsKind) (childKind : ChildDepsKind)
    (h : toChildDepsKind? kind = some childKind) :
    valueDeps kind = nonPhiDeps childKind := by
  cases kind <;> cases h <;> rfl

structure PhiWalkNode where
  phiBlock : Option Nat
  depsKind : ValueDepsKind

abbrev PhiWalkGraph := ConsumerNodeId -> Option PhiWalkNode

mutual
  def dependsOnPhiInBlockExceptFuel :
      Nat -> PhiWalkGraph -> List ConsumerNodeId -> ConsumerNodeId -> Nat -> ConsumerNodeId -> Prop
    | 0, _, _, _, _, _ => False
    | fuel + 1, graph, seen, root, phiBlock, exempt =>
        if root ∈ seen then
          False
        else
          match graph root with
          | none => False
          | some node =>
              ((root ≠ exempt) ∧ node.phiBlock = some phiBlock) ∨
                depListDependsOnPhiFuel fuel graph (root :: seen) (valueDeps node.depsKind) phiBlock exempt

  def depListDependsOnPhiFuel :
      Nat -> PhiWalkGraph -> List ConsumerNodeId -> List ConsumerNodeId -> Nat -> ConsumerNodeId -> Prop
    | 0, _, _, _, _, _ => False
    | fuel + 1, graph, seen, [], _, _ => False
    | fuel + 1, graph, seen, root :: rest, phiBlock, exempt =>
        dependsOnPhiInBlockExceptFuel fuel graph seen root phiBlock exempt ∨
          depListDependsOnPhiFuel fuel graph seen rest phiBlock exempt
end

theorem dependsOnPhiInBlockExceptFuel_here
    (fuel : Nat) (graph : PhiWalkGraph) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock exempt : Nat) (depsKind : ValueDepsKind)
    (hNode : graph root = some { phiBlock := some phiBlock, depsKind := depsKind })
    (hFresh : root ∉ seen)
    (hNe : root ≠ exempt) :
    dependsOnPhiInBlockExceptFuel (fuel + 1) graph seen root phiBlock exempt := by
  simp [dependsOnPhiInBlockExceptFuel, hFresh, hNode, hNe]

theorem depListDependsOnPhiFuel_head
    (fuel : Nat) (graph : PhiWalkGraph) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (rest : List ConsumerNodeId)
    (phiBlock exempt : Nat)
    (hRoot : dependsOnPhiInBlockExceptFuel fuel graph seen root phiBlock exempt) :
    depListDependsOnPhiFuel (fuel + 1) graph seen (root :: rest) phiBlock exempt := by
  simp [depListDependsOnPhiFuel, hRoot]

theorem depListDependsOnPhiFuel_tail
    (fuel : Nat) (graph : PhiWalkGraph) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (rest : List ConsumerNodeId)
    (phiBlock exempt : Nat)
    (hRest : depListDependsOnPhiFuel fuel graph seen rest phiBlock exempt) :
    depListDependsOnPhiFuel (fuel + 1) graph seen (root :: rest) phiBlock exempt := by
  simp [depListDependsOnPhiFuel, hRest]

theorem dependsOnPhiInBlockExceptFuel_of_depList
    (fuel : Nat) (graph : PhiWalkGraph) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (phiBlock exempt : Nat) (node : PhiWalkNode)
    (hNode : graph root = some node)
    (hFresh : root ∉ seen)
    (hDeps : depListDependsOnPhiFuel fuel graph (root :: seen) (valueDeps node.depsKind) phiBlock exempt) :
    dependsOnPhiInBlockExceptFuel (fuel + 1) graph seen root phiBlock exempt := by
  simp [dependsOnPhiInBlockExceptFuel, hFresh, hNode, hDeps]

def examplePhiWalkGraph : PhiWalkGraph
  | 1 => some { phiBlock := none, depsKind := .binary 2 3 }
  | 2 => some { phiBlock := some 7, depsKind := .phi [4] }
  | 3 => some { phiBlock := none, depsKind := .call [5, 6] }
  | 4 => some { phiBlock := some 7, depsKind := .constLike }
  | 5 => some { phiBlock := none, depsKind := .constLike }
  | 6 => some { phiBlock := some 8, depsKind := .constLike }
  | 8 => some { phiBlock := none, depsKind := .binary 8 2 }
  | _ => none

theorem exampleValueDeps_phi_shape :
    valueDeps (.phi [4, 6, 1]) = [4, 6, 1] := rfl

theorem exampleValueDeps_non_phi_matches_childDeps :
    valueDeps (.index3d 4 5 6 1) = nonPhiDeps (.index3d 4 5 6 1) := by
  exact valueDeps_eq_nonPhiDeps_of_toChildDeps _ _ rfl

theorem exampleDepends_direct_phi :
    dependsOnPhiInBlockExceptFuel 3 examplePhiWalkGraph [] 1 7 99 := by
  apply dependsOnPhiInBlockExceptFuel_of_depList
    (fuel := 2) (graph := examplePhiWalkGraph) (seen := []) (root := 1) (phiBlock := 7)
    (exempt := 99) (node := { phiBlock := none, depsKind := .binary 2 3 })
  · rfl
  · simp
  · apply depListDependsOnPhiFuel_head
    exact dependsOnPhiInBlockExceptFuel_here
      (fuel := 0) (graph := examplePhiWalkGraph) (seen := [1]) (root := 2)
      (phiBlock := 7) (exempt := 99) (.phi [4]) rfl (by simp) (by decide)

theorem exampleDepends_exempt_phi_through_arg :
    dependsOnPhiInBlockExceptFuel 3 examplePhiWalkGraph [] 2 7 2 := by
  apply dependsOnPhiInBlockExceptFuel_of_depList
    (fuel := 2) (graph := examplePhiWalkGraph) (seen := []) (root := 2) (phiBlock := 7)
    (exempt := 2) (node := { phiBlock := some 7, depsKind := .phi [4] })
  · rfl
  · simp
  · apply depListDependsOnPhiFuel_head
    exact dependsOnPhiInBlockExceptFuel_here
      (fuel := 0) (graph := examplePhiWalkGraph) (seen := [2]) (root := 4)
      (phiBlock := 7) (exempt := 2) .constLike rfl (by simp) (by decide)

theorem exampleDepends_other_block_ignored :
    ¬ dependsOnPhiInBlockExceptFuel 3 examplePhiWalkGraph [] 3 7 99 := by
  simp [dependsOnPhiInBlockExceptFuel, depListDependsOnPhiFuel, examplePhiWalkGraph, valueDeps]

theorem exampleDepends_self_loop_skips_seen_but_finds_phi :
    dependsOnPhiInBlockExceptFuel 4 examplePhiWalkGraph [] 8 7 99 := by
  apply dependsOnPhiInBlockExceptFuel_of_depList
    (fuel := 3) (graph := examplePhiWalkGraph) (seen := []) (root := 8) (phiBlock := 7)
    (exempt := 99) (node := { phiBlock := none, depsKind := .binary 8 2 })
  · rfl
  · simp
  · apply depListDependsOnPhiFuel_tail
    apply depListDependsOnPhiFuel_head
    exact dependsOnPhiInBlockExceptFuel_here
      (fuel := 0) (graph := examplePhiWalkGraph) (seen := [8]) (root := 2)
      (phiBlock := 7) (exempt := 99) (.phi [4]) rfl (by simp) (by decide)

end RRProofs
