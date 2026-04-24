import RRProofs.MirSubsetHoist

set_option linter.unnecessarySimpa false

namespace RRProofs

structure LoopCfg where
  tmp : Var
  cand : MirValue
  body : List MirInstr

def safeToHoistCfg (c : LoopCfg) : Prop :=
  c.cand.carriedDeps = [] ∧
  (∀ instr, instr ∈ c.body -> instr.write ∉ c.cand.localDeps ∧ instr.write ≠ c.tmp) ∧
  c.tmp ∉ c.cand.localDeps

def preVal (c : LoopCfg) (entry locals : State) : Int :=
  c.cand.eval 0 entry locals locals

def postOriginal (c : LoopCfg) (entry locals : State) : State :=
  execInstrs 1 entry locals locals c.body

def postHoisted (c : LoopCfg) (entry locals : State) : State :=
  execInstrs 1 entry locals (State.update locals c.tmp (preVal c entry locals)) c.body

def runOriginal (c : LoopCfg) (entered : Bool) (entry locals : State) : Int :=
  if entered then
    let post := postOriginal c entry locals
    c.cand.eval 1 entry post post
  else
    preVal c entry locals

def runHoisted (c : LoopCfg) (entered : Bool) (entry locals : State) : Int :=
  if entered then
    (postHoisted c entry locals) c.tmp
  else
    preVal c entry locals

theorem eval_iter_irrelevant_no_carried
    (e : MirValue)
    (i j : Nat)
    (entry carried locals : State)
    (h : e.carriedDeps = []) :
    e.eval i entry carried locals = e.eval j entry carried locals := by
  induction e generalizing carried locals i j with
  | const _ =>
      rfl
  | localVal _ =>
      rfl
  | phi seed loopVar =>
      simp [MirValue.carriedDeps] at h
  | add lhs rhs ihL ihR =>
      have hL : lhs.carriedDeps = [] := by
        cases hLhs : lhs.carriedDeps with
        | nil => simpa using hLhs
        | cons hd tl => simp [MirValue.carriedDeps, hLhs] at h
      have hR : rhs.carriedDeps = [] := by
        cases hRhs : rhs.carriedDeps with
        | nil => simpa using hRhs
        | cons hd tl => simp [MirValue.carriedDeps, hRhs] at h
      simp [MirValue.eval]
      rw [ihL i j carried locals hL, ihR i j carried locals hR]

theorem mirvalue_eval_irrelevant_carried
    (e : MirValue)
    (iter : Nat)
    (entry carried₁ carried₂ locals : State)
    (h : e.carriedDeps = []) :
    e.eval iter entry carried₁ locals = e.eval iter entry carried₂ locals := by
  induction e generalizing locals with
  | const _ =>
      rfl
  | localVal _ =>
      rfl
  | phi seed carried =>
      simp [MirValue.carriedDeps] at h
  | add lhs rhs ihL ihR =>
      have hL : lhs.carriedDeps = [] := by
        cases hLhs : lhs.carriedDeps with
        | nil => simpa using hLhs
        | cons hd tl => simp [MirValue.carriedDeps, hLhs] at h
      have hR : rhs.carriedDeps = [] := by
        cases hRhs : rhs.carriedDeps with
        | nil => simpa using hRhs
        | cons hd tl => simp [MirValue.carriedDeps, hRhs] at h
      simp [MirValue.eval]
      rw [ihL locals hL, ihR locals hR]

