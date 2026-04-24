import RRProofs.PipelineFnCfgIterExecSubset

set_option linter.unusedVariables false
set_option linter.unusedSimpArgs false

namespace RRProofs

structure SrcCfgControlState where
  current : RValue
  remaining : List SrcPostJoinStep

structure MirCfgControlState where
  current : RValue
  remaining : List MirPostJoinStep

structure RCfgControlState where
  current : RValue
  remaining : List RPostJoinStep

structure SrcFnCfgControlProgram where
  joinExecProg : SrcFnCfgJoinExecProgram
  steps : List SrcPostJoinStep

structure MirFnCfgControlProgram where
  joinExecProg : MirFnCfgJoinExecProgram
  steps : List MirPostJoinStep

structure RFnCfgControlProgram where
  joinExecProg : RFnCfgJoinExecProgram
  steps : List RPostJoinStep

def lowerCfgControlState (s : SrcCfgControlState) : MirCfgControlState :=
  { current := s.current
  , remaining := s.remaining.map lowerPostJoinStep
  }

def emitRCfgControlState (s : MirCfgControlState) : RCfgControlState :=
  { current := s.current
  , remaining := s.remaining.map emitRPostJoinStep
  }

def stepSrcCfgControl : SrcCfgControlState → Option SrcCfgControlState
  | { current := current, remaining := [] } => none
  | { current := current, remaining := step :: rest } => do
      let next <- evalSrcPostJoinStep step current
      pure { current := next, remaining := rest }

def stepMirCfgControl : MirCfgControlState → Option MirCfgControlState
  | { current := current, remaining := [] } => none
  | { current := current, remaining := step :: rest } => do
      let next <- evalMirPostJoinStep step current
      pure { current := next, remaining := rest }

def stepRCfgControl : RCfgControlState → Option RCfgControlState
  | { current := current, remaining := [] } => none
  | { current := current, remaining := step :: rest } => do
      let next <- evalRPostJoinStep step current
      pure { current := next, remaining := rest }

def runSrcCfgControl : Nat → SrcCfgControlState → Option RValue
  | 0, st =>
      match st.remaining with
      | [] => some st.current
      | _ => none
  | fuel + 1, st =>
      match st.remaining with
      | [] => some st.current
      | _ =>
          match stepSrcCfgControl st with
          | some st' => runSrcCfgControl fuel st'
          | none => none

def runMirCfgControl : Nat → MirCfgControlState → Option RValue
  | 0, st =>
      match st.remaining with
      | [] => some st.current
      | _ => none
  | fuel + 1, st =>
      match st.remaining with
      | [] => some st.current
      | _ =>
          match stepMirCfgControl st with
          | some st' => runMirCfgControl fuel st'
          | none => none

def runRCfgControl : Nat → RCfgControlState → Option RValue
  | 0, st =>
      match st.remaining with
      | [] => some st.current
      | _ => none
  | fuel + 1, st =>
      match st.remaining with
      | [] => some st.current
      | _ =>
          match stepRCfgControl st with
          | some st' => runRCfgControl fuel st'
          | none => none

def lowerFnCfgControlProgram (p : SrcFnCfgControlProgram) : MirFnCfgControlProgram :=
  { joinExecProg := lowerFnCfgJoinExecProgram p.joinExecProg
  , steps := p.steps.map lowerPostJoinStep
  }

def emitRFnCfgControlProgram (p : MirFnCfgControlProgram) : RFnCfgControlProgram :=
  { joinExecProg := emitRFnCfgJoinExecProgram p.joinExecProg
  , steps := p.steps.map emitRPostJoinStep
  }

def toSrcFnCfgIterExecProgram (p : SrcFnCfgControlProgram) : SrcFnCfgIterExecProgram :=
  { joinExecProg := p.joinExecProg, steps := p.steps }

def toMirFnCfgIterExecProgram (p : MirFnCfgControlProgram) : MirFnCfgIterExecProgram :=
  { joinExecProg := p.joinExecProg, steps := p.steps }

def toRFnCfgIterExecProgram (p : RFnCfgControlProgram) : RFnCfgIterExecProgram :=
  { joinExecProg := p.joinExecProg, steps := p.steps }

