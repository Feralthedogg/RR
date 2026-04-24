import RRProofs.PipelineFnCfgControlStateSubset

namespace RRProofs

structure SrcCfgGraphState where
  current : RValue
  pc : Nat
  table : List SrcPostJoinStep

structure MirCfgGraphState where
  current : RValue
  pc : Nat
  table : List MirPostJoinStep

structure RCfgGraphState where
  current : RValue
  pc : Nat
  table : List RPostJoinStep

structure SrcFnCfgGraphProgram where
  joinExecProg : SrcFnCfgJoinExecProgram
  table : List SrcPostJoinStep

structure MirFnCfgGraphProgram where
  joinExecProg : MirFnCfgJoinExecProgram
  table : List MirPostJoinStep

structure RFnCfgGraphProgram where
  joinExecProg : RFnCfgJoinExecProgram
  table : List RPostJoinStep

def lowerCfgGraphState (s : SrcCfgGraphState) : MirCfgGraphState :=
  { current := s.current
  , pc := s.pc
  , table := s.table.map lowerPostJoinStep
  }

def emitRCfgGraphState (s : MirCfgGraphState) : RCfgGraphState :=
  { current := s.current
  , pc := s.pc
  , table := s.table.map emitRPostJoinStep
  }

def toSrcCfgControlState (s : SrcCfgGraphState) : SrcCfgControlState :=
  { current := s.current
  , remaining := s.table.drop s.pc
  }

def toMirCfgControlState (s : MirCfgGraphState) : MirCfgControlState :=
  { current := s.current
  , remaining := s.table.drop s.pc
  }

def toRCfgControlState (s : RCfgGraphState) : RCfgControlState :=
  { current := s.current
  , remaining := s.table.drop s.pc
  }

def stepSrcCfgGraph (s : SrcCfgGraphState) : Option SrcCfgGraphState := do
  let _ <- stepSrcCfgControl (toSrcCfgControlState s)
  pure { current := (← evalSrcPostJoinSteps (s.table.drop s.pc) s.current)
       , pc := s.pc.succ
       , table := s.table }

def stepMirCfgGraph (s : MirCfgGraphState) : Option MirCfgGraphState := do
  let _ <- stepMirCfgControl (toMirCfgControlState s)
  pure { current := (← evalMirPostJoinSteps (s.table.drop s.pc) s.current)
       , pc := s.pc.succ
       , table := s.table }

def stepRCfgGraph (s : RCfgGraphState) : Option RCfgGraphState := do
  let _ <- stepRCfgControl (toRCfgControlState s)
  pure { current := (← evalRPostJoinSteps (s.table.drop s.pc) s.current)
       , pc := s.pc.succ
       , table := s.table }

def runSrcCfgGraph (fuel : Nat) (s : SrcCfgGraphState) : Option RValue :=
  runSrcCfgControl fuel (toSrcCfgControlState s)

def runMirCfgGraph (fuel : Nat) (s : MirCfgGraphState) : Option RValue :=
  runMirCfgControl fuel (toMirCfgControlState s)

def runRCfgGraph (fuel : Nat) (s : RCfgGraphState) : Option RValue :=
  runRCfgControl fuel (toRCfgControlState s)

def lowerFnCfgGraphProgram (p : SrcFnCfgGraphProgram) : MirFnCfgGraphProgram :=
  { joinExecProg := lowerFnCfgJoinExecProgram p.joinExecProg
  , table := p.table.map lowerPostJoinStep
  }

def emitRFnCfgGraphProgram (p : MirFnCfgGraphProgram) : RFnCfgGraphProgram :=
  { joinExecProg := emitRFnCfgJoinExecProgram p.joinExecProg
  , table := p.table.map emitRPostJoinStep
  }

def toSrcFnCfgControlProgram (p : SrcFnCfgGraphProgram) : SrcFnCfgControlProgram :=
  { joinExecProg := p.joinExecProg, steps := p.table }

def toMirFnCfgControlProgram (p : MirFnCfgGraphProgram) : MirFnCfgControlProgram :=
  { joinExecProg := p.joinExecProg, steps := p.table }

def toRFnCfgControlProgram (p : RFnCfgGraphProgram) : RFnCfgControlProgram :=
  { joinExecProg := p.joinExecProg, steps := p.table }

