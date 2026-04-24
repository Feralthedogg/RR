import RRProofs.VerifyIrBlockAssignBranchSubset
import RRProofs.VerifyIrBlockAssignStoreSubset

namespace RRProofs

def scanDefinedVars (defined : DefSet) : List InstrRecordLite -> DefSet
  | [] => defined
  | instr :: instrs => scanDefinedVars (defined ++ instrRecordWrites instr) instrs

def finalDefinedVars (defined : DefSet) (bb : ActualBlockRecordLite) : DefSet :=
  scanDefinedVars defined bb.instrs

theorem stepInstrFlow_fst_eq_scanSeed
    (table : ActualValueFullTableLite)
    (defined : DefSet) (required : List Var) (instr : InstrRecordLite) :
    (stepInstrFlow table (defined, required) instr).1 = scanDefinedVars defined [instr] := by
  rfl

theorem foldStepInstrFlow_fst_eq_scanDefinedVars
    (table : ActualValueFullTableLite) :
    ∀ (instrs : List InstrRecordLite) (defined : DefSet) (required : List Var),
      (instrs.foldl (stepInstrFlow table) (defined, required)).1 = scanDefinedVars defined instrs
  | [], defined, required => rfl
  | instr :: instrs, defined, required => by
      simpa [scanDefinedVars, List.foldl] using
        foldStepInstrFlow_fst_eq_scanDefinedVars table instrs
          (defined ++ instrRecordWrites instr)
          ((stepInstrFlow table (defined, required) instr).2)

theorem finalDefinedVars_eq_foldStepInstrFlow
    (table : ActualValueFullTableLite) (defined : DefSet) (bb : ActualBlockRecordLite) :
    finalDefinedVars defined bb =
      (bb.instrs.foldl (stepInstrFlow table) (defined, [])).1 := by
  symm
  exact foldStepInstrFlow_fst_eq_scanDefinedVars table bb.instrs defined []

theorem mem_scanDefinedVars_of_mem_init
    {defined : DefSet} {instrs : List InstrRecordLite} {v : Var}
    (h : v ∈ defined) :
    v ∈ scanDefinedVars defined instrs := by
  induction instrs generalizing defined with
  | nil =>
      simpa [scanDefinedVars] using h
  | cons instr instrs ih =>
      have h' : v ∈ defined ++ instrRecordWrites instr := by
        simp [h]
      simpa [scanDefinedVars] using ih h'

theorem mem_finalDefinedVars_of_mem_init
    {defined : DefSet} {bb : ActualBlockRecordLite} {v : Var}
    (h : v ∈ defined) :
    v ∈ finalDefinedVars defined bb := by
  simpa [finalDefinedVars] using
    (mem_scanDefinedVars_of_mem_init (instrs := bb.instrs) h)

theorem exampleGoodActualBlock_finalDefinedVars :
    finalDefinedVars ["y"] exampleGoodActualBlock = ["y", "x"] := by
  rfl

theorem exampleAssignChainBlock_finalDefinedVars :
    finalDefinedVars ["y"] exampleAssignChainBlock = ["y", "loop", "x"] := by
  rfl

theorem exampleAssignBranchBlock_finalDefinedVars :
    finalDefinedVars ["y"] exampleAssignBranchBlock = ["y", "loop", "x"] := by
  rfl

theorem exampleAssignStore1DBlock_finalDefinedVars :
    finalDefinedVars ["y"] exampleAssignStore1DBlock = ["y", "loop", "x"] := by
  rfl

theorem exampleAssignStore2DBlock_finalDefinedVars :
    finalDefinedVars ["y"] exampleAssignStore2DBlock = ["y", "loop", "x"] := by
  rfl

theorem exampleAssignStore3DBlock_finalDefinedVars :
    finalDefinedVars ["y"] exampleAssignStore3DBlock = ["y", "loop", "x"] := by
  rfl

theorem exampleAssignChainBlock_preserves_incomingY :
    "y" ∈ finalDefinedVars ["y"] exampleAssignChainBlock := by
  exact mem_finalDefinedVars_of_mem_init (bb := exampleAssignChainBlock) (by simp)

end RRProofs
