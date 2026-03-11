use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use crate::typeck::{PrimTy, ShapeTy};
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;

mod analysis;
mod api;
mod debug;
mod planning;
mod reconstruct;
mod transform;

use analysis::*;
use api::*;
pub use api::{VOptStats, optimize, optimize_with_stats, optimize_with_stats_with_whitelist};
use debug::*;
pub(crate) use planning::is_builtin_vector_safe_call;
use planning::*;
use reconstruct::*;
use transform::*;
