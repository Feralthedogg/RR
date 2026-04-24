import RRProofs.VerifyIrMustDefSubset

namespace RRProofs

abbrev ReachableMap := MustDefBlockId -> Bool
abbrev PredMap := MustDefBlockId -> List MustDefBlockId
abbrev AssignMap := MustDefBlockId -> DefSet
abbrev DefMap := MustDefBlockId -> DefSet

def reachablePreds (reachable : ReachableMap) (preds : PredMap)
    (bid : MustDefBlockId) : List MustDefBlockId :=
  (preds bid).filter reachable

def stepInDefs (entry : MustDefBlockId) (entryDefs : DefSet)
    (reachable : ReachableMap) (preds : PredMap) (outDefs : DefMap)
    (bid : MustDefBlockId) : DefSet :=
  if !reachable bid then []
  else if bid = entry then entryDefs
  else intersectPredOutDefs (reachablePreds reachable preds bid) outDefs

def stepOutDefs (entry : MustDefBlockId) (entryDefs : DefSet)
    (reachable : ReachableMap) (preds : PredMap) (assigned : AssignMap)
    (outDefs : DefMap) (bid : MustDefBlockId) : DefSet :=
  outDefsOfBlock (stepInDefs entry entryDefs reachable preds outDefs bid) (assigned bid)

def stepOutMap (entry : MustDefBlockId) (entryDefs : DefSet)
    (reachable : ReachableMap) (preds : PredMap) (assigned : AssignMap)
    (outDefs : DefMap) : DefMap :=
  fun bid => stepOutDefs entry entryDefs reachable preds assigned outDefs bid

def iterateOutMap (entry : MustDefBlockId) (entryDefs : DefSet)
    (reachable : ReachableMap) (preds : PredMap) (assigned : AssignMap) :
    Nat -> DefMap -> DefMap
  | 0, seed => seed
  | n + 1, seed => iterateOutMap entry entryDefs reachable preds assigned n
      (stepOutMap entry entryDefs reachable preds assigned seed)

theorem mem_stepInDefs_of_forall_reachable_pred {v : Var}
    {entry bid : MustDefBlockId} {entryDefs : DefSet}
    {reachable : ReachableMap} {preds : PredMap} {outDefs : DefMap}
    (hReach : reachable bid = true)
    (hBid : bid ≠ entry)
    (hPreds : reachablePreds reachable preds bid ≠ [])
    (hAll : ∀ pred, pred ∈ reachablePreds reachable preds bid -> v ∈ outDefs pred) :
    v ∈ stepInDefs entry entryDefs reachable preds outDefs bid := by
  simp [stepInDefs, hReach, hBid]
  exact mem_intersectPredOutDefs_of_forall_pred hPreds hAll

theorem mem_stepOutDefs_of_forall_reachable_pred {v : Var}
    {entry bid : MustDefBlockId} {entryDefs : DefSet}
    {reachable : ReachableMap} {preds : PredMap} {assigned : AssignMap} {outDefs : DefMap}
    (hReach : reachable bid = true)
    (hBid : bid ≠ entry)
    (hPreds : reachablePreds reachable preds bid ≠ [])
    (hAll : ∀ pred, pred ∈ reachablePreds reachable preds bid -> v ∈ outDefs pred) :
    v ∈ stepOutDefs entry entryDefs reachable preds assigned outDefs bid := by
  unfold stepOutDefs
  have hIn : v ∈ stepInDefs entry entryDefs reachable preds outDefs bid :=
    mem_stepInDefs_of_forall_reachable_pred hReach hBid hPreds hAll
  simp [outDefsOfBlock, hIn]

theorem mem_stepOutDefs_entry_of_mem_entryDefs {v : Var}
    {entry : MustDefBlockId} {entryDefs : DefSet}
    {reachable : ReachableMap} {preds : PredMap} {assigned : AssignMap} {outDefs : DefMap}
    (hReach : reachable entry = true)
    (hIn : v ∈ entryDefs) :
    v ∈ stepOutDefs entry entryDefs reachable preds assigned outDefs entry := by
  unfold stepOutDefs
  have hStep : v ∈ stepInDefs entry entryDefs reachable preds outDefs entry := by
    simp [stepInDefs, hReach, hIn]
  simp [outDefsOfBlock, hStep]

theorem iterateOutMap_one_apply (entry : MustDefBlockId) (entryDefs : DefSet)
    (reachable : ReachableMap) (preds : PredMap) (assigned : AssignMap)
    (seed : DefMap) (bid : MustDefBlockId) :
    iterateOutMap entry entryDefs reachable preds assigned 1 seed bid =
      stepOutDefs entry entryDefs reachable preds assigned seed bid := by
  rfl

def exampleReachable : ReachableMap
  | 0 => true
  | 1 => true
  | 2 => true
  | 3 => true
  | _ => false

def examplePredMap : PredMap
  | 3 => [1, 2]
  | _ => []

def exampleAssignMap : AssignMap
  | 3 => ["tmp"]
  | _ => []

def exampleSeedOutDefs : DefMap := exampleOutDefs

theorem example_join_reachablePreds_nonempty :
    reachablePreds exampleReachable examplePredMap 3 ≠ [] := by
  simp [reachablePreds, exampleReachable, examplePredMap]

theorem example_join_stepOut_contains_x :
    "x" ∈ stepOutDefs 0 [] exampleReachable examplePredMap exampleAssignMap exampleSeedOutDefs 3 := by
  apply mem_stepOutDefs_of_forall_reachable_pred
  · simp [exampleReachable]
  · decide
  · exact example_join_reachablePreds_nonempty
  · intro pred hPred
    simp [reachablePreds, exampleReachable, examplePredMap] at hPred
    rcases hPred with rfl | rfl
    · simp [exampleSeedOutDefs, exampleOutDefs]
    · simp [exampleSeedOutDefs, exampleOutDefs]

theorem example_join_after_one_iteration_contains_x :
    "x" ∈ iterateOutMap 0 [] exampleReachable examplePredMap exampleAssignMap 1 exampleSeedOutDefs 3 := by
  simpa [iterateOutMap_one_apply] using example_join_stepOut_contains_x

theorem example_join_required_x_is_flow_clean_after_one_iteration :
    ({ defined := iterateOutMap 0 [] exampleReachable examplePredMap exampleAssignMap 1 exampleSeedOutDefs 3
     , required := ["x"] } : FlowBlockCase).verifyFlow = none := by
  exact verifyFlow_singleton_none_of_must_def example_join_after_one_iteration_contains_x

end RRProofs
