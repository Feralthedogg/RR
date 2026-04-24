import RRProofs.ReducedFnIR

namespace RRProofs

inductive Pc where
  | preheader
  | header
  | body
  | exit
  | halted
deriving DecidableEq, Repr

structure Machine where
  pc : Pc
  locals : State
  result? : Option Int

def Machine.initial (locals : State) : Machine :=
  { pc := .preheader, locals := locals, result? := none }

def stepOriginal (f : ReducedFnIR) (entered : Bool) (entry : State) (m : Machine) : Machine :=
  match m.pc with
  | .preheader => { m with pc := .header }
  | .header =>
      if entered then { m with pc := .body } else { m with pc := .exit }
  | .body =>
      let post := postOriginal f.toCfg entry m.locals
      { pc := .halted
      , locals := post
      , result? := some (f.toCfg.cand.eval 1 entry post post)
      }
  | .exit =>
      { pc := .halted
      , locals := m.locals
      , result? := some (preVal f.toCfg entry m.locals)
      }
  | .halted => m

def stepHoisted (f : ReducedFnIR) (entered : Bool) (entry : State) (m : Machine) : Machine :=
  match m.pc with
  | .preheader => { m with pc := .header }
  | .header =>
      if entered then { m with pc := .body } else { m with pc := .exit }
  | .body =>
      let post := postHoisted f.toCfg entry m.locals
      { pc := .halted
      , locals := post
      , result? := some (post f.toCfg.tmp)
      }
  | .exit =>
      { pc := .halted
      , locals := m.locals
      , result? := some (preVal f.toCfg entry m.locals)
      }
  | .halted => m

def runSteps (step : Machine -> Machine) : Nat -> Machine -> Machine
  | 0, m => m
  | n + 1, m => runSteps step n (step m)

def runOriginalMachine (f : ReducedFnIR) (entered : Bool) (entry locals : State) : Machine :=
  runSteps (stepOriginal f entered entry) 3 (Machine.initial locals)

def runHoistedMachine (f : ReducedFnIR) (entered : Bool) (entry locals : State) : Machine :=
  runSteps (stepHoisted f entered entry) 3 (Machine.initial locals)

theorem zeroTripMachineOriginal
    (f : ReducedFnIR) (entry locals : State) :
    (runOriginalMachine f false entry locals).result? = some (f.runOriginal false entry locals) := by
  simp [runOriginalMachine, runSteps, stepOriginal, Machine.initial, ReducedFnIR.runOriginal,
    runOriginal, preVal]

theorem zeroTripMachineHoisted
    (f : ReducedFnIR) (entry locals : State) :
    (runHoistedMachine f false entry locals).result? = some (f.runHoisted false entry locals) := by
  simp [runHoistedMachine, runSteps, stepHoisted, Machine.initial, ReducedFnIR.runHoisted,
    runHoisted, preVal]

theorem oneTripMachineOriginal
    (f : ReducedFnIR) (entry locals : State) :
    (runOriginalMachine f true entry locals).result? = some (f.runOriginal true entry locals) := by
  simp [runOriginalMachine, runSteps, stepOriginal, Machine.initial, ReducedFnIR.runOriginal,
    runOriginal, postOriginal]

theorem oneTripMachineHoisted
    (f : ReducedFnIR) (entry locals : State) :
    (runHoistedMachine f true entry locals).result? = some (f.runHoisted true entry locals) := by
  simp [runHoistedMachine, runSteps, stepHoisted, Machine.initial, ReducedFnIR.runHoisted,
    runHoisted, postHoisted]

theorem smallStepZeroTripSound
    (f : ReducedFnIR) (entry locals : State) :
    (runOriginalMachine f false entry locals).result? =
      (runHoistedMachine f false entry locals).result? := by
  rw [zeroTripMachineOriginal, zeroTripMachineHoisted, reducedFnIR_zero_trip_sound]

theorem smallStepOneTripSound
    (f : ReducedFnIR) (entry locals : State) (h : safeToHoistCfg f.toCfg) :
    (runOriginalMachine f true entry locals).result? =
      (runHoistedMachine f true entry locals).result? := by
  rw [oneTripMachineOriginal, oneTripMachineHoisted, reducedFnIR_one_trip_sound _ _ _ h]

theorem smallStepPhiTimeUnsound
    (entry locals : State)
    (h : locals "time" + 1 ≠ entry "time0") :
    (runOriginalMachine reducedPhiTimeFn true entry locals).result? ≠
      (runHoistedMachine reducedPhiTimeFn true entry locals).result? := by
  rw [oneTripMachineOriginal, oneTripMachineHoisted]
  intro hEq
  apply reducedPhiTimeFn_unsound entry locals h
  exact Option.some.inj hEq

end RRProofs
