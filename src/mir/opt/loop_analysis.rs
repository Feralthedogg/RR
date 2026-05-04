use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit};
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "loop_analysis/model.rs"]
mod model;
pub(crate) use self::model::*;
#[path = "loop_analysis/analysis.rs"]
mod analysis;
#[path = "loop_analysis/graph.rs"]
mod graph;
pub(crate) use self::graph::*;
