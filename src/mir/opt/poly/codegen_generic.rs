//! Generic polyhedral MIR reconstruction helpers.
//!
//! These routines rebuild structured loop nests directly from affine schedule
//! information when the specialized poly codegen path is unavailable or not a
//! good fit for the selected schedule shape.

use super::ScopRegion;
use super::access::MemoryLayout;
use super::affine::{AffineExpr, AffineSymbol};
use super::schedule::{SchedulePlan, SchedulePlanKind};
use super::scop::{PolyStmt, PolyStmtKind};
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::opt::v_opt::ReduceKind;
use crate::mir::opt::v_opt::vector_apply_site;
use crate::mir::{Facts, FnIR, Instr, Lit, Terminator, ValueId, ValueKind};
use crate::syntax::ast::BinOp;
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};
include!("codegen_generic/config.rs");
include!("codegen_generic/entrypoints.rs");
include!("codegen_generic/fission.rs");
include!("codegen_generic/lowering.rs");
include!("codegen_generic/compat.rs");
include!("codegen_generic/loop_nests.rs");
include!("codegen_generic/body_emit.rs");
include!("codegen_generic/clone_value.rs");
