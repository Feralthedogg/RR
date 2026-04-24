import RRProofs.VerifyIrConsumerGraphSubset

namespace RRProofs

inductive ChildDepsKind where
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
  | phi
  | call (args : List ConsumerNodeId)
  | intrinsic (args : List ConsumerNodeId)
  | recordLit (fields : List (String × ConsumerNodeId))
  | fieldSet (base value : ConsumerNodeId)
  | index1d (base idx : ConsumerNodeId)
  | index2d (base r c : ConsumerNodeId)
  | index3d (base i j k : ConsumerNodeId)

def nonPhiDeps : ChildDepsKind -> List ConsumerNodeId
  | .constLike | .paramLike | .loadLike | .rSymbolLike | .phi => []
  | .len base | .indices base | .unary base | .fieldGet base => [base]
  | .range start finish | .binary start finish | .fieldSet start finish
  | .index1d start finish => [start, finish]
  | .call args | .intrinsic args => args
  | .recordLit fields => fields.map Prod.snd
  | .index2d base r c => [base, r, c]
  | .index3d base i j k => [base, i, j, k]

def depTraversalCleanFuel
    (fuel : Nat) (graph : ConsumerGraph) (seen : List ConsumerNodeId)
    (kind : ChildDepsKind) : Prop :=
  rootListScanCleanFuel fuel graph seen (nonPhiDeps kind)

theorem depTraversalCleanFuel_of_rootList
    (fuel : Nat) (graph : ConsumerGraph) (seen : List ConsumerNodeId)
    (kind : ChildDepsKind)
    (h : rootListScanCleanFuel fuel graph seen (nonPhiDeps kind)) :
    depTraversalCleanFuel fuel graph seen kind := h

theorem depTraversalCleanFuel_constLike
    (fuel : Nat) (graph : ConsumerGraph) (seen : List ConsumerNodeId) :
    depTraversalCleanFuel fuel graph seen .constLike := by
  cases fuel <;> simp [depTraversalCleanFuel, nonPhiDeps, rootListScanCleanFuel]

theorem depTraversalCleanFuel_phi
    (fuel : Nat) (graph : ConsumerGraph) (seen : List ConsumerNodeId) :
    depTraversalCleanFuel fuel graph seen .phi := by
  cases fuel <;> simp [depTraversalCleanFuel, nonPhiDeps, rootListScanCleanFuel]

theorem exampleUnaryDepTraversal_clean :
    depTraversalCleanFuel 2 exampleConsumerGraph [] (.unary 6) := by
  simp [depTraversalCleanFuel, nonPhiDeps]
  exact rootListScanCleanFuel_cons 1 exampleConsumerGraph [] 6 []
    exampleWrappedRecordNode_clean
    (by simp [rootListScanCleanFuel])

theorem exampleBinaryDepTraversal_clean :
    depTraversalCleanFuel 3 exampleConsumerGraph [] (.binary 4 5) := by
  simp [depTraversalCleanFuel, nonPhiDeps]
  apply rootListScanCleanFuel_cons
  · exact exampleSharedCallNode_clean
  · apply rootListScanCleanFuel_cons
    · exact exampleCallIntrinsicNode_clean
    · simp [rootListScanCleanFuel]

theorem exampleCallDeps_clean :
    depTraversalCleanFuel 2 exampleConsumerGraph [] (.call [1, 2]) := by
  simp [depTraversalCleanFuel, nonPhiDeps]
  apply rootListScanCleanFuel_cons
  · simpa [exampleConsumerGraph] using
      nodeScanCleanFuel_meta_of_clean
        (fuel := 1) (graph := exampleConsumerGraph) (seen := []) (root := 1)
        (c := exampleCallConsumer) rfl exampleCallConsumer_clean
  · apply rootListScanCleanFuel_cons
    · simpa [exampleConsumerGraph] using
        nodeScanCleanFuel_meta_of_clean
          (fuel := 0) (graph := exampleConsumerGraph) (seen := [1]) (root := 2)
          (c := exampleIntrinsicConsumer) rfl exampleIntrinsicConsumer_clean
    · simp [rootListScanCleanFuel]

theorem exampleIntrinsicDeps_clean :
    depTraversalCleanFuel 2 exampleConsumerGraph [] (.intrinsic [2, 1]) := by
  simp [depTraversalCleanFuel, nonPhiDeps]
  apply rootListScanCleanFuel_cons
  · simpa [exampleConsumerGraph] using
      nodeScanCleanFuel_meta_of_clean
        (fuel := 1) (graph := exampleConsumerGraph) (seen := []) (root := 2)
        (c := exampleIntrinsicConsumer) rfl exampleIntrinsicConsumer_clean
  · apply rootListScanCleanFuel_cons
    · simpa [exampleConsumerGraph] using
        nodeScanCleanFuel_meta_of_clean
          (fuel := 0) (graph := exampleConsumerGraph) (seen := [2]) (root := 1)
          (c := exampleCallConsumer) rfl exampleCallConsumer_clean
    · simp [rootListScanCleanFuel]

theorem exampleRecordLitDeps_clean :
    depTraversalCleanFuel 2 exampleConsumerGraph [] (.recordLit [("a", 3), ("b", 1)]) := by
  simp [depTraversalCleanFuel, nonPhiDeps]
  apply rootListScanCleanFuel_cons
  · simpa [exampleConsumerGraph] using
      nodeScanCleanFuel_meta_of_clean
        (fuel := 1) (graph := exampleConsumerGraph) (seen := []) (root := 3)
        (c := exampleRecordConsumer) rfl exampleRecordConsumer_clean
  · apply rootListScanCleanFuel_cons
    · simpa [exampleConsumerGraph] using
        nodeScanCleanFuel_meta_of_clean
          (fuel := 0) (graph := exampleConsumerGraph) (seen := [3]) (root := 1)
          (c := exampleCallConsumer) rfl exampleCallConsumer_clean
    · simp [rootListScanCleanFuel]

theorem exampleIndex3dDeps_clean :
    depTraversalCleanFuel 4 exampleConsumerGraph [] (.index3d 4 5 6 1) := by
  simp [depTraversalCleanFuel, nonPhiDeps]
  apply rootListScanCleanFuel_cons
  · exact exampleSharedCallNode_clean
  · apply rootListScanCleanFuel_cons
    · exact exampleCallIntrinsicNode_clean
    · apply rootListScanCleanFuel_cons
      · exact exampleWrappedRecordNode_clean
      · apply rootListScanCleanFuel_cons
        · simpa [exampleConsumerGraph] using
            nodeScanCleanFuel_meta_of_clean
              (fuel := 0) (graph := exampleConsumerGraph) (seen := [6, 5, 4]) (root := 1)
              (c := exampleCallConsumer) rfl exampleCallConsumer_clean
        · simp [rootListScanCleanFuel]

end RRProofs
