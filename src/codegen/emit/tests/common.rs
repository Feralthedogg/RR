pub(crate) use crate::codegen::backend::state::{ActiveScalarLoopIndex, RBackend, ScalarLoopCmp};
pub(crate) use crate::mir::def::EscapeStatus;
pub(crate) use crate::mir::structurizer::StructuredBlock;
pub(crate) use crate::mir::{
    Facts, FnIR, Instr, IntrinsicOp, Lit, Terminator, UnaryOp, Value, ValueKind,
};
pub(crate) use crate::syntax::ast::BinOp;
pub(crate) use crate::typeck::{NaTy, PrimTy, ShapeTy, TypeState, TypeTerm};
pub(crate) use crate::utils::Span;
pub(crate) use rustc_hash::{FxHashMap, FxHashSet};

pub(crate) fn backend_with_sym17_fresh() -> RBackend {
    RBackend::with_fresh_result_calls(FxHashSet::from_iter([String::from("Sym_17")]))
}
