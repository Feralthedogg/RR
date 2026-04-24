namespace RRProofs

abbrev Var := String
abbrev State := Var -> Int
abbrev Update := Var × Int

inductive Expr where
  | const : Int -> Expr
  | var : Var -> Expr
  | add : Expr -> Expr -> Expr
deriving DecidableEq, Repr

def Expr.freeVars : Expr -> List Var
  | .const _ => []
  | .var x => [x]
  | .add lhs rhs => lhs.freeVars ++ rhs.freeVars

def Expr.eval (sigma : State) : Expr -> Int
  | .const n => n
  | .var x => sigma x
  | .add lhs rhs => lhs.eval sigma + rhs.eval sigma

def State.update (sigma : State) (x : Var) (v : Int) : State :=
  fun y => if y = x then v else sigma y

def State.updates (sigma : State) : List Update -> State
  | [] => sigma
  | (x, v) :: rest => (State.update sigma x v).updates rest

def writes (us : List Update) : List Var :=
  us.map Prod.fst

theorem mem_writes_of_mem
    {uv : Update}
    {us : List Update}
    (h : uv ∈ us) :
    uv.1 ∈ writes us := by
  induction h with
  | head =>
      simp [writes]
  | tail _ ih =>
      simp [writes]
      exact Or.inr ⟨uv.2, ih⟩

def licmHoistable (written : List Var) (e : Expr) : Prop :=
  ∀ x, x ∈ e.freeVars → x ∉ written

theorem eval_update_irrelevant
    (e : Expr)
    (sigma : State)
    (x : Var)
    (v : Int)
    (h : x ∉ e.freeVars) :
    e.eval (State.update sigma x v) = e.eval sigma := by
  induction e generalizing sigma with
  | const _ =>
      rfl
  | var y =>
      by_cases hEq : y = x
      · subst hEq
        exfalso
        exact h (by simp [Expr.freeVars])
      · simp [Expr.eval, State.update, hEq]
  | add lhs rhs ihL ihR =>
      have hL : x ∉ lhs.freeVars := by
        intro hx
        exact h (by simpa [Expr.freeVars] using List.mem_append.mpr (Or.inl hx))
      have hR : x ∉ rhs.freeVars := by
        intro hx
        exact h (by simpa [Expr.freeVars] using List.mem_append.mpr (Or.inr hx))
      simp [Expr.eval, ihL _ hL, ihR _ hR]

theorem eval_updates_irrelevant
    (e : Expr)
    (sigma : State)
    (us : List Update)
    (h : licmHoistable (writes us) e) :
    e.eval (sigma.updates us) = e.eval sigma := by
  induction us generalizing sigma with
  | nil =>
      rfl
  | cons uv rest ih =>
      have hHead : uv.1 ∉ e.freeVars := by
        intro hx
        have hw : uv.1 ∈ writes (uv :: rest) := by
          simp [writes]
        exact (h uv.1 hx) hw
      have hTail : licmHoistable (writes rest) e := by
        intro y hy hMem
        have hw : y ∈ writes (uv :: rest) := by
          simpa [writes] using Or.inr hMem
        exact (h y hy) hw
      calc
        e.eval ((State.update sigma uv.1 uv.2).updates rest)
            = e.eval (State.update sigma uv.1 uv.2) := by
                exact ih (sigma := State.update sigma uv.1 uv.2) hTail
        _ = e.eval sigma := by
              exact eval_update_irrelevant e sigma uv.1 uv.2 hHead

theorem time_plus_dt_not_hoistable (dt : Int) :
    ¬ licmHoistable ["time"] (Expr.add (.var "time") (.const dt)) := by
  intro h
  have hFree : "time" ∈ (Expr.add (.var "time") (.const dt)).freeVars := by
    simp [Expr.freeVars]
  have hNotWritten := h "time" hFree
  exact hNotWritten (by simp)

