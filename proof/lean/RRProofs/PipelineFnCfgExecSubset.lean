import RRProofs.PipelineFnCfgSubset

namespace RRProofs

def lookupFnBlockResult (results : List FnBlockResult) (bid : Nat) : Option RValue :=
  match results.find? (fun entry => entry.fst = bid) with
  | some (_, value) => value
  | none => none

def pathEdgesOk (preds : Nat -> List Nat) : List Nat -> Prop
  | [] => True
  | [_] => True
  | src :: dst :: rest => src ∈ preds dst ∧ pathEdgesOk preds (dst :: rest)

structure SrcFnCfgExecProgram where
  fnCfg : SrcFnCfgProgram
  blockOrder : List Nat
  execPath : List Nat

structure MirFnCfgExecProgram where
  fnCfg : MirFnCfgProgram
  blockOrder : List Nat
  execPath : List Nat

structure RFnCfgExecProgram where
  fnCfg : RFnCfgProgram
  blockOrder : List Nat
  execPath : List Nat

def lowerFnCfgExecProgram (p : SrcFnCfgExecProgram) : MirFnCfgExecProgram :=
  { fnCfg := lowerFnCfgProgram p.fnCfg
  , blockOrder := p.blockOrder
  , execPath := p.execPath
  }

def emitRFnCfgExecProgram (p : MirFnCfgExecProgram) : RFnCfgExecProgram :=
  { fnCfg := emitRFnCfgProgram p.fnCfg
  , blockOrder := p.blockOrder
  , execPath := p.execPath
  }

def evalSrcFnCfgExecProgram (p : SrcFnCfgExecProgram) : List FnBlockResult :=
  p.execPath.map (fun bid => (bid, lookupFnBlockResult (evalSrcFnCfgProgram p.fnCfg) bid))

def evalMirFnCfgExecProgram (p : MirFnCfgExecProgram) : List FnBlockResult :=
  p.execPath.map (fun bid => (bid, lookupFnBlockResult (evalMirFnCfgProgram p.fnCfg) bid))

def evalRFnCfgExecProgram (p : RFnCfgExecProgram) : List FnBlockResult :=
  p.execPath.map (fun bid => (bid, lookupFnBlockResult (evalRFnCfgProgram p.fnCfg) bid))

def pathStartsAtEntry (entry : Nat) : List Nat -> Prop
  | [] => True
  | bid :: _ => bid = entry

theorem lowerFnCfgExecProgram_preserves_meta
    (p : SrcFnCfgExecProgram) :
    (lowerFnCfgExecProgram p).fnCfg.name = p.fnCfg.name ∧
      (lowerFnCfgExecProgram p).fnCfg.entry = p.fnCfg.entry ∧
      (lowerFnCfgExecProgram p).fnCfg.bodyHead = p.fnCfg.bodyHead ∧
      (∀ bid, (lowerFnCfgExecProgram p).fnCfg.preds bid = p.fnCfg.preds bid) ∧
      (lowerFnCfgExecProgram p).blockOrder = p.blockOrder ∧
      (lowerFnCfgExecProgram p).execPath = p.execPath := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · intro bid
    rfl
  constructor
  · rfl
  · rfl

theorem emitRFnCfgExecProgram_preserves_meta
    (p : MirFnCfgExecProgram) :
    (emitRFnCfgExecProgram p).fnCfg.name = p.fnCfg.name ∧
      (emitRFnCfgExecProgram p).fnCfg.entry = p.fnCfg.entry ∧
      (emitRFnCfgExecProgram p).fnCfg.bodyHead = p.fnCfg.bodyHead ∧
      (∀ bid, (emitRFnCfgExecProgram p).fnCfg.preds bid = p.fnCfg.preds bid) ∧
      (emitRFnCfgExecProgram p).blockOrder = p.blockOrder ∧
      (emitRFnCfgExecProgram p).execPath = p.execPath := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · intro bid
    rfl
  constructor
  · rfl
  · rfl

