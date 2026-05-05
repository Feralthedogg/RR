use super::*;
use crate::syntax::ast::BinOp;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone)]
pub struct LoopInfo {
    pub header: BlockId,
    pub latch: BlockId,           // The block that jumps back to header
    pub exits: Vec<BlockId>,      // Blocks outside loop targeted by loop blocks
    pub body: FxHashSet<BlockId>, // All blocks in the loop

    pub is_seq_len: Option<ValueId>,   // If it's 1:N, stores N
    pub is_seq_along: Option<ValueId>, // If it's seq_along(X), stores X
    pub iv: Option<InductionVar>,
    pub limit: Option<ValueId>,
    pub limit_adjust: i64,
}

#[derive(Debug, Clone)]
pub struct InductionVar {
    pub phi_val: ValueId,
    pub init_val: ValueId,
    pub step: i64,      // +1, -1, etc.
    pub step_op: BinOp, // Add/Sub
}

pub struct LoopAnalyzer<'a> {
    pub(crate) fn_ir: &'a FnIR,
    pub(crate) preds: FxHashMap<BlockId, Vec<BlockId>>,
}
