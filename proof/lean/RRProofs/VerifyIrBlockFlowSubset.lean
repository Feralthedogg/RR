import RRProofs.VerifyIrBlockRecordSubset
import RRProofs.VerifyIrFlowLite

namespace RRProofs

def lookupActualValueOriginVar
    (table : ActualValueFullTableLite) (root : ConsumerNodeId) : Option Var :=
  match lookupActualValueFullRow table root with
  | some row => row.originVar
  | none => none

def valueIdsToVars
    (table : ActualValueFullTableLite) (ids : List ConsumerNodeId) : List Var :=
  ids.filterMap (lookupActualValueOriginVar table)

def instrRecordReads
    (table : ActualValueFullTableLite) (instr : InstrRecordLite) : List Var :=
  match instr with
  | .assign _ src _ => valueIdsToVars table [src]
  | .eval val _ => valueIdsToVars table [val]
  | .storeIndex1D base idx val _ => valueIdsToVars table [base, idx, val]
  | .storeIndex2D base r c val _ => valueIdsToVars table [base, r, c, val]
  | .storeIndex3D base i j k val _ => valueIdsToVars table [base, i, j, k, val]

def instrRecordWrites (instr : InstrRecordLite) : List Var :=
  match instr with
  | .assign dst _ _ => [dst]
  | .eval .. => []
  | .storeIndex1D .. => []
  | .storeIndex2D .. => []
  | .storeIndex3D .. => []

def terminatorRecordReads
    (table : ActualValueFullTableLite) (term : TerminatorRecordLite) : List Var :=
  match term with
  | .goto _ => []
  | .branch cond _ _ => valueIdsToVars table [cond]
  | .ret (some val) => valueIdsToVars table [val]
  | .ret none => []
  | .unreachable => []

def missingVars (defined reads : List Var) : List Var :=
  reads.filter (fun v => !(v ∈ defined))

def stepInstrFlow
    (table : ActualValueFullTableLite)
    (state : List Var × List Var) (instr : InstrRecordLite) : List Var × List Var :=
  let defined := state.1
  let required := state.2
  let required' := required ++ missingVars defined (instrRecordReads table instr)
  let defined' := defined ++ instrRecordWrites instr
  (defined', required')

def blockRequiredVars
    (table : ActualValueFullTableLite)
    (initDefined : List Var) (bb : ActualBlockRecordLite) : List Var :=
  let state := bb.instrs.foldl (stepInstrFlow table) (initDefined, [])
  let defined := state.1
  let required := state.2
  required ++ missingVars defined (terminatorRecordReads table bb.term)

def flowCaseOfActualBlock
    (table : ActualValueFullTableLite)
    (initDefined : List Var) (bb : ActualBlockRecordLite) : FlowBlockCase :=
  { defined := initDefined
  , required := blockRequiredVars table initDefined bb
  }

def flowCasesOfActualBlocks
    (table : ActualValueFullTableLite)
    (initDefs : List (List Var))
    (blocks : List ActualBlockRecordLite) : List FlowBlockCase :=
  List.zipWith (flowCaseOfActualBlock table) initDefs blocks

def flowLiteCaseOfFnBlock
    (base : VerifyIrStructLiteCase)
    (fnBlock : FnBlockRecordLite)
    (initDefs : List (List Var)) : VerifyIrFlowLiteCase :=
  { base := base
  , blocks := flowCasesOfActualBlocks (fnBlockRecordToFnRecord fnBlock).values initDefs fnBlock.blocks
  }

def exampleBadActualBlock : ActualBlockRecordLite :=
  { id := 10
  , instrs := [.eval 3 .source]
  , term := .ret (some 4)
  }

def exampleGoodActualBlock : ActualBlockRecordLite :=
  { id := 11
  , instrs := [.assign "x" 4 .source, .eval 3 .source]
  , term := .ret (some 3)
  }

def exampleBadFnBlockRecord : FnBlockRecordLite :=
  { shell := exampleFnHintMapRecord
  , blocks := [exampleBadActualBlock]
  }

def exampleGoodFnBlockRecord : FnBlockRecordLite :=
  { shell := exampleFnHintMapRecord
  , blocks := [exampleGoodActualBlock]
  }

def exampleBadFlowLiteCase : VerifyIrFlowLiteCase :=
  flowLiteCaseOfFnBlock exampleFlowBase exampleBadFnBlockRecord [["y"]]

def exampleGoodFlowLiteCase : VerifyIrFlowLiteCase :=
  flowLiteCaseOfFnBlock exampleFlowBase exampleGoodFnBlockRecord [["y"]]

theorem exampleBadActualBlock_required :
    flowCaseOfActualBlock exampleActualValueFullTable ["y"] exampleBadActualBlock =
      { defined := ["y"], required := ["x"] } := by
  rfl

theorem exampleGoodActualBlock_required :
    flowCaseOfActualBlock exampleActualValueFullTable ["y"] exampleGoodActualBlock =
      { defined := ["y"], required := [] } := by
  rfl

theorem exampleBadFlowLiteCase_rejects :
    exampleBadFlowLiteCase.verifyIrFlowLite = some (.useBeforeDef "x") := by
  rfl

theorem exampleGoodFlowLiteCase_accepts :
    exampleGoodFlowLiteCase.verifyIrFlowLite = none := by
  rfl

end RRProofs