theorem lowerFnCfgExecProgram_preserves_eval
    (p : SrcFnCfgExecProgram) :
    evalMirFnCfgExecProgram (lowerFnCfgExecProgram p) = evalSrcFnCfgExecProgram p := by
  cases p with
  | mk fnCfg blockOrder execPath =>
      induction execPath with
      | nil =>
          rfl
      | cons bid rest ih =>
          simp [evalMirFnCfgExecProgram, evalSrcFnCfgExecProgram, lowerFnCfgExecProgram]
          rw [lowerFnCfgProgram_preserves_eval]
          simp

theorem emitRFnCfgExecProgram_preserves_eval
    (p : MirFnCfgExecProgram) :
    evalRFnCfgExecProgram (emitRFnCfgExecProgram p) = evalMirFnCfgExecProgram p := by
  cases p with
  | mk fnCfg blockOrder execPath =>
      induction execPath with
      | nil =>
          rfl
      | cons bid rest ih =>
          simp [evalRFnCfgExecProgram, evalMirFnCfgExecProgram, emitRFnCfgExecProgram]
          rw [emitRFnCfgProgram_preserves_eval]
          simp

theorem lowerEmitFnCfgExecProgram_preserves_eval
    (p : SrcFnCfgExecProgram) :
    evalRFnCfgExecProgram (emitRFnCfgExecProgram (lowerFnCfgExecProgram p)) =
      evalSrcFnCfgExecProgram p := by
  rw [emitRFnCfgExecProgram_preserves_eval, lowerFnCfgExecProgram_preserves_eval]

def twoBlockFnCfgExecProgram : SrcFnCfgExecProgram :=
  { fnCfg := twoBlockFnCfgProgram
  , blockOrder := [7, 11]
  , execPath := [7, 11]
  }

theorem twoBlockFnCfgExecProgram_path_starts_at_entry :
    pathStartsAtEntry twoBlockFnCfgExecProgram.fnCfg.entry twoBlockFnCfgExecProgram.execPath := by
  simp [pathStartsAtEntry, twoBlockFnCfgExecProgram, twoBlockFnCfgProgram]

theorem twoBlockFnCfgExecProgram_path_edges_ok :
    pathEdgesOk twoBlockFnCfgExecProgram.fnCfg.preds twoBlockFnCfgExecProgram.execPath := by
  simp [pathEdgesOk, twoBlockFnCfgExecProgram, twoBlockFnCfgProgram]

theorem twoBlockFnCfgExecProgram_meta_preserved :
    (lowerFnCfgExecProgram twoBlockFnCfgExecProgram).fnCfg.name = "toy_cfg_fn" ∧
      (lowerFnCfgExecProgram twoBlockFnCfgExecProgram).fnCfg.entry = 7 ∧
      (lowerFnCfgExecProgram twoBlockFnCfgExecProgram).fnCfg.bodyHead = 11 ∧
      (lowerFnCfgExecProgram twoBlockFnCfgExecProgram).fnCfg.preds 11 = [7] ∧
      (emitRFnCfgExecProgram (lowerFnCfgExecProgram twoBlockFnCfgExecProgram)).execPath = [7, 11] := by
  simp [twoBlockFnCfgExecProgram, lowerFnCfgExecProgram, emitRFnCfgExecProgram,
    lowerFnCfgProgram, emitRFnCfgProgram, twoBlockFnCfgProgram]

theorem twoBlockFnCfgProgram_src_results :
    evalSrcFnCfgProgram twoBlockFnCfgProgram =
      [(7, some (.int 7)), (11, some (.int 12))] := by
  simp [twoBlockFnCfgProgram, evalSrcFnCfgProgram, evalSrcFnBlocks,
    incomingFieldBlockProgram, incomingBranchBlockProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

theorem twoBlockFnCfgExecProgram_preserved :
    evalRFnCfgExecProgram
      (emitRFnCfgExecProgram (lowerFnCfgExecProgram twoBlockFnCfgExecProgram)) =
      [(7, some (.int 7)), (11, some (.int 12))] := by
  rw [lowerEmitFnCfgExecProgram_preserves_eval]
  simp [evalSrcFnCfgExecProgram, twoBlockFnCfgExecProgram]
  rw [twoBlockFnCfgProgram_src_results]
  simp [lookupFnBlockResult]

end RRProofs
