use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::mir::semantics::call_model::is_namespaced_r_call;
use crate::mir::*;
use rustc_hash::FxHashSet;
use std::fmt;

#[path = "verify/error.rs"]
mod error;
pub(crate) use self::error::*;
#[path = "verify/core.rs"]
mod core;
pub use self::core::verify_ir;
#[path = "verify/graph.rs"]
mod graph;
pub(crate) use self::graph::*;
#[path = "verify/flow.rs"]
mod flow;
pub(crate) use self::flow::*;
#[cfg(test)]
#[path = "verify/tests.rs"]
mod tests;
