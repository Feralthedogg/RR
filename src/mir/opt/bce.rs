use crate::mir::analyze::na::NaState;
use crate::mir::analyze::range::ensure_value_range;
use crate::mir::analyze::range::{RangeInterval, SymbolicBound};
use crate::mir::*;
use rustc_hash::FxHashSet;

#[path = "bce/loop_rules.rs"]
mod loop_rules;
pub(crate) use self::loop_rules::*;
#[path = "bce/safety_analysis.rs"]
mod safety_analysis;
pub(crate) use self::safety_analysis::*;