theorem time_plus_dt_not_invariant
    (sigma : State)
    (newTime : Int)
    (dt : Int)
    (h : newTime ≠ sigma "time") :
    (Expr.add (.var "time") (.const dt)).eval (State.update sigma "time" newTime) ≠
      (Expr.add (.var "time") (.const dt)).eval sigma := by
  intro hEq
  have : newTime = sigma "time" := by
    simpa [Expr.eval, State.update] using hEq
  exact h this

theorem disjoint_updates_make_loop_invariant
    (e : Expr)
    (sigma : State)
    (us : List Update)
    (hDisjoint : licmHoistable (writes us) e) :
    e.eval (sigma.updates us) = e.eval sigma := by
  exact eval_updates_irrelevant e sigma us hDisjoint

theorem updates_preserve_unwritten
    (sigma : State)
    (us : List Update)
    (x : Var)
    (h : x ∉ writes us) :
    (sigma.updates us) x = sigma x := by
  induction us generalizing sigma with
  | nil =>
      rfl
  | cons uv rest ih =>
      have hHead : x ≠ uv.1 := by
        intro hEq
        apply h
        simp [writes, hEq]
      have hTail : x ∉ writes rest := by
        intro hx
        apply h
        simpa [writes] using Or.inr hx
      calc
        ((State.update sigma uv.1 uv.2).updates rest) x
            = (State.update sigma uv.1 uv.2) x := by
                exact ih (sigma := State.update sigma uv.1 uv.2) hTail
        _ = sigma x := by
              simp [State.update, hHead]

theorem update_commute_distinct
    (sigma : State)
    (x y : Var)
    (vx vy : Int)
    (hxy : x ≠ y) :
    State.update (State.update sigma x vx) y vy =
      State.update (State.update sigma y vy) x vx := by
  funext z
  by_cases hzX : z = x
  · by_cases hzY : z = y
    · subst hzX
      subst hzY
      contradiction
    · subst hzX
      simp [State.update, hzY]
  · by_cases hzY : z = y
    · subst hzY
      simp [State.update, hzX]
    · simp [State.update, hzX, hzY]

theorem updates_commute_fresh_temp
    (sigma : State)
    (tmp : Var)
    (v : Int)
    (us : List Update)
    (h : tmp ∉ writes us) :
    (State.update sigma tmp v).updates us =
      State.update (sigma.updates us) tmp v := by
  induction us generalizing sigma with
  | nil =>
      rfl
  | cons uv rest ih =>
      have hHead : tmp ≠ uv.1 := by
        intro hEq
        apply h
        simp [writes, hEq]
      have hTail : tmp ∉ writes rest := by
        intro hx
        apply h
        simpa [writes] using Or.inr hx
      calc
        (State.update sigma tmp v).updates (uv :: rest)
            = (State.update (State.update sigma tmp v) uv.1 uv.2).updates rest := by
                rfl
        _ = (State.update (State.update sigma uv.1 uv.2) tmp v).updates rest := by
              rw [update_commute_distinct sigma tmp uv.1 v uv.2 hHead]
        _ = State.update ((State.update sigma uv.1 uv.2).updates rest) tmp v := by
              rw [ih (sigma := State.update sigma uv.1 uv.2) hTail]
        _ = State.update (sigma.updates (uv :: rest)) tmp v := by
              rfl

theorem licm_hoist_sound_for_concrete_updates
    (sigma : State)
    (tmp : Var)
    (e : Expr)
    (us : List Update)
    (hFresh : tmp ∉ writes us)
    (hInv : licmHoistable (writes us) e) :
    (State.update sigma tmp (e.eval sigma)).updates us =
      State.update (sigma.updates us) tmp (e.eval (sigma.updates us)) := by
  have hEval : e.eval (sigma.updates us) = e.eval sigma := by
    exact eval_updates_irrelevant e sigma us hInv
  calc
    (State.update sigma tmp (e.eval sigma)).updates us
        = State.update (sigma.updates us) tmp (e.eval sigma) := by
            exact updates_commute_fresh_temp sigma tmp (e.eval sigma) us hFresh
    _ = State.update (sigma.updates us) tmp (e.eval (sigma.updates us)) := by
          simp [hEval]

end RRProofs
