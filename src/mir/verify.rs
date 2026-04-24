use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::mir::semantics::call_model::is_namespaced_r_call;
use crate::mir::*;
use rustc_hash::FxHashSet;
use std::fmt;

include!("verify/error.rs");
include!("verify/core.rs");
include!("verify/graph.rs");
include!("verify/flow.rs");
include!("verify/tests.rs");
