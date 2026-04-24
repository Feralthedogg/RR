import RRProofs.VectorizeSubset

namespace RRProofs

inductive ReducedVectorPlan where
  | exprMap : List LoopInstr -> Int -> Int -> ReducedVectorPlan
  | condMap : List LoopInstr -> List LoopInstr -> Int -> Int -> ReducedVectorPlan
deriving Repr

def scalarResult : ReducedVectorPlan -> Int
  | .exprMap _ scalar _ => scalar
  | .condMap _ _ scalar _ => scalar

def vectorResult : ReducedVectorPlan -> Int
  | .exprMap _ _ vec => vec
  | .condMap _ _ _ vec => vec

def certifyPlan : ReducedVectorPlan -> Bool
  | .exprMap body _ _ => certifyExprMap body
  | .condMap thenBranch elseBranch _ _ => certifyCondMap thenBranch elseBranch

def transactionalApply : ReducedVectorPlan -> Int
  | plan =>
      if certifyPlan plan then vectorResult plan else scalarResult plan

def resultPreserving (plan : ReducedVectorPlan) : Prop :=
  vectorResult plan = scalarResult plan

theorem transactionalApply_rolls_back_on_reject
    (plan : ReducedVectorPlan)
    (hReject : certifyPlan plan = false) :
    transactionalApply plan = scalarResult plan := by
  simp [transactionalApply, hReject]

theorem transactionalApply_commits_preserving_plan
    (plan : ReducedVectorPlan)
    (hCert : certifyPlan plan = true)
    (hPres : resultPreserving plan) :
    transactionalApply plan = scalarResult plan := by
  simp [transactionalApply, hCert, resultPreserving] at *
  simp [hPres]

def pureExprMapCase : ReducedVectorPlan :=
  .exprMap [.pureAssign, .pureAssign] 7 7

def rejectExprMapCase : ReducedVectorPlan :=
  .exprMap [.pureAssign, .eval] 7 99

def storeOnlyCondCase : ReducedVectorPlan :=
  .condMap [.store] [.store] 4 4

def rejectCondBranchCase : ReducedVectorPlan :=
  .condMap [.store, .pureAssign] [.store] 4 99

theorem pureExprMapCase_preserved :
    transactionalApply pureExprMapCase = 7 := by
  apply transactionalApply_commits_preserving_plan
  · decide
  · rfl

theorem rejectExprMapCase_rolls_back :
    transactionalApply rejectExprMapCase = 7 := by
  apply transactionalApply_rolls_back_on_reject
  decide

theorem storeOnlyCondCase_preserved :
    transactionalApply storeOnlyCondCase = 4 := by
  apply transactionalApply_commits_preserving_plan
  · decide
  · rfl

theorem rejectCondBranchCase_rolls_back :
    transactionalApply rejectCondBranchCase = 4 := by
  apply transactionalApply_rolls_back_on_reject
  decide

end RRProofs
