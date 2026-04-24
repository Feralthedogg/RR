import RRProofs.PhasePlanCollectionSoundness

namespace RRProofs.ProgramOptPlanSoundness

structure ReducedProgramFunctionEntry where
  functionId : Nat
  irSize : Nat
  score : Nat
  hotWeight : Nat
  conservative : Bool
deriving Repr, DecidableEq

structure ReducedProgramOptPlan where
  programLimit : Nat
  fnLimit : Nat
  totalIr : Nat
  maxFnIr : Nat
  selectiveMode : Bool
  selectedFunctions : List Nat
deriving Repr, DecidableEq

def entryWeightedScore (entry : ReducedProgramFunctionEntry) : Nat :=
  entry.score * entry.hotWeight / 1024

def entryDensity (entry : ReducedProgramFunctionEntry) : Nat :=
  entryWeightedScore entry * 1024 / max entry.irSize 1

def eligibleFunctionIds (entries : List ReducedProgramFunctionEntry) : List Nat :=
  entries.filterMap fun entry =>
    if entry.conservative then none else some entry.functionId

def selectUnderBudget (entries : List ReducedProgramFunctionEntry) : List Nat :=
  eligibleFunctionIds entries

def selectWithinBudget
    (programLimit softFnLimit : Nat)
    (entries : List ReducedProgramFunctionEntry) : List Nat :=
  let candidates := entries.filter fun entry => !entry.conservative && entry.irSize <= softFnLimit
  let step (used : Nat × List Nat) (entry : ReducedProgramFunctionEntry) : Nat × List Nat :=
    let (usedBudget, selected) := used
    if usedBudget + entry.irSize <= programLimit then
      (usedBudget + entry.irSize, entry.functionId :: selected)
    else
      used
  (candidates.foldl step (0, [])).2.reverse

def insertByIrSize
    (entry : ReducedProgramFunctionEntry)
    (entries : List ReducedProgramFunctionEntry) : List ReducedProgramFunctionEntry :=
  match entries with
  | [] => [entry]
  | x :: rest =>
      if entry.irSize < x.irSize || (entry.irSize = x.irSize && entry.functionId < x.functionId) then
        entry :: x :: rest
      else
        x :: insertByIrSize entry rest

def sortByIrSize (entries : List ReducedProgramFunctionEntry) : List ReducedProgramFunctionEntry :=
  entries.foldr insertByIrSize []

def fallbackSmallestEligible? (entries : List ReducedProgramFunctionEntry) : Option Nat :=
  match sortByIrSize (entries.filter fun entry => !entry.conservative) with
  | [] => none
  | entry :: _ => some entry.functionId

def buildProgramOptPlan
    (programLimit fnLimit totalIr maxFnIr : Nat)
    (entries : List ReducedProgramFunctionEntry) : ReducedProgramOptPlan :=
  let needsBudget := totalIr > programLimit || maxFnIr > fnLimit
  if !needsBudget then
    { programLimit, fnLimit, totalIr, maxFnIr
    , selectiveMode := false
    , selectedFunctions := selectUnderBudget entries
    }
  else
    let softFnLimit := min fnLimit (max 64 fnLimit)
    let selected := selectWithinBudget programLimit softFnLimit entries
    let selected' :=
      if selected.isEmpty then
        match fallbackSmallestEligible? entries with
        | some functionId => [functionId]
        | none => []
      else
        selected
    { programLimit, fnLimit, totalIr, maxFnIr
    , selectiveMode := true
    , selectedFunctions := selected'
    }

def underBudgetSampleEntries : List ReducedProgramFunctionEntry :=
  [ { functionId := 1, irSize := 10, score := 20, hotWeight := 1024, conservative := false }
  , { functionId := 2, irSize := 12, score := 18, hotWeight := 1024, conservative := false }
  , { functionId := 3, irSize := 8, score := 7, hotWeight := 1024, conservative := true }
  ]

def overBudgetSampleEntries : List ReducedProgramFunctionEntry :=
  [ { functionId := 10, irSize := 40, score := 100, hotWeight := 1024, conservative := false }
  , { functionId := 11, irSize := 60, score := 90, hotWeight := 1024, conservative := false }
  , { functionId := 12, irSize := 200, score := 5, hotWeight := 1024, conservative := false }
  ]

def fallbackSampleEntries : List ReducedProgramFunctionEntry :=
  [ { functionId := 20, irSize := 200, score := 5, hotWeight := 1024, conservative := false }
  , { functionId := 21, irSize := 80, score := 4, hotWeight := 1024, conservative := false }
  ]

theorem under_budget_plan_selects_all_safe :
    (buildProgramOptPlan 128 128 32 12 underBudgetSampleEntries).selectedFunctions = [1, 2] := by
  decide

theorem over_budget_plan_is_selective :
    (buildProgramOptPlan 100 50 300 200 overBudgetSampleEntries).selectiveMode = true := by
  decide

theorem over_budget_plan_selects_within_budget_prefix :
    (buildProgramOptPlan 100 50 300 200 overBudgetSampleEntries).selectedFunctions = [10] := by
  decide

theorem fallback_plan_selects_smallest_when_budget_selection_empty :
    (buildProgramOptPlan 10 10 280 200 fallbackSampleEntries).selectedFunctions = [21] := by
  decide

end RRProofs.ProgramOptPlanSoundness
