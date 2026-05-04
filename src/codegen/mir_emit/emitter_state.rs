// MIR-to-R emission coordinator.
//
// The heavy lifting is delegated to `codegen/emit/*` and `codegen/backend/*`.
// This file keeps the entrypoints, shared regex helpers, and wiring that tie
// those child modules into a single deterministic emission pipeline.

use crate::codegen::backend::state::{
    ActiveScalarLoopIndex, BranchSnapshot, LastAssignedValueUndo, RBackend, ScalarLoopCmp,
    ValueBindingUndo, VarValueBindingUndo, VarVersionUndo,
};
use crate::mir::def::{
    BinOp, BlockId, FnIR, Instr, IntrinsicOp, Lit, Terminator, UnaryOp, Value, ValueKind,
    value_dependencies,
};
use crate::mir::flow::Facts;
use crate::mir::opt::poly::is_generated_poly_loop_var_name;
use crate::mir::structurizer::StructuredBlock;
use crate::typeck::{PrimTy, ShapeTy, TypeTerm};
use crate::utils::Span;
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};

pub(crate) const IDENT_PATTERN: &str = r"(?:[A-Za-z_][A-Za-z0-9._]*|\.[A-Za-z_][A-Za-z0-9._]*)";
pub(crate) const GENERATED_POLY_LOOP_IV_PREFIX: &str = ".__poly_gen_iv_";

#[path = "../emit/assign.rs"]
pub(crate) mod assign;
#[path = "../emit/bindings.rs"]
pub(crate) mod bindings_emit;
#[path = "../emit/branches.rs"]
pub(crate) mod branches_emit;
#[path = "../emit/cse.rs"]
pub(crate) mod cse_emit;
#[path = "../emit/cse_prune.rs"]
pub(crate) mod cse_prune_emit;
#[path = "../emit/index.rs"]
pub(crate) mod index_emit;
#[path = "../emit/instr.rs"]
pub(crate) mod instr_emit;
#[path = "../emit/render.rs"]
pub(crate) mod render_emit;
#[path = "../emit/resolve.rs"]
pub(crate) mod resolve_emit;
#[path = "../emit/rewrite.rs"]
pub(crate) mod rewrite_emit;
#[path = "../emit/structured_analysis.rs"]
pub(crate) mod structured_analysis_emit;
#[path = "../emit/structured.rs"]
pub(crate) mod structured_emit;

pub(crate) fn compile_regex(pattern: String) -> Option<Regex> {
    Regex::new(&pattern).ok()
}

pub struct MirEmitter {
    pub(crate) backend: RBackend,
}

pub(crate) fn is_recognized_loop_index_name(name: &str) -> bool {
    matches!(name, "i" | "j" | "k")
        || name.starts_with("i_")
        || name.starts_with("j_")
        || name.starts_with("k_")
        || is_generated_poly_loop_var_name(name)
}
