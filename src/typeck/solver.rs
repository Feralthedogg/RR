use crate::compiler::scheduler::{
    CompilerParallelConfig, CompilerParallelStage, CompilerScheduler,
};
use crate::diagnostic::{DiagnosticBuilder, finish_diagnostics};
use crate::error::{InternalCompilerError, RR, RRCode, RRException, Stage};
use crate::hir::def::Ty;
use crate::mir::{FnIR, Instr, Terminator, ValueId, ValueKind};
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

use super::builtin_sigs::{
    infer_builtin, infer_builtin_term, infer_package_binding, infer_package_binding_term,
    infer_package_call, infer_package_call_term,
};
use super::constraints::{ConstraintSet, TypeConstraint};
use super::lattice::{LenSym, NaTy, PrimTy, ShapeTy, TypeState};
use super::term::{
    TypeTerm, from_hir_ty as term_from_hir_ty,
    from_hir_ty_with_symbols as term_from_hir_ty_with_symbols, from_lit as lit_term,
};

#[path = "solver/index_demands.rs"]
mod index_demands;
#[path = "solver/terms.rs"]
mod terms;

include!("solver/config.rs");
include!("solver/program.rs");
include!("solver/validation.rs");
include!("solver/function.rs");
include!("solver/index_returns.rs");
include!("solver/terms_bridge.rs");
include!("solver/value_inference.rs");
include!("solver/public_helpers.rs");
include!("solver/tests.rs");
