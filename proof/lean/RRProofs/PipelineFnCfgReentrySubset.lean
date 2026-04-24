import RRProofs.PipelineFnCfgGraphStateSubset

set_option linter.unusedVariables false
set_option linter.unusedSimpArgs false

namespace RRProofs

structure SrcCfgReentryState where
  current : RValue
  trace : List Nat
  table : List SrcPostJoinStep

structure MirCfgReentryState where
  current : RValue
  trace : List Nat
  table : List MirPostJoinStep

structure RCfgReentryState where
  current : RValue
  trace : List Nat
  table : List RPostJoinStep

structure SrcFnCfgReentryProgram where
  joinExecProg : SrcFnCfgJoinExecProgram
  table : List SrcPostJoinStep
  trace : List Nat

structure MirFnCfgReentryProgram where
  joinExecProg : MirFnCfgJoinExecProgram
  table : List MirPostJoinStep
  trace : List Nat

structure RFnCfgReentryProgram where
  joinExecProg : RFnCfgJoinExecProgram
  table : List RPostJoinStep
  trace : List Nat

def nthStep? {α : Type} : List α → Nat → Option α
  | [], _ => none
  | x :: _, 0 => some x
  | _ :: xs, n + 1 => nthStep? xs n

theorem nthStep?_map {α β : Type} (f : α → β) (xs : List α) (idx : Nat) :
    nthStep? (xs.map f) idx = Option.map f (nthStep? xs idx) := by
  induction xs generalizing idx with
  | nil =>
      cases idx <;> rfl
  | cons x xs ih =>
      cases idx with
      | zero =>
          rfl
      | succ n =>
          simpa using ih n

def lowerCfgReentryState (s : SrcCfgReentryState) : MirCfgReentryState :=
  { current := s.current
  , trace := s.trace
  , table := s.table.map lowerPostJoinStep
  }

def emitRCfgReentryState (s : MirCfgReentryState) : RCfgReentryState :=
  { current := s.current
  , trace := s.trace
  , table := s.table.map emitRPostJoinStep
  }

def evalSrcReentryTrace : List SrcPostJoinStep → List Nat → RValue → Option RValue
  | _, [], current => some current
  | table, idx :: rest, current => do
      let step <- nthStep? table idx
      let next <- evalSrcPostJoinStep step current
      evalSrcReentryTrace table rest next

def evalMirReentryTrace : List MirPostJoinStep → List Nat → RValue → Option RValue
  | _, [], current => some current
  | table, idx :: rest, current => do
      let step <- nthStep? table idx
      let next <- evalMirPostJoinStep step current
      evalMirReentryTrace table rest next

def evalRReentryTrace : List RPostJoinStep → List Nat → RValue → Option RValue
  | _, [], current => some current
  | table, idx :: rest, current => do
      let step <- nthStep? table idx
      let next <- evalRPostJoinStep step current
      evalRReentryTrace table rest next

def stepSrcCfgReentry : SrcCfgReentryState → Option SrcCfgReentryState
  | { current := current, trace := [], table := table } => none
  | { current := current, trace := idx :: rest, table := table } => do
      let step <- nthStep? table idx
      let next <- evalSrcPostJoinStep step current
      pure { current := next, trace := rest, table := table }

def stepMirCfgReentry : MirCfgReentryState → Option MirCfgReentryState
  | { current := current, trace := [], table := table } => none
  | { current := current, trace := idx :: rest, table := table } => do
      let step <- nthStep? table idx
      let next <- evalMirPostJoinStep step current
      pure { current := next, trace := rest, table := table }

def stepRCfgReentry : RCfgReentryState → Option RCfgReentryState
  | { current := current, trace := [], table := table } => none
  | { current := current, trace := idx :: rest, table := table } => do
      let step <- nthStep? table idx
      let next <- evalRPostJoinStep step current
      pure { current := next, trace := rest, table := table }

def runSrcCfgReentry : Nat → SrcCfgReentryState → Option RValue
  | 0, st =>
      match st.trace with
      | [] => some st.current
      | _ => none
  | fuel + 1, st =>
      match st.trace with
      | [] => some st.current
      | _ =>
          match stepSrcCfgReentry st with
          | some st' => runSrcCfgReentry fuel st'
          | none => none

