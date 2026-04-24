import RRProofs.VectorizeDecisionSubset

set_option linter.unusedVariables false

namespace RRProofs

inductive TinyTree where
  | constInt : TinyValueId -> Option String -> Int -> TinyTree
  | load : TinyValueId -> Option String -> String -> TinyTree
  | add : TinyValueId -> Option String -> TinyTree -> TinyTree -> TinyTree
deriving Repr

def treeId : TinyTree -> TinyValueId
  | .constInt id _ _ => id
  | .load id _ _ => id
  | .add id _ _ _ => id

def treeOrigin : TinyTree -> Option String
  | .constInt _ origin _ => origin
  | .load _ origin _ => origin
  | .add _ origin _ _ => origin

def treeKindSig : TinyTree -> TinyKind
  | .load _ _ v => .load v
  | _ => .other

def treeNode (t : TinyTree) : TinyNode :=
  { id := treeId t, originVar? := treeOrigin t, kind := treeKindSig t }

def eraseTree : TinyTree -> RewriteExpr
  | .constInt _ _ i => .constInt i
  | .load _ _ v => .load v
  | .add _ _ lhs rhs => .add (eraseTree lhs) (eraseTree rhs)

def evalTree (ρ : ValEnv) (t : TinyTree) : Int :=
  evalRewriteExpr ρ (eraseTree t)

abbrev RewriteResult := TinyValueId × RewriteExpr × TinyValueId × Bool

def rrId (r : RewriteResult) : TinyValueId := r.1
def rrExpr (r : RewriteResult) : RewriteExpr := r.2.1
def rrNext (r : RewriteResult) : TinyValueId := r.2.2.1
def rrChanged (r : RewriteResult) : Bool := r.2.2.2

def treeSize : TinyTree -> Nat
  | .constInt _ _ _ => 1
  | .load _ _ _ => 1
  | .add _ _ lhs rhs => treeSize lhs + treeSize rhs + 1

def rewriteTreeFuel
    (target : String)
    (replacementExpr : RewriteExpr)
    (replacementId : TinyValueId)
    (nextId : TinyValueId) : Nat -> TinyTree -> RewriteResult
  | 0, tree => (treeId tree, eraseTree tree, nextId, false)
  | fuel + 1, tree =>
      let baseId := boundaryRewrite target replacementId (treeNode tree)
      if baseId == treeId tree then
        match tree with
          | .constInt id _ i => (id, .constInt i, nextId, false)
          | .load id _ v => (id, .load v, nextId, false)
          | .add id _ lhs rhs =>
              let (lid, lhsExpr, next1, changedLhs) :=
              rewriteTreeFuel target replacementExpr replacementId nextId fuel lhs
              let (rid, rhsExpr, next2, changedRhs) :=
              rewriteTreeFuel target replacementExpr replacementId next1 fuel rhs
              if changedLhs || changedRhs then
                (allocateRewriteId next2 id true, .add lhsExpr rhsExpr, next2 + 1, true)
              else
                (id, .add lhsExpr rhsExpr, next2, false)
      else
        (baseId, replacementExpr, nextId, true)

def rewriteTree
    (target : String)
    (replacementExpr : RewriteExpr)
    (replacementId : TinyValueId)
    (nextId : TinyValueId)
    (tree : TinyTree) : RewriteResult :=
  rewriteTreeFuel target replacementExpr replacementId nextId (treeSize tree) tree

def sampleTreeUnchanged : TinyTree :=
  .constInt 4 none 7

def sampleTreeChanged : TinyTree :=
  .add 4 none (.constInt 1 (some "dest") 7) (.constInt 2 none 3)

theorem sampleTreeUnchanged_reuses_root :
    rrId (rewriteTree "dest" (.constInt 7) 9 20 sampleTreeUnchanged) = 4 := by
  native_decide

theorem sampleTreeChanged_allocates_fresh :
    rrId (rewriteTree "dest" (.constInt 7) 9 20 sampleTreeChanged) = 20 := by
  native_decide

theorem sampleTreeChanged_nextId_advanced :
    rrNext (rewriteTree "dest" (.constInt 7) 9 20 sampleTreeChanged) = 21 := by
  native_decide

theorem sampleTreeChanged_nextId_monotone :
    20 ≤ rrNext (rewriteTree "dest" (.constInt 7) 9 20 sampleTreeChanged) := by
  native_decide

theorem sampleTreeChanged_preserves_eval :
    evalRewriteExpr sampleUseEnv (rrExpr (rewriteTree "dest" sampleReplacementUse 9 20 sampleTreeChanged)) = 10 := by
  native_decide

end RRProofs