def evalSrcFnCfgGraphProgram
    (p : SrcFnCfgGraphProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalSrcFnCfgJoinExecProgram p.joinExecProg choice
  runSrcCfgGraph p.table.length { current := joined, pc := 0, table := p.table }

def evalMirFnCfgGraphProgram
    (p : MirFnCfgGraphProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalMirFnCfgJoinExecProgram p.joinExecProg choice
  runMirCfgGraph p.table.length { current := joined, pc := 0, table := p.table }

def evalRFnCfgGraphProgram
    (p : RFnCfgGraphProgram) (choice : BranchChoice) : Option RValue := do
  let joined <- evalRFnCfgJoinExecProgram p.joinExecProg choice
  runRCfgGraph p.table.length { current := joined, pc := 0, table := p.table }

theorem runSrcCfgGraph_eq_control
    (fuel : Nat) (s : SrcCfgGraphState) :
    runSrcCfgGraph fuel s = runSrcCfgControl fuel (toSrcCfgControlState s) := by
  rfl

theorem runMirCfgGraph_eq_control
    (fuel : Nat) (s : MirCfgGraphState) :
    runMirCfgGraph fuel s = runMirCfgControl fuel (toMirCfgControlState s) := by
  rfl

theorem runRCfgGraph_eq_control
    (fuel : Nat) (s : RCfgGraphState) :
    runRCfgGraph fuel s = runRCfgControl fuel (toRCfgControlState s) := by
  rfl

theorem evalSrcFnCfgGraphProgram_eq_control
    (p : SrcFnCfgGraphProgram) (choice : BranchChoice) :
    evalSrcFnCfgGraphProgram p choice =
      evalSrcFnCfgControlProgram (toSrcFnCfgControlProgram p) choice := by
  simp [evalSrcFnCfgGraphProgram, evalSrcFnCfgControlProgram, runSrcCfgGraph,
    toSrcCfgControlState, toSrcFnCfgControlProgram]

theorem evalMirFnCfgGraphProgram_eq_control
    (p : MirFnCfgGraphProgram) (choice : BranchChoice) :
    evalMirFnCfgGraphProgram p choice =
      evalMirFnCfgControlProgram (toMirFnCfgControlProgram p) choice := by
  simp [evalMirFnCfgGraphProgram, evalMirFnCfgControlProgram, runMirCfgGraph,
    toMirCfgControlState, toMirFnCfgControlProgram]

theorem evalRFnCfgGraphProgram_eq_control
    (p : RFnCfgGraphProgram) (choice : BranchChoice) :
    evalRFnCfgGraphProgram p choice =
      evalRFnCfgControlProgram (toRFnCfgControlProgram p) choice := by
  simp [evalRFnCfgGraphProgram, evalRFnCfgControlProgram, runRCfgGraph,
    toRCfgControlState, toRFnCfgControlProgram]

theorem lowerFnCfgGraphProgram_preserves_meta
    (p : SrcFnCfgGraphProgram) :
    (lowerFnCfgGraphProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (lowerFnCfgGraphProgram p).table.length = p.table.length := by
  constructor
  · rfl
  · simp [lowerFnCfgGraphProgram]

theorem emitRFnCfgGraphProgram_preserves_meta
    (p : MirFnCfgGraphProgram) :
    (emitRFnCfgGraphProgram p).joinExecProg.phiProg.joinBid = p.joinExecProg.phiProg.joinBid ∧
      (emitRFnCfgGraphProgram p).table.length = p.table.length := by
  constructor
  · rfl
  · simp [emitRFnCfgGraphProgram]

theorem lowerFnCfgGraphProgram_preserves_eval
    (p : SrcFnCfgGraphProgram) (choice : BranchChoice) :
    evalMirFnCfgGraphProgram (lowerFnCfgGraphProgram p) choice =
      evalSrcFnCfgGraphProgram p choice := by
  rw [evalMirFnCfgGraphProgram_eq_control, evalSrcFnCfgGraphProgram_eq_control]
  simpa [toSrcFnCfgControlProgram, toMirFnCfgControlProgram, lowerFnCfgGraphProgram,
    lowerFnCfgControlProgram] using
    (lowerFnCfgControlProgram_preserves_eval (toSrcFnCfgControlProgram p) choice)

theorem emitRFnCfgGraphProgram_preserves_eval
    (p : MirFnCfgGraphProgram) (choice : BranchChoice) :
    evalRFnCfgGraphProgram (emitRFnCfgGraphProgram p) choice =
      evalMirFnCfgGraphProgram p choice := by
  rw [evalRFnCfgGraphProgram_eq_control, evalMirFnCfgGraphProgram_eq_control]
  simpa [toMirFnCfgControlProgram, toRFnCfgControlProgram, emitRFnCfgGraphProgram,
    emitRFnCfgControlProgram] using
    (emitRFnCfgControlProgram_preserves_eval (toMirFnCfgControlProgram p) choice)

theorem lowerEmitFnCfgGraphProgram_preserves_eval
    (p : SrcFnCfgGraphProgram) (choice : BranchChoice) :
    evalRFnCfgGraphProgram (emitRFnCfgGraphProgram (lowerFnCfgGraphProgram p)) choice =
      evalSrcFnCfgGraphProgram p choice := by
  rw [emitRFnCfgGraphProgram_preserves_eval, lowerFnCfgGraphProgram_preserves_eval]

def branchingFnCfgGraphProgram : SrcFnCfgGraphProgram :=
  { joinExecProg := branchingFnCfgJoinExecProgram
  , table := branchingFnCfgControlProgram.steps
  }

theorem branchingFnCfgGraphProgram_meta_preserved :
    (lowerFnCfgGraphProgram branchingFnCfgGraphProgram).joinExecProg.phiProg.joinBid = 17 ∧
      (lowerFnCfgGraphProgram branchingFnCfgGraphProgram).table.length = 2 := by
  constructor
  · rfl
  · rfl

theorem branchingFnCfgGraphProgram_then_preserved :
    evalRFnCfgGraphProgram
      (emitRFnCfgGraphProgram (lowerFnCfgGraphProgram branchingFnCfgGraphProgram))
      .thenBranch = some (.int 28) := by
  rw [lowerEmitFnCfgGraphProgram_preserves_eval, evalSrcFnCfgGraphProgram_eq_control]
  simpa [branchingFnCfgGraphProgram, toSrcFnCfgControlProgram] using
    branchingFnCfgControlProgram_then_preserved

theorem branchingFnCfgGraphProgram_else_preserved :
    evalRFnCfgGraphProgram
      (emitRFnCfgGraphProgram (lowerFnCfgGraphProgram branchingFnCfgGraphProgram))
      .elseBranch = some (.int 41) := by
  rw [lowerEmitFnCfgGraphProgram_preserves_eval, evalSrcFnCfgGraphProgram_eq_control]
  simpa [branchingFnCfgGraphProgram, toSrcFnCfgControlProgram] using
    branchingFnCfgControlProgram_else_preserved

end RRProofs
