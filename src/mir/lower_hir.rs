//! HIR-to-MIR lowering with sealed-SSA construction.
//!
//! This module turns structured HIR into the MIR form consumed by validation,
//! optimization, and codegen while preserving user-visible control/data flow.

use crate::error::{InternalCompilerError, RR, RRException, Stage};
use crate::hir::def as hir;
use crate::mir::flow::Facts;
use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit};
use crate::typeck::solver::{hir_ty_to_type_state, hir_ty_to_type_term_with_symbols};
use crate::utils::{Span, did_you_mean};
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "lower_hir/loops.rs"]
mod loops;
#[path = "lower_hir/matching.rs"]
mod matching;

#[derive(Clone, Copy)]
struct LoopTargets {
    break_bb: BlockId,
    continue_bb: BlockId,
    continue_step: Option<(hir::LocalId, ValueId)>,
}
pub struct MirLowerer<'a> {
    fn_ir: FnIR,

    // SSA construction state.
    curr_block: BlockId,

    // Current definitions per block (sealed SSA construction).
    defs: FxHashMap<BlockId, FxHashMap<hir::LocalId, ValueId>>,

    // Deferred phi operands for unsealed blocks.
    incomplete_phis: FxHashMap<BlockId, Vec<(hir::LocalId, ValueId)>>,
    sealed_blocks: FxHashSet<BlockId>,
    // Predecessor map for SSA reads.
    preds: FxHashMap<BlockId, Vec<BlockId>>,

    // Name mapping for codegen.
    var_names: FxHashMap<hir::LocalId, String>,

    // Symbol table (borrowed from caller).
    symbols: &'a FxHashMap<hir::SymbolId, String>,
    known_functions: &'a FxHashMap<String, usize>,
    loop_stack: Vec<LoopTargets>,
    tidy_mask_depth: usize,
}

include!("lower_hir/default_args.rs");
include!("lower_hir/construction.rs");
include!("lower_hir/lower_entry.rs");
include!("lower_hir/lower_stmt.rs");
include!("lower_hir/lower_expr.rs");
include!("lower_hir/value_helpers.rs");
include!("lower_hir/interop.rs");
include!("lower_hir/tests.rs");
