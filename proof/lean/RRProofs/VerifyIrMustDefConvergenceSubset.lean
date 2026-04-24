import RRProofs.VerifyIrMustDefFixedPointSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

def outMapStable (entry : MustDefBlockId) (entryDefs : DefSet)
    (reachable : ReachableMap) (preds : PredMap) (assigned : AssignMap)
    (seed : DefMap) : Prop :=
  stepOutMap entry entryDefs reachable preds assigned seed = seed

theorem iterateOutMap_of_stable (entry : MustDefBlockId) (entryDefs : DefSet)
    (reachable : ReachableMap) (preds : PredMap) (assigned : AssignMap)
    (seed : DefMap)
    (hStable : outMapStable entry entryDefs reachable preds assigned seed) :
    ∀ fuel, iterateOutMap entry entryDefs reachable preds assigned fuel seed = seed
  | 0 => rfl
  | n + 1 => by
      simp [iterateOutMap]
      rw [hStable]
      exact iterateOutMap_of_stable entry entryDefs reachable preds assigned seed hStable n

def exampleStableReachable : ReachableMap := exampleReachable

def exampleStablePredMap : PredMap
  | 1 => [0]
  | 2 => [0]
  | 3 => [1, 2]
  | _ => []

def exampleStableAssignMap : AssignMap
  | 1 => ["x"]
  | 2 => ["x"]
  | 3 => ["tmp"]
  | _ => []

def exampleStableSeed : DefMap
  | 0 => []
  | 1 => ["x"]
  | 2 => ["x"]
  | 3 => ["tmp", "x"]
  | _ => []

theorem exampleStableSeed_is_stable :
    outMapStable 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
      exampleStableSeed := by
  funext bid
  cases bid with
  | zero =>
      native_decide
  | succ bid =>
      cases bid with
      | zero =>
          native_decide
      | succ bid =>
          cases bid with
          | zero =>
              native_decide
          | succ bid =>
              cases bid with
              | zero =>
                  native_decide
              | succ bid =>
                  simp [outMapStable, stepOutMap, stepOutDefs, stepInDefs, exampleStableReachable,
                    exampleReachable, exampleStablePredMap, exampleStableAssignMap,
                    exampleStableSeed, reachablePreds, intersectPredOutDefs, outDefsOfBlock]

theorem exampleStableSeed_iterate_five_block3 :
    iterateOutMap 0 [] exampleStableReachable exampleStablePredMap exampleStableAssignMap
      5 exampleStableSeed 3 = ["tmp", "x"] := by
  have hEq :=
    congrArg (fun m => m 3)
      (iterateOutMap_of_stable 0 [] exampleStableReachable exampleStablePredMap
        exampleStableAssignMap exampleStableSeed exampleStableSeed_is_stable 5)
  simpa [exampleStableSeed] using hEq

theorem exampleStableSeed_required_x_is_flow_clean_after_five :
    ({ defined := iterateOutMap 0 [] exampleStableReachable exampleStablePredMap
         exampleStableAssignMap 5 exampleStableSeed 3
     , required := ["x"] } : FlowBlockCase).verifyFlow = none := by
  apply verifyFlow_singleton_none_of_must_def
  rw [exampleStableSeed_iterate_five_block3]
  simp

end RRProofs
