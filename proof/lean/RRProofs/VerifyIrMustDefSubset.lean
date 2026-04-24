import RRProofs.VerifyIrFlowLite

namespace RRProofs

abbrev MustDefBlockId := Nat
abbrev DefSet := List Var

def intersectDefs : DefSet -> DefSet -> DefSet
  | [], _ => []
  | x :: xs, ys =>
      if x ∈ ys then
        x :: intersectDefs xs ys
      else
        intersectDefs xs ys

def foldPredIntersections (base : DefSet)
    (preds : List MustDefBlockId) (outDefs : MustDefBlockId -> DefSet) : DefSet :=
  preds.foldl (fun acc pred => intersectDefs acc (outDefs pred)) base

def intersectPredOutDefs
    (preds : List MustDefBlockId) (outDefs : MustDefBlockId -> DefSet) : DefSet :=
  match preds with
  | [] => []
  | p :: rest => foldPredIntersections (outDefs p) rest outDefs

def outDefsOfBlock (inDefs assigned : DefSet) : DefSet :=
  assigned ++ inDefs

def inDefsFromPreds (entry : MustDefBlockId) (entryDefs : DefSet)
    (preds : List MustDefBlockId) (outDefs : MustDefBlockId -> DefSet)
    (bid : MustDefBlockId) : DefSet :=
  if bid = entry then entryDefs else intersectPredOutDefs preds outDefs

theorem mem_intersectDefs_of_mem {v : Var} :
    ∀ {xs ys : DefSet}, v ∈ xs -> v ∈ ys -> v ∈ intersectDefs xs ys
  | [], _, h, _ => by
      cases h
  | x :: xs, ys, hxs, hys => by
      by_cases hxy : x ∈ ys
      · by_cases hxv : v = x
        · subst hxv
          simp [intersectDefs, hxy]
        · simp [intersectDefs, hxy, hxv] at hxs ⊢
          exact mem_intersectDefs_of_mem hxs hys
      · simp [intersectDefs, hxy] at hxs ⊢
        cases hxs with
        | inl hx =>
            exact False.elim (hxy (hx ▸ hys))
        | inr hrest =>
            exact mem_intersectDefs_of_mem hrest hys

theorem mem_foldPredIntersections_of_forall_pred {v : Var}
    {base : DefSet} {preds : List MustDefBlockId} {outDefs : MustDefBlockId -> DefSet}
    (hBase : v ∈ base)
    (hAll : ∀ pred, pred ∈ preds -> v ∈ outDefs pred) :
    v ∈ foldPredIntersections base preds outDefs := by
  induction preds generalizing base with
  | nil =>
      simpa [foldPredIntersections] using hBase
  | cons pred preds ih =>
      simp [foldPredIntersections]
      have hStep : v ∈ intersectDefs base (outDefs pred) :=
        mem_intersectDefs_of_mem hBase (hAll pred (by simp))
      apply ih hStep
      intro pred' hPred'
      exact hAll pred' (by simp [hPred'])

theorem mem_intersectPredOutDefs_of_forall_pred {v : Var}
    {preds : List MustDefBlockId} {outDefs : MustDefBlockId -> DefSet}
    (hPreds : preds ≠ [])
    (hAll : ∀ pred, pred ∈ preds -> v ∈ outDefs pred) :
    v ∈ intersectPredOutDefs preds outDefs := by
  cases preds with
  | nil =>
      contradiction
  | cons pred preds =>
      simp [intersectPredOutDefs]
      exact mem_foldPredIntersections_of_forall_pred
        (hAll pred (by simp))
        (by
          intro pred' hPred'
          exact hAll pred' (by simp [hPred']))

theorem mem_outDefsOfBlock_of_mem_assigned {v : Var}
    {inDefs assigned : DefSet}
    (h : v ∈ assigned) :
    v ∈ outDefsOfBlock inDefs assigned := by
  simp [outDefsOfBlock, h]

theorem mem_inDefsFromPreds_of_forall_pred {v : Var}
    {entry bid : MustDefBlockId} {entryDefs : DefSet}
    {preds : List MustDefBlockId} {outDefs : MustDefBlockId -> DefSet}
    (hBid : bid ≠ entry)
    (hPreds : preds ≠ [])
    (hAll : ∀ pred, pred ∈ preds -> v ∈ outDefs pred) :
    v ∈ inDefsFromPreds entry entryDefs preds outDefs bid := by
  simp [inDefsFromPreds, hBid]
  exact mem_intersectPredOutDefs_of_forall_pred hPreds hAll

theorem firstMissingVar_singleton_none_of_mem {defs : DefSet} {v : Var}
    (h : v ∈ defs) :
    firstMissingVar defs [v] = none := by
  simp [firstMissingVar, h]

theorem verifyFlow_singleton_none_of_must_def {defs : DefSet} {v : Var}
    (h : v ∈ defs) :
    ({ defined := defs, required := [v] } : FlowBlockCase).verifyFlow = none := by
  simp [FlowBlockCase.verifyFlow, firstMissingVar_singleton_none_of_mem h]

def examplePreds : List MustDefBlockId := [1, 2]

def exampleOutDefs : MustDefBlockId -> DefSet
  | 1 => ["x", "y"]
  | 2 => ["x", "z"]
  | _ => []

def exampleAssigned : DefSet := ["tmp"]

theorem example_outDefs_contains_tmp :
    "tmp" ∈ outDefsOfBlock ["param"] exampleAssigned := by
  exact mem_outDefsOfBlock_of_mem_assigned (by simp [exampleAssigned])

theorem example_join_contains_x :
    "x" ∈ inDefsFromPreds 0 [] examplePreds exampleOutDefs 3 := by
  apply mem_inDefsFromPreds_of_forall_pred
  · decide
  · simp [examplePreds]
  · intro pred hPred
    simp [examplePreds] at hPred ⊢
    rcases hPred with rfl | rfl
    · simp [exampleOutDefs]
    · simp [exampleOutDefs]

theorem example_join_required_x_is_flow_clean :
    ({ defined := inDefsFromPreds 0 [] examplePreds exampleOutDefs 3
     , required := ["x"] } : FlowBlockCase).verifyFlow = none := by
  exact verifyFlow_singleton_none_of_must_def example_join_contains_x

end RRProofs
