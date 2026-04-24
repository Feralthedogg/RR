import RRProofs.MirSubsetHoist

set_option linter.unnecessarySimpa false

namespace RRProofs

structure MiniHoistCase where
  tmp : Var
  cand : MirValue
  body : List MirInstr

def safeToHoistCase (c : MiniHoistCase) : Prop :=
  c.cand.carriedDeps = [] ∧
  (∀ instr, instr ∈ c.body -> instr.write ∉ c.cand.localDeps ∧ instr.write ≠ c.tmp) ∧
  c.tmp ∉ c.cand.localDeps

def originalNextHeaderValue (c : MiniHoistCase) (entry locals : State) : Int :=
  let post := execInstrs 1 entry locals locals c.body
  c.cand.eval 1 entry post post

def hoistedValueAfterBody (c : MiniHoistCase) (entry locals : State) : Int :=
  let hoisted := c.cand.eval 1 entry locals locals
  let post := execInstrs 1 entry locals (State.update locals c.tmp hoisted) c.body
  post c.tmp

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
        | nil =>
            simpa using hLhs
        | cons hd tl =>
            simp [MirValue.carriedDeps, hLhs] at h
      have hR : rhs.carriedDeps = [] := by
        cases hRhs : rhs.carriedDeps with
        | nil =>
            simpa using hRhs
        | cons hd tl =>
            simp [MirValue.carriedDeps, hRhs] at h
      simp [MirValue.eval, ihL _ hL, ihR _ hR]

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
          have hRest :
              ∀ instr, instr ∈ rest -> instr.write ≠ x := by
                intro instr hMem
                exact h instr (by simp [hMem])
          have hRec :=
            ih hRest (locals := State.update locals dst (rhs.eval iter entry carried locals))
          have hHead' : x ≠ dst := by
            intro hEq
            exact hHead hEq.symm
          simp [State.update, hHead'] at hRec
          exact hRec

theorem hoist_safe_case_sound
    (c : MiniHoistCase)
    (entry locals : State)
    (h : safeToHoistCase c) :
    originalNextHeaderValue c entry locals = hoistedValueAfterBody c entry locals := by
  rcases h with ⟨hCarried, hWrites, hTmpFresh⟩
  dsimp [originalNextHeaderValue, hoistedValueAfterBody]
  let post := execInstrs 1 entry locals locals c.body
  let preVal := c.cand.eval 1 entry locals locals
  have hCarry :
      c.cand.eval 1 entry post post = c.cand.eval 1 entry locals post := by
    apply mirvalue_eval_irrelevant_carried
    exact hCarried
  have hLocalWrites : bodyWritesDisjoint c.body c.cand := by
    intro instr hMem
    exact (hWrites instr hMem).1
  have hLocal :
      c.cand.eval 1 entry locals post = c.cand.eval 1 entry locals locals := by
    simpa [post] using
      mirvalue_eval_exec_irrelevant_body c.cand 1 entry locals locals c.body hLocalWrites
  have hTmpWrites : ∀ instr, instr ∈ c.body -> instr.write ≠ c.tmp := by
    intro instr hMem
    exact (hWrites instr hMem).2
  have hTmp :
      (execInstrs 1 entry locals (State.update locals c.tmp preVal) c.body) c.tmp = preVal := by
    have hTmp' := execInstrs_preserve_unwritten_var 1 entry locals
      (State.update locals c.tmp preVal) c.body c.tmp hTmpWrites
    simpa [State.update] using hTmp'
  calc
    c.cand.eval 1 entry post post = c.cand.eval 1 entry locals post := hCarry
    _ = c.cand.eval 1 entry locals locals := hLocal
    _ = preVal := rfl
    _ = (execInstrs 1 entry locals (State.update locals c.tmp preVal) c.body) c.tmp := by
          symm
          exact hTmp

def timeBumpBody : List MirInstr :=
  [MirInstr.assign "time" (MirValue.add (.localVal "time") (.const 1))]

def phiTimeCase : MiniHoistCase :=
  { tmp := "licm_time", cand := .phi "time0" "time", body := timeBumpBody }

theorem phi_time_case_not_safe : ¬ safeToHoistCase phiTimeCase := by
  simp [phiTimeCase, safeToHoistCase, timeBumpBody, MirValue.carriedDeps]

theorem phi_time_case_unsound
    (entry locals : State)
    (h : locals "time" + 1 ≠ locals "time") :
    originalNextHeaderValue phiTimeCase entry locals ≠
      hoistedValueAfterBody phiTimeCase entry locals := by
  intro hEq
  simp [phiTimeCase, timeBumpBody, originalNextHeaderValue, hoistedValueAfterBody,
    execInstrs, execInstr, MirValue.eval, State.update] at hEq
  exact h hEq

end RRProofs
