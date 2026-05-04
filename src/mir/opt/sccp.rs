use crate::mir::*;
use crate::syntax::ast::Lit;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::VecDeque;

#[path = "sccp/lattice.rs"]
mod lattice;
pub(crate) use self::lattice::*;
#[path = "sccp/solver.rs"]
mod solver;
#[path = "sccp/user_kind.rs"]
mod user_kind;
#[path = "sccp/user_map.rs"]
mod user_map;
pub(crate) use self::user_kind::*;
