import RRProofs.CfgHoist

set_option linter.unusedVariables false

namespace RRProofs

inductive BlockId where
  | preheader
  | header
  | body
  | exit
deriving DecidableEq, Repr

inductive Terminator where
  | goto : BlockId -> Terminator
  | branchOnEntered : BlockId -> BlockId -> Terminator
  | retCand : Terminator
  | retTmp : Var -> Terminator
deriving DecidableEq, Repr

structure Block where
  id : BlockId
  instrs : List MirInstr
  term : Terminator
deriving Repr

structure ReducedFnIR where
  tmp : Var
  cand : MirValue
  bodyInstrs : List MirInstr
deriving Repr

def ReducedFnIR.toCfg (f : ReducedFnIR) : LoopCfg :=
  { tmp := f.tmp, cand := f.cand, body := f.bodyInstrs }

def ReducedFnIR.preheader (f : ReducedFnIR) : Block :=
  { id := .preheader, instrs := [], term := .goto .header }

def ReducedFnIR.header (f : ReducedFnIR) : Block :=
  { id := .header, instrs := [], term := .branchOnEntered .body .exit }

def ReducedFnIR.bodyOriginal (f : ReducedFnIR) : Block :=
  { id := .body, instrs := f.bodyInstrs, term := .retCand }

def ReducedFnIR.bodyHoisted (f : ReducedFnIR) (entry locals : State) : Block :=
  { id := .body
  , instrs := MirInstr.assign f.tmp (.const (preVal f.toCfg entry locals)) :: f.bodyInstrs
  , term := .retTmp f.tmp
  }

def ReducedFnIR.exit (f : ReducedFnIR) : Block :=
  { id := .exit, instrs := [], term := .retCand }

def ReducedFnIR.blocksOriginal (f : ReducedFnIR) : List Block :=
  [f.preheader, f.header, f.bodyOriginal, f.exit]

def ReducedFnIR.blocksHoisted (f : ReducedFnIR) (entry locals : State) : List Block :=
  [f.preheader, f.header, f.bodyHoisted entry locals, f.exit]

def ReducedFnIR.runOriginal (f : ReducedFnIR) (entered : Bool) (entry locals : State) : Int :=
  RRProofs.runOriginal f.toCfg entered entry locals

def ReducedFnIR.runHoisted (f : ReducedFnIR) (entered : Bool) (entry locals : State) : Int :=
  RRProofs.runHoisted f.toCfg entered entry locals

theorem reducedFnIR_zero_trip_sound
    (f : ReducedFnIR)
    (entry locals : State) :
    f.runOriginal false entry locals = f.runHoisted false entry locals := by
  simpa [ReducedFnIR.runOriginal, ReducedFnIR.runHoisted] using
    runOriginalFalse_eq_runHoistedFalse f.toCfg entry locals

theorem reducedFnIR_one_trip_sound
    (f : ReducedFnIR)
    (entry locals : State)
    (h : safeToHoistCfg f.toCfg) :
    f.runOriginal true entry locals = f.runHoisted true entry locals := by
  simpa [ReducedFnIR.runOriginal, ReducedFnIR.runHoisted] using
    runOriginalTrue_eq_runHoistedTrue f.toCfg entry locals h

def reducedPhiTimeFn : ReducedFnIR :=
  { tmp := phiTimeCfg.tmp, cand := phiTimeCfg.cand, bodyInstrs := phiTimeCfg.body }

theorem reducedPhiTimeFn_not_safe : ¬ safeToHoistCfg reducedPhiTimeFn.toCfg := by
  simpa [reducedPhiTimeFn, ReducedFnIR.toCfg] using phiTimeCfg_not_safe

theorem reducedPhiTimeFn_unsound
    (entry locals : State)
    (h : locals "time" + 1 ≠ entry "time0") :
    reducedPhiTimeFn.runOriginal true entry locals ≠
      reducedPhiTimeFn.runHoisted true entry locals := by
  simpa [reducedPhiTimeFn, ReducedFnIR.runOriginal, ReducedFnIR.runHoisted, ReducedFnIR.toCfg]
    using phiTimeCfg_true_trip_unsound entry locals h

end RRProofs
