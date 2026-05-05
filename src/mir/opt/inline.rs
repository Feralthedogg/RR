use crate::mir::flow::Facts;
use crate::mir::*;
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

#[path = "inline/state.rs"]
mod state;
pub(crate) use self::state::*;
#[path = "inline/driver.rs"]
mod driver;
#[path = "inline/policy.rs"]
mod policy;
#[path = "inline/remap.rs"]
mod remap;
pub(crate) use self::policy::*;
