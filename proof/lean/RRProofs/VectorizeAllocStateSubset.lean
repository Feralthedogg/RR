import RRProofs.VectorizeTreeRewriteSubset

namespace RRProofs

abbrev TreeRewriteOut := List (TinyValueId × RewriteExpr)

def rewriteTreeList
    (target : String)
    (replacementExpr : RewriteExpr)
    (replacementId : TinyValueId)
    (nextId : TinyValueId) : List TinyTree -> TreeRewriteOut × TinyValueId
  | [] => ([], nextId)
  | tree :: rest =>
      let rewritten := rewriteTree target replacementExpr replacementId nextId tree
      let tail := rewriteTreeList target replacementExpr replacementId (rrNext rewritten) rest
      ((rrId rewritten, rrExpr rewritten) :: tail.1, tail.2)

def evalTreeList (ρ : ValEnv) (trees : List TinyTree) : List Int :=
  trees.map (evalTree ρ)

def evalRewriteOut (ρ : ValEnv) (out : TreeRewriteOut) : List Int :=
  out.map (fun (_, expr) => evalRewriteExpr ρ expr)

def sampleTreeList : List TinyTree :=
  [sampleTreeUnchanged, sampleTreeChanged]

def sampleChangedTreeList : List TinyTree :=
  [sampleTreeChanged, sampleTreeChanged]

theorem sampleTreeList_preserved :
    evalRewriteOut sampleUseEnv (rewriteTreeList "dest" sampleReplacementUse 9 20 sampleTreeList).1
      = evalTreeList sampleUseEnv sampleTreeList := by
  native_decide

theorem sampleTreeList_final_nextId :
    (rewriteTreeList "dest" sampleReplacementUse 9 20 sampleTreeList).2 = 21 := by
  native_decide

theorem sampleChangedTreeList_final_nextId :
    (rewriteTreeList "dest" sampleReplacementUse 9 20 sampleChangedTreeList).2 = 22 := by
  native_decide

theorem sampleChangedTreeList_fresh_ids :
    (rewriteTreeList "dest" sampleReplacementUse 9 20 sampleChangedTreeList).1.map Prod.fst = [20, 21] := by
  native_decide

end RRProofs