theorem execInstrs_preserve_unwritten_var
    (iter : Nat)
    (entry carried locals : State)
    (body : List MirInstr)
    (x : Var)
    (h : ∀ instr, instr ∈ body -> instr.write ≠ x) :
    (execInstrs iter entry carried locals body) x = locals x := by
  induction body generalizing locals with
  | nil =>
      rfl
  | cons instr rest ih =>
      cases instr with
      | assign dst rhs =>
          simp [execInstrs, execInstr]
          have hHead : dst ≠ x := h (.assign dst rhs) (by simp)
          have hRest : ∀ instr, instr ∈ rest -> instr.write ≠ x := by
            intro instr hMem
            exact h instr (by simp [hMem])
          have hRec :=
            ih hRest (locals := State.update locals dst (rhs.eval iter entry carried locals))
          have hHead' : x ≠ dst := by
            intro hEq
            exact hHead hEq.symm
          simp [State.update, hHead'] at hRec
          exact hRec

theorem runOriginalFalse_eq_runHoistedFalse
    (c : LoopCfg)
    (entry locals : State) :
    runOriginal c false entry locals = runHoisted c false entry locals := by
  simp [runOriginal, runHoisted, preVal]

theorem runOriginalTrue_eq_runHoistedTrue
    (c : LoopCfg)
    (entry locals : State)
    (h : safeToHoistCfg c) :
    runOriginal c true entry locals = runHoisted c true entry locals := by
  rcases h with ⟨hCarried, hWrites, hTmpFresh⟩
  have hWritesLocal : bodyWritesDisjoint c.body c.cand := by
    intro instr hMem
    exact (hWrites instr hMem).1
  have hWritesTmp : ∀ instr, instr ∈ c.body -> instr.write ≠ c.tmp := by
    intro instr hMem
    exact (hWrites instr hMem).2
  simp [runOriginal, runHoisted, postOriginal, postHoisted]
  let post := execInstrs 1 entry locals locals c.body
  have hIter :
      c.cand.eval 1 entry post post = c.cand.eval 0 entry post post := by
    apply eval_iter_irrelevant_no_carried
    exact hCarried
  have hCarry :
      c.cand.eval 1 entry post post = c.cand.eval 1 entry locals post := by
    apply mirvalue_eval_irrelevant_carried
    exact hCarried
  have hLocals :
      c.cand.eval 1 entry locals post = c.cand.eval 1 entry locals locals := by
    simpa [post] using
      mirvalue_eval_exec_irrelevant_body c.cand 1 entry locals locals c.body hWritesLocal
  have hIter :
      c.cand.eval 1 entry locals locals = c.cand.eval 0 entry locals locals := by
    apply eval_iter_irrelevant_no_carried
    exact hCarried
  have hTmp :
      (execInstrs 1 entry locals (State.update locals c.tmp (preVal c entry locals)) c.body) c.tmp =
        preVal c entry locals := by
    have hTmp' := execInstrs_preserve_unwritten_var 1 entry locals
      (State.update locals c.tmp (preVal c entry locals)) c.body c.tmp hWritesTmp
    simpa [preVal, State.update] using hTmp'
  calc
    c.cand.eval 1 entry post post = c.cand.eval 1 entry locals post := hCarry
    _ = c.cand.eval 1 entry locals locals := hLocals
    _ = c.cand.eval 0 entry locals locals := hIter
    _ = preVal c entry locals := rfl
    _ = (execInstrs 1 entry locals (State.update locals c.tmp (preVal c entry locals)) c.body) c.tmp := by
          symm
          exact hTmp

def phiTimeCfg : LoopCfg :=
  { tmp := "licm_time"
  , cand := .phi "time0" "time"
  , body := [MirInstr.assign "time" (.add (.localVal "time") (.const 1))] }

theorem phiTimeCfg_not_safe : ¬ safeToHoistCfg phiTimeCfg := by
  intro h
  have hHoist : hoistSafeOver phiTimeCfg.body (.phi "time0" "time") := by
    refine ⟨h.1, ?_⟩
    intro instr hMem
    exact (h.2.1 instr hMem).1
  exact phi_not_safe_to_hoist_over_any_body "time0" "time" phiTimeCfg.body hHoist

theorem phiTimeCfg_true_trip_unsound
    (entry locals : State)
    (h : locals "time" + 1 ≠ entry "time0") :
    runOriginal phiTimeCfg true entry locals ≠ runHoisted phiTimeCfg true entry locals := by
  intro hEq
  simp [runOriginal, runHoisted, phiTimeCfg, postOriginal, postHoisted, preVal,
    execInstrs, execInstr, MirValue.eval, State.update] at hEq
  exact h hEq

end RRProofs
