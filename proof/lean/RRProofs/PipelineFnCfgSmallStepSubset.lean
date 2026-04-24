import RRProofs.PipelineFnCfgExecSubset

set_option linter.unusedSimpArgs false

namespace RRProofs

structure TraceMachine where
  cursor : Nat
  trace : List FnBlockResult
deriving Repr

def TraceMachine.initial : TraceMachine :=
  { cursor := 0, trace := [] }

def stepTraceMachine (results : List FnBlockResult) (path : List Nat)
    (m : TraceMachine) : TraceMachine :=
  match (path.drop m.cursor).head? with
  | some bid =>
      { cursor := m.cursor + 1
      , trace := m.trace ++ [(bid, lookupFnBlockResult results bid)]
      }
  | none => m

def runTraceMachine (results : List FnBlockResult) (path : List Nat) :
    Nat -> TraceMachine -> TraceMachine
  | 0, m => m
  | n + 1, m => runTraceMachine results path n (stepTraceMachine results path m)

def runSrcFnCfgMachine (p : SrcFnCfgExecProgram) : TraceMachine :=
  runTraceMachine (evalSrcFnCfgExecProgram p) p.execPath p.execPath.length TraceMachine.initial

def runMirFnCfgMachine (p : MirFnCfgExecProgram) : TraceMachine :=
  runTraceMachine (evalMirFnCfgExecProgram p) p.execPath p.execPath.length TraceMachine.initial

def runRFnCfgMachine (p : RFnCfgExecProgram) : TraceMachine :=
  runTraceMachine (evalRFnCfgExecProgram p) p.execPath p.execPath.length TraceMachine.initial

theorem runTraceMachine_eq_of_results_eq
    (path : List Nat) {results₁ results₂ : List FnBlockResult}
    (hResults : results₁ = results₂) :
    ∀ fuel m, runTraceMachine results₁ path fuel m = runTraceMachine results₂ path fuel m
  | 0, m => by
      simp [runTraceMachine]
  | fuel + 1, m => by
      simp [runTraceMachine]
      subst hResults
      rfl

theorem lowerFnCfgExecProgram_preserves_machine
    (p : SrcFnCfgExecProgram) :
    runMirFnCfgMachine (lowerFnCfgExecProgram p) = runSrcFnCfgMachine p := by
  unfold runMirFnCfgMachine runSrcFnCfgMachine
  exact runTraceMachine_eq_of_results_eq p.execPath
    (lowerFnCfgExecProgram_preserves_eval p) p.execPath.length TraceMachine.initial

theorem emitRFnCfgExecProgram_preserves_machine
    (p : MirFnCfgExecProgram) :
    runRFnCfgMachine (emitRFnCfgExecProgram p) = runMirFnCfgMachine p := by
  unfold runRFnCfgMachine runMirFnCfgMachine
  exact runTraceMachine_eq_of_results_eq p.execPath
    (emitRFnCfgExecProgram_preserves_eval p) p.execPath.length TraceMachine.initial

theorem lowerEmitFnCfgExecProgram_preserves_machine
    (p : SrcFnCfgExecProgram) :
    runRFnCfgMachine (emitRFnCfgExecProgram (lowerFnCfgExecProgram p)) = runSrcFnCfgMachine p := by
  rw [emitRFnCfgExecProgram_preserves_machine, lowerFnCfgExecProgram_preserves_machine]

theorem twoBlockFnCfgExecProgram_small_step_preserved :
    (runRFnCfgMachine (emitRFnCfgExecProgram (lowerFnCfgExecProgram twoBlockFnCfgExecProgram))).trace =
      [(7, some (.int 7)), (11, some (.int 12))] := by
  rw [lowerEmitFnCfgExecProgram_preserves_machine]
  simp [runSrcFnCfgMachine, runTraceMachine, stepTraceMachine, TraceMachine.initial,
    twoBlockFnCfgExecProgram, evalSrcFnCfgExecProgram, lookupFnBlockResult]
  rw [twoBlockFnCfgProgram_src_results]
  simp [lookupFnBlockResult]

end RRProofs
