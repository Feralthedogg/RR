import RRProofs.PipelineBlockEnvSubset

namespace RRProofs

abbrev FnBlockResult := Nat × Option RValue

def evalSrcFnBlocks : List SrcBlockEnvProgram -> List FnBlockResult
  | [] => []
  | bb :: rest => (bb.bid, evalSrcBlockEnvProgram bb) :: evalSrcFnBlocks rest

def evalMirFnBlocks : List MirBlockEnvProgram -> List FnBlockResult
  | [] => []
  | bb :: rest => (bb.bid, evalMirBlockEnvProgram bb) :: evalMirFnBlocks rest

def evalRFnBlocks : List RBlockEnvProgram -> List FnBlockResult
  | [] => []
  | bb :: rest => (bb.bid, evalRBlockEnvProgram bb) :: evalRFnBlocks rest

def lowerFnBlocks : List SrcBlockEnvProgram -> List MirBlockEnvProgram
  | [] => []
  | bb :: rest => lowerBlockEnvProgram bb :: lowerFnBlocks rest

def emitRFnBlocks : List MirBlockEnvProgram -> List RBlockEnvProgram
  | [] => []
  | bb :: rest => emitRBlockEnvProgram bb :: emitRFnBlocks rest

theorem lowerFnBlocks_preserves_eval
    (blocks : List SrcBlockEnvProgram) :
    evalMirFnBlocks (lowerFnBlocks blocks) = evalSrcFnBlocks blocks := by
  induction blocks with
  | nil =>
      rfl
  | cons bb rest ih =>
      simp [lowerFnBlocks, evalMirFnBlocks, evalSrcFnBlocks,
        lowerBlockEnvProgram_preserves_block_id, lowerBlockEnvProgram_preserves_eval, ih]

theorem emitRFnBlocks_preserves_eval
    (blocks : List MirBlockEnvProgram) :
    evalRFnBlocks (emitRFnBlocks blocks) = evalMirFnBlocks blocks := by
  induction blocks with
  | nil =>
      rfl
  | cons bb rest ih =>
      simp [emitRFnBlocks, evalRFnBlocks, evalMirFnBlocks,
        emitRBlockEnvProgram_preserves_block_id, emitRBlockEnvProgram_preserves_eval, ih]

structure SrcFnEnvProgram where
  name : String
  entry : Nat
  bodyHead : Nat
  blocks : List SrcBlockEnvProgram
deriving Repr

structure MirFnEnvProgram where
  name : String
  entry : Nat
  bodyHead : Nat
  blocks : List MirBlockEnvProgram
deriving Repr

structure RFnEnvProgram where
  name : String
  entry : Nat
  bodyHead : Nat
  blocks : List RBlockEnvProgram
deriving Repr

def lowerFnEnvProgram (p : SrcFnEnvProgram) : MirFnEnvProgram :=
  { name := p.name
  , entry := p.entry
  , bodyHead := p.bodyHead
  , blocks := lowerFnBlocks p.blocks
  }

def emitRFnEnvProgram (p : MirFnEnvProgram) : RFnEnvProgram :=
  { name := p.name
  , entry := p.entry
  , bodyHead := p.bodyHead
  , blocks := emitRFnBlocks p.blocks
  }

def evalSrcFnEnvProgram (p : SrcFnEnvProgram) : List FnBlockResult :=
  evalSrcFnBlocks p.blocks

def evalMirFnEnvProgram (p : MirFnEnvProgram) : List FnBlockResult :=
  evalMirFnBlocks p.blocks

def evalRFnEnvProgram (p : RFnEnvProgram) : List FnBlockResult :=
  evalRFnBlocks p.blocks

theorem lowerFnEnvProgram_preserves_meta
    (p : SrcFnEnvProgram) :
    (lowerFnEnvProgram p).name = p.name ∧
      (lowerFnEnvProgram p).entry = p.entry ∧
      (lowerFnEnvProgram p).bodyHead = p.bodyHead := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem emitRFnEnvProgram_preserves_meta
    (p : MirFnEnvProgram) :
    (emitRFnEnvProgram p).name = p.name ∧
      (emitRFnEnvProgram p).entry = p.entry ∧
      (emitRFnEnvProgram p).bodyHead = p.bodyHead := by
  constructor
  · rfl
  constructor
  · rfl
  · rfl

theorem lowerFnEnvProgram_preserves_eval
    (p : SrcFnEnvProgram) :
    evalMirFnEnvProgram (lowerFnEnvProgram p) = evalSrcFnEnvProgram p := by
  cases p with
  | mk name entry bodyHead blocks =>
      simp [evalMirFnEnvProgram, evalSrcFnEnvProgram, lowerFnEnvProgram,
        lowerFnBlocks_preserves_eval]

theorem emitRFnEnvProgram_preserves_eval
    (p : MirFnEnvProgram) :
    evalRFnEnvProgram (emitRFnEnvProgram p) = evalMirFnEnvProgram p := by
  cases p with
  | mk name entry bodyHead blocks =>
      simp [evalRFnEnvProgram, evalMirFnEnvProgram, emitRFnEnvProgram,
        emitRFnBlocks_preserves_eval]

theorem lowerEmitFnEnvProgram_preserves_eval
    (p : SrcFnEnvProgram) :
    evalRFnEnvProgram (emitRFnEnvProgram (lowerFnEnvProgram p)) =
      evalSrcFnEnvProgram p := by
  rw [emitRFnEnvProgram_preserves_eval, lowerFnEnvProgram_preserves_eval]

def twoBlockFnEnvProgram : SrcFnEnvProgram :=
  { name := "toy_fn"
  , entry := 7
  , bodyHead := 11
  , blocks := [incomingFieldBlockProgram, incomingBranchBlockProgram]
  }

theorem twoBlockFnEnvProgram_meta_preserved :
    (lowerFnEnvProgram twoBlockFnEnvProgram).name = "toy_fn" ∧
      (lowerFnEnvProgram twoBlockFnEnvProgram).entry = 7 ∧
      (lowerFnEnvProgram twoBlockFnEnvProgram).bodyHead = 11 ∧
      (emitRFnEnvProgram (lowerFnEnvProgram twoBlockFnEnvProgram)).name = "toy_fn" := by
  simp [twoBlockFnEnvProgram, lowerFnEnvProgram, emitRFnEnvProgram]

theorem twoBlockFnEnvProgram_preserved :
    evalRFnEnvProgram (emitRFnEnvProgram (lowerFnEnvProgram twoBlockFnEnvProgram)) =
      [(7, some (.int 7)), (11, some (.int 12))] := by
  rw [lowerEmitFnEnvProgram_preserves_eval]
  simp [twoBlockFnEnvProgram, evalSrcFnEnvProgram, evalSrcFnBlocks,
    incomingFieldBlockProgram, incomingBranchBlockProgram,
    evalSrcBlockEnvProgram, execSrcStmts, execSrcStmt, evalSrc, evalSrcLet, lookupField]

end RRProofs
