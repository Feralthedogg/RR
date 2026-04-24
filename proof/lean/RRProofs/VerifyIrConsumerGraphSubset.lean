import RRProofs.VerifyIrConsumerMetaSubset

set_option linter.unusedVariables false

namespace RRProofs

abbrev ConsumerNodeId := Nat

inductive ConsumerGraphNode where
  | leaf (c : ConsumerMeta) : ConsumerGraphNode
  | unaryWrap (child : ConsumerNodeId) : ConsumerGraphNode
  | binaryWrap (lhs rhs : ConsumerNodeId) : ConsumerGraphNode

abbrev ConsumerGraph := ConsumerNodeId -> Option ConsumerGraphNode

def nodeScanCleanFuel : Nat -> ConsumerGraph -> List ConsumerNodeId -> ConsumerNodeId -> Prop
  | 0, _, _, _ => True
  | fuel + 1, graph, seen, root =>
      if root ∈ seen then
        True
      else
        match graph root with
        | none => True
        | some (.leaf c) => c.clean
        | some (.unaryWrap child) => nodeScanCleanFuel fuel graph (root :: seen) child
        | some (.binaryWrap lhs rhs) =>
            nodeScanCleanFuel fuel graph (root :: seen) lhs ∧
              nodeScanCleanFuel fuel graph (root :: seen) rhs

def rootListScanCleanFuel : Nat -> ConsumerGraph -> List ConsumerNodeId -> List ConsumerNodeId -> Prop
  | 0, _, _, _ => True
  | fuel + 1, graph, seen, [] => True
  | fuel + 1, graph, seen, root :: rest =>
      nodeScanCleanFuel (fuel + 1) graph seen root ∧
        rootListScanCleanFuel fuel graph (root :: seen) rest

theorem nodeScanCleanFuel_meta_of_clean
    (fuel : Nat) (graph : ConsumerGraph) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (c : ConsumerMeta)
    (hNode : graph root = some (.leaf c))
    (hClean : c.clean) :
    nodeScanCleanFuel (fuel + 1) graph seen root := by
  by_cases hSeen : root ∈ seen
  · simp [nodeScanCleanFuel, hSeen]
  · simp [nodeScanCleanFuel, hSeen, hNode, hClean]

theorem nodeScanCleanFuel_wrap1_of_child
    (fuel : Nat) (graph : ConsumerGraph) (seen : List ConsumerNodeId)
    (root child : ConsumerNodeId)
    (hNode : graph root = some (.unaryWrap child))
    (hChild : nodeScanCleanFuel fuel graph (root :: seen) child) :
    nodeScanCleanFuel (fuel + 1) graph seen root := by
  by_cases hSeen : root ∈ seen
  · simp [nodeScanCleanFuel, hSeen]
  · simp [nodeScanCleanFuel, hSeen, hNode, hChild]

theorem nodeScanCleanFuel_wrap2_of_children
    (fuel : Nat) (graph : ConsumerGraph) (seen : List ConsumerNodeId)
    (root lhs rhs : ConsumerNodeId)
    (hNode : graph root = some (.binaryWrap lhs rhs))
    (hLhs : nodeScanCleanFuel fuel graph (root :: seen) lhs)
    (hRhs : nodeScanCleanFuel fuel graph (root :: seen) rhs) :
    nodeScanCleanFuel (fuel + 1) graph seen root := by
  by_cases hSeen : root ∈ seen
  · simp [nodeScanCleanFuel, hSeen]
  · simp [nodeScanCleanFuel, hSeen, hNode, hLhs, hRhs]

theorem rootListScanCleanFuel_cons
    (fuel : Nat) (graph : ConsumerGraph) (seen : List ConsumerNodeId)
    (root : ConsumerNodeId) (rest : List ConsumerNodeId)
    (hRoot : nodeScanCleanFuel (fuel + 1) graph seen root)
    (hRest : rootListScanCleanFuel fuel graph (root :: seen) rest) :
    rootListScanCleanFuel (fuel + 1) graph seen (root :: rest) := by
  simp [rootListScanCleanFuel, hRoot, hRest]

def exampleConsumerGraph : ConsumerGraph
  | 1 => some (.leaf exampleCallConsumer)
  | 2 => some (.leaf exampleIntrinsicConsumer)
  | 3 => some (.leaf exampleRecordConsumer)
  | 4 => some (.binaryWrap 1 1)
  | 5 => some (.binaryWrap 1 2)
  | 6 => some (.unaryWrap 3)
  | _ => none

theorem exampleSharedCallNode_clean :
    nodeScanCleanFuel 2 exampleConsumerGraph [] 4 := by
  simpa [exampleConsumerGraph] using
    nodeScanCleanFuel_wrap2_of_children
      (fuel := 1) (graph := exampleConsumerGraph) (seen := []) (root := 4)
      (lhs := 1) (rhs := 1) rfl
      (by
        simpa [exampleConsumerGraph] using
          nodeScanCleanFuel_meta_of_clean
            (fuel := 0) (graph := exampleConsumerGraph) (seen := [4]) (root := 1)
            (c := exampleCallConsumer) rfl exampleCallConsumer_clean)
      (by
        simpa [exampleConsumerGraph] using
          nodeScanCleanFuel_meta_of_clean
            (fuel := 0) (graph := exampleConsumerGraph) (seen := [4]) (root := 1)
            (c := exampleCallConsumer) rfl exampleCallConsumer_clean)

theorem exampleCallIntrinsicNode_clean :
    nodeScanCleanFuel 2 exampleConsumerGraph [] 5 := by
  simpa [exampleConsumerGraph] using
    nodeScanCleanFuel_wrap2_of_children
      (fuel := 1) (graph := exampleConsumerGraph) (seen := []) (root := 5)
      (lhs := 1) (rhs := 2) rfl
      (by
        simpa [exampleConsumerGraph] using
          nodeScanCleanFuel_meta_of_clean
            (fuel := 0) (graph := exampleConsumerGraph) (seen := [5]) (root := 1)
            (c := exampleCallConsumer) rfl exampleCallConsumer_clean)
      (by
        simpa [exampleConsumerGraph] using
          nodeScanCleanFuel_meta_of_clean
            (fuel := 0) (graph := exampleConsumerGraph) (seen := [5]) (root := 2)
            (c := exampleIntrinsicConsumer) rfl exampleIntrinsicConsumer_clean)

theorem exampleWrappedRecordNode_clean :
    nodeScanCleanFuel 2 exampleConsumerGraph [] 6 := by
  simpa [exampleConsumerGraph] using
    nodeScanCleanFuel_wrap1_of_child
      (fuel := 1) (graph := exampleConsumerGraph) (seen := []) (root := 6)
      (child := 3) rfl
      (by
        simpa [exampleConsumerGraph] using
          nodeScanCleanFuel_meta_of_clean
            (fuel := 0) (graph := exampleConsumerGraph) (seen := [6]) (root := 3)
            (c := exampleRecordConsumer) rfl exampleRecordConsumer_clean)

theorem exampleConsumerGraph_root_list_clean :
    rootListScanCleanFuel 4 exampleConsumerGraph [] [4, 5, 6] := by
  apply rootListScanCleanFuel_cons
  · exact exampleSharedCallNode_clean
  · apply rootListScanCleanFuel_cons
    · exact exampleCallIntrinsicNode_clean
    · apply rootListScanCleanFuel_cons
      · exact exampleWrappedRecordNode_clean
      · simp [rootListScanCleanFuel]

end RRProofs