def evalSrcFnCfgControlProgram
    (p : SrcFnCfgControlProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalSrcFnCfgJoinExecProgram p.joinExecProg choice
  runSrcCfgControl p.steps.length { current := joined, remaining := p.steps }

def evalMirFnCfgControlProgram
    (p : MirFnCfgControlProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalMirFnCfgJoinExecProgram p.joinExecProg choice
  runMirCfgControl p.steps.length { current := joined, remaining := p.steps }

def evalRFnCfgControlProgram
    (p : RFnCfgControlProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalRFnCfgJoinExecProgram p.joinExecProg choice
  runRCfgControl p.steps.length { current := joined, remaining := p.steps }

theorem runSrcCfgControl_eq_iter
    (steps : List SrcPostJoinStep) (current : RValue) :
    runSrcCfgControl steps.length { current := current, remaining := steps } =
      evalSrcPostJoinSteps steps current := by
  induction steps generalizing current with
  | nil =>
      rfl
  | cons step rest ih =>
      cases h : evalSrcPostJoinStep step current with
      | none =>
          simp [runSrcCfgControl, stepSrcCfgControl, evalSrcPostJoinSteps, h]
      | some next =>
          simp [runSrcCfgControl, stepSrcCfgControl, evalSrcPostJoinSteps, h, ih]

theorem runMirCfgControl_eq_iter
    (steps : List MirPostJoinStep) (current : RValue) :
    runMirCfgControl steps.length { current := current, remaining := steps } =
      evalMirPostJoinSteps steps current := by
  induction steps generalizing current with
  | nil =>
      rfl
  | cons step rest ih =>
      cases h : evalMirPostJoinStep step current with
      | none =>
          simp [runMirCfgControl, stepMirCfgControl, evalMirPostJoinSteps, h]
      | some next =>
          simp [runMirCfgControl, stepMirCfgControl, evalMirPostJoinSteps, h, ih]

theorem runRCfgControl_eq_iter
    (steps : List RPostJoinStep) (current : RValue) :
    runRCfgControl steps.length { current := current, remaining := steps } =
      evalRPostJoinSteps steps current := by
  induction steps generalizing current with
  | nil =>
      rfl
  | cons step rest ih =>
      cases h : evalRPostJoinStep step current with
      | none =>
          simp [runRCfgControl, stepRCfgControl, evalRPostJoinSteps, h]
      | some next =>
          simp [runRCfgControl, stepRCfgControl, evalRPostJoinSteps, h, ih]

theorem evalSrcFnCfgControlProgram_eq_iter
    (p : SrcFnCfgControlProgram) (choice : BranchChoice) :
    evalSrcFnCfgControlProgram p choice =
      evalSrcFnCfgIterExecProgram (toSrcFnCfgIterExecProgram p) choice := by
  unfold evalSrcFnCfgControlProgram evalSrcFnCfgIterExecProgram toSrcFnCfgIterExecProgram
  cases h : evalSrcFnCfgJoinExecProgram p.joinExecProg choice <;> simp [runSrcCfgControl_eq_iter, h]

theorem evalMirFnCfgControlProgram_eq_iter
    (p : MirFnCfgControlProgram) (choice : BranchChoice) :
    evalMirFnCfgControlProgram p choice =
      evalMirFnCfgIterExecProgram (toMirFnCfgIterExecProgram p) choice := by
  unfold evalMirFnCfgControlProgram evalMirFnCfgIterExecProgram toMirFnCfgIterExecProgram
  cases h : evalMirFnCfgJoinExecProgram p.joinExecProg choice <;> simp [runMirCfgControl_eq_iter, h]

theorem evalRFnCfgControlProgram_eq_iter
    (p : RFnCfgControlProgram) (choice : BranchChoice) :
    evalRFnCfgControlProgram p choice =
      evalRFnCfgIterExecProgram (toRFnCfgIterExecProgram p) choice := by
  unfold evalRFnCfgControlProgram evalRFnCfgIterExecProgram toRFnCfgIterExecProgram
  cases h : evalRFnCfgJoinExecProgram p.joinExecProg choice <;> simp [runRCfgControl_eq_iter, h]

theorem lowerFnCfgControlProgram_preserves_meta
    (p : SrcFnCfgControlProgram) :
    (lowerFnCfgControlProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (lowerFnCfgControlProgram p).steps.length = p.steps.length := by
  constructor
  · rfl
  · simp [lowerFnCfgControlProgram]

theorem emitRFnCfgControlProgram_preserves_meta
    (p : MirFnCfgControlProgram) :
    (emitRFnCfgControlProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (emitRFnCfgControlProgram p).steps.length = p.steps.length := by
  constructor
  · rfl
  · simp [emitRFnCfgControlProgram]

theorem lowerFnCfgControlProgram_preserves_eval
    (p : SrcFnCfgControlProgram) (choice : BranchChoice) :
    evalMirFnCfgControlProgram (lowerFnCfgControlProgram p) choice =
      evalSrcFnCfgControlProgram p choice := by
  rw [evalMirFnCfgControlProgram_eq_iter, evalSrcFnCfgControlProgram_eq_iter]
  simpa [toSrcFnCfgIterExecProgram, toMirFnCfgIterExecProgram, lowerFnCfgControlProgram,
    lowerFnCfgIterExecProgram] using
    (lowerFnCfgIterExecProgram_preserves_eval (toSrcFnCfgIterExecProgram p) choice)

theorem emitRFnCfgControlProgram_preserves_eval
    (p : MirFnCfgControlProgram) (choice : BranchChoice) :
    evalRFnCfgControlProgram (emitRFnCfgControlProgram p) choice =
      evalMirFnCfgControlProgram p choice := by
  rw [evalRFnCfgControlProgram_eq_iter, evalMirFnCfgControlProgram_eq_iter]
  simpa [toMirFnCfgIterExecProgram, toRFnCfgIterExecProgram, emitRFnCfgControlProgram,
    emitRFnCfgIterExecProgram] using
    (emitRFnCfgIterExecProgram_preserves_eval (toMirFnCfgIterExecProgram p) choice)

theorem lowerEmitFnCfgControlProgram_preserves_eval
    (p : SrcFnCfgControlProgram) (choice : BranchChoice) :
    evalRFnCfgControlProgram (emitRFnCfgControlProgram (lowerFnCfgControlProgram p)) choice =
      evalSrcFnCfgControlProgram p choice := by
  rw [emitRFnCfgControlProgram_preserves_eval, lowerFnCfgControlProgram_preserves_eval]

def branchingFnCfgControlProgram : SrcFnCfgControlProgram :=
  { joinExecProg := branchingFnCfgJoinExecProgram
  , steps := branchingFnCfgIterExecProgram.steps
  }

theorem branchingFnCfgControlProgram_meta_preserved :
    (lowerFnCfgControlProgram branchingFnCfgControlProgram).joinExecProg.phiProg.joinBid = 17 ∧
      (lowerFnCfgControlProgram branchingFnCfgControlProgram).steps.length = 2 := by
  constructor
  · rfl
  · rfl

theorem branchingFnCfgControlProgram_then_preserved :
    evalRFnCfgControlProgram
      (emitRFnCfgControlProgram (lowerFnCfgControlProgram branchingFnCfgControlProgram))
      .thenBranch = some (.int 28) := by
  rw [lowerEmitFnCfgControlProgram_preserves_eval, evalSrcFnCfgControlProgram_eq_iter]
  simpa [branchingFnCfgControlProgram, toSrcFnCfgIterExecProgram] using
    branchingFnCfgIterExecProgram_then_preserved

theorem branchingFnCfgControlProgram_else_preserved :
    evalRFnCfgControlProgram
      (emitRFnCfgControlProgram (lowerFnCfgControlProgram branchingFnCfgControlProgram))
      .elseBranch = some (.int 41) := by
  rw [lowerEmitFnCfgControlProgram_preserves_eval, evalSrcFnCfgControlProgram_eq_iter]
  simpa [branchingFnCfgControlProgram, toSrcFnCfgIterExecProgram] using
    branchingFnCfgIterExecProgram_else_preserved

end RRProofs
