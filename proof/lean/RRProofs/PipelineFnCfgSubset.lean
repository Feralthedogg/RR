import RRProofs.PipelineFnEnvSubset

namespace RRProofs

structure SrcFnCfgProgram where
  name : String
  entry : Nat
  bodyHead : Nat
  preds : Nat -> List Nat
  blocks : List SrcBlockEnvProgram

structure MirFnCfgProgram where
  name : String
  entry : Nat
  bodyHead : Nat
  preds : Nat -> List Nat
  blocks : List MirBlockEnvProgram

structure RFnCfgProgram where
  name : String
  entry : Nat
  bodyHead : Nat
  preds : Nat -> List Nat
  blocks : List RBlockEnvProgram

def lowerFnCfgProgram (p : SrcFnCfgProgram) : MirFnCfgProgram :=
  { name := p.name
  , entry := p.entry
  , bodyHead := p.bodyHead
  , preds := p.preds
  , blocks := lowerFnBlocks p.blocks
  }

def emitRFnCfgProgram (p : MirFnCfgProgram) : RFnCfgProgram :=
  { name := p.name
  , entry := p.entry
  , bodyHead := p.bodyHead
  , preds := p.preds
  , blocks := emitRFnBlocks p.blocks
  }

def evalSrcFnCfgProgram (p : SrcFnCfgProgram) : List FnBlockResult :=
  evalSrcFnBlocks p.blocks

def evalMirFnCfgProgram (p : MirFnCfgProgram) : List FnBlockResult :=
  evalMirFnBlocks p.blocks

def evalRFnCfgProgram (p : RFnCfgProgram) : List FnBlockResult :=
  evalRFnBlocks p.blocks

theorem lowerFnCfgProgram_preserves_meta
    (p : SrcFnCfgProgram) :
    (lowerFnCfgProgram p).name = p.name ∧
      (lowerFnCfgProgram p).entry = p.entry ∧
      (lowerFnCfgProgram p).bodyHead = p.bodyHead ∧
      (∀ bid, (lowerFnCfgProgram p).preds bid = p.preds bid) := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  · intro bid
    rfl

theorem emitRFnCfgProgram_preserves_meta
    (p : MirFnCfgProgram) :
    (emitRFnCfgProgram p).name = p.name ∧
      (emitRFnCfgProgram p).entry = p.entry ∧
      (emitRFnCfgProgram p).bodyHead = p.bodyHead ∧
      (∀ bid, (emitRFnCfgProgram p).preds bid = p.preds bid) := by
  constructor
  · rfl
  constructor
  · rfl
  constructor
  · rfl
  · intro bid
    rfl

theorem lowerFnCfgProgram_preserves_eval
    (p : SrcFnCfgProgram) :
    evalMirFnCfgProgram (lowerFnCfgProgram p) = evalSrcFnCfgProgram p := by
  cases p with
  | mk name entry bodyHead preds blocks =>
      simp [evalMirFnCfgProgram, evalSrcFnCfgProgram, lowerFnCfgProgram,
        lowerFnBlocks_preserves_eval]

theorem emitRFnCfgProgram_preserves_eval
    (p : MirFnCfgProgram) :
    evalRFnCfgProgram (emitRFnCfgProgram p) = evalMirFnCfgProgram p := by
  cases p with
  | mk name entry bodyHead preds blocks =>
      simp [evalRFnCfgProgram, evalMirFnCfgProgram, emitRFnCfgProgram,
        emitRFnBlocks_preserves_eval]

theorem lowerEmitFnCfgProgram_preserves_eval
    (p : SrcFnCfgProgram) :
    evalRFnCfgProgram (emitRFnCfgProgram (lowerFnCfgProgram p)) =
      evalSrcFnCfgProgram p := by
  rw [emitRFnCfgProgram_preserves_eval, lowerFnCfgProgram_preserves_eval]

def twoBlockFnCfgProgram : SrcFnCfgProgram :=
  { name := "toy_cfg_fn"
  , entry := 7
  , bodyHead := 11
  , preds := fun
      | 7 => []
      | 11 => [7]
      | _ => []
  , blocks := [incomingFieldBlockProgram, incomingBranchBlockProgram]
  }

theorem twoBlockFnCfgProgram_meta_preserved :
    (lowerFnCfgProgram twoBlockFnCfgProgram).name = "toy_cfg_fn" ∧
      (lowerFnCfgProgram twoBlockFnCfgProgram).entry = 7 ∧
      (lowerFnCfgProgram twoBlockFnCfgProgram).bodyHead = 11 ∧
      (lowerFnCfgProgram twoBlockFnCfgProgram).preds 11 = [7] ∧
      (emitRFnCfgProgram (lowerFnCfgProgram twoBlockFnCfgProgram)).preds 11 = [7] := by
  simp [twoBlockFnCfgProgram, lowerFnCfgProgram, emitRFnCfgProgram]

theorem twoBlockFnCfgProgram_preserved :
    evalRFnCfgProgram (emitRFnCfgProgram (lowerFnCfgProgram twoBlockFnCfgProgram)) =
      [(7, some (.int 7)), (11, some (.int 12))] := by
  rw [lowerEmitFnCfgProgram_preserves_eval]
  simp [twoBlockFnCfgProgram, evalSrcFnCfgProgram, evalSrcFnBlocks,
    incomingFieldBlockProgram, incomingBranchBlockProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

end RRProofs
