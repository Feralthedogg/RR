use crate::mir::opt::loop_analysis::{LoopInfo, build_pred_map};
use crate::mir::{BlockId, FnIR, Instr, Terminator};
use std::collections::BTreeSet;

use super::affine::{AffineConstraint, AffineConstraintKind, PresburgerSet, try_lift_affine_expr};
use super::{codegen_generic, poly_trace_enabled};

#[path = "scop/model_extract.rs"]
mod model_extract;
pub use self::model_extract::*;
#[path = "scop/nested_extract.rs"]
mod nested_extract;
pub(crate) use self::nested_extract::*;
