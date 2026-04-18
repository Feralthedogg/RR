pub(super) use crate::codegen::mir_emit::{ActiveScalarLoopIndex, RBackend, ScalarLoopCmp};
pub(super) use crate::mir::def::{
    BinOp, EscapeStatus, FnIR, Instr, IntrinsicOp, Lit, Terminator, UnaryOp, Value, ValueKind,
};
pub(super) use crate::mir::flow::Facts;
pub(super) use crate::mir::structurizer::StructuredBlock;
pub(super) use crate::typeck::{NaTy, PrimTy, ShapeTy, TypeState, TypeTerm};
pub(super) use crate::utils::Span;
pub(super) use rustc_hash::{FxHashMap, FxHashSet};

pub(super) fn backend_with_sym17_fresh() -> RBackend {
    RBackend::with_fresh_result_calls(FxHashSet::from_iter([String::from("Sym_17")]))
}