def runMirCfgReentry : Nat → MirCfgReentryState → Option RValue
  | 0, st =>
      match st.trace with
      | [] => some st.current
      | _ => none
  | fuel + 1, st =>
      match st.trace with
      | [] => some st.current
      | _ =>
          match stepMirCfgReentry st with
          | some st' => runMirCfgReentry fuel st'
          | none => none

def runRCfgReentry : Nat → RCfgReentryState → Option RValue
  | 0, st =>
      match st.trace with
      | [] => some st.current
      | _ => none
  | fuel + 1, st =>
      match st.trace with
      | [] => some st.current
      | _ =>
          match stepRCfgReentry st with
          | some st' => runRCfgReentry fuel st'
          | none => none

def lowerFnCfgReentryProgram (p : SrcFnCfgReentryProgram) : MirFnCfgReentryProgram :=
  { joinExecProg := lowerFnCfgJoinExecProgram p.joinExecProg
  , table := p.table.map lowerPostJoinStep
  , trace := p.trace
  }

def emitRFnCfgReentryProgram (p : MirFnCfgReentryProgram) : RFnCfgReentryProgram :=
  { joinExecProg := emitRFnCfgJoinExecProgram p.joinExecProg
  , table := p.table.map emitRPostJoinStep
  , trace := p.trace
  }

def toSrcFnCfgGraphProgram (p : SrcFnCfgReentryProgram) : SrcFnCfgGraphProgram :=
  { joinExecProg := p.joinExecProg, table := p.table }

def toMirFnCfgGraphProgram (p : MirFnCfgReentryProgram) : MirFnCfgGraphProgram :=
  { joinExecProg := p.joinExecProg, table := p.table }

def toRFnCfgGraphProgram (p : RFnCfgReentryProgram) : RFnCfgGraphProgram :=
  { joinExecProg := p.joinExecProg, table := p.table }

