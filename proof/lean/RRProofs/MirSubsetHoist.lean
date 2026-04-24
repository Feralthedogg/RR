import RRProofs.LicmLoopCarried

set_option linter.unusedSimpArgs false

namespace RRProofs

abbrev Locals := State

inductive MirValue where
  | const : Int -> MirValue
  | localVal : Var -> MirValue
  | phi : Var -> Var -> MirValue
  | add : MirValue -> MirValue -> MirValue
deriving DecidableEq, Repr

def MirValue.localDeps : MirValue -> List Var
  | .const _ => []
  | .localVal x => [x]
  | .phi _ _ => []
  | .add lhs rhs => lhs.localDeps ++ rhs.localDeps

def MirValue.carriedDeps : MirValue -> List Var
  | .const _ => []
  | .localVal _ => []
  | .phi _ carried => [carried]
  | .add lhs rhs => lhs.carriedDeps ++ rhs.carriedDeps

def MirValue.eval (iter : Nat) (entry carried locals : State) : MirValue -> Int
  | .const n => n
  | .localVal x => locals x
  | .phi seed loopVar => if iter = 0 then entry seed else carried loopVar
  | .add lhs rhs => lhs.eval iter entry carried locals + rhs.eval iter entry carried locals

inductive MirInstr where
  | assign : Var -> MirValue -> MirInstr
deriving DecidableEq, Repr

def MirInstr.write : MirInstr -> Var
  | .assign dst _ => dst

def execInstr (iter : Nat) (entry carried locals : State) : MirInstr -> State
  | .assign dst rhs => State.update locals dst (rhs.eval iter entry carried locals)

def execInstrs (iter : Nat) (entry carried locals : State) : List MirInstr -> State
  | [] => locals
  | instr :: rest => execInstrs iter entry carried (execInstr iter entry carried locals instr) rest

def bodyWritesDisjoint (body : List MirInstr) (e : MirValue) : Prop :=
  ∀ instr, instr ∈ body -> instr.write ∉ e.localDeps

def hoistSafeOver (body : List MirInstr) (e : MirValue) : Prop :=
  e.carriedDeps = [] ∧ bodyWritesDisjoint body e

theorem mirvalue_eval_update_irrelevant_local
    (e : MirValue)
    (iter : Nat)
    (entry carried locals : State)
    (x : Var)
    (v : Int)
    (h : x ∉ e.localDeps) :
    e.eval iter entry carried (State.update locals x v) = e.eval iter entry carried locals := by
  induction e generalizing locals with
  | const _ =>
      rfl
  | localVal y =>
      simp [MirValue.localDeps] at h
      by_cases hEq : y = x
      · subst hEq
        exfalso
        exact h (by simp [MirValue.localDeps])
      · simp [MirValue.eval, State.update, hEq]
  | phi seed loopVar =>
      rfl
  | add lhs rhs ihL ihR =>
      simp [MirValue.localDeps, List.mem_append] at h
      rcases h with ⟨hL, hR⟩
      simp [MirValue.eval, ihL _ hL, ihR _ hR]

theorem mirvalue_eval_exec_irrelevant_body
    (e : MirValue)
    (iter : Nat)
    (entry carried locals : State)
    (body : List MirInstr)
    (h : bodyWritesDisjoint body e) :
    e.eval iter entry carried (execInstrs iter entry carried locals body) =
      e.eval iter entry carried locals := by
  induction body generalizing locals with
  | nil =>
      rfl
  | cons instr rest ih =>
      cases instr with
      | assign dst rhs =>
          simp [execInstrs, execInstr, MirInstr.write] at ih ⊢
          have hHead : dst ∉ e.localDeps := h (.assign dst rhs) (by simp)
          have hTail : bodyWritesDisjoint rest e := by
            intro instr hMem
            exact h instr (by simp [hMem])
          have h1 := mirvalue_eval_update_irrelevant_local e iter entry carried locals dst
            (rhs.eval iter entry carried locals) hHead
          have h2 := ih hTail (locals := State.update locals dst (rhs.eval iter entry carried locals))
          exact h2.trans h1

theorem phi_has_carried_dependency (seed carried : Var) :
    MirValue.carriedDeps (.phi seed carried) = [carried] := by
  rfl

theorem phi_not_safe_to_hoist_over_any_body
    (seed carried : Var)
    (body : List MirInstr) :
    ¬ hoistSafeOver body (.phi seed carried) := by
  simp [hoistSafeOver, MirValue.carriedDeps]

theorem hoist_sound_over_body
    (e : MirValue)
    (iter : Nat)
    (entry carried locals : State)
    (body : List MirInstr)
    (h : hoistSafeOver body e) :
    e.eval iter entry carried (execInstrs iter entry carried locals body) =
      e.eval iter entry carried locals := by
  rcases h with ⟨hCarried, hWrites⟩
  have hNoCarried : e.carriedDeps = [] := hCarried
  exact mirvalue_eval_exec_irrelevant_body e iter entry carried locals body hWrites

theorem phi_plus_local_not_hoistable
    (seed carried x : Var)
    (body : List MirInstr) :
    ¬ hoistSafeOver body (.add (.phi seed carried) (.localVal x)) := by
  simp [hoistSafeOver, MirValue.carriedDeps]

end RRProofs
