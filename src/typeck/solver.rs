use crate::compiler::scheduler::{
    CompilerParallelConfig, CompilerParallelStage, CompilerScheduler,
};
use crate::diagnostic::{DiagnosticBuilder, finish_diagnostics};
use crate::error::{InternalCompilerError, RR, RRCode, RRException, Stage};
use crate::hir::def::Ty;
use crate::mir::{Block, FnIR, Instr, Terminator, Value, ValueId, ValueKind};
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

#[path = "solver/config.rs"]
mod config;
pub use self::config::*;
#[path = "solver/program.rs"]
mod program;
pub use self::program::{analyze_program, analyze_program_with_compiler_parallel};
#[path = "solver/validation.rs"]
mod validation;
pub(crate) use self::validation::*;
#[path = "solver/na_refine.rs"]
mod na_refine;
pub(crate) use self::na_refine::*;
#[path = "solver/function.rs"]
mod function;
pub(crate) use self::function::*;
#[path = "solver/index_returns.rs"]
mod index_returns;
pub(crate) use self::index_returns::*;
#[path = "solver/terms_bridge.rs"]
mod terms_bridge;
pub(crate) use self::terms_bridge::*;
#[path = "solver/value_inference.rs"]
mod value_inference;
pub(crate) use self::value_inference::*;
#[path = "solver/public_helpers.rs"]
mod public_helpers;
pub(crate) use self::public_helpers::*;
pub use self::public_helpers::{
    hir_ty_to_type_state, hir_ty_to_type_term, hir_ty_to_type_term_with_symbols,
};
#[cfg(test)]
#[path = "solver/tests.rs"]
mod tests;