def evalSrcFnCfgReentryProgram
    (p : SrcFnCfgReentryProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalSrcFnCfgJoinExecProgram p.joinExecProg choice
  runSrcCfgReentry p.trace.length { current := joined, trace := p.trace, table := p.table }

def evalMirFnCfgReentryProgram
    (p : MirFnCfgReentryProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalMirFnCfgJoinExecProgram p.joinExecProg choice
  runMirCfgReentry p.trace.length { current := joined, trace := p.trace, table := p.table }

def evalRFnCfgReentryProgram
    (p : RFnCfgReentryProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalRFnCfgJoinExecProgram p.joinExecProg choice
  runRCfgReentry p.trace.length { current := joined, trace := p.trace, table := p.table }

theorem evalMirReentryTrace_preserves_eval
    (table : List SrcPostJoinStep) (trace : List Nat) (current : RValue) :
    evalMirReentryTrace (table.map lowerPostJoinStep) trace current =
      evalSrcReentryTrace table trace current := by
  have hStep :
      ∀ (step : SrcPostJoinStep) (current : RValue),
        evalMirPostJoinStep (lowerPostJoinStep step) current =
          evalSrcPostJoinStep step current := by
    intro step current
    simpa [evalMirPostJoinStep, evalSrcPostJoinStep, lowerPostJoinStep, lowerBlockEnvProgram] using
      (lowerBlockEnvProgram_preserves_eval
        { step.block with inEnv := (step.bindName, current) :: step.block.inEnv })
  induction trace generalizing current with
  | nil =>
      rfl
  | cons idx rest ih =>
      simp [evalMirReentryTrace, evalSrcReentryTrace, nthStep?_map]
      cases h : nthStep? table idx with
      | none =>
          simp [h]
      | some step =>
          simp [h]
          rw [hStep step current]
          cases hs : evalSrcPostJoinStep step current <;> simp [hs, ih]

theorem emitRReentryTrace_preserves_eval
    (table : List MirPostJoinStep) (trace : List Nat) (current : RValue) :
    evalRReentryTrace (table.map emitRPostJoinStep) trace current =
      evalMirReentryTrace table trace current := by
  have hStep :
      ∀ (step : MirPostJoinStep) (current : RValue),
        evalRPostJoinStep (emitRPostJoinStep step) current =
          evalMirPostJoinStep step current := by
    intro step current
    simpa [evalRPostJoinStep, evalMirPostJoinStep, emitRPostJoinStep, emitRBlockEnvProgram] using
      (emitRBlockEnvProgram_preserves_eval
        { step.block with inEnv := (step.bindName, current) :: step.block.inEnv })
  induction trace generalizing current with
  | nil =>
      rfl
  | cons idx rest ih =>
      simp [evalRReentryTrace, evalMirReentryTrace, nthStep?_map]
      cases h : nthStep? table idx with
      | none =>
          simp [h]
      | some step =>
          simp [h]
          rw [hStep step current]
          cases hs : evalMirPostJoinStep step current <;> simp [hs, ih]

theorem runSrcCfgReentry_eq_trace
    (trace : List Nat) (table : List SrcPostJoinStep) (current : RValue) :
    runSrcCfgReentry trace.length { current := current, trace := trace, table := table } =
      evalSrcReentryTrace table trace current := by
  induction trace generalizing current with
  | nil =>
      rfl
  | cons idx rest ih =>
      simp [runSrcCfgReentry, stepSrcCfgReentry, evalSrcReentryTrace]
      cases h : nthStep? table idx <;> simp [h]
      case some step =>
        cases hs : evalSrcPostJoinStep step current <;> simp [hs, ih]

theorem runMirCfgReentry_eq_trace
    (trace : List Nat) (table : List MirPostJoinStep) (current : RValue) :
    runMirCfgReentry trace.length { current := current, trace := trace, table := table } =
      evalMirReentryTrace table trace current := by
  induction trace generalizing current with
  | nil =>
      rfl
  | cons idx rest ih =>
      simp [runMirCfgReentry, stepMirCfgReentry, evalMirReentryTrace]
      cases h : nthStep? table idx <;> simp [h]
      case some step =>
        cases hs : evalMirPostJoinStep step current <;> simp [hs, ih]

theorem runRCfgReentry_eq_trace
    (trace : List Nat) (table : List RPostJoinStep) (current : RValue) :
    runRCfgReentry trace.length { current := current, trace := trace, table := table } =
      evalRReentryTrace table trace current := by
  induction trace generalizing current with
  | nil =>
      rfl
  | cons idx rest ih =>
      simp [runRCfgReentry, stepRCfgReentry, evalRReentryTrace]
      cases h : nthStep? table idx <;> simp [h]
      case some step =>
        cases hs : evalRPostJoinStep step current <;> simp [hs, ih]

theorem lowerFnCfgReentryProgram_preserves_meta
    (p : SrcFnCfgReentryProgram) :
    (lowerFnCfgReentryProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (lowerFnCfgReentryProgram p).table.length = p.table.length ∧
      (lowerFnCfgReentryProgram p).trace = p.trace := by
  constructor
  · rfl
  constructor
  · simp [lowerFnCfgReentryProgram]
  · rfl

theorem emitRFnCfgReentryProgram_preserves_meta
    (p : MirFnCfgReentryProgram) :
    (emitRFnCfgReentryProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (emitRFnCfgReentryProgram p).table.length = p.table.length ∧
      (emitRFnCfgReentryProgram p).trace = p.trace := by
  constructor
  · rfl
  constructor
  · simp [emitRFnCfgReentryProgram]
  · rfl

theorem lowerFnCfgReentryProgram_preserves_eval
    (p : SrcFnCfgReentryProgram) (choice : BranchChoice) :
    evalMirFnCfgReentryProgram (lowerFnCfgReentryProgram p) choice =
      evalSrcFnCfgReentryProgram p choice := by
  unfold evalMirFnCfgReentryProgram evalSrcFnCfgReentryProgram
  simp [lowerFnCfgReentryProgram]
  rw [lowerFnCfgJoinExecProgram_preserves_eval]
  cases h : evalSrcFnCfgJoinExecProgram p.joinExecProg choice <;> simp [runMirCfgReentry_eq_trace, runSrcCfgReentry_eq_trace]
  case some joined =>
    simpa using evalMirReentryTrace_preserves_eval p.table p.trace joined

theorem emitRFnCfgReentryProgram_preserves_eval
    (p : MirFnCfgReentryProgram) (choice : BranchChoice) :
    evalRFnCfgReentryProgram (emitRFnCfgReentryProgram p) choice =
      evalMirFnCfgReentryProgram p choice := by
  unfold evalRFnCfgReentryProgram evalMirFnCfgReentryProgram
  simp [emitRFnCfgReentryProgram]
  rw [emitRFnCfgJoinExecProgram_preserves_eval]
  cases h : evalMirFnCfgJoinExecProgram p.joinExecProg choice <;> simp [runRCfgReentry_eq_trace, runMirCfgReentry_eq_trace]
  case some joined =>
    simpa using emitRReentryTrace_preserves_eval p.table p.trace joined

theorem lowerEmitFnCfgReentryProgram_preserves_eval
    (p : SrcFnCfgReentryProgram) (choice : BranchChoice) :
    evalRFnCfgReentryProgram (emitRFnCfgReentryProgram (lowerFnCfgReentryProgram p)) choice =
      evalSrcFnCfgReentryProgram p choice := by
  rw [emitRFnCfgReentryProgram_preserves_eval, lowerFnCfgReentryProgram_preserves_eval]

def branchingFnCfgReentryProgram : SrcFnCfgReentryProgram :=
  { joinExecProg := branchingFnCfgJoinExecProgram
  , table := branchingFnCfgControlProgram.steps
  , trace := [0, 1, 0]
  }

theorem branchingFnCfgReentryProgram_meta_preserved :
    (lowerFnCfgReentryProgram branchingFnCfgReentryProgram).joinExecProg.phiProg.joinBid = 17 ∧
      (lowerFnCfgReentryProgram branchingFnCfgReentryProgram).table.length = 2 ∧
      (lowerFnCfgReentryProgram branchingFnCfgReentryProgram).trace = [0, 1, 0] := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem branchingFnCfgReentryProgram_then_preserved :
    evalRFnCfgReentryProgram
      (emitRFnCfgReentryProgram (lowerFnCfgReentryProgram branchingFnCfgReentryProgram))
      .thenBranch = some (.int 35) := by
  rw [lowerEmitFnCfgReentryProgram_preserves_eval]
  have hJoin :
      evalSrcFnCfgJoinExecProgram branchingFnCfgJoinExecProgram .thenBranch =
        some (.int 17) := by
    simp [branchingFnCfgJoinExecProgram, evalSrcFnCfgJoinExecProgram,
      evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]
    rw [branchingFnCfgPhiProgram_then_src]
    rfl
  simp [branchingFnCfgReentryProgram, evalSrcFnCfgReentryProgram]
  rw [hJoin]
  simp
  have hRun :
      runSrcCfgReentry 3
        { current := (.int 17), trace := [0, 1, 0], table := branchingFnCfgControlProgram.steps } =
          evalSrcReentryTrace branchingFnCfgControlProgram.steps [0, 1, 0] (.int 17) := by
    simpa using
      (runSrcCfgReentry_eq_trace [0, 1, 0] branchingFnCfgControlProgram.steps (.int 17))
  rw [hRun]
  rfl

theorem branchingFnCfgReentryProgram_else_preserved :
    evalRFnCfgReentryProgram
      (emitRFnCfgReentryProgram (lowerFnCfgReentryProgram branchingFnCfgReentryProgram))
      .elseBranch = some (.int 48) := by
  rw [lowerEmitFnCfgReentryProgram_preserves_eval]
  have hJoin :
      evalSrcFnCfgJoinExecProgram branchingFnCfgJoinExecProgram .elseBranch =
        some (.int 30) := by
    simp [branchingFnCfgJoinExecProgram, evalSrcFnCfgJoinExecProgram,
      evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]
    rw [branchingFnCfgPhiProgram_else_src]
    rfl
  simp [branchingFnCfgReentryProgram, evalSrcFnCfgReentryProgram]
  rw [hJoin]
  simp
  have hRun :
      runSrcCfgReentry 3
        { current := (.int 30), trace := [0, 1, 0], table := branchingFnCfgControlProgram.steps } =
          evalSrcReentryTrace branchingFnCfgControlProgram.steps [0, 1, 0] (.int 30) := by
    simpa using
      (runSrcCfgReentry_eq_trace [0, 1, 0] branchingFnCfgControlProgram.steps (.int 30))
  rw [hRun]
  rfl

end RRProofs
