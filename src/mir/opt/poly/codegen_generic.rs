//! Generic polyhedral MIR reconstruction helpers.
//!
//! These routines rebuild structured loop nests directly from affine schedule
//! information when the specialized poly codegen path is unavailable or not a
//! good fit for the selected schedule shape.

use super::access::MemoryLayout;
use super::affine::{AffineExpr, AffineSymbol};
use super::schedule::{SchedulePlan, SchedulePlanKind};
use super::scop::{PolyStmt, PolyStmtKind};
use super::{LoopDimension, ScopRegion, access, poly_trace_enabled};
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::opt::v_opt::ReduceKind;
use crate::mir::opt::v_opt::vector_apply_site;
use crate::mir::{Facts, FnIR, Instr, Lit, Terminator, ValueId, ValueKind};
use crate::syntax::ast::BinOp;
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};
#[path = "codegen_generic/config.rs"]
mod config;
pub(crate) use self::config::*;
#[path = "codegen_generic/entrypoints.rs"]
mod entrypoints;
pub(crate) use self::entrypoints::*;
#[path = "codegen_generic/fission.rs"]
mod fission;
pub(crate) use self::fission::*;
#[path = "codegen_generic/lowering.rs"]
mod lowering;
pub(crate) use self::lowering::*;
#[path = "codegen_generic/compat.rs"]
mod compat;
pub(crate) use self::compat::*;
#[path = "codegen_generic/loop_nests.rs"]
mod loop_nests;
pub(crate) use self::loop_nests::*;
#[path = "codegen_generic/body_emit.rs"]
mod body_emit;
pub(crate) use self::body_emit::*;
#[path = "codegen_generic/clone_value.rs"]
mod clone_value;
pub(crate) use self::clone_value::*;
